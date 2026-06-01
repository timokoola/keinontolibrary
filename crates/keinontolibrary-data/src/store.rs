//! The runtime lookup store: loads an [`Artifact`] and serves forms via
//! [`keinontolibrary_core::FormStore`].

use std::collections::HashMap;
use std::io;
use std::path::Path;

use keinontolibrary_core::{Case, Engine, FormStore, Forms, ParadigmRef, Source, Status};

use crate::artifact::{slot_index, Artifact, LemmaRecord, Meta};

/// An in-memory, query-ready view over a packed [`Artifact`].
///
/// Loading deserializes the artifact once and builds a lemma → record index; lookups are
/// then a hash probe plus a small linear scan over a lemma's paradigms and slots.
#[derive(Debug)]
pub struct LookupData {
    meta: Meta,
    lemmas: Vec<LemmaRecord>,
    index: HashMap<String, usize>,
}

impl LookupData {
    /// Build from an already-decoded artifact.
    #[must_use]
    pub fn from_artifact(artifact: Artifact) -> Self {
        let index = artifact
            .lemmas
            .iter()
            .enumerate()
            .map(|(i, l)| (l.lemma.clone(), i))
            .collect();
        Self {
            meta: artifact.meta,
            lemmas: artifact.lemmas,
            index,
        }
    }

    /// Load from a packed artifact file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or decoded.
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self::from_artifact(Artifact::read_from(path)?))
    }

    /// Decode from in-memory bytes (e.g. an embedded artifact).
    ///
    /// # Errors
    /// Returns an error if the bytes cannot be decoded.
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        Ok(Self::from_artifact(
            Artifact::decode(bytes).map_err(io::Error::other)?,
        ))
    }

    /// Build/provenance metadata.
    #[must_use]
    pub fn meta(&self) -> &Meta {
        &self.meta
    }

    /// Number of lemmas.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lemmas.len()
    }

    /// Whether the store is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lemmas.is_empty()
    }

    fn record(&self, lemma: &str) -> Option<&LemmaRecord> {
        self.index.get(lemma).map(|&i| &self.lemmas[i])
    }
}

impl FormStore for LookupData {
    fn paradigms(&self, lemma: &str) -> Vec<ParadigmRef> {
        let Some(rec) = self.record(lemma) else {
            return Vec::new();
        };
        rec.paradigms
            .iter()
            .map(|p| ParadigmRef::new(None, p.tn).with_av(p.av))
            .collect()
    }

    fn forms(
        &self,
        lemma: &str,
        reference: &ParadigmRef,
        number: keinontolibrary_core::Number,
        case: Case,
    ) -> Option<Forms> {
        let rec = self.record(lemma)?;
        let paradigm = rec.paradigms.iter().find(|p| p.tn == reference.tn)?;
        let slot = slot_index(number, case);
        let record = paradigm.slots.iter().find(|s| s.slot == slot)?;

        let mut forms = match record.status {
            Status::Present => Forms::present(record.variants.clone(), Source::Lookup),
            Status::Rare => Forms::rare(record.variants.clone(), Source::Lookup),
            Status::Missing => Forms::missing(),
        };
        if let Some(ci) = record.coincides_with {
            forms.coincides_with = Case::ALL.get(usize::from(ci)).copied();
        }
        Some(forms)
    }
}

/// Convenience: build an [`Engine`] whose lookup store is the artifact at `path`.
///
/// # Errors
/// Returns an error if the artifact cannot be loaded.
pub fn load_engine(path: impl AsRef<Path>) -> io::Result<Engine> {
    Ok(Engine::builder()
        .lookup(Box::new(LookupData::load(path)?))
        .build())
}

/// An [`Engine`] plus a handle to its overlay and the artifact metadata, for surfaces
/// (CLI/server) that both query the engine and mutate the overlay.
#[derive(Debug)]
pub struct EngineBundle {
    /// The query engine (overlay → lookup → [rule fallback]).
    pub engine: Engine,
    /// A shared handle to the overlay, for admin add/override.
    pub overlay: crate::overlay::Overlay,
    /// Artifact provenance/metadata.
    pub meta: Meta,
}

/// Build an engine backed by the artifact at `artifact_path` plus a persistent overlay at
/// `overlay_path`.
///
/// # Errors
/// Returns an error if the artifact or overlay cannot be loaded.
pub fn build_engine(
    artifact_path: impl AsRef<Path>,
    overlay_path: impl AsRef<Path>,
) -> io::Result<EngineBundle> {
    let lookup = LookupData::load(artifact_path)?;
    let meta = lookup.meta().clone();
    let overlay = crate::overlay::Overlay::open(overlay_path)?;
    let engine = Engine::builder()
        .lookup(Box::new(lookup))
        .overlay(Box::new(overlay.clone()))
        .build();
    Ok(EngineBundle {
        engine,
        overlay,
        meta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{ParadigmRecord, SlotRecord};
    use keinontolibrary_core::Number;

    fn sample() -> LookupData {
        let artifact = Artifact {
            meta: Meta::default(),
            lemmas: vec![LemmaRecord {
                lemma: "talo".into(),
                paradigms: vec![ParadigmRecord {
                    tn: 1,
                    av: None,
                    rare: false,
                    slots: vec![
                        SlotRecord {
                            slot: slot_index(Number::Singular, Case::Inessive),
                            status: Status::Present,
                            variants: vec!["talossa".into()],
                            coincides_with: None,
                        },
                        SlotRecord {
                            slot: slot_index(Number::Singular, Case::Accusative),
                            status: Status::Present,
                            variants: vec!["talon".into()],
                            coincides_with: Some(u8::try_from(Case::Genitive.index()).unwrap()),
                        },
                    ],
                }],
            }],
        };
        LookupData::from_artifact(artifact)
    }

    #[test]
    fn serves_present_form() {
        let data = sample();
        let r = ParadigmRef::new(None, 1);
        let f = data
            .forms("talo", &r, Number::Singular, Case::Inessive)
            .unwrap();
        assert_eq!(f.variants, vec!["talossa"]);
        assert_eq!(f.source, Source::Lookup);
    }

    #[test]
    fn accusative_reports_coincidence() {
        let data = sample();
        let r = ParadigmRef::new(None, 1);
        let f = data
            .forms("talo", &r, Number::Singular, Case::Accusative)
            .unwrap();
        assert_eq!(f.coincides_with, Some(Case::Genitive));
    }

    #[test]
    fn absent_slot_is_none() {
        let data = sample();
        let r = ParadigmRef::new(None, 1);
        assert!(data
            .forms("talo", &r, Number::Plural, Case::Abessive)
            .is_none());
        assert!(data.paradigms("ankka").is_empty());
    }

    #[test]
    fn engine_over_store_declines() {
        let data = sample();
        let engine = Engine::builder().lookup(Box::new(data)).build();
        let f = engine
            .decline("talo", Number::Singular, Case::Inessive)
            .unwrap();
        assert_eq!(f.primary(), Some("talossa"));
    }
}
