//! Coverage harness over the FULL Kotus list — how many rows the live engine can decline,
//! including the ~72k rows Kotus leaves without a declension type (transparent compounds and
//! derivations). This is a different metric from the QA gate: the gate measures correctness
//! over the in-scope inventory; this measures runtime *reach* over the whole list (compound
//! splitter + class inference + numerals).
//!
//! A row counts as "declinable" if `engine.paradigm(lemma)` yields a non-empty plural
//! inessive (a representative oblique slot every real noun has).
//!
//! Run: `cargo run --release -p keinontolibrary-ingest --bin keinontolibrary-coverage`
//! Env: `KEINONTO_KOTUS` (in), `KEINONTO_ARTIFACT`, `KEINONTO_OVERLAY`,
//!      `KEINONTO_COVERAGE_MISSES` (out, default `qa/coverage-misses.tsv`).

// Counts are word tallies (< 10^6); the f64 percentage casts cannot lose precision.
#![allow(clippy::cast_precision_loss)]

use std::collections::BTreeMap;
use std::io::{BufWriter, Write};

use keinontolibrary_core::{Case, Number};
use keinontolibrary_data::build_engine;

fn env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

/// `(declinable, total)` per bucket key.
#[derive(Default, Clone, Copy)]
struct Tally {
    ok: u64,
    total: u64,
}

fn main() -> std::io::Result<()> {
    let kotus_path = env("KEINONTO_KOTUS", "data/sources/nykysuomensanalista2024.txt");
    let artifact = env("KEINONTO_ARTIFACT", "data/artifact/keinontolibrary.bin");
    let overlay = env("KEINONTO_OVERLAY", "data/overlay.jsonl");
    let miss_path = env("KEINONTO_COVERAGE_MISSES", "qa/coverage-misses.tsv");

    let bundle = build_engine(&artifact, &overlay)
        .map_err(|e| std::io::Error::other(format!("loading artifact {artifact}: {e}")))?;
    let engine = &bundle.engine;

    let text = std::fs::read_to_string(&kotus_path)?;
    // Buckets keyed by "(sanaluokka, has-tn)" so we can see where the misses concentrate.
    let mut buckets: BTreeMap<String, Tally> = BTreeMap::new();
    let mut misses: Vec<(String, String)> = Vec::new(); // (lemma, sanaluokka)
    let mut total = Tally::default();
    let mut indeclinable = 0u64; // tn99 rows skipped (out of scope)

    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue; // header / blanks
        }
        let cols: Vec<&str> = line.split('\t').collect();
        let lemma = cols.first().copied().unwrap_or("").trim();
        if lemma.is_empty() {
            continue;
        }
        let sanaluokka = cols.get(2).copied().unwrap_or("").trim();
        let tn_field = cols.get(3).map_or("", |t| t.trim());
        let has_tn = !tn_field.is_empty();
        // Only the nominal word classes are in our remit (verbs are out of scope).
        if !matches!(sanaluokka, "substantiivi" | "adjektiivi" | "numeraali") {
            continue;
        }
        // Kotus tn99 = "no inflection": indeclinable by definition (alias, aprilli, ensi,
        // the colloquial -isen approximates). Out of scope, like verbs — not a declension
        // candidate, so excluded from the denominator.
        let primary_tn = tn_field.split([',', ' ', '*', '(']).next().unwrap_or("");
        if primary_tn == "99" {
            indeclinable += 1;
            continue;
        }
        // Declinable iff the engine yields a real oblique form (plural inessive — present
        // for every count noun, the natural primary slot for plurale tantum). An *ambiguous*
        // lemma (several paradigms) is still declinable: pick the first paradigm, exactly as
        // a caller would with `--tn`. Only a genuinely unresolved lemma falls to the
        // compound/inference path of `paradigm()`.
        // A real oblique form in EITHER number: nouns have both, plurale tantum only the
        // plural, numerals only the singular (kahdeksassakymmenessä). Any one suffices.
        let has_plural_inessive = |p: &keinontolibrary_core::Paradigm| {
            !p.get(Number::Plural, Case::Inessive).variants.is_empty()
                || !p.get(Number::Singular, Case::Inessive).variants.is_empty()
        };
        let declinable = if let Ok(p) = engine.paradigm(lemma) {
            has_plural_inessive(&p)
        } else {
            // `paradigm()` errors on an *ambiguous* lemma (several paradigms); it is still
            // declinable — pick the first paradigm, as a caller would with `--tn`. (Unknown
            // lemmas already exhausted the compound/inference path inside `paradigm()`.)
            let refs = engine.resolve(&keinontolibrary_core::normalize(lemma));
            refs.len() > 1
                && engine
                    .paradigm_with(lemma, &refs[0])
                    .ok()
                    .is_some_and(|p| has_plural_inessive(&p))
        };

        let key = format!(
            "{sanaluokka:<12} {}",
            if has_tn { "with-tn" } else { "NO-tn  " }
        );
        let b = buckets.entry(key).or_default();
        b.total += 1;
        total.total += 1;
        if declinable {
            b.ok += 1;
            total.ok += 1;
        } else {
            misses.push((lemma.to_owned(), sanaluokka.to_owned()));
        }
    }

    // Write the misses for inspection / category analysis.
    if let Some(parent) = std::path::Path::new(&miss_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut w = BufWriter::new(std::fs::File::create(&miss_path)?);
    for (lemma, sl) in &misses {
        writeln!(w, "{lemma}\t{sl}")?;
    }
    w.flush()?;

    let pct = |t: &Tally| {
        if t.total == 0 {
            100.0
        } else {
            100.0 * t.ok as f64 / t.total as f64
        }
    };
    eprintln!("Kotus nominal rows declinable by the live engine:\n");
    for (key, t) in &buckets {
        eprintln!("  {key}  {:>6}/{:<6}  {:5.1}%", t.ok, t.total, pct(t));
    }
    eprintln!(
        "\n  TOTAL              {:>6}/{:<6}  {:5.1}%",
        total.ok,
        total.total,
        pct(&total)
    );
    eprintln!("\n  {} misses -> {miss_path}", misses.len());
    eprintln!("  ({indeclinable} tn99 indeclinable rows excluded — out of scope)");
    Ok(())
}
