//! QA dump: every artifact lemma × paradigm × (number, case) slot, three ways — the full
//! engine (lookup → rule fallback, i.e. what users get), the rule engine alone, and the
//! corpus attestation — one JSON line per slot. The offline QA loop (`scripts/qa/`) joins
//! this with a batch Voikko verdict to triage failures.
//!
//! Run: `cargo run --release -p keinontolibrary-ingest --bin keinontolibrary-qa-dump`
//! Env: `KEINONTO_ARTIFACT` (in, default `data/artifact/keinontolibrary.bin`),
//!      `KEINONTO_QA_DUMP` (out, default `qa/generated.jsonl`).

use std::io::{BufWriter, Write};
use std::path::PathBuf;

use keinontolibrary_core::{Case, Engine, Forms, Generator, Number, ParadigmRef, Status};
use keinontolibrary_data::{slot_index, Artifact, LookupData};
use keinontolibrary_rules::RuleEngine;
use rayon::prelude::*;
use serde_json::{json, Value};

fn status_name(s: Status) -> &'static str {
    match s {
        Status::Present => "present",
        Status::Rare => "rare",
        Status::Missing => "missing",
    }
}

fn source_name(s: keinontolibrary_core::Source) -> &'static str {
    match s {
        keinontolibrary_core::Source::Lookup => "lookup",
        keinontolibrary_core::Source::Generated => "generated",
        keinontolibrary_core::Source::Overlay => "overlay",
    }
}

fn engine_json(f: &Forms) -> Value {
    json!({
        "variants": f.variants,
        "status": status_name(f.status),
        "source": source_name(f.source),
    })
}

#[derive(Default)]
struct Counts {
    slots: u64,
    engine_errors: u64,
    rules_supported: u64,
    corpus_attested: u64,
}

impl Counts {
    fn add(mut self, other: &Counts) -> Counts {
        self.slots += other.slots;
        self.engine_errors += other.engine_errors;
        self.rules_supported += other.rules_supported;
        self.corpus_attested += other.corpus_attested;
        self
    }
}

fn main() -> std::io::Result<()> {
    let artifact_path = std::env::var("KEINONTO_ARTIFACT")
        .unwrap_or_else(|_| "data/artifact/keinontolibrary.bin".to_owned());
    let out_path = std::env::var("KEINONTO_QA_DUMP")
        .map_or_else(|_| PathBuf::from("qa/generated.jsonl"), PathBuf::from);

    let artifact = Artifact::read_from(&artifact_path)?;
    let lemmas = artifact.lemmas.clone();
    let engine = Engine::builder()
        .lookup(Box::new(LookupData::from_artifact(artifact)))
        .generator(Box::new(RuleEngine::new()))
        .build();
    let rules = RuleEngine::new();

    let (chunks, counts): (Vec<String>, Vec<Counts>) = lemmas
        .par_iter()
        .map(|lemma| {
            let mut out = String::new();
            let mut counts = Counts::default();
            for paradigm in &lemma.paradigms {
                let reference = ParadigmRef::new(None, paradigm.tn)
                    .with_av(paradigm.av)
                    .with_adjective(lemma.adjective)
                    .with_front_harmony(lemma.front_harmony);
                for number in Number::ALL {
                    for case in Case::ALL {
                        counts.slots += 1;
                        let corpus = paradigm
                            .slots
                            .iter()
                            .find(|s| s.slot == slot_index(number, case));
                        // decline_with() bypasses the engine's compound routing, so for
                        // single-paradigm lemmas fall back to decline() on error — that
                        // is what users get (tn50/51 compounds, harmony overrides).
                        let declined = engine
                            .decline_with(&lemma.lemma, number, case, &reference)
                            .or_else(|e| {
                                if lemma.paradigms.len() == 1 {
                                    engine.decline(&lemma.lemma, number, case)
                                } else {
                                    Err(e)
                                }
                            });
                        let generated = rules.generate(&lemma.lemma, &reference, number, case);
                        counts.engine_errors += u64::from(declined.is_err());
                        counts.rules_supported += u64::from(generated.is_some());
                        counts.corpus_attested += u64::from(corpus.is_some());
                        let row = json!({
                            "lemma": lemma.lemma,
                            "tn": paradigm.tn,
                            "av": paradigm.av,
                            "rare": paradigm.rare,
                            "number": number.name(),
                            "case": case.name(),
                            "engine": declined.as_ref().map_or(Value::Null, engine_json),
                            "engine_error": declined
                                .as_ref()
                                .err()
                                .map_or(Value::Null, |e| json!(e.to_string())),
                            "rules": generated.as_ref().map_or(Value::Null, |f| {
                                json!({
                                    "variants": f.variants,
                                    "status": status_name(f.status),
                                })
                            }),
                            "corpus": corpus.map_or(Value::Null, |s| {
                                json!({
                                    "variants": s.variants,
                                    "status": status_name(s.status),
                                })
                            }),
                        });
                        out.push_str(&row.to_string());
                        out.push('\n');
                    }
                }
            }
            (out, counts)
        })
        .unzip();

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut w = BufWriter::new(std::fs::File::create(&out_path)?);
    for chunk in &chunks {
        w.write_all(chunk.as_bytes())?;
    }
    w.flush()?;

    let total = counts.iter().fold(Counts::default(), Counts::add);
    eprintln!("lemmas:          {}", lemmas.len());
    eprintln!("slots dumped:    {}", total.slots);
    eprintln!("engine errors:   {}", total.engine_errors);
    eprintln!("rules supported: {}", total.rules_supported);
    eprintln!("corpus attested: {}", total.corpus_attested);
    eprintln!("wrote {}", out_path.display());
    Ok(())
}
