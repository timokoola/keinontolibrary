---
name: finnish-testgen
description: >-
  Mint Voikko-verified declension test material for a Finnish noun by pulling its
  fi.wiktionary inflection table and validating every form through Voikko. Use when a word
  declines wrong in keinontolibrary, when adding coverage for a tricky/irregular noun, or
  when you need authoritative forms to write red tests or exception entries against.
---

# finnish-testgen

Turn a single Finnish noun into trustworthy test material for `keinontolibrary`:
**fi.wiktionary** supplies the full paradigm, **Voikko** independently verifies each surface
form (right baseform + case + number), and the result is emitted as ingest-compatible
corpus JSONL. This is the "fix loop" for declension bugs: get the correct forms, prove them,
then write tests / exceptions against them.

## When to use

- A word is declined wrong (e.g. `aika -> aian` instead of `ajan`) and you need the correct
  full paradigm, double-sourced, before fixing.
- Adding coverage for an irregular or rare noun.
- Generating red tests or `exceptions.toml` entries from an authoritative source rather than
  by hand.

## Prerequisites (install Voikko locally)

```sh
# macOS (Homebrew bundles a usable Finnish dictionary with the bottle)
brew install libvoikko

# use a venv (python.org Python lacks CA certs for urllib; requests bundles them)
python3 -m venv .venv && .venv/bin/pip install requests beautifulsoup4 libvoikko
```

On Homebrew macOS the Python binding needs the native dylib on the loader path — run the
script with `DYLD_LIBRARY_PATH=/opt/homebrew/lib` (the script also probes
`VOIKKO_DICTIONARY_PATH` and `/opt/homebrew/lib/voikko` for the dictionary):

```sh
DYLD_LIBRARY_PATH=/opt/homebrew/lib .venv/bin/python3 \
  .claude/skills/finnish-testgen/scripts/mint_testdata.py aika --kotus data/sources/nykysuomensanalista2024.txt
```

Verify Voikko loads:
`DYLD_LIBRARY_PATH=/opt/homebrew/lib .venv/bin/python3 -c "from libvoikko import Voikko; print(Voikko('fi').analyze('ajan'))"`.
If Voikko can't load, the script still runs with `--no-voikko` (wiktionary-only, forms marked `UNVERIFIED`).

## Workflow

1. **Mint + verify** the forms. Point it at the Kotus list for the lemma's class/gradation:
   ```sh
   python3 .claude/skills/finnish-testgen/scripts/mint_testdata.py aika \
     --kotus data/sources/nykysuomensanalista2024.txt --jsonl /tmp/aika.jsonl
   ```
   Read the table: every row should say `ok`. Any `MISMATCH` means wiktionary and Voikko
   disagree — investigate before trusting that form (don't auto-emit it).

2. **Diff against the engine** to localize the bug. For each slot, compare the verified form
   with what keinontolibrary produces:
   ```sh
   cargo run -p keinontolibrary-cli -- paradigm aika --tn 9
   ```
   The slots that differ are the bug surface.

3. **Decide the fix kind:**
   - **Systematic** (many words share the wrong pattern) -> fix the rule in
     `keinontolibrary-rules` (`generate.rs` / `gradation.rs`). Add the verified forms as
     unit tests in `generate.rs` or `lib.rs`.
   - **Lexical/irregular** (a handful of words; the wrong form is not predictable, e.g.
     `aika`/`poika` k:j gradation) -> add the weak/irregular slots to
     `crates/keinontolibrary-rules/exceptions.toml` with a one-line `reason`, and add
     `RuleEngine` tests in `lib.rs`. The registry is capped by **distinct lemmas** (see
     `tests/parity.rs`), so a fully-specified irregular costs 1 against the cap.

4. **Use the JSONL as corpus test material.** The emitted shard is in the same Voikko-format
   the reference corpus uses, so dropping it into `data/sources/voikko/` and re-running
   `cargo run -p keinontolibrary-ingest` makes those forms authoritative Lookup data and
   feeds the rule↔lookup parity harness.

## Notes & caveats

- Accusative and comitative are intentionally **not** emitted as JSONL: the corpus derives
  accusative and uses the `-ineen` possessive citation for comitative, so plain forms there
  wouldn't round-trip through the ingest filter. They still appear in the table for review.
  Accusative rows always show `MISMATCH` — Voikko reports that form as `nimento`/`omanto`,
  never `kohdanto` — which is expected, not a real disagreement.
- Voikko is the **arbiter**. If wiktionary lists a form Voikko won't confirm as
  `lemma + case + number`, treat it as suspect, not as test material.
- Keep `exceptions.toml` for genuine irregularities. If you find yourself adding many
  different lemmas for the same pattern, that's a rule gap — fix the rule instead.
