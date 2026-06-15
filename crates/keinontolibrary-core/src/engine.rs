//! The declension [`Engine`]: provider traits plus the lookup → overlay → rule-fallback
//! orchestration that backs `decline`/`paradigm`.

use std::collections::HashMap;
use std::fmt;

use crate::case::{Case, Number};
use crate::error::Error;
use crate::forms::{Forms, Paradigm};
use crate::normalize::normalize;
use crate::paradigm_ref::ParadigmRef;

/// A source of attested forms: the precomputed lookup artifact, or the runtime overlay.
///
/// All lemmas passed in are already normalized (see [`normalize`]).
pub trait FormStore: fmt::Debug + Send + Sync {
    /// Candidate paradigms this store knows for `lemma`. Empty means "not present here".
    fn paradigms(&self, lemma: &str) -> Vec<ParadigmRef>;

    /// Forms for one slot of one paradigm.
    ///
    /// Returns `Some(Forms { status: Missing, .. })` for a slot known to be *defective*,
    /// and `None` for a slot simply *absent* from this store (which lets the rule engine
    /// fill it).
    fn forms(
        &self,
        lemma: &str,
        reference: &ParadigmRef,
        number: Number,
        case: Case,
    ) -> Option<Forms>;
}

/// The rule-based fallback generator (implemented by `keinontolibrary-rules`).
pub trait Generator: fmt::Debug + Send + Sync {
    /// Generate forms for a slot from the paradigm's declension class, if possible.
    fn generate(
        &self,
        lemma: &str,
        reference: &ParadigmRef,
        number: Number,
        case: Case,
    ) -> Option<Forms>;

    /// Infer a paradigm for an *unlisted* lemma from productive morphology — e.g. any
    /// `-nen` word inflects as Kotus tn38, any `-uus/-yys` abstract as tn40. Used only as a
    /// last resort, after lookup and compound splitting fail, to reach the ~72k tn-less
    /// Kotus rows (and beyond). Returns `None` when no confident inference applies.
    fn infer(&self, _lemma: &str) -> Option<ParadigmRef> {
        None
    }
}

/// A head lemma reached via its plural-nominative surface — the value of the plural-head
/// reverse index. Carries the lemma string and its paradigm so a plurale-tantum compound
/// (`ajovalot` = `ajo` + plural of `valo`) can be declined on the head in the plural.
#[derive(Debug, Clone)]
pub struct PluralHead {
    /// The head lemma (singular base), e.g. `valo` for the tail `valot`.
    pub lemma: String,
    /// Its paradigm.
    pub reference: ParadigmRef,
}

/// The declension engine: holds the providers and runs the resolution pipeline.
pub struct Engine {
    lookup: Box<dyn FormStore>,
    overlay: Option<Box<dyn FormStore>>,
    generator: Option<Box<dyn Generator>>,
    /// Reverse index `plural-nominative surface -> head lemma`, for plural-head compounds
    /// (`ajovalot`, `aluevaalit`). Empty unless populated at construction (`build_engine`).
    plural_index: HashMap<String, PluralHead>,
}

impl fmt::Debug for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Engine")
            .field("lookup", &self.lookup)
            .field("has_overlay", &self.overlay.is_some())
            .field("has_generator", &self.generator.is_some())
            .field("plural_index_len", &self.plural_index.len())
            .finish()
    }
}

impl Engine {
    /// Start building an engine.
    pub fn builder() -> EngineBuilder {
        EngineBuilder::default()
    }

    /// An engine that knows nothing — every query returns [`Error::UnknownWord`]. Used as
    /// the default before a real data-backed engine is installed.
    pub fn empty() -> Self {
        Self {
            lookup: Box::new(EmptyStore),
            overlay: None,
            generator: None,
            plural_index: HashMap::new(),
        }
    }

    /// Decline `lemma` into one slot, erroring if the lemma is ambiguous.
    pub fn decline(&self, lemma: &str, number: Number, case: Case) -> Result<Forms, Error> {
        let norm = normalize(lemma);
        let refs = self.resolve(&norm);
        match refs.as_slice() {
            // Unknown as a whole: it may be a compound whose final component is known, or
            // (last resort) a productive form whose class we can infer (-nen -> tn38).
            [] => self
                .compound_slot(&norm, number, case)
                .or_else(|| self.plural_head_slot(&norm, number, case))
                .or_else(|| self.inferred_slot(&norm, number, case))
                .ok_or(Error::UnknownWord(norm)),
            // A tn51 compound: both parts inflect (isoveli -> isoissaveljissä). Falls back to
            // the head-only reading (the accepted tn50 variant), then the normal path.
            [only] if only.tn == COMPOUND_BOTH_TN => self
                .compound_both_slot(&norm, number, case)
                .or_else(|| self.compound_slot(&norm, number, case))
                .map_or_else(|| self.resolve_slot(&norm, only, number, case), Ok),
            // A Kotus-listed compound (tn50): decline on the final component, modifier frozen
            // (so harmony follows the head). Falls through if it can't be segmented.
            [only] if only.tn == COMPOUND_TN => self
                .compound_slot(&norm, number, case)
                .map_or_else(|| self.resolve_slot(&norm, only, number, case), Ok),
            // Compound ordinals (kahdeskymmenes, tn45) inflect BOTH parts:
            // kahdennenkymmenennen. Falls back to the direct path (registry/lookup)
            // for simple ordinals and slots the parts cannot fill.
            [only] if only.tn == 45 => self
                .ordinal_both_slot(&norm, number, case)
                .map_or_else(|| self.resolve_slot(&norm, only, number, case), Ok),
            // Known word, but a known compound whose final component flips harmony
            // (punaviini -> punaviiniä, not -nia): override with the component's harmony.
            [only] => match self.compound_harmony_slot(&norm, number, case) {
                Some(forms) => Ok(forms),
                None => self.resolve_slot(&norm, only, number, case),
            },
            _ => Err(Error::Ambiguous {
                lemma: norm,
                paradigms: refs,
            }),
        }
    }

