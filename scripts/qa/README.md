# QA loop

Full-library quality gate, run **locally**: generate every form the library can produce,
verify each against two independent oracles, triage, fix, rerun.

```
 corpus + kotus ──▶ ingest ──▶ artifact
                                  │
                    qa-dump (Rust, rayon)          qa/generated.jsonl   ~960k slots
                                  │                (engine + rules + corpus per slot)
              ┌───────────────────┴───────────────────┐
   corpus leg (embedded in the dump:                 voikko leg: verify_voikko.py
   artifact slots ARE the corpus)                    spell + analyze every variant
              └───────────────────┬───────────────────┘
                        report.py — classify, diff vs qa/baseline.json
                                  │
                 fix (rules / exceptions.toml) ──▶ rerun
```

## One-time setup

```sh
brew install libvoikko          # native library + voikko-fi dictionary
scripts/qa/run.sh setup         # .venv with the libvoikko python wrapper
scripts/qa/run.sh sync          # public Kotus list (curl); the private reference corpus
                                #   only if KEINONTO_CORPUS_URI is set (maintainer-only —
                                #   the corpus is not redistributed)
```

## Running

```sh
scripts/qa/run.sh all           # ingest → dump → verify (~10 min) → report --gate
scripts/qa/run.sh quick         # sampled verify (2000 lemmas) for the inner dev loop
scripts/qa/run.sh harmony       # re-mint data/harmony-overrides.jsonl from the dump
scripts/qa/run.sh report --update-baseline   # accept current failures (in a fixing PR)
```

`data/harmony-overrides.jsonl` (committed) carries per-lemma vowel-harmony overrides
for compounds whose final component flips harmony (`antigeenissä`): minted by
`gen_harmony_overrides.py`, which probes Voikko with the dump's generated forms and
their harmony-flipped twins (analyze + BASEFORM match; ≥3 unanimous votes; sticky
across regenerations). Regenerate after rule changes that affect many stems, then
re-ingest — the set converges in one or two iterations.

`data/alternant-overrides.jsonl` (committed) carries **slot-level alternant completions**:
Voikko-verified alternant forms the rule engine produces but the reference corpus
under-attested. The engine resolves a slot lookup-first and stops, so a corpus that
attested only `omenilta` shadows the equally-correct `omenoilta` the rules also generate.
`gen_alternant_overrides.py` reads the dump, and for every lookup-answered slot keeps the
rule alternants Voikko confirms as `lemma + case + number` (the verifier's own gate) that
the served forms lack; ingest unions them into the slot (corpus form stays primary). A
blind union of rule output would inject the generator's ~2% wrong forms — the Voikko gate
is what makes it safe. Sticky and convergent: after a re-ingest the slot serves both forms,
so the next run mints nothing new. The `harmony` subcommand mints this too (it needs a dump
to read, so run `dump` first, or use `all`).

Outputs land in `qa/` (gitignored except `baseline.json`):
`generated.jsonl` (dump), `voikko-verdicts.jsonl`, `report.json`, `report.md`.

## Reading the report

Failing categories (⚠ in `report.md`, tracked in the baseline):

| category | meaning | typical fix |
| --- | --- | --- |
| `engine_bug` | rules disagree with corpus AND Voikko rejects the form | rule arm fix, or exceptions.toml entry |
| `parity_fail` | rules disagree with corpus; Voikko can't judge the lemma | same, corpus is the witness |
| `oracle_conflict` | rules match corpus but Voikko rejects | suspect corpus row — investigate the shard |
| `suspect_misspelled` / `suspect_analysis` | corpus silent, Voikko rejects | likely rule bug beyond corpus coverage |
| `served_misspelled` | a form the **engine actually serves** is a non-word | highest priority, whatever the source |

Informational: `pass`, `pass_voikko`, `extra_variant` (we generate a legitimate alternant
the corpus never attested), `rules_gap` (lookup serves it, rules can't), `unimplemented`,
`unverified` (lemma outside Voikko's lexicon — Kotus ⊃ Voikko).

The **gate** (`report.py --gate`, exit 1) fires only on failures *not in*
`qa/baseline.json` — i.e. regressions. Burn-down therefore works incrementally: fix a
class, rerun, `--update-baseline` in the same PR (never to bury a regression).

**What the coverage % means.** The reported coverage (e.g. `100.00%`) is over the
**in-scope inventory only** — the ~32k Kotus lemmas that carry a declension type, expanded
to every paradigm slot. It is a *correctness* measure (zero wrong forms among what the engine
generates), **not** whole-language declinability. Kotus lists ~72k further rows (transparent
compounds and derivations) *without* a tn; those are not in this denominator. To measure the
runtime reach over that frontier — class inference (`-nen`→38, `-uus`→40), the compound
splitter, numerals — use `scripts/qa/run.sh coverage` (the missing-rows harness: a Rust bin
that runs every Kotus row through the live engine and reports declinable % by word class),
which is a separate metric from this gate.

## Fix workflow per failure

1. Pick a failing slot from `report.md` (worst-tn table first).
2. Mint gold data for the lemma: the `finnish-testgen` skill /
   `.claude/skills/finnish-testgen/scripts/mint_testdata.py` (Wiktionary + Voikko).
3. Fix the rule arm in `keinontolibrary-rules`, or add a (CI-capped) `exceptions.toml`
   entry if it's a genuine irregular.
4. `scripts/qa/run.sh quick` while iterating; `all` + `--update-baseline` to land.

## Enclitics caveat

Raw corpus `BOOKWORD`s may carry `-kin/-han/-ko` or possessive suffixes; the analysis
fields (`FOCUS`, `KYSYMYSLIITE`, `POSSESSIVE`) embed that. This loop reads corpus data
only via the artifact (already filtered by `voikko.rs`), and the Voikko verifier accepts
only clean analyses — except the comitative, whose citation form is possessive-3
(`-ineen`). Keep that rule if you touch `analysis_matches()`.
