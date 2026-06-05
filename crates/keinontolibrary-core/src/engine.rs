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
}

/// The declension engine: holds the providers and runs the resolution pipeline.
pub struct Engine {
    lookup: Box<dyn FormStore>,
    overlay: Option<Box<dyn FormStore>>,
    generator: Option<Box<dyn Generator>>,
}

impl fmt::Debug for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Engine")
            .field("lookup", &self.lookup)
            .field("has_overlay", &self.overlay.is_some())
            .field("has_generator", &self.generator.is_some())
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
        }
    }

    /// Decline `lemma` into one slot, erroring if the lemma is ambiguous.
    pub fn decline(&self, lemma: &str, number: Number, case: Case) -> Result<Forms, Error> {
        let norm = normalize(lemma);
        let refs = self.resolve(&norm);
        match refs.as_slice() {
            // Unknown as a whole: it may be a compound whose final component is known.
            [] => self
                .compound_slot(&norm, number, case)
                .ok_or(Error::UnknownWord(norm)),
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
                .ok_or(Error::UnknownWord(norm)),
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

    /// The longest suffix of `norm` that is a known lemma, as `(prefix, component)`.
    /// Char-boundary safe (Finnish ä/ö are multibyte); requires a prefix of >= 2 and a
    /// component of >= 3 chars to avoid spurious splits on tiny coincidental suffixes.
    fn split_compound(&self, norm: &str) -> Option<(String, String)> {
        const MIN_PREFIX_CHARS: usize = 2;
        const MIN_COMPONENT_CHARS: usize = 3;
        let offsets: Vec<usize> = norm.char_indices().map(|(i, _)| i).collect();
        let n = offsets.len();
        if n < MIN_PREFIX_CHARS + MIN_COMPONENT_CHARS {
            return None;
        }
        // Grow the prefix; the first known suffix found is the longest one. `at` is the byte
        // offset where the candidate final component starts.
        for &at in &offsets[MIN_PREFIX_CHARS..=(n - MIN_COMPONENT_CHARS)] {
            if !self.resolve(&norm[at..]).is_empty() {
                return Some((norm[..at].to_owned(), norm[at..].to_owned()));
            }
        }
        None
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

    /// Should we override a *known* word's harmony because it's really a compound whose final
    /// component flips harmony? Conservative: the split must exist, the **prefix must itself be
    /// a known lemma** (so `punaviini` = puna+viini qualifies but `laviini` — `la` is not a
    /// lemma — does not), and the whole-word vs component harmony must actually differ.
    fn compound_harmony_ok(&self, norm: &str) -> bool {
        let Some((prefix, component, _)) = self.compound_parts(norm) else {
            return false;
        };
        !self.resolve(&prefix).is_empty() && is_back(norm) != is_back(&component)
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

/// Whether a word takes back-vowel harmony (contains any of a/o/u). Mirrors the rule
/// engine's harmony test; used only to detect a compound flipping harmony.
fn is_back(s: &str) -> bool {
    s.chars().any(|c| matches!(c, 'a' | 'o' | 'u'))
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
}

impl fmt::Debug for EngineBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EngineBuilder")
            .field("has_lookup", &self.lookup.is_some())
            .field("has_overlay", &self.overlay.is_some())
            .field("has_generator", &self.generator.is_some())
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

    /// Finish building. Without a lookup store, an empty one is used.
    pub fn build(self) -> Engine {
        Engine {
            lookup: self.lookup.unwrap_or_else(|| Box::new(EmptyStore)),
            overlay: self.overlay,
            generator: self.generator,
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
