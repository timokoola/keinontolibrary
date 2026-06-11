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
pub use generate::generate;

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
        if let Some(forms) = self.exceptions.get(lemma, reference.tn, number, case) {
            return Some(Forms::present(forms.to_vec(), Source::Generated));
        }
        if let Some(forms) = self
            .exceptions
            .get_compound(lemma, reference.tn, number, case)
        {
            return Some(Forms::present(forms, Source::Generated));
        }
        let variants = generate::generate(
            lemma,
            reference.tn,
            reference.av,
            reference.adjective,
            number,
            case,
        )?;
        if variants.is_empty() {
            return None;
        }
        Some(Forms::present(variants, Source::Generated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // The rule generator would produce "aieen"; the registry corrects it to "aikeen".
        let forms = r
            .generate(
                "aie",
                &ParadigmRef::new(None, 48),
                Number::Singular,
                Case::Genitive,
            )
            .unwrap();
        assert_eq!(forms.primary(), Some("aikeen"));
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

    // The registry now covers every irregular aie slot, including the derived accusative.
    #[test]
    fn aie_accusative_uses_k_insertion_stem() {
        let r = RuleEngine::new();
        let f = |n, c| {
            r.generate("aie", &ParadigmRef::new(None, 48), n, c)
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
    fn unsupported_class_yields_none() {
        let r = RuleEngine::new();
        // tn16 (the comparative -mpi class) has neither a rule arm nor a registry entry.
        assert!(r
            .generate(
                "pienempi",
                &ParadigmRef::new(None, 16),
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
