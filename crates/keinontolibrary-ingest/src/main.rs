//! CLI wrapper around the ingest pipeline.
//!
//! Paths default to the repository's `data/` layout and can be overridden via environment
//! variables: `KEINONTO_KOTUS`, `KEINONTO_VOIKKO`, `KEINONTO_ARTIFACT`, `KEINONTO_REPORT`,
//! `KEINONTO_HARMONY`.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use keinontolibrary_ingest::{run, Config};

fn env_path(key: &str, default: &str) -> PathBuf {
    std::env::var_os(key).map_or_else(|| PathBuf::from(default), PathBuf::from)
}

fn main() -> Result<()> {
    let config = Config {
        kotus_path: env_path("KEINONTO_KOTUS", "data/sources/nykysuomensanalista2024.txt"),
        voikko_dir: env_path("KEINONTO_VOIKKO", "data/sources/voikko"),
        artifact_path: env_path("KEINONTO_ARTIFACT", "data/artifact/keinontolibrary.bin"),
        report_path: env_path("KEINONTO_REPORT", "ingest-report.txt"),
        harmony_path: env_path("KEINONTO_HARMONY", "data/harmony-overrides.jsonl"),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    };

    eprintln!("ingesting:");
    eprintln!("  kotus:    {}", config.kotus_path.display());
    eprintln!("  voikko:   {}", config.voikko_dir.display());
    eprintln!("  artifact: {}", config.artifact_path.display());

    let started = Instant::now();
    let report = run(&config)?;
    let elapsed = started.elapsed();

    print!("{}", report_to_string(&report));
    eprintln!("done in {:.1}s", elapsed.as_secs_f64());
    Ok(())
}

fn report_to_string(report: &keinontolibrary_ingest::Report) -> String {
    format!(
        "kotus lemmas: {}\nreference forms kept: {}\nlemmas with forms: {}\nlemmas without forms: {}\ntotal forms: {}\n",
        report.kotus_lemmas,
        report.reference_forms_kept,
        report.lemmas_with_forms,
        report.lemmas_without_forms,
        report.total_forms,
    )
}
