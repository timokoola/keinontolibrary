# Decline from the command line

Quick lookups and scripting, no code.

## Install

```sh
# From a checkout (until published to a package channel — see ../DISTRIBUTION.md):
cargo install --path crates/keinontolibrary-cli
# or run in place:
cargo run -p keinontolibrary-cli -- <args>
```

The CLI reads the artifact at `data/artifact/keinontolibrary.bin` (override with
`--artifact` or `$KEINONTO_ARTIFACT`) and an overlay at `data/overlay.jsonl`
(`--overlay` / `$KEINONTO_OVERLAY`).

## Decline one slot

```sh
keinontolibrary decline hevonen --number plural --case inessive
# hevonen (plural inessive): hevosissa  (Present, Lookup)

keinontolibrary decline kuka --number singular --case accusative
# kuka (singular accusative): kenet  (Present, Generated)
```

## The whole paradigm

```sh
keinontolibrary paradigm talo
# talo (tn=1)
#   singular nominative   talo
#   singular genitive     talon
#   ...
```

## Disambiguate homonyms

```sh
keinontolibrary decline kuusi --number singular --case inessive
# 'kuusi' is ambiguous; pass --tn (or --hn):
#   tn=24
#   tn=27
keinontolibrary decline kuusi --number singular --case inessive --tn 27
# kuusi (singular inessive): kuusessa
```

## Add or correct a word (overlay)

```sh
keinontolibrary add --lemma uudissana --tn 9 \
    --number singular --case inessive --forms uudissanassa
# overlay: uudissana singular inessive = ["uudissanassa"]
```

`override` is an alias of `add` (the overlay is upsert-by-key — last write wins). New
overlay entries are immediately declinable and persist to the overlay file.

## JSON output

```sh
keinontolibrary decline talo --number plural --case adessive --json
# {"variants":["taloilla"],"status":"present","source":"lookup","coincides_with":null}
```

## Exit codes (for scripts)

| code | meaning |
| --- | --- |
| 0 | success |
| 3 | the word could not be declined — unknown, ambiguous, or defective form |
| 1 | setup/usage error (bad artifact path, I/O) |
| 2 | argument parsing error (from clap) |

```sh
if keinontolibrary decline "$w" --number singular --case genitive --json >/tmp/out; then
    jq -r '.variants[0]' /tmp/out
else
    echo "no form for $w (exit $?)"
fi
```

## `validate` — inspect the loaded artifact

```sh
keinontolibrary validate
# version, lemma count, form count, and the Kotus / reference-corpus provenance.
```
