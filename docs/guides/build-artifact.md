# Build the data artifact

The packed artifact (`data/artifact/keinontolibrary.bin`) is **not committed** — it's a build
product, rebuilt from the sources below. (Licensing and provenance: see `LICENSING.md`.)

## Sources

1. **Kotus *Nykysuomen sanalista 2024*** (CC BY 4.0) — the lemma inventory:
   <https://kaino.kotus.fi/lataa/nykysuomensanalista2024.txt> → `data/sources/`
2. **Reference corpus** — Voikko-labeled JSONL shards (collected by the project). Place
   the `*.jsonl` shards in `data/sources/voikko/`.

## Ingest

```sh
cargo run -p keinontolibrary-ingest          # Kotus + corpus -> data/artifact/keinontolibrary.bin
```

The artifact is framed (`KEIN` magic + format-version byte + CRC32); a corrupt, truncated,
or version-mismatched file is rejected loudly on load. The build is deterministic — same
sources produce a byte-identical artifact.

## Overrides (optional, Voikko-required)

Four probe-minted sidecars in `data/` refine forms the spelling alone cannot determine —
vowel harmony (`antigeenissä`), comitative style (`-ine` vs `-ineen`), and foreign
citations (`parfait'n`, `cd:n`). They are committed; regenerate after rule changes with:

```sh
scripts/qa/run.sh harmony      # needs libvoikko + the Python venv (run.sh setup)
```

## Verify with the QA loop

The QA loop generates every form, checks each against Voikko + the corpus, and gates on
regressions and total coverage:

```sh
scripts/qa/run.sh setup        # one-time: venv + libvoikko
scripts/qa/run.sh all          # ingest -> dump -> verify -> report --gate
```

The gate holds two invariants: **0 failing slots** and **100% total coverage** (every
Kotus nominal × every slot answered or declared defective). See
[`scripts/qa/README.md`](../../scripts/qa/README.md) for the full workflow and the
accepted-list mechanism.
