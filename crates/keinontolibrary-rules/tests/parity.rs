//! Rule↔lookup parity harness.
//!
//! For every corpus-attested slot of every lemma in the artifact, generate the form with
//! the rule engine and check whether the corpus's primary form is among the generated
//! variants. Reports the match rate per declension class and overall, for the classes the
//! rule engine supports.
//!
//! Requires the built artifact (gitignored), so it **skips** when absent. Run with:
//! `cargo run --release -p keinontolibrary-ingest` then
//! `cargo test --release -p keinontolibrary-rules --test parity -- --nocapture`.
#![allow(clippy::cast_precision_loss)] // percentage reporting in a test

use std::collections::BTreeMap;
use std::path::Path;

use keinontolibrary_core::{Generator, ParadigmRef};
use keinontolibrary_data::{slot_parts, Artifact};
use keinontolibrary_rules::{Exceptions, RuleEngine};

/// CI cap on the exception registry, measured in **distinct irregular lemmas** — the
/// meaningful unit. A genuine irregular needs many slots (aika alone is 19), so capping
/// rows punishes depth; capping lemmas still flags the real smell (a systematic rule gap
/// patched word-by-word) while letting each true irregular be fully specified.
const LEMMA_CAP: usize = 64;
/// Secondary backstop on raw slot count, to catch a runaway file.
const ENTRY_CAP: usize = 1500;

#[test]
fn exception_registry_within_cap() {
    let ex = Exceptions::load();
    let lemmas = ex.lemma_count();
    let entries = ex.len();
    eprintln!(
        "exception registry: {lemmas} lemmas / {entries} slots (caps: {LEMMA_CAP} lemmas, {ENTRY_CAP} slots)"
    );
    assert!(
        lemmas <= LEMMA_CAP,
        "exception registry has {lemmas} irregular lemmas, over the cap of {LEMMA_CAP} \
         — if these are systematic, fix the rule instead of patching per-lemma"
    );
    assert!(
        entries <= ENTRY_CAP,
        "exception registry has {entries} slots, over the backstop of {ENTRY_CAP}"
    );
}

#[derive(Default, Clone, Copy)]
struct Tally {
    matched: usize,
    total: usize,
}

#[test]
fn rule_lookup_parity() {
    let artifact_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/artifact/keinontolibrary.bin");
    if !artifact_path.exists() {
        eprintln!(
            "SKIP parity: {} missing (run the ingest first).",
            artifact_path.display()
        );
        return;
    }
    let artifact = Artifact::read_from(&artifact_path).expect("load artifact");
    // Generate through the RuleEngine so the exception registry is applied.
    let engine = RuleEngine::new();

    let mut per_class: BTreeMap<u8, Tally> = BTreeMap::new();
    let mut overall = Tally::default();
    let mut unsupported_slots = 0usize;
    let mut examples: Vec<String> = Vec::new();

    for lemma in &artifact.lemmas {
        for paradigm in &lemma.paradigms {
            for slot in &paradigm.slots {
                if slot.variants.is_empty() {
                    continue;
                }
                let (number, case) = slot_parts(slot.slot);
                let attested = &slot.variants[0]; // corpus primary
                let reference = ParadigmRef::new(None, paradigm.tn)
                    .with_av(paradigm.av)
                    .with_adjective(lemma.adjective)
                    .with_front_harmony(lemma.front_harmony);
                match engine.generate(&lemma.lemma, &reference, number, case) {
                    None => unsupported_slots += 1,
                    Some(forms) => {
                        let generated = &forms.variants;
                        let t = per_class.entry(paradigm.tn).or_default();
                        t.total += 1;
                        overall.total += 1;
                        if generated.iter().any(|g| g == attested) {
                            t.matched += 1;
                            overall.matched += 1;
                        } else if examples.len() < 30 {
                            examples.push(format!(
                                "tn{} {} {number} {case}: corpus {attested:?} not in {generated:?}",
                                paradigm.tn, lemma.lemma
                            ));
                        }
                    }
                }
            }
        }
    }

    eprintln!("\nRule↔lookup parity (supported classes):");
    eprintln!(
        "  {:>4}  {:>8}  {:>8}  {:>6}",
        "tn", "matched", "total", "rate"
    );
    for (tn, t) in &per_class {
        if t.total == 0 {
            continue;
        }
        let pct = 100.0 * t.matched as f64 / t.total as f64;
        eprintln!("  {tn:>4}  {:>8}  {:>8}  {pct:>5.1}%", t.matched, t.total);
    }
    let overall_pct = if overall.total == 0 {
        0.0
    } else {
        100.0 * overall.matched as f64 / overall.total as f64
    };
    eprintln!(
        "  overall: {}/{} = {overall_pct:.2}%   (slots in unsupported classes: {unsupported_slots})",
        overall.matched, overall.total
    );
    eprintln!("\nsample mismatches:");
    for e in &examples {
        eprintln!("  {e}");
    }

    // Gate: the supported high-frequency classes should reproduce the corpus well.
    assert!(
        overall_pct >= 90.0,
        "rule parity {overall_pct:.2}% below 90% on supported classes"
    );
}
