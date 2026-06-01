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

## Our reference corpus (generated with Voikko)

The precomputed surface forms (the `BOOKWORD` values keyed by lemma/number/case) are **our
reference corpus**, which we generated using [Voikko](https://voikko.puimula.org/) as a
morphology tool and store as JSONL. The corpus is ours; Voikko is the tool that produced
the forms.

> ⚠️ **OPEN ACTION ITEM (must be resolved before publishing).** Because the forms were
> generated *with* Voikko, confirm that Voikko's dictionary license (Voikko is GPL/LGPL; the
> morphology dictionary has its own terms) permits redistributing the resulting form set
> inside the package. If redistribution is not permitted, options are: (a) ship only the rule
> engine + Kotus metadata and generate forms on the consumer's machine, or (b) obtain/
> relicense the dictionary. Record the final conclusion and the basis for it here.

**Status:** UNRESOLVED. Do not publish the bundled reference corpus until this is settled.

## Raw source files

The raw Kotus list and the reference-corpus JSONL shards (Voikko-format) live under
`data/sources/` and are **gitignored** (see [`.gitignore`](.gitignore)) — they are fetched
at build time, not committed.