    /// Decline `lemma` using an explicit paradigm (to resolve homonyms).
    ///
    /// The `paradigm` is matched against the known paradigms by `(hn, tn)`; a `None` `hn`
    /// matches on `tn` alone.
    pub fn decline_with(
        &self,
        lemma: &str,
        number: Number,
        case: Case,
        paradigm: &ParadigmRef,
    ) -> Result<Forms, Error> {
        let norm = normalize(lemma);
        let chosen = self.choose(&norm, paradigm)?;
        self.resolve_slot(&norm, &chosen, number, case)
    }

    /// Build the full paradigm table for `lemma`, erroring if ambiguous.
    pub fn paradigm(&self, lemma: &str) -> Result<Paradigm, Error> {
        let norm = normalize(lemma);
        let refs = self.resolve(&norm);
        match refs.as_slice() {
            [] => self
                .compound_paradigm(&norm)
                .or_else(|| self.plural_head_paradigm(&norm))
                .or_else(|| self.inferred_paradigm(&norm))
                .ok_or(Error::UnknownWord(norm)),
            [only] if only.tn == COMPOUND_BOTH_TN => Ok(self
                .compound_both_paradigm(&norm)
                .or_else(|| self.compound_paradigm(&norm))
                .unwrap_or_else(|| self.build_paradigm(&norm, only))),
            [only] if only.tn == COMPOUND_TN => Ok(self
                .compound_paradigm(&norm)
                .unwrap_or_else(|| self.build_paradigm(&norm, only))),
            // Compound ordinals: both parts inflect per slot (kahdennenkymmenennen).
            [only] if only.tn == 45 => {
                let reference = only.clone();
                Ok(Paradigm::build(norm.clone(), reference.clone(), |n, c| {
                    self.ordinal_both_slot(&norm, n, c)
                        .or_else(|| self.slot(&norm, &reference, n, c))
                        .unwrap_or_else(Forms::missing)
                }))
            }
            [only] => Ok(self
                .compound_harmony_paradigm(&norm)
                .unwrap_or_else(|| self.build_paradigm(&norm, only))),
            _ => Err(Error::Ambiguous {
                lemma: norm,
                paradigms: refs,
            }),
        }
    }

    /// Build the full paradigm table for an explicit paradigm of `lemma`.
    pub fn paradigm_with(&self, lemma: &str, paradigm: &ParadigmRef) -> Result<Paradigm, Error> {
        let norm = normalize(lemma);
        let chosen = self.choose(&norm, paradigm)?;
        Ok(self.build_paradigm(&norm, &chosen))
    }

    /// The candidate paradigms for a normalized lemma: the union of overlay and lookup,
    /// deduplicated by `(hn, tn)` with overlay entries taking precedence.
    pub fn resolve(&self, normalized_lemma: &str) -> Vec<ParadigmRef> {
        let mut out: Vec<ParadigmRef> = Vec::new();
        if let Some(overlay) = &self.overlay {
            merge_refs(&mut out, overlay.paradigms(normalized_lemma));
        }
        merge_refs(&mut out, self.lookup.paradigms(normalized_lemma));
        out
    }

    /// Pick the known paradigm matching a user-supplied reference.
    fn choose(&self, norm: &str, wanted: &ParadigmRef) -> Result<ParadigmRef, Error> {
        let refs = self.resolve(norm);
        if refs.is_empty() {
            return Err(Error::UnknownWord(norm.to_owned()));
        }
        refs.into_iter()
            .find(|r| r.matches(wanted.hn, Some(wanted.tn)))
            // A known word without the requested paradigm: treat as unknown for that key.
            .ok_or_else(|| Error::UnknownWord(norm.to_owned()))
    }

    /// Resolve a single slot through overlay → lookup → generator, mapping a defective
    /// slot to [`Error::DefectiveForm`].
    fn resolve_slot(
        &self,
        norm: &str,
        reference: &ParadigmRef,
        number: Number,
        case: Case,
    ) -> Result<Forms, Error> {
        match self.slot(norm, reference, number, case) {
            Some(forms) if !forms.is_missing() => Ok(forms),
            // Known but defective, or no form obtainable at all.
            _ => Err(Error::DefectiveForm {
                lemma: norm.to_owned(),
                number,
                case,
            }),
        }
    }

    /// The raw slot value from the provider stack (no error mapping).
    fn slot(
        &self,
        norm: &str,
        reference: &ParadigmRef,
        number: Number,
        case: Case,
    ) -> Option<Forms> {
        self.overlay
            .as_ref()
            .and_then(|o| o.forms(norm, reference, number, case))
            .or_else(|| self.lookup.forms(norm, reference, number, case))
            .or_else(|| {
                self.generator
                    .as_ref()
                    .and_then(|g| g.generate(norm, reference, number, case))
            })
    }

    /// Last-resort slot from an inferred class (the generator's [`Generator::infer`]):
    /// for an unlisted lemma whose morphology is productive (`-nen` -> tn38). `None` if no
    /// class is inferred or the inferred class yields nothing for this slot.
    fn inferred_slot(&self, norm: &str, number: Number, case: Case) -> Option<Forms> {
        let reference = self.generator.as_ref()?.infer(norm)?;
        self.slot(norm, &reference, number, case)
            .filter(|f| !f.is_missing())
    }

