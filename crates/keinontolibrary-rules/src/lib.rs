//! `keinontolibrary-rules` — rule-based declension generator for the high-frequency Kotus
//! classes with consonant gradation (astevaihtelu).
//!
//! The engine generates forms from `(lemma, tn, av)` for the pragmatic high-frequency set
//! (classes 1-15, 17-20, 23, 24, 26-28, 32-34, 38-41, 43, 47, 48 (34 in all)). It is wired in as the rule **fallback**
//! behind the corpus lookup: [`RuleEngine`] implements [`keinontolibrary_core::Generator`],
//! so the engine only calls it for slots the lookup/overlay don't already answer.

mod exceptions;
mod generate;
mod gradation;
mod harmony;

pub use exceptions::Exceptions;
pub use generate::{citation_forms, generate, is_plurale_tantum};

use keinontolibrary_core::{Case, Forms, Generator, Number, ParadigmRef, Source};

/// The rule-based fallback generator, including the exception registry.
#[derive(Debug, Clone)]
pub struct RuleEngine {
    exceptions: Exceptions,
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleEngine {
    /// Construct the rule engine, loading the exception registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            exceptions: Exceptions::load(),
        }
    }

    /// The exception registry this engine consults.
    #[must_use]
    pub fn exceptions(&self) -> &Exceptions {
        &self.exceptions
    }
}

impl Generator for RuleEngine {
    fn generate(
        &self,
        lemma: &str,
        reference: &ParadigmRef,
        number: Number,
        case: Case,
    ) -> Option<Forms> {
        // The exception registry overrides the rule generator for documented irregulars,
        // including compounds whose head is a registered irregular (adventtiaika → ajan).
        // An empty registered slot declares a true defective (hän has no plural).
        if let Some(forms) = self.exceptions.get(lemma, reference.tn, number, case) {
            if forms.is_empty() {
                return Some(Forms::missing());
            }
            return Some(Forms::present(forms.to_vec(), Source::Generated));
        }
        if let Some(forms) = self
            .exceptions
            .get_compound(lemma, reference.tn, number, case)
        {
            return Some(Forms::present(forms, Source::Generated));
        }
        // Foreign/letter-word citations decline on the pronunciation behind a
        // separator (parfait'n, cd:tä) — the sidecar-minted style carries what the
        // spelling cannot.
        if let Some(c) = reference.citation {
            let variants = generate::citation_forms(lemma, c, number, case)?;
            return Some(Forms::present(variants, Source::Generated));
        }
        // Plurale tantum citations (sakset, arpajaiset, rattaat, alkeet) have no
        // singular at all: answer Missing, not a gap.
        if number == Number::Singular
            && generate::is_plurale_tantum(lemma, reference.tn, reference.av)
        {
            return Some(Forms::missing());
        }
        if let Some(variants) = generate::generate(
            lemma,
            reference.tn,
            reference.av,
            reference.adjective,
            reference.front_harmony,
            number,
            case,
        ) {
            if !variants.is_empty() {
                return Some(Forms::present(variants, Source::Generated));
            }
        }
        // Fallback: clitic pronominals whose clitic stays at the very end — the stem
        // inflects and the clitic re-attaches (kumpikin -> kummankin, kumpainenkin ->
        // kumpaisenkin, joltinenkin -> joltisenkin; Voikko-verified). Distinct from
        // jokin/kukaan, whose clitic moves inside (registry-served, returned above).
        for clitic in ["kin", "kaan", "kään"] {
            if let Some(base) = lemma.strip_suffix(clitic) {
                if base.len() >= 4 {
                    if let Some(forms) = self.generate(base, reference, number, case) {
                        if !forms.is_missing() {
                            let variants = forms
                                .variants
                                .iter()
                                .map(|v| format!("{v}{clitic}"))
                                .collect();
                            return Some(Forms::present(variants, Source::Generated));
                        }
                    }
                }
            }
        }
        None
    }

    fn infer(&self, lemma: &str) -> Option<ParadigmRef> {
        infer_class(lemma)
    }
}

