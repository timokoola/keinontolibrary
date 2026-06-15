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
> <10 MB container. Every nominal declension class in the **Kotus 1–51 range is in scope** —
> the rule generator handles the bulk directly (~98% rule↔corpus agreement) and the
> Voikko-verified exception registry + overrides serve the irregulars the generator doesn't.
> The QA gate runs at **0 failing slots across the in-scope inventory** — i.e. correctness,
> not whole-language coverage. That **denominator is the ~32k Kotus lemmas that carry a
> declension type**; Kotus leaves ~72k further rows (transparent compounds and derivations)
> *without* a tn, expecting inflection through their final component. Runtime resolvers reach
> that frontier — the compound splitter (`linja-auto`, `avoauto`), productive class inference
> (`ahdaskatseinen`→tn38, `-uus`→tn40), and a plural-head reverse index (`ajovalot`→`valo`) —
> covering **99.5% of all Kotus nominal rows** (measure it with `scripts/qa/run.sh coverage`).
> The remaining ~0.5% (compound numerals, a few foreign/abbreviation compounds) and the
> Cloudflare Workers target are the open work (see [Roadmap](#roadmap)). The working/repo name
> is `keinontolibrary` (`keinonto` = the instructive case); the crate prefix is
> `keinontolibrary-*`.

## Workspace layout

| Crate                     | Purpose                                                        |
| ------------------------- | ------------------------------------------------------------- |
| `keinontolibrary-core`    | Public API: `Case`/`Number` enums, `Forms`, `decline`, `paradigm`. |
| `keinontolibrary-rules`   | Rule generator for Kotus classes 1–49 + gradation A–M.        |
| `keinontolibrary-ingest`  | Offline pipeline: Kotus + reference corpus → packed artifact. |
| `keinontolibrary-data`    | Packed artifact + zero-copy (mmap/embedded) loader.           |
| `keinontolibrary-server`  | axum HTTP service (the container deployment).                 |
| `keinontolibrary-cli`     | CLI: `decline`, `paradigm`, `table`, `add`, `override`, `validate`, `selftest`. |
| `keinontolibrary-ffi`     | FFI scaffold (UniFFI/Swift; feature-gated wasm + PyO3).       |

## Scope (v1.0)

- **In:** all **nominals** — substantives, **adjectives**, and **numerals** — in declension
  classes 1–49 with gradation, all 15 cases, both numbers, multi-paradigm homonyms. Nominals
  share the declension classes, so the class-driven engine handles them uniformly (the ingest
  keeps every nominal word class, not just `substantiivi`). Plus the **pronouns** (Kotus
  tn 101 — irregular: the personal `minä/sinä/hän/me/te/he`, the demonstratives
  `se/tämä/tuo/nämä/nuo/ne`, and the interrogative/relative `kuka/mikä/kumpi/joka` with their
  suppletive oblique stems — `kuka → kenen/ketä`, `mikä → minkä`, `joka → jonka`), served from
  the Voikko-verified exception registry rather than the rule generator. Plus the **special numeral classes**: the productive **ordinals** (tn 45
  — `kolmas → kolmannen`, `kymmenes`, `neljäs`, … incl. the pronominal `mones`) via a rule
  arm — including **compound ordinals** like `kahdeskymmenes`, where *both* components decline
  (`kahdennenkymmenennen`, `kahdettakymmenettä`; Voikko-verified) — and the singletons
  `kaksi`/`yksi` (tn 31) and `tuhat` (tn 46) from the registry. Plus
  **compounds**, both Kotus classes plus any productive compound whose final component is a
  known lemma: **tn 50** head-inflecting (modifier frozen, harmony follows the head —
  `koirankeksi` → `koirankeksissä`, `halpakauppa` → `halpakaupoissa`) and **tn 51**
  both-parts-inflecting (`isoveli` → `isoissaveljissä`, `täysikuu` → `täysissäkuissa`).
  Segmentation prefers a split where both parts are known lemmas. Validated by a
  Voikko-oracle compound-parity harness at ~99.8% (tn 50) / ~99.7% (tn 51) of judged slots;
  design and test-data plan: [`docs/compound-nouns.md`](docs/compound-nouns.md).
- **Out:** **verbs** — conjugation is the one word class not handled, and the only
  out-of-scope item (a roadmap item; see [Roadmap](#roadmap)). Every **nominal** declension
  class in the **Kotus 1–51 range** is in scope; if you find a 1–51 case that isn't covered,
  that's a bug, not a scope boundary (scope has widened as the engine matured).
- **Not declension, so not produced** (these are separate morphology, not scope exclusions):
  comparative/superlative *derivation* from a positive (`suuri → suurempi`) — but the derived
  word itself **declines** as an ordinary adjective once known (`parempi → paremman`,
  `vanhin → vanhimman`, tn 16/6); possessive suffixes and clitics (`taloni`, `talokin`),
  beyond the comitative's obligatory `-ineen`; and class *inference* for an unlisted simplex
  word — the engine declines a word whose class it knows (from Kotus, the registry, compound
  segmentation, or an overlay `add`), it does not guess a class for a word it has never seen.

## Documentation

Task-oriented guides live in [`docs/guides/`](docs/guides/):
[embed in Rust](docs/guides/embed-rust.md) ·
[CLI](docs/guides/cli.md) ·
[HTTP service](docs/guides/http-service.md) ·
[build the artifact](docs/guides/build-artifact.md) ·
[contributing](docs/guides/contributing.md).
See also [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md) (install channels) and
[`docs/SITE_PLAN.md`](docs/SITE_PLAN.md) (the keinonto.com docs site).

## Building

Requires Rust ≥ 1.85 (MSRV).

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
- **Reference corpus** (JSONL; collected by us, labeled with Voikko): a private,
  access-controlled store, **not redistributed**. It bootstrapped and cross-checked the
  engine; the rule engine + registry + overrides now give full coverage without it
  (verified by the test suite and the Voikko QA gate), so building from the public Kotus
  list alone is supported. Maintainers point `KEINONTO_CORPUS_URI` at their own copy.

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
  `veitsi` (30) — all at 100% parity. The slots the *rule arm* still doesn't generate
  natively (a few numerals, the dual-stem `askel` (49), the loanword-harmony / 39–40 `-Us`
  long tail) are served by the registry, overrides, and lookup, so end coverage across 1–51
  is complete — these are rule-generator polish, not scope gaps. (Comparative and superlative
  *forms* decline normally as tn 16/6 adjectives; only their *derivation* from the positive is
  a separate, non-declension concern.)
- ⬜ **Cloudflare Workers target** (`keinontolibrary-worker`): edge deployment backed by
  KV/D1/R2. The storage abstraction it needs already exists as the `FormStore` trait in
  `keinontolibrary-core`.
- ⬜ **Verb conjugation** — the one word class still out of scope. Finnish verbs (Kotus
  taivutustyypit 52–78) are a separate inflection system (tense, mood, person, voice,
  infinitives, participles); they would extend, not modify, the nominal engine. Not started.

## Data provenance & attribution

This project bundles data from the Kotus *Nykysuomen sanalista 2024* (CC BY 4.0) and our
reference corpus (collected over 3 years, labeled with Voikko). See [`LICENSING.md`](LICENSING.md) for full attribution and the open question
about redistributing our (Voikko-generated) reference corpus.

## License

Source code: MIT (see [`LICENSE`](LICENSE)). Bundled data: see [`LICENSING.md`](LICENSING.md).