    /// The full paradigm from an inferred class. `None` if no class is inferred or the
    /// inferred class produces an empty paradigm (so the caller still reports unknown).
    fn inferred_paradigm(&self, norm: &str) -> Option<Paradigm> {
        let reference = self.generator.as_ref()?.infer(norm)?;
        let paradigm = self.build_paradigm(norm, &reference);
        // Guard against inferring a class the generator cannot actually fill.
        (!paradigm
            .get(Number::Plural, Case::Inessive)
            .variants
            .is_empty())
        .then_some(paradigm)
    }

    fn build_paradigm(&self, norm: &str, reference: &ParadigmRef) -> Paradigm {
        Paradigm::build(norm, reference.clone(), |number, case| {
            self.slot(norm, reference, number, case)
                .unwrap_or_else(Forms::missing)
        })
    }

    // --- compound-noun support --------------------------------------------------------
    //
    // A word absent from the inventory may be a compound whose final component is known.
    // Finnish compounds inflect on the final component only; the modifier prefix is fixed.
    // Declining the bare component also makes vowel harmony follow it
    // (koira + keksi -> koirankeksissä, not -ssa, because `keksi` is front-harmonic).
    // Kotus-listed compounds carry tn50 (see COMPOUND_TN) and are routed here explicitly.

    /// Split `norm` into `(prefix, component)` where the component is a known lemma. Scans from
    /// the longest component down and **prefers a split whose prefix is itself a known modifier**
    /// (a real two-word compound, `koiran`+keksi), falling back to the longest known component
    /// when no part-of-the-prefix is known (a frozen/foreign modifier, `beaujolais`+viini).
    /// Char-boundary safe (Finnish ä/ö are multibyte); requires a prefix of >= 2 and a
    /// component of >= 3 chars to avoid spurious splits on tiny coincidental suffixes.
    fn split_compound(&self, norm: &str) -> Option<(String, String)> {
        const MIN_PREFIX_CHARS: usize = 2;
        const MIN_COMPONENT_CHARS: usize = 3;
        // The unknown-modifier fallback needs a longer prefix: real frozen/foreign
        // modifiers are words (`beaujolais`+viini), while a short residue is almost
        // always a false split of a simplex word the inventory just doesn't know
        // (pökkylä is not pök+kylä — issue #26).
        const MIN_FALLBACK_PREFIX_CHARS: usize = 4;

        // Explicit hyphen boundary: acronym/letter/short modifiers the author already
        // delimited (`3D-tulostin`, `A-pylväs`, `av-väline`, `4H-kerho`). The boundary is
        // given, not guessed, so split at the last hyphen when the tail is a known lemma —
        // keeping the hyphen on the frozen prefix.
        if let Some(idx) = norm.rfind('-') {
            let (prefix, tail) = (&norm[..=idx], &norm[idx + 1..]);
            if tail.chars().count() >= 2 && !self.resolve(tail).is_empty() {
                return Some((prefix.to_owned(), tail.to_owned()));
            }
        }

        // A known 2-char head (`yö`: aamu+yö, kesä+yö, sydän+yö) is too short for the
        // general scan. Allow it only behind a *known modifier* prefix, so an arbitrary
        // word ending in those two letters is not split.
        for head in SHORT_HEADS {
            if let Some(prefix) = norm.strip_suffix(head) {
                if self.is_known_modifier(prefix) && !self.resolve(head).is_empty() {
                    return Some((prefix.to_owned(), (*head).to_owned()));
                }
            }
        }

        let offsets: Vec<usize> = norm.char_indices().map(|(i, _)| i).collect();
        let n = offsets.len();
        if n < MIN_PREFIX_CHARS + MIN_COMPONENT_CHARS {
            return None;
        }
        // `at` is the byte offset where the candidate final component starts; the loop visits
        // the longest component first (shortest prefix). Work on string slices and only
        // allocate the owned pair once a split is actually chosen.
        let mut fallback: Option<usize> = None;
        for &at in &offsets[MIN_PREFIX_CHARS..=(n - MIN_COMPONENT_CHARS)] {
            let (prefix, component) = (&norm[..at], &norm[at..]);
            if self.resolve(component).is_empty() {
                continue;
            }
            // Accept a known modifier (a real word linker) or a productive bound prefix
            // (`avo`+auto, `ali`+nopeus, `yli`+hinta) even though it is too short to be a
            // free word — these are exceptionless compound-forming morphemes.
            if self.is_known_modifier(prefix) || is_bound_prefix(prefix) {
                return Some((prefix.to_owned(), component.to_owned()));
            }
            if fallback.is_none() && prefix.chars().count() >= MIN_FALLBACK_PREFIX_CHARS {
                fallback = Some(at);
            }
        }
        fallback.map(|at| (norm[..at].to_owned(), norm[at..].to_owned()))
    }

    /// `(prefix, component, chosen paradigm)` for a compound, or `None`. If the component is
    /// ambiguous the first paradigm is used — a lemma's paradigms share the same vowels, so
    /// the harmony (a/ä) choice is unaffected.
    fn compound_parts(&self, norm: &str) -> Option<(String, String, ParadigmRef)> {
        let (prefix, component) = self.split_compound(norm)?;
        let chosen = self.resolve(&component).into_iter().next()?;
        Some((prefix, component, chosen))
    }

    /// Build one slot of a compound by declining its final component and re-attaching the
    /// fixed prefix to every variant.
    fn compound_slot(&self, norm: &str, number: Number, case: Case) -> Option<Forms> {
        let (prefix, component, chosen) = self.compound_parts(norm)?;
        let mut forms = self.slot(&component, &chosen, number, case)?;
        if forms.is_missing() {
            return None;
        }
        forms.variants = forms
            .variants
            .iter()
            .map(|v| format!("{prefix}{v}"))
            .collect();
        Some(forms)
    }

