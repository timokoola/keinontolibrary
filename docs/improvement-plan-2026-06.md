# Improvement plan — codebase review 2026-06-11

Output of a full-workspace review (core, rules, ingest, data, server, cli, ffi, CI,
Docker, scripts). Two parts: **A)** proposed GitHub tickets, ready to file;
**B)** design for the automated full-library QA loop
(generate → Voikko + corpus verify → report → fix → rerun).

To file any ticket: `gh issue create --title "<title>" --body-file <(...)` — or ask
Claude to file the lot.

---

## Part A — Proposed tickets

### P1 — correctness & data integrity

#### 1. server: harden admin auth and request limits
`crates/keinontolibrary-server/src/lib.rs:148` compares the bearer token with `==`,
which is not constant-time; use `subtle::ConstantTimeEq` (or compare SHA-256 digests).
While here: add an explicit `RequestBodyLimitLayer` (axum's implicit 2 MB default is
generous for a ~200-byte overlay entry) and reject `word` query params over ~256 bytes
in `/decline` / `/paradigm`. Rate limiting can stay out of scope (fronting proxy's job)
but should be documented as such.
*Labels: security, server*

#### 2. server: graceful shutdown on SIGTERM/SIGINT
`crates/keinontolibrary-server/src/main.rs` calls `axum::serve(...)` with no
`with_graceful_shutdown`. In the container deployment a SIGTERM kills in-flight
requests, including admin overlay appends (the overlay write path is not atomic — see
ticket 9). The `tokio` workspace dep already enables the `signal` feature, so this is
a ~10-line fix.
*Labels: server, reliability*

#### 3. ingest: unreadable corpus shards are silently treated as empty
`crates/keinontolibrary-ingest/src/pipeline.rs` (`parse_all`) does
`std::fs::read_to_string(path).unwrap_or_default()` — a permissions/I/O error on one of
the 1201 shards silently drops every form in it, and the ingest "succeeds" with an
incomplete artifact. Propagate the error (or at minimum count and report failed shards
and fail above a threshold). Same family: `list_shards` returning an empty vec should
be a hard error ("no .jsonl shards found"), and `parse_shard` should return a
dropped-line count that lands in the ingest `Report` instead of vanishing.
*Labels: ingest, data-integrity*

#### 4. data: artifact has no magic number, version check, or checksum
`crates/keinontolibrary-data/src/artifact.rs` writes raw bincode. A truncated,
corrupted, or format-incompatible file either fails with an opaque bincode error or —
worse — deserializes into garbage. Add a small header: magic bytes + format-version
byte + CRC32 (e.g. `crc32fast`) of the payload, validated in `read_from`. Also validate
`meta.n_lemmas == lemmas.len()` (and recompute `n_forms`) on load. Add tests that
truncated/bit-flipped/garbage files return `Err` rather than panic. Bump = one-time
artifact rebuild, so do this before the format has external consumers.
*Labels: data, data-integrity*

#### 5. rules: exceptions.toml accepts duplicate slot keys silently
`crates/keinontolibrary-rules/src/exceptions.rs` inserts entries into a HashMap, so a
duplicate `(lemma, tn, number, case)` — easy to create in a merge conflict in a
3400-line TOML file — silently last-writer-wins. Make `parse()` error on duplicates.
Related hardening in the same file: `Exceptions::load()` unwraps the parse ("checked by
tests"); fine for the embedded file, but worth a doc comment stating panics-on-init is
intentional.
*Labels: rules, data-integrity*

### P2 — robustness & process

#### 6. cli: failed decline/paradigm exits 0
`crates/keinontolibrary-cli/src/main.rs` prints errors via `print_error` but `run()`
still returns `Ok(())`, so scripts can't detect unknown/ambiguous/defective results
without parsing stderr. Return a nonzero exit code on `Err` results (and consider a
distinct code for "unknown word" vs "bad usage").
*Labels: cli*

#### 7. ci: add MSRV job and dependency audit
`rust-version = "1.82"` is declared but CI only runs stable, so an MSRV break ships
silently. Add a `dtolnay/rust-toolchain@1.82` job. Also add `cargo audit` (or
`cargo-deny` with an advisories check) — there is currently no CVE detection at all.
*Labels: ci*

#### 8. ingest: end-to-end test with synthetic fixtures that runs in CI
The only e2e coverage (`tests/corpus_roundtrip.rs`) gracefully skips when source data
is absent — i.e. always, in CI. Add a small committed fixture (a dozen Kotus lines + a
tiny synthetic JSONL shard), run the full `run()` pipeline against it in a tempdir, and
assert the artifact contents. Include a determinism check (ingest twice, compare bytes)
so the shard-order re-sort that guarantees reproducibility is protected by a test.
*Labels: ingest, testing*

#### 9. data: overlay hardening (validation, malformed lines, dedup, lock poisoning)
`crates/keinontolibrary-data/src/overlay.rs`:
- `append()` (reachable from `/admin/add`) accepts entries with empty `variants`,
  empty-string variants, or out-of-range `tn` — validate before persisting.
- `open()` silently drops unparseable JSONL lines — log line number + error.
- The file is append-only with no upsert/compaction, so repeated overrides of the same
  slot grow it unboundedly — dedupe on replay and offer a compaction.
- Three `.expect("overlay lock poisoned")` sites turn one panicked request into a
  permanently broken service — map to an error instead.
*Labels: data, server*

#### 10. engine: ambiguous compound components silently use first paradigm
In the tn51 both-parts path, `crates/keinontolibrary-core/src/engine.rs` (~line 325)
resolves modifier and head with `.into_iter().next()?` — for a homonymous component the
choice of paradigm (and hence grade) is arbitrary. Decide and document: return `None`
(fall back to head-only), or generate the union of valid combinations.
*Labels: core, compounds*

#### 11. rules: parity harness only validates the primary variant
`crates/keinontolibrary-rules/tests/parity.rs` checks the corpus *primary* form is among
generated variants, so wrong extra variants (or bad variant ordering) in slots that
legitimately alternate (gen.pl, illative, comitative — *omenoiden/omenoitten/omenain*)
pass unnoticed. Extend it to also flag generated variants the corpus has never
attested, as a warning tier first.
*Labels: rules, testing*

#### 12. docker: HEALTHCHECK, pinned builder image, non-root user
Dockerfile uses the floating `rust:1-alpine` tag (non-reproducible builds), has no
`HEALTHCHECK` (the server already exposes `/healthz`), and runs as root in the runtime
stage. Pin the builder (`rust:1.82-alpine`), add a healthcheck, add `USER nobody`.
Note: `FROM scratch` has no shell or curl, so the healthcheck needs either a tiny
`--health` flag on the server binary or a static probe binary.
*Labels: ops*

#### 13. server: structured logging with tracing
The server has three `eprintln!` calls and nothing else — no request logs, no auth-
failure visibility. `tower-http` with the `trace` feature is *already* a workspace
dependency; wire up `TraceLayer` + `tracing-subscriber` and log admin-endpoint
rejections.
*Labels: server, observability*

### P3 — quality of life

#### 14. tests: property tests for harmony/gradation invariants, fuzz normalize()
No property-based coverage exists. Add a small `proptest` suite: every slot of any
generated paradigm has consistent vowel harmony; gradation never breaks harmony;
`normalize()` is idempotent on arbitrary Unicode (combining marks, ZWJ, RTL marks).
*Labels: testing*

#### 15. perf: small cleanups in hot-ish paths
- `engine.rs` `split_compound`: allocates two `String`s per candidate split before
  checking `is_known_modifier` — test on the `&str` slices first.
- `case.rs` `Case::from_str`: linear scan + `to_ascii_lowercase` allocation per parse —
  a `match` on the lowercased str suffices.
- `pipeline.rs` `group_forms`: clones the lemma `String` per form (~400k clones).
None are user-visible today; batch them as one cleanup PR.
*Labels: performance, good-first-issue*

#### 16. server: clarify /admin/add vs /admin/override
Both routes hit the same handler. Either document them as aliases or give `override`
replace-only / `add` insert-only semantics. Tie into ticket 9's validation work.
*Labels: server, docs*

#### 17. core: document ParadigmRef::matches() wildcard semantics
`paradigm_ref.rs` treats `None` fields as wildcards; `ParadigmRef::new(None, tn)`
matching any homonym is surprising. Doc-comment it, or add explicit
`by_tn` / `exact(hn, tn)` constructors.
*Labels: core, api, docs*

---

## Part B — Automated full-library QA loop (epic)

**Goal:** `just qa` (locally or nightly in CI) generates every form the library can
produce, verifies each against two independent oracles (Voikko and the reference
corpus), produces a triaged report diffed against a committed baseline, and feeds a
fix-then-rerun cycle. Voikko *is* the de-facto Finnish proofreading library, so the
"proofreading" leg is `voikkospell` + Voikko morphological analysis; fi.wiktionary
(via the existing `finnish-testgen` skill) is the tie-breaking third source.

```
 corpus + kotus ─▶ ingest ─▶ artifact
                                │
                    [1] qa dump (Rust)            qa/generated.jsonl  (~770k slots)
                                │
              ┌─────────────────┴──────────────────┐
   [2] corpus check (exists today:                [3] voikko batch oracle (Python)
       corpus_roundtrip + metrics.rs)                 spell + analyze every variant
              └─────────────────┬──────────────────┘
                    [4] triage + report (diff vs qa/baseline.json)
                                │
                    [5] fix: rules / exceptions.toml / overlay
                       (finnish-testgen mints gold data per failing lemma)
                                │
                            rerun `just qa`
```

### Components to build

**[1] `qa dump` — batch generator (Rust, new bin in `keinontolibrary-ingest` or CLI
subcommand).** Iterate every lemma × paradigm in the artifact × 30 slots through the
full engine (lookup → overlay → rule fallback) and emit JSONL:
`{lemma, tn, hn, av, number, case, variants, status, source}`. With rayon this is
seconds of work for ~25.7k lemmas / ~770k slots. Crucially it records `source`
(lookup vs rules vs exception), so the report can separate "rule engine wrong" from
"corpus data wrong".

**[2] Corpus check — mostly exists.** `corpus_roundtrip.rs` (0-mismatch gate),
`parity.rs` (rule↔corpus, ≥90% gate) and `metrics.rs` already cover this leg. Needed
additions: emit machine-readable failure rows (JSONL, same shape as [1]) instead of
only printed samples, so [4] can join them.

**[3] Voikko batch oracle — `scripts/qa/verify_voikko.py`.** Generalizes what
`mint_testdata.py` already does per-word: for every generated variant run
(a) `voikko.spell(form)` — catches outright non-words — and (b) `voikko.analyze(form)`,
accepting iff some analysis has `BASEFORM == lemma`, matching `SIJAMUOTO`/`NUMBER`, and
a nominal `CLASS`. Output verdict per (lemma, slot, variant):
`ok | misspelled | wrong-analysis | not-in-voikko`. ~1M Voikko calls; libvoikko does
tens of thousands of analyses/sec, so with `multiprocessing` this is minutes, not
hours. libvoikko + `voikko-fi` are plain apt packages on Ubuntu → runs in GitHub
Actions, no exotic setup (locally: the existing
`DYLD_LIBRARY_PATH=/opt/homebrew/lib` dance, already documented in the skill).

**[4] Triage + report — `scripts/qa/report.py`.** Join [1]+[2]+[3] per slot and
classify:

| corpus | voikko | verdict |
| --- | --- | --- |
| matches | ok | PASS |
| mismatch | agrees with corpus | **ENGINE_BUG** — fix rules or add exception |
| missing | ok | COVERAGE_GAP — fine, corpus is not exhaustive |
| matches | disagrees | **ORACLE_CONFLICT** — corpus label suspect, human/Claude review |
| mismatch | misspelled | **HARD_FAIL** — we generate a non-word |

Emit `qa/report.json` (full) + `qa/report.md` (per-tn / per-case buckets, top samples)
and **diff against a committed `qa/baseline.json`**: new failures = regression (exit
nonzero, CI-gateable); resolved failures = update baseline in the fixing PR. This makes
the loop incremental — you burn down the 2% parity gap class by class without a wall of
known failures drowning new ones.

**[5] Fix path — existing tools, now fed automatically.** For each ENGINE_BUG lemma the
report links the exact failing slots; the `finnish-testgen` skill mints
Voikko+Wiktionary gold JSONL for it (red test), then the fix is a rule arm change or a
capped `exceptions.toml` entry — both already gated by `parity.rs`. ORACLE_CONFLICTs go
to a review queue (could reuse the import-reports → GitHub-issue renderer in
`scripts/import-reports.mjs`, which already knows how to file labeled issues).

### Orchestration

- **`justfile`** (or `scripts/qa-loop.sh`): `just qa` = ingest (if sources present) →
  dump → voikko verify → corpus verify → report+diff. `just qa-quick` = sample mode
  (e.g. 2k lemmas stratified by tn) for the inner edit-test loop, since full Voikko
  verification is minutes.
- **Nightly GitHub Actions workflow** (`qa.yml`): apt-install `libvoikko-dev voikko-fi`,
  pull corpus shards from `gs://REDACTED-CORPUS-BUCKET/` with a service-account
  secret (cache them — they change rarely), run `just qa`, upload `qa/report.*` as
  artifacts, and on new regressions open a GitHub issue (reuse the import-reports
  pattern). Trend: append the summary numbers to `metrics/metrics.json` history so the
  LinkedIn chart gets a time axis for free.
- **Optional closing of the loop:** a scheduled Claude Code agent that runs `just qa`,
  triages new ENGINE_BUGs with the finnish-testgen skill, and opens a draft PR with the
  exception entries / rule fix + minted red tests. The CI caps on exceptions.toml
  (64 lemmas / 1500 rows) already act as the guardrail against the agent papering over
  rule bugs with exceptions.

### Suggested build order

1. `qa dump` bin + `verify_voikko.py` + `report.py` with baseline diff (the new 80%).
2. `justfile` + sample mode.
3. `qa.yml` nightly with corpus cache + report artifact + regression issue.
4. Machine-readable output from parity/roundtrip ([2] joins), trend history.
5. Scheduled auto-fix agent.

Filed as one epic + sub-tickets 1–5, this also subsumes review tickets 3, 8 and 11
(their fixes land naturally while building [1]–[4]).
