# Fix a wrong declension (the right way)

The library is gated at **100% coverage / 0 failing slots** against Voikko + the corpus.
Every change keeps it there. The workflow for fixing or adding a form:

## 1. Reproduce

```sh
keinontolibrary decline <word> --number <n> --case <c>
```

Decide whether it's a **rule** problem (a whole class is wrong), a **lexical** one (one
irregular word), or a **gap** (no form at all).

## 2. Mint Voikko-verified gold data

Never hand-write forms from memory — mint them. The `finnish-testgen` skill pulls the
fi.wiktionary table and validates every form through Voikko:

```sh
DYLD_LIBRARY_PATH=/opt/homebrew/lib .venv/bin/python \
  .claude/skills/finnish-testgen/scripts/mint_testdata.py <word> \
  --kotus data/sources/nykysuomensanalista2024.txt
```

## 3. Make the fix at the right altitude

- **Rule** — edit the class arm in `crates/keinontolibrary-rules/src/generate.rs` (or the
  gradation/harmony helpers). Prefer generalizing over special-casing.
- **Irregular** — add Voikko-verified rows to `crates/keinontolibrary-rules/exceptions.toml`
  (the registry rejects duplicate keys and is CI-capped).
- **Override** — for harmony / comitative / citation quirks, regenerate the sidecar
  (`scripts/qa/run.sh harmony`) rather than editing forms by hand.

Add a unit test with the Voikko-verified forms next to the change.

## 4. Run the gate

```sh
scripts/qa/run.sh quick      # while iterating (sampled)
scripts/qa/run.sh all        # full: ingest -> dump -> verify -> report --gate
```

The gate fails on any **new failing slot** or **coverage drop**. If a slot genuinely
cannot be judged by Voikko (a lemma outside its lexicon, a Kotus↔Voikko disagreement),
add it to `qa/accepted.jsonl` **with a reason** — never re-baseline over a real
regression. Update the baseline only in the same PR as the fix:

```sh
scripts/qa/run.sh report -- --update-baseline
```

## 5. Standard checks

```sh
cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features
```

CI additionally runs the MSRV build (1.85) and `cargo-audit`. See
[`scripts/qa/README.md`](../../scripts/qa/README.md) for the loop's internals.