/// Infer a declension class for an *unlisted* lemma from productive, exceptionless
/// morphology. Kotus leaves transparent derivations without a declension type, expecting
/// inflection through their shape; these two suffixes are fully regular:
/// - any word ending in `-nen` is **tn38** (`nainen`, `hevonen`, `ahdaskatseinen`);
/// - any abstract noun in `-uus`/`-yys` is **tn40** (`ystävyys`, `ajankohtaisuus`).
///
/// Deliberately conservative: only suffixes with a single possible class. Plain `-us`
/// (tn39 *or* 40) and `-in` (tn33/49) are ambiguous and left out.
#[must_use]
pub fn infer_class(lemma: &str) -> Option<ParadigmRef> {
    let n = lemma.chars().count();
    if n >= 5 && (lemma.ends_with("uus") || lemma.ends_with("yys")) {
        return Some(ParadigmRef::new(None, 40));
    }
    // "-nen" alone is 3 chars; require a real stem before it.
    if n >= 5 && lemma.ends_with("nen") {
        return Some(ParadigmRef::new(None, 38));
    }
    // -ias/-iäs adjectives are tn41 (vieras-type: vuotias -> vuotiaan): exceptionless among
    // unlisted words — the few foreign -ias (alias tn99, iskias tn39) are all in the lookup,
    // so this only fires for productive forms like the `-vuotias` age adjectives.
    if n >= 6 && (lemma.ends_with("ias") || lemma.ends_with("iäs")) {
        return Some(ParadigmRef::new(None, 41));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_class_maps_productive_suffixes() {
        // -nen -> tn38, -uus/-yys -> tn40; both exceptionless. Other suffixes: no inference.
        assert_eq!(infer_class("ahdaskatseinen").map(|r| r.tn), Some(38));
        assert_eq!(infer_class("nainen").map(|r| r.tn), Some(38));
        assert_eq!(infer_class("ajankohtaisuus").map(|r| r.tn), Some(40));
        assert_eq!(infer_class("ystävyys").map(|r| r.tn), Some(40));
        // Ambiguous / too short -> no inference (lookup/compound must answer instead).
        assert_eq!(infer_class("vastaus"), None); // -us is tn39 or 40
        assert_eq!(infer_class("talo"), None);
        assert_eq!(infer_class("nen"), None);
        // The inferred class actually generates correct forms via the engine path.
        let r = RuleEngine::new();
        let ref38 = infer_class("ahdaskatseinen").unwrap();
        assert_eq!(
            r.generate("ahdaskatseinen", &ref38, Number::Singular, Case::Genitive)
                .and_then(|f| f.primary().map(String::from)),
            Some("ahdaskatseisen".to_owned())
        );
    }

    #[test]
    fn rule_engine_generates_tagged_generated() {
        let r = RuleEngine::new();
        let forms = r
            .generate(
                "valo",
                &ParadigmRef::new(None, 1),
                Number::Singular,
                Case::Inessive,
            )
            .unwrap();
        assert_eq!(forms.primary(), Some("valossa"));
        assert_eq!(forms.source, Source::Generated);
    }

    #[test]
    fn exception_registry_overrides_rules() {
        let r = RuleEngine::new();
        // mies is registry-served (tn42 singleton); the rules alone cannot produce it.
        let forms = r
            .generate(
                "mies",
                &ParadigmRef::new(None, 42),
                Number::Singular,
                Case::Genitive,
            )
            .unwrap();
        assert_eq!(forms.primary(), Some("miehen"));
        assert_eq!(forms.source, Source::Generated);
    }

    // aika/poika are the classic k:j irregulars. Kotus marks them gradation D (k:∅, like
    // vika->vian), but their weak grade is k:j: aika->ajan, poika->pojan. The rule engine
    // applies D and produces aian/poian; the exception registry must correct the weak slots
    // while leaving the strong-grade slots (partitive, illative, essive) untouched.
    fn prim(r: &RuleEngine, lemma: &str, tn: u8, n: Number, c: Case) -> String {
        r.generate(lemma, &ParadigmRef::new(None, tn).with_av(Some('D')), n, c)
            .unwrap()
            .primary()
            .unwrap()
            .to_string()
    }

    // Compounds ending in a registered irregular head decline the head the same way:
    // adventtiaika -> adventtiajan (not *adventtiaian), koulupoika -> koulupojan.
    // Strong-grade slots stay with the rule generator (adventtiaikaa). Found by the QA loop.
    #[test]
    fn compound_heads_inherit_lexical_exceptions() {
        let r = RuleEngine::new();
        assert_eq!(
            prim(&r, "adventtiaika", 9, Number::Singular, Case::Genitive),
            "adventtiajan"
        );
        assert_eq!(
            prim(&r, "adventtiaika", 9, Number::Plural, Case::Inessive),
            "adventtiajoissa"
        );
        assert_eq!(
            prim(&r, "koulupoika", 10, Number::Singular, Case::Genitive),
            "koulupojan"
        );
        // Strong grade is regular and not in the registry.
        assert_eq!(
            prim(&r, "adventtiaika", 9, Number::Singular, Case::Partitive),
            "adventtiaikaa"
        );
        // The shortest real modifiers are two-letter compounds...
        assert_eq!(
            prim(&r, "yöaika", 9, Number::Singular, Case::Genitive),
            "yöajan"
        );
        // ...while taika is t+aika only by spelling: its own lemma, regular k-elision.
        assert_eq!(
            prim(&r, "taika", 9, Number::Singular, Case::Genitive),
            "taian"
        );
    }

    // The reverse-D k-insertion rule (gradation.rs) covers every aie slot, including
    // the derived accusative — the registry rows it replaced are gone.
    #[test]
    fn aie_accusative_uses_k_insertion_stem() {
        let r = RuleEngine::new();
        let f = |n, c| {
            r.generate("aie", &ParadigmRef::new(None, 48).with_av(Some('D')), n, c)
                .unwrap()
                .primary()
                .unwrap()
                .to_string()
        };
        assert_eq!(f(Number::Singular, Case::Accusative), "aikeen");
        assert_eq!(f(Number::Plural, Case::Elative), "aikeista");
        assert_eq!(f(Number::Plural, Case::Comitative), "aikeineen");
    }

    #[test]
    fn aika_kj_gradation_singular() {
        let r = RuleEngine::new();
        // weak grade -> aja-
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Genitive),
            "ajan"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Inessive),
            "ajassa"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Elative),
            "ajasta"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Adessive),
            "ajalla"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Ablative),
            "ajalta"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Allative),
            "ajalle"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Translative),
            "ajaksi"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Abessive),
            "ajatta"
        );
        // strong grade kept
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Partitive),
            "aikaa"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Illative),
            "aikaan"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Singular, Case::Essive),
            "aikana"
        );
    }

    #[test]
    fn aika_kj_gradation_plural() {
        let r = RuleEngine::new();
        // weak grade -> ajoi-
        assert_eq!(
            prim(&r, "aika", 9, Number::Plural, Case::Nominative),
            "ajat"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Plural, Case::Inessive),
            "ajoissa"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Plural, Case::Elative),
            "ajoista"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Plural, Case::Adessive),
            "ajoilla"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Plural, Case::Allative),
            "ajoille"
        );
        // strong grade kept
        assert_eq!(
            prim(&r, "aika", 9, Number::Plural, Case::Partitive),
            "aikoja"
        );
        assert_eq!(
            prim(&r, "aika", 9, Number::Plural, Case::Illative),
            "aikoihin"
        );
    }

    #[test]
    fn poika_kj_gradation() {
        let r = RuleEngine::new();
        assert_eq!(
            prim(&r, "poika", 10, Number::Singular, Case::Genitive),
            "pojan"
        );
        assert_eq!(
            prim(&r, "poika", 10, Number::Singular, Case::Inessive),
            "pojassa"
        );
        assert_eq!(
            prim(&r, "poika", 10, Number::Singular, Case::Allative),
            "pojalle"
        );
        assert_eq!(
            prim(&r, "poika", 10, Number::Plural, Case::Nominative),
            "pojat"
        );
        assert_eq!(
            prim(&r, "poika", 10, Number::Plural, Case::Inessive),
            "pojissa"
        );
        // strong grade kept
        assert_eq!(
            prim(&r, "poika", 10, Number::Singular, Case::Partitive),
            "poikaa"
        );
        assert_eq!(
            prim(&r, "poika", 10, Number::Plural, Case::Illative),
            "poikiin"
        );
    }

    #[test]
    fn unimplemented_class_yields_none() {
        let r = RuleEngine::new();
        // tn22 (silent-letter foreign citations, parfait'n) has neither a rule arm nor
        // a registry entry — the apostrophe/colon orthography needs per-lemma
        // pronunciation knowledge.
        assert!(r
            .generate(
                "parfait",
                &ParadigmRef::new(None, 22),
                Number::Singular,
                Case::Genitive
            )
            .is_none());
    }

    // Pronouns (tn 101) are irregular and have no rule arm; the engine serves them entirely
    // from the exception registry, keyed on tn 101 and their inherent number (minä is
    // singular, me/ne plural). Suppletive obliques (minun, meidän, niiden) come straight from
    // the registry.
    #[test]
    fn pronouns_resolve_via_registry() {
        let r = RuleEngine::new();
        let pron = |lemma, n, c| {
            r.generate(lemma, &ParadigmRef::new(None, 101), n, c)
                .and_then(|f| f.primary().map(str::to_string))
        };
        assert_eq!(
            pron("minä", Number::Singular, Case::Genitive).as_deref(),
            Some("minun")
        );
        assert_eq!(
            pron("hän", Number::Singular, Case::Partitive).as_deref(),
            Some("häntä")
        );
        assert_eq!(
            pron("me", Number::Plural, Case::Inessive).as_deref(),
            Some("meissä")
        );
        assert_eq!(
            pron("ne", Number::Plural, Case::Genitive).as_deref(),
            Some("niiden")
        );
        assert_eq!(
            pron("tämä", Number::Singular, Case::Illative).as_deref(),
            Some("tähän")
        );
        // A pronoun's non-inherent number is suppletive/absent — no rule arm fills it.
        assert!(pron("minä", Number::Plural, Case::Genitive).is_none());
    }

    // kaksi/yksi (tn 31) and tuhat (tn 46) are one-off irregulars served from the registry.
    // (The productive ordinals, tn 45, go through the rule generator instead.)
    #[test]
    fn irregular_numerals_resolve_via_registry() {
        let r = RuleEngine::new();
        let num = |lemma, tn, n, c| {
            r.generate(lemma, &ParadigmRef::new(None, tn), n, c)
                .and_then(|f| f.primary().map(str::to_string))
        };
        assert_eq!(
            num("kaksi", 31, Number::Singular, Case::Genitive).as_deref(),
            Some("kahden")
        );
        assert_eq!(
            num("yksi", 31, Number::Singular, Case::Partitive).as_deref(),
            Some("yhtä")
        );
        assert_eq!(
            num("tuhat", 46, Number::Singular, Case::Inessive).as_deref(),
            Some("tuhannessa")
        );
        assert_eq!(
            num("tuhat", 46, Number::Plural, Case::Inessive).as_deref(),
            Some("tuhansissa")
        );
    }
}
