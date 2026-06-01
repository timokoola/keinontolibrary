//! `keinontolibrary-data` — the packed lookup artifact plus its loader.
//!
//! [`artifact`] defines the on-disk format (written by `keinontolibrary-ingest`). The
//! zero-copy/in-memory loader implementing [`keinontolibrary_core::FormStore`] lands in
//! Phase 3.

pub mod artifact;

pub use artifact::{
    slot_index, slot_parts, Artifact, LemmaRecord, Meta, ParadigmRecord, SlotRecord,
};
