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
    /// Description of the reference corpus (our generated form set, and the tool used).
    pub reference_source: String,
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
    /// Whether every Kotus reading of the lemma is a modifier (adjective/numeral) — these
    /// take the bare `-ine` plural comitative instead of the noun citation `-ineen`.
    #[serde(default)]
    pub adjective: bool,
    /// Vowel-harmony override: `Some(true)` = front endings, `Some(false)` = back,
    /// `None` = derive from the lemma. Minted from Voikko's compound segmentation
    /// (suffix harmony follows the final component: antigeenissä, not *antigeenissa).
    #[serde(default)]
    pub front_harmony: Option<bool>,
    /// Foreign/letter-word citation style (parfait'n, cd:n), from the citation
    /// overrides sidecar.
    #[serde(default)]
    pub citation: Option<keinontolibrary_core::ForeignCitation>,
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

/// Header magic: "KEIN" — identifies a keinontolibrary artifact and rejects unrelated
/// files before bincode ever sees them.
const MAGIC: [u8; 4] = *b"KEIN";
/// On-disk format version. Bump when the layout changes incompatibly; old files then
/// fail loudly instead of deserializing into garbage.
const FORMAT_VERSION: u8 = 1;
/// `MAGIC` (4) + version (1) + CRC32 of the payload (4, little-endian).
const HEADER_LEN: usize = 9;

impl Artifact {
    /// Encode to the framed on-disk bytes: `KEIN` magic, a format-version byte, a CRC32
    /// of the bincode payload, then the payload.
    ///
    /// # Errors
    /// Propagates `bincode` serialization failures.
    pub fn encode(&self) -> Result<Vec<u8>, bincode::Error> {
        let payload = bincode::serialize(self)?;
        let crc = crc32fast::hash(&payload);
        let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
        out.extend_from_slice(&MAGIC);
        out.push(FORMAT_VERSION);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&payload);
        Ok(out)
    }

    /// Decode framed artifact bytes, validating the magic, version and CRC32 first.
    ///
    /// # Errors
    /// Returns an error if the bytes are too short, not a keinontolibrary artifact, a
    /// future format version, corrupt (CRC mismatch), undecodable, or carry metadata
    /// inconsistent with the contents.
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        let bad = |msg: &str| io::Error::new(io::ErrorKind::InvalidData, msg.to_owned());
        if bytes.len() < HEADER_LEN {
            return Err(bad("artifact too short (truncated header)"));
        }
        if bytes[..4] != MAGIC {
            return Err(bad("not a keinontolibrary artifact (bad magic)"));
        }
        let version = bytes[4];
        if version != FORMAT_VERSION {
            return Err(bad(&format!(
                "unsupported artifact format version {version} (expected {FORMAT_VERSION})"
            )));
        }
        let stored_crc = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]);
        let payload = &bytes[HEADER_LEN..];
        if crc32fast::hash(payload) != stored_crc {
            return Err(bad("artifact checksum mismatch (corrupt or truncated)"));
        }
        let artifact: Self = bincode::deserialize(payload).map_err(io::Error::other)?;
        // The header survived; sanity-check the metadata against the actual contents so a
        // stale n_lemmas can't mislead callers that trust it.
        if artifact.meta.n_lemmas as usize != artifact.lemmas.len() {
            return Err(bad(&format!(
                "artifact metadata mismatch: n_lemmas={} but {} lemma records",
                artifact.meta.n_lemmas,
                artifact.lemmas.len()
            )));
        }
        Ok(artifact)
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
        Self::decode(&bytes)
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
                adjective: false,
                front_harmony: None,
                citation: None,
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
        assert_eq!(&bytes[..4], b"KEIN");
        let back = Artifact::decode(&bytes).unwrap();
        assert_eq!(back.lemmas[0].lemma, "talo");
        assert_eq!(
            back.lemmas[0].paradigms[0].slots[0].variants,
            vec!["talossa"]
        );
    }

    fn sample_bytes() -> Vec<u8> {
        Artifact {
            meta: Meta {
                version: "test".into(),
                n_lemmas: 1,
                n_forms: 1,
                ..Meta::default()
            },
            lemmas: vec![LemmaRecord {
                lemma: "talo".into(),
                adjective: false,
                front_harmony: None,
                citation: None,
                paradigms: vec![],
            }],
        }
        .encode()
        .unwrap()
    }

    // Corrupt/foreign/truncated inputs must error, never panic or deserialize garbage.
    #[test]
    fn decode_rejects_corruption() {
        assert!(Artifact::decode(b"").is_err());
        assert!(Artifact::decode(b"not an artifact at all").is_err());
        // Truncated payload.
        let bytes = sample_bytes();
        assert!(Artifact::decode(&bytes[..bytes.len() - 5]).is_err());
        // Bit flip in the payload trips the CRC.
        let mut flipped = sample_bytes();
        let last = flipped.len() - 1;
        flipped[last] ^= 0xff;
        assert!(Artifact::decode(&flipped).is_err());
        // Wrong format version.
        let mut wrong_ver = sample_bytes();
        wrong_ver[4] = 99;
        assert!(Artifact::decode(&wrong_ver).is_err());
    }

    #[test]
    fn decode_rejects_metadata_mismatch() {
        // Encode an artifact whose n_lemmas lies about the record count.
        let art = Artifact {
            meta: Meta {
                n_lemmas: 7,
                ..Meta::default()
            },
            lemmas: vec![],
        };
        let bytes = art.encode().unwrap();
        assert!(Artifact::decode(&bytes).is_err());
    }
}