    /// Build the whole paradigm of a compound from its final component (prefix re-attached).
    fn compound_paradigm(&self, norm: &str) -> Option<Paradigm> {
        let (prefix, component, chosen) = self.compound_parts(norm)?;
        Some(Paradigm::build(norm, chosen.clone(), |number, case| {
            let mut forms = self
                .slot(&component, &chosen, number, case)
                .unwrap_or_else(Forms::missing);
            forms.variants = forms
                .variants
                .iter()
                .map(|v| format!("{prefix}{v}"))
                .collect();
            forms
        }))
    }

    // --- plural-head (plurale-tantum) compounds -----------------------------------------
    //
    // `ajovalot`, `aluevaalit`, `arkiolot`: Kotus lists these as plurals with no tn, and the
    // head surfaces as a plural (`valot` = plural of `valo`), so the singular-lemma splitter
    // cannot see it. The plural-head reverse index maps a plural-nominative surface back to
    // its head lemma; the compound then declines on that head in the plural (singular slots
    // are defective — the lemma is a lexical plural).

    /// Split `norm` into `(prefix, head)` where the tail is a known lemma's plural
    /// nominative (per the reverse index). Longest tail first. The prefix must be a known
    /// modifier, a bound prefix, or a long (>= 4) frozen modifier, so an arbitrary word
    /// ending in a common plural is not mis-split.
    fn split_plural_head(&self, norm: &str) -> Option<(String, PluralHead)> {
        const MIN_PREFIX_CHARS: usize = 2;
        const MIN_TAIL_CHARS: usize = 3;
        if self.plural_index.is_empty() {
            return None;
        }
        let offsets: Vec<usize> = norm.char_indices().map(|(i, _)| i).collect();
        let n = offsets.len();
        if n < MIN_PREFIX_CHARS + MIN_TAIL_CHARS {
            return None;
        }
        let mut fallback: Option<(usize, PluralHead)> = None;
        for &at in &offsets[MIN_PREFIX_CHARS..=(n - MIN_TAIL_CHARS)] {
            let (prefix, tail) = (&norm[..at], &norm[at..]);
            let Some(head) = self.plural_index.get(tail) else {
                continue;
            };
            if self.is_known_modifier(prefix) || is_bound_prefix(prefix) {
                return Some((prefix.to_owned(), head.clone()));
            }
            if fallback.is_none() && prefix.chars().count() >= 4 {
                fallback = Some((at, head.clone()));
            }
        }
        fallback.map(|(at, head)| (norm[..at].to_owned(), head))
    }

    /// One slot of a plural-head compound. Plural slots decline the head in the plural with
    /// the prefix re-attached; singular slots are defective (`None`).
    fn plural_head_slot(&self, norm: &str, number: Number, case: Case) -> Option<Forms> {
        if number == Number::Singular {
            return None;
        }
        let (prefix, head) = self.split_plural_head(norm)?;
        let mut forms = self.slot(&head.lemma, &head.reference, Number::Plural, case)?;
        if forms.is_missing() {
            return None;
        }
        forms.variants = forms
            .variants
            .iter()
            .map(|v| format!("{prefix}{v}"))
            .collect();
        Some(forms)
    }

    /// The whole paradigm of a plural-head compound: plural slots filled, singular missing.
    fn plural_head_paradigm(&self, norm: &str) -> Option<Paradigm> {
        let (prefix, head) = self.split_plural_head(norm)?;
        let reference = head.reference.clone();
        Some(Paradigm::build(norm, reference, |number, case| {
            if number == Number::Singular {
                return Forms::missing();
            }
            let mut forms = self
                .slot(&head.lemma, &head.reference, Number::Plural, case)
                .unwrap_or_else(Forms::missing);
            if !forms.is_missing() {
                forms.variants = forms
                    .variants
                    .iter()
                    .map(|v| format!("{prefix}{v}"))
                    .collect();
            }
            forms
        }))
    }

    /// Whether `prefix` is a valid compound modifier: a known lemma either bare (the
    /// nominative linker, `puna`+viini) or in its genitive-singular linking form (`koira`+n =
    /// `koiran`, as in `koiran`+keksi). The genitive `-n` is Finnish's most common linker, so
    /// without this `koirankeksi` would not be recognized as a compound.
    fn is_known_modifier(&self, prefix: &str) -> bool {
        if !self.resolve(prefix).is_empty() {
            return true;
        }
        matches!(prefix.strip_suffix('n'), Some(stem) if !self.resolve(stem).is_empty())
    }

    /// Build one slot of a tn51 compound where **both parts inflect**: decline the modifier
    /// and the head in the same number/case and concatenate (`iso` + `veli` @ plural inessive
    /// → `isoissa` + `veljissä` → `isoissaveljissä`). Both parts must be known, declinable
    /// lemmas; returns `None` otherwise (the caller falls back to the head-only reading).
    fn compound_both_slot(&self, norm: &str, number: Number, case: Case) -> Option<Forms> {
        let (modifier, component) = self.split_compound(norm)?;
        // If a component is homonymous, the first resolved paradigm is used (issue #36).
        // This is safe in practice: a lemma's paradigms share the same vowels and stem
        // shape, so the harmony and the concatenated form are unaffected by the choice.
        // No Kotus tn51 compound currently has a component that resolves ambiguously.
        let mod_ref = self.resolve(&modifier).into_iter().next()?;
        let head_ref = self.resolve(&component).into_iter().next()?;
        let modf = self.slot(&modifier, &mod_ref, number, case)?;
        let head = self.slot(&component, &head_ref, number, case)?;
        if modf.is_missing() || head.is_missing() {
            return None;
        }
        let (m, h) = (modf.variants.first()?, head.variants.first()?);
        // In the comitative only the head carries the possessive citation; the modifier
        // agrees bare (aavoine + merineen -> aavoinemerineen, not *aavoineenmerineen).
        let m = if case == Case::Comitative {
            modf.variants
                .iter()
                .find(|v| v.ends_with("ne"))
                .map_or_else(|| m.strip_suffix("en").unwrap_or(m), String::as_str)
        } else {
            m
        };
        Some(Forms::present(vec![format!("{m}{h}")], head.source))
    }

