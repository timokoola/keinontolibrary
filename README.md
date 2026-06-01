# keinontolibrary

A fast, embeddable Rust library that declines **simple Finnish nouns**.

```text
decline("hevonen", Number::Singular, Case::Inessive) -> ["hevosessa"]
```

It is **data-backed** (precomputed forms from a Voikko-generated corpus) with a
**rule-based fallback** for the Kotus declension classes (taivutustyypit 1–49 with
consonant gradation), and is validated against a ~400k-form corpus so test coverage is
near-exhaustive by construction.

> **Status:** under construction. The working/repo name is `keinontolibrary`
> (`keinonto` = the instructive case). The crate prefix is `keinontolibrary-*`.

## Workspace layout

| Crate                     | Purpose                                                        |
| ------------------------- | ------------------------------------------------------------- |
| `keinontolibrary-core`    | Public API: `Case`/`Number` enums, `Forms`, `decline`, `paradigm`. |
| `keinontolibrary-rules`   | Rule generator for Kotus classes 1–49 + gradation A–M.        |
| `keinontolibrary-ingest`  | Offline pipeline: Kotus + Voikko → packed lookup artifact.    |
| `keinontolibrary-data`    | Packed artifact + zero-copy (mmap/embedded) loader.           |
| `keinontolibrary-server`  | axum HTTP service (the container deployment).                 |
| `keinontolibrary-cli`     | CLI: `decline`, `paradigm`, `add`, `override`, `validate`.    |
| `keinontolibrary-ffi`     | FFI scaffold (UniFFI/Swift; feature-gated wasm + PyO3).       |

## Scope (v1.0)

- **In:** simple (non-compound) nouns, declension classes 1–49 with gradation, all 15
  cases, both numbers, multi-paradigm homonyms.
- **Out:** verbs, comparison, compound nouns (Kotus types 50/51), a general
  possessive-suffix system, adjectives, class inference for unlisted words.

## Building

Requires a stable Rust toolchain.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### Data ingest

Source data is **not committed** (it is large and separately licensed — see
[`LICENSING.md`](LICENSING.md)). Fetch it into `data/sources/`, then:

```sh
cargo run -p keinontolibrary-ingest    # Kotus + Voikko -> data/artifact/
```

- **Kotus list** (CC BY 4.0): <https://kaino.kotus.fi/lataa/nykysuomensanalista2024.txt>
- **Voikko JSONL corpus**: bucket `gs://REDACTED-CORPUS-BUCKET/` (1201 shards).

## Data provenance & attribution

This project bundles data derived from the Kotus *Nykysuomen sanalista 2024* (CC BY 4.0)
and Voikko. See [`LICENSING.md`](LICENSING.md) for full attribution and the open question
about redistributing Voikko-derived forms.

## License

Source code: MIT (see [`LICENSE`](LICENSE)). Bundled data: see [`LICENSING.md`](LICENSING.md).
