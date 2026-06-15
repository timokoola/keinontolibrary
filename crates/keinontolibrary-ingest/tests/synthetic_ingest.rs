//! End-to-end ingest on a tiny synthetic corpus, in a tempdir. Unlike
//! `corpus_roundtrip.rs` (which skips when the real ~1 GB sources are absent), this runs
//! everywhere — including CI — so orchestration bugs between the pipeline stages
//! (`list_shards`, `parse_all`, `group_forms`, `build_slots`) and the artifact format
//! are caught on every push.

use std::path::PathBuf;

use keinontolibrary_data::Artifact;
use keinontolibrary_ingest::{run, Config};

/// A unique scratch dir under the system temp, removed on drop.
struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("kl-ingest-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write(path: &std::path::Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

fn config(scratch: &Scratch) -> Config {
    let d = &scratch.0;
    Config {
        kotus_path: d.join("kotus.txt"),
        voikko_dir: d.join("voikko"),
        artifact_path: d.join("artifact.bin"),
        report_path: d.join("report.txt"),
        harmony_path: d.join("missing-harmony.jsonl"),
        comitative_path: d.join("missing-comitative.jsonl"),
        citation_path: d.join("missing-citation.jsonl"),
        version: "test".into(),
    }
}

#[test]
fn ingest_builds_a_valid_artifact_from_synthetic_sources() {
    let scratch = Scratch::new("ok");
    let cfg = config(&scratch);
    write(
        &cfg.kotus_path,
        "Hakusana\tHomonymia\tSanaluokka\tTaivutustiedot\n\
         talo\t\tsubstantiivi\t1\n\
         kissa\t\tsubstantiivi\t9\n",
    );
    write(
        &cfg.voikko_dir.join("shard1.jsonl"),
        "{\"BASEFORM\":\"talo\",\"tn\":1,\"av\":\"_\",\"CLASS\":\"nimisana\",\"NUMBER\":\"singular\",\"SIJAMUOTO\":\"sisaolento\",\"BOOKWORD\":\"talossa\"}\n\
         {\"BASEFORM\":\"kissa\",\"tn\":9,\"av\":\"_\",\"CLASS\":\"nimisana\",\"NUMBER\":\"singular\",\"SIJAMUOTO\":\"omanto\",\"BOOKWORD\":\"kissan\"}\n",
    );

    let report = run(&cfg).expect("ingest run");
    assert_eq!(report.kotus_lemmas, 2);
    assert!(report.reference_forms_kept >= 2);

    // The artifact loads through the framed decoder (magic + CRC + metadata check).
    let artifact = Artifact::read_from(&cfg.artifact_path).expect("read framed artifact");
    assert_eq!(artifact.meta.n_lemmas as usize, artifact.lemmas.len());
    let talo = artifact.lemmas.iter().find(|l| l.lemma == "talo").unwrap();
    assert_eq!(talo.paradigms[0].tn, 1);
}

#[test]
fn ingest_is_deterministic() {
    let scratch = Scratch::new("det");
    let cfg = config(&scratch);
    write(
        &cfg.kotus_path,
        "Hakusana\tHomonymia\tSanaluokka\tTaivutustiedot\n\
         talo\t\tsubstantiivi\t1\n\
         auto\t\tsubstantiivi\t1\n\
         kissa\t\tsubstantiivi\t9\n",
    );
    // Two shards, out of name order on purpose: the artifact must be byte-identical
    // across runs regardless of parallel parse order (the shard-sort guarantee).
    write(
        &cfg.voikko_dir.join("b.jsonl"),
        "{\"BASEFORM\":\"auto\",\"tn\":1,\"av\":\"_\",\"CLASS\":\"nimisana\",\"NUMBER\":\"plural\",\"SIJAMUOTO\":\"sisaolento\",\"BOOKWORD\":\"autoissa\"}\n",
    );
    write(
        &cfg.voikko_dir.join("a.jsonl"),
        "{\"BASEFORM\":\"talo\",\"tn\":1,\"av\":\"_\",\"CLASS\":\"nimisana\",\"NUMBER\":\"singular\",\"SIJAMUOTO\":\"sisaolento\",\"BOOKWORD\":\"talossa\"}\n",
    );

    run(&cfg).expect("first ingest");
    let first = std::fs::read(&cfg.artifact_path).unwrap();
    run(&cfg).expect("second ingest");
    let second = std::fs::read(&cfg.artifact_path).unwrap();
    assert_eq!(first, second, "artifact must be reproducible byte-for-byte");
}

#[test]
fn empty_shard_dir_is_an_error() {
    let scratch = Scratch::new("empty");
    let cfg = config(&scratch);
    write(
        &cfg.kotus_path,
        "Hakusana\tHomonymia\tSanaluokka\tTaivutustiedot\ntalo\t\tsubstantiivi\t1\n",
    );
    std::fs::create_dir_all(&cfg.voikko_dir).unwrap(); // exists but holds no shards
    assert!(run(&cfg).is_err());
}
