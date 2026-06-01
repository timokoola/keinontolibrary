//! The on-disk lookup artifact: the data model plus (de)serialization.
//!
//! The artifact is a `bincode`-encoded [`Artifact`] holding one [`LemmaRecord`] per
//! in-scope lemma. The reader (see [`crate::store`]) loads it once and builds an in-memory
//! index. The format is intentionally simple for v1; an `fst`/`mmap` zero-copy layout is a
//! future optimization (see `LICENSING.md` / the project plan).

use std::io;
use std::path::Path;

use keinontolibrary_core::{Case, Number, Status};
use serde::{Deserialize, Serialize};

/// Number of `(number, case)` slots in a full paradigm.
pub const N_SLOTS: usize = 2 * 15;

/// The packed slot index for a `(number, case)` pair: `number * 15 + case`.
#[must_use]
pub fn slot_index(number: Number, case: Case) -> u8 {
    u8::try_from(number.index() * Case::ALL.len() + case.index()).expect("slot index < 30")
}

/// Decode a packed slot index back into `(number, case)`.
#[must_use]
pub fn slot_parts(slot: u8) -> (Number, Case) {
    let n = usize::from(slot) / Case::ALL.len();
    let c = usize::from(slot) % Case::ALL.len();
    (Number::ALL[n], Case::ALL[c])
}

/// Top-level artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Provenance and build metadata.
    pub meta: Meta,
    /// One record per lemma, sorted by lemma for determinism.
    pub lemmas: Vec<LemmaRecord>,
}

/// Build/provenance metadata, surfaced by the server's `/about`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Meta {
    /// Crate/data version string.
    pub version: String,
    /// Description of the Kotus source (name + license).
    pub kotus_source: String,
    /// Description of the Voikko source.
    pub voikko_source: String,
    /// Number of lemmas in the artifact.
    pub n_lemmas: u32,
    /// Number of distinct (lemma, paradigm, slot, variant) forms.
    pub n_forms: u64,
}

/// All paradigms for a single lemma.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LemmaRecord {
    /// The normalized lemma.
    pub lemma: String,
    /// The distinct declension paradigms (deduplicated by `(tn, av)`), primary first.
    pub paradigms: Vec<ParadigmRecord>,
}

/// One declension paradigm of a lemma.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParadigmRecord {
    /// Kotus declension class (1–49).
    pub tn: u8,
    /// Consonant-gradation letter, if any.
    pub av: Option<char>,
    /// Whether this is a secondary/rare paradigm (parenthesized in the Kotus list).
    pub rare: bool,
    /// Populated slots only (sparse). Slots absent here are left to the rule fallback.
    pub slots: Vec<SlotRecord>,
}

/// The corpus-attested forms for one `(number, case)` slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotRecord {
    /// Packed `(number, case)` index (see [`slot_index`]).
    pub slot: u8,
    /// Existence status.
    pub status: Status,
    /// Surface forms, primary first. Empty when `status == Missing`.
    pub variants: Vec<String>,
    /// Set when this slot has no independent ending (accusative); the coinciding case
    /// index.
    pub coincides_with: Option<u8>,
}

impl Artifact {
    /// Encode to `bincode` bytes.
    ///
    /// # Errors
    /// Propagates `bincode` serialization failures.
    pub fn encode(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Decode from `bincode` bytes.
    ///
    /// # Errors
    /// Propagates `bincode` deserialization failures.
    pub fn decode(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    /// Write the encoded artifact to `path`.
    ///
    /// # Errors
    /// Returns an error if encoding or the file write fails.
    pub fn write_to(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let bytes = self.encode().map_err(io::Error::other)?;
        std::fs::write(path, bytes)
    }

    /// Read and decode an artifact from `path`.
    ///
    /// # Errors
    /// Returns an error if the read or decoding fails.
    pub fn read_from(path: impl AsRef<Path>) -> io::Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::decode(&bytes).map_err(io::Error::other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_index_round_trips() {
        for number in Number::ALL {
            for case in Case::ALL {
                let (n, c) = slot_parts(slot_index(number, case));
                assert_eq!((n, c), (number, case));
            }
        }
    }

    #[test]
    fn artifact_round_trips_through_bincode() {
        let art = Artifact {
            meta: Meta {
                version: "test".into(),
                n_lemmas: 1,
                n_forms: 1,
                ..Meta::default()
            },
            lemmas: vec![LemmaRecord {
                lemma: "talo".into(),
                paradigms: vec![ParadigmRecord {
                    tn: 1,
                    av: None,
                    rare: false,
                    slots: vec![SlotRecord {
                        slot: slot_index(Number::Singular, Case::Inessive),
                        status: Status::Present,
                        variants: vec!["talossa".into()],
                        coincides_with: None,
                    }],
                }],
            }],
        };
        let bytes = art.encode().unwrap();
        let back = Artifact::decode(&bytes).unwrap();
        assert_eq!(back.lemmas[0].lemma, "talo");
        assert_eq!(
            back.lemmas[0].paradigms[0].slots[0].variants,
            vec!["talossa"]
        );
    }
}
