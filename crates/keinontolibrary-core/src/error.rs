//! The public [`Error`] type returned by `decline`/`paradigm`.

use crate::case::{Case, Number};
use crate::paradigm_ref::ParadigmRef;

/// Failures from declension queries.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The lemma is not in the known inventory (and so cannot be generated either).
    #[error("unknown word: {0:?}")]
    UnknownWord(String),

    /// The lemma has multiple paradigms and no disambiguator was supplied. Retry with
    /// `decline_with` / `paradigm_with` and one of the listed `paradigms`.
    #[error("ambiguous word {lemma:?}: {} candidate paradigms", paradigms.len())]
    Ambiguous {
        /// The normalized lemma.
        lemma: String,
        /// The candidate paradigms to choose between.
        paradigms: Vec<ParadigmRef>,
    },

    /// The requested slot is defective for this lemma (e.g. singular of a plurale tantum,
    /// or no productive comitative singular).
    #[error("defective form: {lemma:?} has no {number} {case}")]
    DefectiveForm {
        /// The normalized lemma.
        lemma: String,
        /// The requested number.
        number: Number,
        /// The requested case.
        case: Case,
    },
}
