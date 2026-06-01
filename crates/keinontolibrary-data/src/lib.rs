//! `keinontolibrary-data` — the packed lookup artifact plus its loader.
//!
//! [`artifact`] defines the on-disk format (written by `keinontolibrary-ingest`).
//! [`store`] loads it and serves forms via [`keinontolibrary_core::FormStore`].

pub mod artifact;
pub mod overlay;
pub mod store;

pub use artifact::{
    slot_index, slot_parts, Artifact, LemmaRecord, Meta, ParadigmRecord, SlotRecord,
};
pub use overlay::{Overlay, OverlayEntry};
pub use store::{build_engine, load_engine, EngineBundle, LookupData};
