//! `keinontolibrary-ingest` — offline pipeline turning the Kotus word list and our
//! reference corpus (Voikko-format JSONL) into the packed lookup artifact (see
//! [`keinontolibrary_data::Artifact`]).
//!
//! The entry point is [`run`]; [`kotus`] and [`voikko`] hold the source parsers.

pub mod kotus;
pub mod pipeline;
pub mod voikko;

pub use pipeline::{run, Config, Report};
