# keinontolibrary

A fast, embeddable Rust library that declines **simple Finnish nouns**.

```text
decline("hevonen", Number::Singular, Case::Inessive) -> ["hevosessa"]
```

It is **data-backed** (precomputed forms from our reference corpus — collected over 3 years and labeled with Voikko) with a
**rule-based fallback** for the Kotus declension classes (taivutustyypit 1–49 with
consonant gradation), and is validated against a ~400k-form corpus so test coverage is
near-exhaustive by construction.

> **Status:** runnable end to end — library, CLI, HTTP service, overlay, FFI scaffold, and a
> <10 MB container. The rule-engine fallback covers 34 Kotus classes at ~98% agreement with
> the reference corpus; the remaining irregular classes and the Cloudflare Workers target are
> the open work (see [Roadmap](#roadmap)). The working/repo name is `keinontolibrary`
> (`keinonto` = the instructive case); the crate prefix is `keinontolibrary-*`.

## Workspace layout

| Crate                     | Purpose                                                        |
| ------------------------- | ------------------------------------------------------------- |
| `keinontolibrary-core`    | Public API: `Case`/`Number` enums, `Forms`, `decline`, `paradigm`. |
| `keinontolibrary-rules`   | Rule generator for Kotus classes 1–49 + gradation A–M.        |
| `keinontolibrary-ingest`  | Offline pipeline: Kotus + reference corpus → packed artifact. |
| `keinontolibrary-data`    | Packed artifact + zero-copy (mmap/embedded) loader.           |
| `keinontolibrary-server`  | axum HTTP service (the container deployment).                 |
| `keinontolibrary-cli`     | CLI: `decline`, `paradigm`, `add`, `override`, `validate`.    |
| `keinontolibrary-ffi`     | FFI scaffold (UniFFI/Swift; feature-gated wasm + PyO3).       |

## Scope (v1.0)

- **In:** all **nominals** — substantives, **adjectives**, and **numerals** — in declension
  classes 1–49 with gradation, all 15 cases, both numbers, multi-paradigm homonyms. Nominals
  share the declension classes, so the class-driven engine handles them uniformly (the ingest
  keeps every nominal word class, not just `substantiivi`). Plus the **core pronouns** (Kotus
  tn 101 — irregular: the personal `minä/sinä/hän/me/te/he` and the demonstratives
  `se/tämä/tuo/nämä/nuo/ne`), served from the Voikko-verified exception registry rather than
  the rule generator. Plus the **special numeral classes**: the productive **ordinals** (tn 45
  — `kolmas → kolmannen`, `kymmenes`, `neljäs`, … incl. the pronominal `mones`) via a rule
  arm, and the singletons `kaksi`/`yksi` (tn 31) and `tuhat` (tn 46) from the registry. Plus
  **head-inflecting compounds** — the Kotus **tn 50** class and any productive compound whose
  final component is a known lemma — declined on that final component with the modifier frozen,
  so vowel harmony follows the head (`koirankeksi` → `koirankeksissä`, `punaviini` →
  `punaviiniä`, `halpakauppa` → `halpakaupoissa`). Segmentation prefers a split where both
  parts are known lemmas. Design and test-data plan:
  [`docs/compound-nouns.md`](docs/compound-nouns.md).
- **Out:** verbs; adjective **comparison** (comparative/superlative); the **interrogative /
  relative pronouns** (`kuka/mikä/kumpi/joka` — irregular oblique stems, tracked separately);
  a general possessive-suffix system; class inference for unlisted *simple* words; and
  **compounds whose modifier also inflects** (Kotus **tn 51**, `isoveli` → `isoissaveljissä`,
  and the compound ordinals like `kahdeskymmenes` — for now only the head declines; see the
  design doc).

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
cargo run -p keinontolibrary-ingest    # Kotus + reference corpus -> data/artifact/
```

- **Kotus list** (CC BY 4.0): <https://kaino.kotus.fi/lataa/nykysuomensanalista2024.txt>
- **Reference corpus** (JSONL; collected by us, labeled with Voikko): bucket `gs://REDACTED-CORPUS-BUCKET/` (1201 shards).

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
- 🟡 **Rule engine** (`keinontolibrary-rules`): 34 Kotus classes (1–15, 17–20, 23, 24,
  26–28, 32–34, 38–41, 43, 47, 48) + gradation A–M (incl. reverse gradation), wired in as
  the live fallback behind the lookup. **98.0% rule↔corpus parity** (`--test parity`),
  covering ~99.4% of all corpus slots. An **exception registry** (`exceptions.toml`,
  CI-capped) overrides the generator for documented irregulars: the `aie` k-insertion
  family plus the singleton/irregular classes `mies` (42), `kevät` (44), `lapsi` (29),
  `veitsi` (30) — all at 100% parity. Remaining: a few numerals/comparatives, the dual-stem
  `askel` (49), and the long tail (loanword harmony, the 39/40 `-Us` boundary).
- ⬜ **Cloudflare Workers target** (`keinontolibrary-worker`): edge deployment backed by
  KV/D1/R2. The storage abstraction it needs already exists as the `FormStore` trait in
  `keinontolibrary-core`.

## Data provenance & attribution

This project bundles data from the Kotus *Nykysuomen sanalista 2024* (CC BY 4.0) and our
reference corpus (collected over 3 years, labeled with Voikko). See [`LICENSING.md`](LICENSING.md) for full attribution and the open question
about redistributing our (Voikko-generated) reference corpus.

## License

Source code: MIT (see [`LICENSE`](LICENSE)). Bundled data: see [`LICENSING.md`](LICENSING.md).
