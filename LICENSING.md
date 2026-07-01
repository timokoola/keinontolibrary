# Licensing & data provenance

This repository contains two kinds of material under different terms:

1. **Source code** — MIT licensed (see [`LICENSE`](LICENSE)).
2. **Linguistic data** — derived from third-party sources, described below.

## Kotus *Nykysuomen sanalista 2024*

The lemma inventory and inflection metadata (declension class `tn`, gradation `av`,
homonym number `hn`) come from the Kotus *Nykysuomen sanalista 2024*.

- Source: <https://kaino.kotus.fi/lataa/nykysuomensanalista2024.txt>
- Overview: <https://kotus.fi/sanakirjat/kielitoimiston-sanakirja/nykysuomen-sana-aineistot/nykysuomen-sanalista/>
- **License: Creative Commons Attribution 4.0 International (CC BY 4.0).**

**Required attribution** (bundled in the package and surfaced by the HTTP service at
`/about`):

> Contains data from the *Nykysuomen sanalista* by the Institute for the Languages of
> Finland (Kotimaisten kielten keskus, Kotus), licensed under CC BY 4.0
> (<https://creativecommons.org/licenses/by/4.0/>). Modified: filtered to simple nouns and
> repackaged into a lookup artifact.

## Our reference corpus (collected by us, labeled with Voikko)

The precomputed surface forms (the `BOOKWORD` values keyed by lemma/number/case) are **our
reference corpus**, which we **collected over three years** and morphologically **labeled**
using [Voikko](https://voikko.puimula.org/), stored as JSONL. The corpus is ours; Voikko is
the labeling tool.

No Voikko material is bundled in this package — not libvoikko code, not the voikko-fi
morphology, and not the Joukahainen word list. Voikko is a build-time analysis and validation
tool only. The corpus forms and their labels are our own work and are redistributed as part of
the package.

## Raw source files

The raw Kotus list and the reference-corpus JSONL shards (Voikko-format) live under
`data/sources/` and are **gitignored** (see [`.gitignore`](.gitignore)) — they are fetched
at build time, not committed.