    /// The whole paradigm of a tn51 both-parts-inflect compound.
    fn compound_both_paradigm(&self, norm: &str) -> Option<Paradigm> {
        let (_, component) = self.split_compound(norm)?;
        let head_ref = self.resolve(&component).into_iter().next()?;
        Some(Paradigm::build(norm, head_ref, |number, case| {
            self.compound_both_slot(norm, number, case)
                .unwrap_or_else(Forms::missing)
        }))
    }

    /// One slot of a compound ordinal — a tn45 lemma whose head is itself the tn45
    /// ordinal `kymmenes`: BOTH parts decline in the same slot and concatenate
    /// (`kahdeskymmenes` → `kahdennen` + `kymmenennen`, Voikko-verified).
    fn ordinal_both_slot(&self, norm: &str, number: Number, case: Case) -> Option<Forms> {
        let prefix = norm.strip_suffix("kymmenes").filter(|p| !p.is_empty())?;
        let ord = |lemma: &str| self.resolve(lemma).into_iter().find(|r| r.tn == 45);
        let pref_ref = ord(prefix)?;
        let head_ref = ord("kymmenes")?;
        // The parts come from the GENERATOR, not the lookup-first slot(): the corpus
        // carries mislabeled tn39-reading rows under kymmenes/tn45 (kymmeneksen) that
        // would otherwise leak into the concatenation.
        let generator = self.generator.as_ref()?;
        let p = generator.generate(prefix, &pref_ref, number, case)?;
        let h = generator.generate("kymmenes", &head_ref, number, case)?;
        if p.is_missing() || h.is_missing() {
            return None;
        }
        let (a, b) = (p.variants.first()?, h.variants.first()?);
        Some(Forms::present(
            vec![format!("{a}{b}")],
            crate::Source::Generated,
        ))
    }

    /// Should we override a *known* word's harmony because it's really a compound whose final
    /// component flips harmony? Conservative: the split must exist, the **prefix must be a known
    /// modifier** (a lemma bare or genitive-linked — so `punaviini` = puna+viini and
    /// `koirankeksi` = koiran+keksi qualify, but `laviini` — `la` is not a lemma — does not),
    /// and the whole-word vs component harmony must actually differ.
    fn compound_harmony_ok(&self, norm: &str) -> bool {
        let Some((prefix, component, _)) = self.compound_parts(norm) else {
            return false;
        };
        self.is_known_modifier(&prefix) && is_back(norm) != is_back(&component)
    }

    fn compound_harmony_slot(&self, norm: &str, number: Number, case: Case) -> Option<Forms> {
        if !self.compound_harmony_ok(norm) {
            return None;
        }
        self.compound_slot(norm, number, case)
    }

    fn compound_harmony_paradigm(&self, norm: &str) -> Option<Paradigm> {
        if !self.compound_harmony_ok(norm) {
            return None;
        }
        self.compound_paradigm(norm)
    }
}

/// Kotus class for head-inflecting compounds: declined on the final component with the
/// modifier frozen, so it is routed to the compound decliner rather than a rule arm.
const COMPOUND_TN: u8 = 50;

/// Kotus class for compounds where *both* parts inflect (`isoveli` → `isoissaveljissä`):
/// the modifier and head are declined in the same slot and concatenated.
const COMPOUND_BOTH_TN: u8 = 51;

/// Productive bound prefixes — compound-forming morphemes that are not free words, so the
/// splitter would otherwise reject them as too short. Each is exceptionless as a modifier
/// (`avo`+auto, `ali`+nopeus, `yli`+hinta, `etä`+työ). Only accepted when the head is a
/// known lemma, so a coincidental match (`ali`+bi) cannot fire — `bi` is not a lemma.
const BOUND_PREFIXES: &[&str] = &[
    "avo", "ali", "ala", "yli", "ylä", "ulko", "sisä", "etu", "taka", "etä", "eri", "esi", "iki",
    "eko", "bio", "geo", "neo",
];

/// Known short heads (2 chars) that the general scan (min 3) skips but that form real
/// compounds: `yö` (aamu+yö, sydän+yö). Accepted only behind a known-modifier prefix.
const SHORT_HEADS: &[&str] = &["yö"];

fn is_bound_prefix(prefix: &str) -> bool {
    BOUND_PREFIXES.contains(&prefix)
}

