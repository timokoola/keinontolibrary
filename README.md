# keinontolibrary

A fast, embeddable Rust library that declines **simple Finnish nouns**.

```text
decline("hevonen", Number::Singular, Case::Inessive) -> ["hevosessa"]
```

It is **data-backed** (precomputed forms from a Voikko-generated corpus) with a
**rule-based fallback** for the Kotus declension classes (taivutustyypit 1–49 with
consonant gradation), and is validated against a ~400k-form corpus so test coverage is
near-exhaustive by construction.

> **Status:** the data-backed lookup path is complete and runnable — library, CLI,
> HTTP service, overlay, and a <10 MB container all work. The rule-engine fallback
> (Kotus classes 1–49) and the Cloudflare Workers target are the remaining work (see
> [Roadmap](#roadmap)). The working/repo name is `keinontolibrary` (`keinonto` = the
> instructive case); the crate prefix is `keinontolibrary-*`.

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

## Running

The CLI and server read the artifact at `data/artifact/keinontolibrary.bin` (override with
`KEINONTO_ARTIFACT`) and an overlay at `data/overlay.jsonl` (`KEINONTO_OVERLAY`).

```sh
# CLI
cargo run -p keinontolibrary-cli -- decline hevonen --number singular --case inessive
cargo run -p keinontolibrary-cli -- paradigm talo
cargo run -p keinontolibrary-cli -- add --lemma uudissana --tn 9 \
    --number singular --case inessive --forms uudissanassa
cargo run -p keinontolibrary-cli -- validate

# HTTP service
cargo run -p keinontolibrary-server          # listens on 0.0.0.0:8080
curl 'localhost:8080/decline?word=hevonen&number=singular&case=inessive'
curl 'localhost:8080/paradigm?word=talo'
```

Endpoints: `GET /decline`, `GET /paradigm` (both accept `&hn=&tn=` to disambiguate
homonyms), `GET /healthz`, `GET /about`, and bearer-auth `POST /admin/add` &
`POST /admin/override` (enabled only when `KEINONTO_ADMIN_TOKEN` is set).

### Container

```sh
cargo run -p keinontolibrary-ingest          # produce data/artifact/keinontolibrary.bin
docker build -t keinontolibrary .            # ~10 MB static-musl scratch image
docker run -p 8080:8080 keinontolibrary
```

## Roadmap

- ✅ Data-backed lookup: core API, ingest, packed artifact, corpus round-trip gate, CLI,
  HTTP service, overlay, container, FFI scaffold.
- 🟡 **Rule engine** (`keinontolibrary-rules`): high-frequency classes (1, 2, 5, 6, 9, 10,
  12, 38, 39, 40, 48) + gradation A–M, wired in as the live fallback behind the lookup.
  **95.8% rule↔corpus parity** on those classes (`--test parity`). Remaining: the other
  classes and the long tail (loanword harmony, `-ain` variants, an exception registry) to
  reach the ≥99.9% gate.
- ⬜ **Cloudflare Workers target** (`keinontolibrary-worker`): edge deployment backed by
  KV/D1/R2. The storage abstraction it needs already exists as the `FormStore` trait in
  `keinontolibrary-core`.

## Data provenance & attribution

This project bundles data derived from the Kotus *Nykysuomen sanalista 2024* (CC BY 4.0)
and Voikko. See [`LICENSING.md`](LICENSING.md) for full attribution and the open question
about redistributing Voikko-derived forms.

## License

Source code: MIT (see [`LICENSE`](LICENSE)). Bundled data: see [`LICENSING.md`](LICENSING.md).
