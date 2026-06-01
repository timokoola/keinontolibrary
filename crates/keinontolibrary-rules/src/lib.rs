//! `keinontolibrary-rules` — rule-based declension generator for the high-frequency Kotus
//! classes with consonant gradation (astevaihtelu).
//!
//! The engine generates forms from `(lemma, tn, av)` for the pragmatic high-frequency set
//! (classes 1-15, 17-20, 23, 24, 26-28, 32-34, 38-41, 43, 47, 48 (34 in all)). It is wired in as the rule **fallback**
//! behind the corpus lookup: [`RuleEngine`] implements [`keinontolibrary_core::Generator`],
//! so the engine only calls it for slots the lookup/overlay don't already answer.

mod generate;
mod gradation;
mod harmony;

pub use generate::generate;

use keinontolibrary_core::{Case, Forms, Generator, Number, ParadigmRef, Source};

/// The rule-based fallback generator.
#[derive(Debug, Default, Clone, Copy)]
pub struct RuleEngine;

impl RuleEngine {
    /// Construct the rule engine.
    #[must_use]
    pub fn new() -> Self {
        Self
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
        let variants = generate::generate(lemma, reference.tn, reference.av, number, case)?;
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
    fn unsupported_class_yields_none() {
        let r = RuleEngine::new();
        assert!(r
            .generate(
                "kevät",
                &ParadigmRef::new(None, 44),
                Number::Singular,
                Case::Genitive
            )
            .is_none());
    }
}
