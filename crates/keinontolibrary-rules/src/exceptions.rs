//! The exception registry: irregular forms the rule engine does not model.
//!
//! The registry is the `exceptions.toml` file (compiled in via `include_str!`). It is
//! consulted by [`crate::RuleEngine`] before the rule generator, so the fallback returns
//! the correct form for documented irregulars. The corpus lookup already serves these for
//! attested words; the registry additionally covers them when generating and is what the
//! parity harness measures against.

use std::collections::HashMap;

use keinontolibrary_core::{normalize, Case, Number};
use serde::Deserialize;

const REGISTRY: &str = include_str!("../exceptions.toml");

#[derive(Debug, Deserialize)]
struct Raw {
    #[serde(default)]
    exception: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    lemma: String,
    number: String,
    case: String,
    forms: Vec<String>,
    #[allow(dead_code)] // documentation only
    reason: String,
    #[serde(default)]
    tn: Option<u8>,
}

/// A parsed, queryable view over the exception registry.
#[derive(Debug, Clone, Default)]
pub struct Exceptions {
    // (normalized lemma, number, case) -> forms. `tn` is folded into the key when present.
    by_slot: HashMap<(String, Option<u8>, Number, Case), Vec<String>>,
    count: usize,
}

impl Exceptions {
    /// Load and parse the compiled-in registry.
    ///
    /// # Panics
    /// Panics if `exceptions.toml` is malformed — it is compiled in, so this is a build-time
    /// authoring error, caught by the registry's own tests.
    #[must_use]
    pub fn load() -> Self {
        Self::parse(REGISTRY).expect("exceptions.toml is valid (checked by tests)")
    }

    fn parse(text: &str) -> Result<Self, String> {
        let raw: Raw = toml::from_str(text).map_err(|e| e.to_string())?;
        let mut by_slot = HashMap::new();
        for e in &raw.exception {
            let number: Number = e
                .number
                .parse()
                .map_err(|_| format!("bad number {:?}", e.number))?;
            let case: Case = e
                .case
                .parse()
                .map_err(|_| format!("bad case {:?}", e.case))?;
            if e.forms.is_empty() {
                return Err(format!("empty forms for {:?}", e.lemma));
            }
            by_slot.insert((normalize(&e.lemma), e.tn, number, case), e.forms.clone());
        }
        let count = by_slot.len();
        Ok(Self { by_slot, count })
    }

    /// The registered forms for a slot, if any. Matches a `tn`-qualified entry first, then a
    /// `tn`-agnostic one.
    #[must_use]
    pub fn get(&self, lemma: &str, tn: u8, number: Number, case: Case) -> Option<&[String]> {
        let lemma = normalize(lemma);
        self.by_slot
            .get(&(lemma.clone(), Some(tn), number, case))
            .or_else(|| self.by_slot.get(&(lemma, None, number, case)))
            .map(Vec::as_slice)
    }

    /// Number of registered slots (raw row count; backstop for the parity gate).
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Number of distinct irregular lemmas. This is the meaningful cap unit: a genuine
    /// irregular needs many slots (aika alone is 19), so counting lemmas flags systematic
    /// rule-gap dumping rather than punishing fully specifying one true irregular.
    #[must_use]
    pub fn lemma_count(&self) -> usize {
        self.by_slot
            .keys()
            .map(|(lemma, ..)| lemma.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_parses_and_is_nonempty() {
        let ex = Exceptions::load();
        assert!(!ex.is_empty());
    }

    #[test]
    fn aie_genitive_is_registered() {
        let ex = Exceptions::load();
        assert_eq!(
            ex.get("aie", 48, Number::Singular, Case::Genitive),
            Some(["aikeen".to_string()].as_slice())
        );
        // An unregistered slot returns None.
        assert!(ex
            .get("talo", 1, Number::Singular, Case::Genitive)
            .is_none());
    }
}