/// Whether a word takes back-vowel harmony: the last strong vowel decides (back `a/o/u`
/// vs front `ä/ö`), no strong vowel → front. Mirrors `keinontolibrary-rules`' harmony
/// test (keep in sync); used only to detect a compound flipping harmony.
fn is_back(s: &str) -> bool {
    s.chars()
        .rev()
        .find_map(|c| match c {
            'a' | 'o' | 'u' => Some(true),
            'ä' | 'ö' => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

/// Append `incoming` refs into `out`, skipping any with a `(hn, tn)` already present.
fn merge_refs(out: &mut Vec<ParadigmRef>, incoming: Vec<ParadigmRef>) {
    for r in incoming {
        if !out.iter().any(|e| e.hn == r.hn && e.tn == r.tn) {
            out.push(r);
        }
    }
}

/// Builder for [`Engine`].
#[derive(Default)]
pub struct EngineBuilder {
    lookup: Option<Box<dyn FormStore>>,
    overlay: Option<Box<dyn FormStore>>,
    generator: Option<Box<dyn Generator>>,
    plural_index: HashMap<String, PluralHead>,
}

impl fmt::Debug for EngineBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EngineBuilder")
            .field("has_lookup", &self.lookup.is_some())
            .field("has_overlay", &self.overlay.is_some())
            .field("has_generator", &self.generator.is_some())
            .field("plural_index_len", &self.plural_index.len())
            .finish()
    }
}

impl EngineBuilder {
    /// Set the primary corpus-backed lookup store.
    #[must_use]
    pub fn lookup(mut self, store: Box<dyn FormStore>) -> Self {
        self.lookup = Some(store);
        self
    }

    /// Set the runtime overlay store, consulted before the lookup store.
    #[must_use]
    pub fn overlay(mut self, store: Box<dyn FormStore>) -> Self {
        self.overlay = Some(store);
        self
    }

    /// Set the rule-based fallback generator.
    #[must_use]
    pub fn generator(mut self, generator: Box<dyn Generator>) -> Self {
        self.generator = Some(generator);
        self
    }

    /// Install the plural-head reverse index (`plural-nominative surface -> head lemma`),
    /// enabling plurale-tantum compound resolution (`ajovalot` -> `ajovaloissa`).
    #[must_use]
    pub fn plural_index(mut self, index: HashMap<String, PluralHead>) -> Self {
        self.plural_index = index;
        self
    }

    /// Finish building. Without a lookup store, an empty one is used.
    pub fn build(self) -> Engine {
        Engine {
            lookup: self.lookup.unwrap_or_else(|| Box::new(EmptyStore)),
            overlay: self.overlay,
            generator: self.generator,
            plural_index: self.plural_index,
        }
    }
}

/// A [`FormStore`] that knows nothing.
#[derive(Debug)]
struct EmptyStore;

impl FormStore for EmptyStore {
    fn paradigms(&self, _lemma: &str) -> Vec<ParadigmRef> {
        Vec::new()
    }
    fn forms(&self, _: &str, _: &ParadigmRef, _: Number, _: Case) -> Option<Forms> {
        None
    }
}

/// A simple in-memory [`FormStore`], used by tests and as the backend for the runtime
/// overlay store.
#[derive(Debug, Default)]
pub struct MemoryStore {
    entries: HashMap<String, Vec<MemEntry>>,
}

#[derive(Debug)]
struct MemEntry {
    reference: ParadigmRef,
    slots: HashMap<(Number, Case), Forms>,
}

impl MemoryStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert (or overwrite) the forms for one slot of `(lemma, reference)`.
    pub fn insert(
        &mut self,
        lemma: impl Into<String>,
        reference: ParadigmRef,
        number: Number,
        case: Case,
        forms: Forms,
    ) {
        let bucket = self.entries.entry(lemma.into()).or_default();
        let idx = bucket
            .iter()
            .position(|e| e.reference.hn == reference.hn && e.reference.tn == reference.tn);
        let entry = if let Some(i) = idx {
            &mut bucket[i]
        } else {
            bucket.push(MemEntry {
                reference,
                slots: HashMap::new(),
            });
            bucket.last_mut().expect("just pushed")
        };
        entry.slots.insert((number, case), forms);
    }
}

impl FormStore for MemoryStore {
    fn paradigms(&self, lemma: &str) -> Vec<ParadigmRef> {
        let Some(bucket) = self.entries.get(lemma) else {
            return Vec::new();
        };
        bucket.iter().map(|e| e.reference.clone()).collect()
    }

