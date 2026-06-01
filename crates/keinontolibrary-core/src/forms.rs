//! Inflected-form results: [`Forms`] for a single slot and [`Paradigm`] for the whole table.

use crate::case::{Case, Number};
use crate::paradigm_ref::ParadigmRef;

/// Existence status of a `(number, case)` slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Status {
    /// At least one ordinary, attested form exists.
    Present,
    /// A form exists but is marginal/lexicalized (e.g. singular instructive).
    Rare,
    /// No productive form exists for this slot (defective / plurale tantum / etc.).
    Missing,
}

/// Where a set of forms came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Source {
    /// Read from the precomputed corpus-backed lookup artifact.
    Lookup,
    /// Produced by the rule engine fallback.
    Generated,
    /// Supplied by the runtime overlay store (admin add/override).
    Overlay,
}

/// The form(s) for one `(number, case)` slot.
///
/// `variants` is primary-first. Genitive plural and illative commonly have several
/// legitimate variants (e.g. `omenoiden / omenoitten / omenain`), so this is always a
/// list. When `status` is [`Status::Missing`], `variants` is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Forms {
    /// The surface forms, primary first.
    pub variants: Vec<String>,
    /// Existence status of the slot.
    pub status: Status,
    /// Provenance of these forms.
    pub source: Source,
    /// Set when this slot has no independent ending and coincides with another case
    /// (the accusative: singular = genitive, plural = nominative).
    pub coincides_with: Option<Case>,
}

impl Forms {
    /// A present slot with the given variants (primary first).
    pub fn present(variants: Vec<String>, source: Source) -> Self {
        Self {
            variants,
            status: Status::Present,
            source,
            coincides_with: None,
        }
    }

    /// A rare/marginal slot.
    pub fn rare(variants: Vec<String>, source: Source) -> Self {
        Self {
            variants,
            status: Status::Rare,
            source,
            coincides_with: None,
        }
    }

    /// A defective/non-existent slot (empty variants).
    pub fn missing() -> Self {
        Self {
            variants: Vec::new(),
            status: Status::Missing,
            source: Source::Lookup,
            coincides_with: None,
        }
    }

    /// Builder-style setter recording that this slot coincides with another case.
    #[must_use]
    pub fn coinciding_with(mut self, case: Case) -> Self {
        self.coincides_with = Some(case);
        self
    }

    /// The primary (first) variant, if any.
    pub fn primary(&self) -> Option<&str> {
        self.variants.first().map(String::as_str)
    }

    /// Whether the slot is defective (no form exists).
    pub fn is_missing(&self) -> bool {
        self.status == Status::Missing
    }
}

/// A complete paradigm: every `number × case` slot for one resolved paradigm of a lemma.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Paradigm {
    /// The normalized lemma this paradigm belongs to.
    pub lemma: String,
    /// Which paradigm of the lemma this is.
    pub reference: ParadigmRef,
    /// All 2×15 slots, indexed by `number.index() * 15 + case.index()`.
    slots: Vec<Forms>,
}

impl Paradigm {
    const N_CASES: usize = Case::ALL.len();
    const N_SLOTS: usize = Number::ALL.len() * Self::N_CASES;

    /// Build a paradigm from a slot-filling closure invoked for every `(number, case)`.
    pub fn build(
        lemma: impl Into<String>,
        reference: ParadigmRef,
        mut fill: impl FnMut(Number, Case) -> Forms,
    ) -> Self {
        let mut slots = Vec::with_capacity(Self::N_SLOTS);
        for number in Number::ALL {
            for case in Case::ALL {
                slots.push(fill(number, case));
            }
        }
        Self {
            lemma: lemma.into(),
            reference,
            slots,
        }
    }

    #[inline]
    fn slot_index(number: Number, case: Case) -> usize {
        number.index() * Self::N_CASES + case.index()
    }

    /// The forms for a single slot.
    pub fn get(&self, number: Number, case: Case) -> &Forms {
        &self.slots[Self::slot_index(number, case)]
    }

    /// Iterate over every slot in stable order.
    pub fn iter(&self) -> impl Iterator<Item = (Number, Case, &Forms)> {
        Number::ALL.into_iter().flat_map(move |number| {
            Case::ALL
                .into_iter()
                .map(move |case| (number, case, self.get(number, case)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paradigm_get_matches_fill() {
        let p = Paradigm::build("testi", ParadigmRef::new(None, 5), |num, case| {
            Forms::present(vec![format!("{num}-{case}")], Source::Generated)
        });
        assert_eq!(
            p.get(Number::Plural, Case::Inessive).primary(),
            Some("plural-inessive")
        );
        assert_eq!(p.iter().count(), 30);
    }

    #[test]
    fn missing_forms_are_empty() {
        let f = Forms::missing();
        assert!(f.is_missing());
        assert!(f.variants.is_empty());
        assert_eq!(f.primary(), None);
    }
}
