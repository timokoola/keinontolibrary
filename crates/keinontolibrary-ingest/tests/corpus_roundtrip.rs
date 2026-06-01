//! The corpus round-trip gate: for every cleaned `(lemma, number, case, variant)` in our
//! reference corpus, assert `decline_with` returns it among the variants.
//!
//! This is the near-exhaustive coverage test. It requires the local source data and the
//! built artifact, both of which are gitignored, so it **skips gracefully** when they are
//! absent (e.g. in CI without the data). To run it:
//!
//! ```sh
//! cargo run --release -p keinontolibrary-ingest      # build the artifact
//! cargo test  -p keinontolibrary-ingest --test corpus_roundtrip -- --nocapture
//! ```

use std::path::{Path, PathBuf};

use keinontolibrary_core::Error;
use keinontolibrary_data::load_engine;
use keinontolibrary_ingest::voikko::parse_shard;
use rayon::prelude::*;

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn corpus_round_trips() {
    let data = data_dir();
    let artifact = data.join("artifact/keinontolibrary.bin");
    let voikko = data.join("sources/voikko");

    if !artifact.exists() || !voikko.exists() {
        eprintln!(
            "SKIP corpus_round_trips: missing {} or {}.\n      \
             Run `cargo run --release -p keinontolibrary-ingest` first.",
            artifact.display(),
            voikko.display()
        );
        return;
    }

    let engine = load_engine(&artifact).expect("load artifact");

    let mut shards: Vec<PathBuf> = std::fs::read_dir(&voikko)
        .expect("read voikko dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    shards.sort();
    assert!(!shards.is_empty(), "no shards found");

    // Parse all shards in parallel into clean forms.
    let forms: Vec<_> = shards
        .par_iter()
        .flat_map(|p| parse_shard(&std::fs::read_to_string(p).unwrap_or_default()))
        .collect();
    assert!(!forms.is_empty(), "no forms parsed");

    let total = forms.len();
    let mut matched = 0usize;
    let mut dropped = 0usize; // lemma/paradigm not in artifact (compound/adjective/tn-mismatch)
    let mut mismatches: Vec<String> = Vec::new();

    for f in &forms {
        let reference = keinontolibrary_core::ParadigmRef::new(None, f.tn);
        match engine.decline_with(&f.lemma, f.number, f.case, &reference) {
            Ok(forms) if forms.variants.iter().any(|v| v == &f.form) => matched += 1,
            Ok(forms) => {
                if mismatches.len() < 25 {
                    mismatches.push(format!(
                        "{} {} {}: expected {:?} in {:?}",
                        f.lemma, f.number, f.case, f.form, forms.variants
                    ));
                }
            }
            Err(Error::UnknownWord(_)) => dropped += 1,
            Err(e) => {
                if mismatches.len() < 25 {
                    mismatches.push(format!("{} {} {}: {e}", f.lemma, f.number, f.case));
                }
            }
        }
    }

    let mismatch_count = total - matched - dropped;
    eprintln!(
        "corpus round-trip: {total} forms | matched {matched} | dropped(not-in-artifact) \
         {dropped} | mismatches {mismatch_count}"
    );
    for m in &mismatches {
        eprintln!("  MISMATCH: {m}");
    }

    assert_eq!(
        mismatch_count, 0,
        "{mismatch_count} corpus forms failed to round-trip"
    );
}