    fn forms(
        &self,
        lemma: &str,
        reference: &ParadigmRef,
        number: Number,
        case: Case,
    ) -> Option<Forms> {
        let bucket = self.entries.get(lemma)?;
        let entry = bucket
            .iter()
            .find(|e| e.reference.hn == reference.hn && e.reference.tn == reference.tn)?;
        entry.slots.get(&(number, case)).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::Source;

    fn store_with(lemma: &str, reference: ParadigmRef) -> MemoryStore {
        let mut s = MemoryStore::new();
        s.insert(
            lemma,
            reference.clone(),
            Number::Singular,
            Case::Inessive,
            Forms::present(vec![format!("{lemma}-ssa")], Source::Lookup),
        );
        s.insert(
            lemma,
            reference,
            Number::Singular,
            Case::Genitive,
            Forms::missing(),
        );
        s
    }

    #[test]
    fn empty_engine_reports_unknown() {
        let e = Engine::empty();
        assert_eq!(
            e.decline("hevonen", Number::Singular, Case::Inessive),
            Err(Error::UnknownWord("hevonen".into()))
        );
    }

    /// A stub generator that infers `-nen` -> tn38 and "generates" a marker form for it.
    #[derive(Debug)]
    struct InferGen;
    impl Generator for InferGen {
        fn generate(
            &self,
            lemma: &str,
            reference: &ParadigmRef,
            _n: Number,
            _c: Case,
        ) -> Option<Forms> {
            (reference.tn == 38)
                .then(|| Forms::present(vec![format!("{lemma}!")], Source::Generated))
        }
        fn infer(&self, lemma: &str) -> Option<ParadigmRef> {
            lemma.ends_with("nen").then(|| ParadigmRef::new(None, 38))
        }
    }

    #[test]
    fn unlisted_lemma_resolves_via_inference() {
        // No lookup entry for the word; the generator infers tn38 from the -nen suffix and
        // the engine serves the generated form instead of UnknownWord.
        let e = Engine::builder().generator(Box::new(InferGen)).build();
        let f = e
            .decline("ajankohtainen", Number::Singular, Case::Inessive)
            .unwrap();
        assert_eq!(f.primary(), Some("ajankohtainen!"));
        assert_eq!(f.source, Source::Generated);
        // A word the generator cannot infer still reports unknown.
        assert!(e.decline("talo", Number::Singular, Case::Inessive).is_err());
        // The whole-paradigm path infers too.
        assert!(e.paradigm("ajankohtainen").is_ok());
        assert!(e.paradigm("talo").is_err());
    }

    #[test]
    fn single_paradigm_resolves_slot() {
        let e = Engine::builder()
            .lookup(Box::new(store_with("talo", ParadigmRef::new(None, 1))))
            .build();
        let f = e
            .decline("  Talo ", Number::Singular, Case::Inessive)
            .unwrap();
        assert_eq!(f.primary(), Some("talo-ssa"));
        assert_eq!(f.source, Source::Lookup);
    }

    #[test]
    fn missing_slot_is_defective_error() {
        let e = Engine::builder()
            .lookup(Box::new(store_with("talo", ParadigmRef::new(None, 1))))
            .build();
        assert_eq!(
            e.decline("talo", Number::Singular, Case::Genitive),
            Err(Error::DefectiveForm {
                lemma: "talo".into(),
                number: Number::Singular,
                case: Case::Genitive,
            })
        );
    }

    #[test]
    fn two_paradigms_are_ambiguous_then_disambiguated() {
        let mut lookup = MemoryStore::new();
        lookup.insert(
            "kuusi",
            ParadigmRef::new(Some(1), 24),
            Number::Singular,
            Case::Inessive,
            Forms::present(vec!["kuusessa".into()], Source::Lookup),
        );
        lookup.insert(
            "kuusi",
            ParadigmRef::new(Some(2), 27),
            Number::Singular,
            Case::Inessive,
            Forms::present(vec!["kuudessa".into()], Source::Lookup),
        );
        let e = Engine::builder().lookup(Box::new(lookup)).build();

        match e.decline("kuusi", Number::Singular, Case::Inessive) {
            Err(Error::Ambiguous { paradigms, .. }) => assert_eq!(paradigms.len(), 2),
            other => panic!("expected Ambiguous, got {other:?}"),
        }

        let by_tn = ParadigmRef::new(None, 27);
        let f = e
            .decline_with("kuusi", Number::Singular, Case::Inessive, &by_tn)
            .unwrap();
        assert_eq!(f.primary(), Some("kuudessa"));
    }

    #[test]
    fn overlay_takes_precedence_over_lookup() {
        let mut lookup = MemoryStore::new();
        let r = ParadigmRef::new(None, 1);
        lookup.insert(
            "talo",
            r.clone(),
            Number::Singular,
            Case::Inessive,
            Forms::present(vec!["talossa".into()], Source::Lookup),
        );
        let mut overlay = MemoryStore::new();
        overlay.insert(
            "talo",
            r,
            Number::Singular,
            Case::Inessive,
            Forms::present(vec!["TALOSSA".into()], Source::Overlay),
        );
        let e = Engine::builder()
            .lookup(Box::new(lookup))
            .overlay(Box::new(overlay))
            .build();
        let f = e.decline("talo", Number::Singular, Case::Inessive).unwrap();
        assert_eq!(f.primary(), Some("TALOSSA"));
        assert_eq!(f.source, Source::Overlay);
    }

    #[test]
    fn full_paradigm_marks_absent_slots_missing() {
        let e = Engine::builder()
            .lookup(Box::new(store_with("talo", ParadigmRef::new(None, 1))))
            .build();
        let p = e.paradigm("talo").unwrap();
        assert_eq!(p.iter().count(), 30);
        assert!(p.get(Number::Singular, Case::Inessive).status == crate::forms::Status::Present);
        // A slot the store never populated is reported as Missing, not an error.
        assert!(p.get(Number::Plural, Case::Abessive).is_missing());
    }

    #[test]
    fn compound_inflects_on_final_component() {
        // `koirankeksi` is unknown as a whole; its final component `keksi` is known. The
        // compound declines on `keksi` and re-attaches the fixed prefix `koiran` — so
        // harmony follows `keksi` (front: -ssä), not the back vowels of `koira`.
        let mut store = MemoryStore::new();
        store.insert(
            "keksi",
            ParadigmRef::new(None, 5),
            Number::Singular,
            Case::Inessive,
            Forms::present(vec!["keksissä".into()], Source::Lookup),
        );
        let e = Engine::builder().lookup(Box::new(store)).build();
        let f = e
            .decline("koirankeksi", Number::Singular, Case::Inessive)
            .unwrap();
        assert_eq!(f.primary(), Some("koirankeksissä"));
    }

    #[test]
    fn known_compound_overrides_harmony_via_genitive_linker() {
        // `koirankeksi` is itself a known Kotus lemma (tn5), so it takes the known-word path
        // and its stored (back-harmonic) form would otherwise win. Its genitive-linked prefix
        // `koiran` (koira + n) must still be recognized as a modifier so harmony follows the
        // front-harmonic component `keksi`: koirankeksi -> koirankekseissä, not -ssa.
        let mut store = MemoryStore::new();
        let r = ParadigmRef::new(None, 5);
        store.insert(
            "koirankeksi",
            r.clone(),
            Number::Plural,
            Case::Inessive,
            Forms::present(vec!["koirankekseissa".into()], Source::Lookup),
        );
        // The modifier base `koira` is known (but the linking form `koiran` is not a lemma)...
        store.insert(
            "koira",
            ParadigmRef::new(None, 10),
            Number::Singular,
            Case::Nominative,
            Forms::present(vec!["koira".into()], Source::Lookup),
        );
        // ...and the final component `keksi` declines front.
        store.insert(
            "keksi",
            r,
            Number::Plural,
            Case::Inessive,
            Forms::present(vec!["kekseissä".into()], Source::Lookup),
        );
        let e = Engine::builder().lookup(Box::new(store)).build();
        let f = e
            .decline("koirankeksi", Number::Plural, Case::Inessive)
            .unwrap();
        assert_eq!(f.primary(), Some("koirankekseissä"));
    }

    #[test]
    fn compound_paradigm_prefixes_all_slots() {
        let mut store = MemoryStore::new();
        let r = ParadigmRef::new(None, 5);
        store.insert(
            "viini",
            r.clone(),
            Number::Singular,
            Case::Inessive,
            Forms::present(vec!["viinissä".into()], Source::Lookup),
        );
        store.insert(
            "viini",
            r,
            Number::Plural,
            Case::Inessive,
            Forms::present(vec!["viineissä".into()], Source::Lookup),
        );
        let e = Engine::builder().lookup(Box::new(store)).build();
        let p = e.paradigm("beaujolaisviini").unwrap();
        assert_eq!(
            p.get(Number::Singular, Case::Inessive).primary(),
            Some("beaujolaisviinissä")
        );
        assert_eq!(
            p.get(Number::Plural, Case::Inessive).primary(),
            Some("beaujolaisviineissä")
        );
    }

    // An unknown simplex word must not be served as a false compound: pökkylä is not
    // pök+kylä (issue #26) — a fallback split (unknown modifier) needs a modifier of
    // at least 4 chars, like the genuine beaujolais+viini above.
    #[test]
    fn unknown_word_is_not_false_split_on_short_residue() {
        let mut store = MemoryStore::new();
        store.insert(
            "kylä",
            ParadigmRef::new(None, 10),
            Number::Plural,
            Case::Inessive,
            Forms::present(vec!["kylissä".into()], Source::Lookup),
        );
        let e = Engine::builder().lookup(Box::new(store)).build();
        assert!(matches!(
            e.decline("pökkylä", Number::Plural, Case::Inessive),
            Err(Error::UnknownWord(_))
        ));
    }

    #[test]
    fn tn50_known_compound_declines_on_head() {
        // aitokana is a known Kotus lemma tagged tn50 (compound). It must route to the
        // compound decliner — decline the head `kana`, freeze the modifier `aito` — rather
        // than try a (nonexistent) tn50 rule.
        let mut store = MemoryStore::new();
        store.insert(
            "aitokana",
            ParadigmRef::new(None, 50),
            Number::Singular,
            Case::Nominative,
            Forms::present(vec!["aitokana".into()], Source::Lookup),
        );
        // The modifier base `aito` is known (so the split is taken as aito+kana)...
        store.insert(
            "aito",
            ParadigmRef::new(None, 1),
            Number::Singular,
            Case::Nominative,
            Forms::present(vec!["aito".into()], Source::Lookup),
        );
        // ...and the head `kana` supplies the inflected form.
        store.insert(
            "kana",
            ParadigmRef::new(None, 9),
            Number::Plural,
            Case::Inessive,
            Forms::present(vec!["kanoissa".into()], Source::Lookup),
        );
        let e = Engine::builder().lookup(Box::new(store)).build();
        let f = e
            .decline("aitokana", Number::Plural, Case::Inessive)
            .unwrap();
        assert_eq!(f.primary(), Some("aitokanoissa"));
    }

    #[test]
    fn tn51_compound_inflects_both_parts() {
        // isoveli (tn51): the modifier `iso` and the head `veli` both inflect and join —
        // isoissa + veljissä -> isoissaveljissä, not the frozen isoveljissä.
        let mut store = MemoryStore::new();
        store.insert(
            "isoveli",
            ParadigmRef::new(None, 51),
            Number::Singular,
            Case::Nominative,
            Forms::present(vec!["isoveli".into()], Source::Lookup),
        );
        store.insert(
            "iso",
            ParadigmRef::new(None, 1),
            Number::Plural,
            Case::Inessive,
            Forms::present(vec!["isoissa".into()], Source::Lookup),
        );
        store.insert(
            "veli",
            ParadigmRef::new(None, 7),
            Number::Plural,
            Case::Inessive,
            Forms::present(vec!["veljissä".into()], Source::Lookup),
        );
        let e = Engine::builder().lookup(Box::new(store)).build();
        let f = e
            .decline("isoveli", Number::Plural, Case::Inessive)
            .unwrap();
        assert_eq!(f.primary(), Some("isoissaveljissä"));
    }

    #[test]
    fn unknown_without_known_component_stays_unknown() {
        let mut store = MemoryStore::new();
        store.insert(
            "keksi",
            ParadigmRef::new(None, 5),
            Number::Singular,
            Case::Inessive,
            Forms::present(vec!["keksissä".into()], Source::Lookup),
        );
        let e = Engine::builder().lookup(Box::new(store)).build();
        assert!(matches!(
            e.decline("xyzzy", Number::Singular, Case::Inessive),
            Err(Error::UnknownWord(_))
        ));
    }
}
