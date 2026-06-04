# Compound nouns (Kotus 50/51) — design & test-data plan

Compounds are the bulk of any Finnish dictionary, and they're the largest remaining gap in
keinontolibrary. This doc records the design and, in particular, **how we get enough test
data to trust it**.

## What ships today

`Engine` has a final-component fallback (see `keinontolibrary-core/src/engine.rs`): when a
lemma is unknown as a whole, it splits off the **longest suffix that is a known lemma**
(using the existing inventory via `resolve()`), declines that component through the normal
lookup→rules path, and re-attaches the fixed modifier prefix. So:

```
koirankeksi → split → koiran + keksi → decline keksi → keksissä → koirankeksissä
```

This is correct for the overwhelmingly common Finnish pattern — **the head (final
component) inflects; the modifier is frozen** — and it makes vowel harmony fall out for free
(harmony follows the head). It is the right 80% with ~30 lines and no new data.

### Known limits of the heuristic

1. **Greedy/wrong splits.** Longest-suffix can mis-segment when a long suffix is
   coincidentally a word (`taatelitaikina` → `taatelita` + `ikina`? no, but adversarial
   cases exist). Mitigated by min prefix/component lengths; not eliminated.
2. **Ambiguous heads.** If the head has several paradigms (`viini` = tn 5 / 26), we take the
   first; harmony is unaffected but the stem could be wrong for the minority reading.
3. **Modifier-inflecting compounds.** A minority inflect the modifier too — numerals
   (`kahdeksankymmentä` → `kahdeksaakymmentä`), a few lexicalized nouns. The heuristic keeps
   the modifier frozen, which is wrong for these.
4. **Linking elements & foreign modifiers.** `-n-`/`-en-` linkers (`koira**n**keksi`) are
   part of the frozen prefix and need no handling; foreign modifiers (`beaujolaisviini`)
   work because only the head is looked up.

## What "full Kotus 50/51" adds

Kotus marks compounds 50/51 (modifier+head, with/without modifier inflection). Full support
means:

1. **Reliable segmentation.** Prefer splits where *both* parts are known lemmas, score
   candidates (head frequency, prefix plausibility, linker shape), and fall back to the
   single-known-head heuristic. Consider Voikko's own compound analysis as an oracle at
   ingest time (not at runtime).
2. **Modifier inflection.** A small, explicit class (numerals + a curated lexicalized list)
   that inflects both parts; everything else freezes the modifier.
3. **Head paradigm selection.** When the head is ambiguous, disambiguate via the compound's
   own Kotus entry (50/51 rows carry the head's class) rather than guessing.

The runtime stays lexicon-light: segmentation uses the packed inventory already loaded; the
heavy lifting (which compounds exist, their head class, modifier-inflection flag) is resolved
**at ingest** and baked into the artifact.

## The hard part: collecting enough test data

We cannot hand-write paradigms for compounds at scale, and most compounds have no Wiktionary
page. Three complementary sources, all **Voikko-validated** so they're trustworthy:

### 1. Mine the reference corpus (highest value)

The corpus already contains compound surface forms — they're currently **dropped at the
Kotus join** because compounds aren't in the 1–49 list. Instead:

- Keep corpus rows whose `BASEFORM` Voikko analyses as a compound (Voikko exposes the
  word-part boundaries in `WORDBASES`/`STRUCTURE`).
- For each, record `(compound_lemma, number, case) → surface form` as **attested gold**.
- This yields tens of thousands of *real, attested* compound forms for free, and is the
  primary parity target: run the segmentation+decline path and check it reproduces them.

### 2. Voikko-validated synthetic compounds (coverage at scale)

The corpus is sparse per slot (same funnel as simple nouns). To fill gaps:

- Take the ~25.7k Kotus heads × a curated set of frequent modifiers (in genitive and
  nominative linking forms), forming candidate compounds (`koiran-` + head, `työ-` + head…).
- For each candidate, **ask Voikko to generate/validate the full paradigm** (Voikko knows
  the compound boundary and the head's inflection), keeping only forms Voikko confirms.
- This is the `finnish-testgen` skill generalized from one word to a compound matrix; output
  is the same ingest-compatible JSONL. Gate on Voikko agreement so synthetic ≠ wrong.

### 3. Wiktionary for the lexicalized/irregular tail

For the modifier-inflecting and lexicalized compounds (numerals, fixed expressions), pull
the explicit tables from fi.wiktionary via `finnish-testgen` and add them as exceptions /
gold. Small set, high value, where rules and synthetics are least reliable.

### Acceptance gate

Extend the rule↔lookup parity harness with a **compound parity** metric: % of mined-corpus
compound slots the segmentation+decline path reproduces, reported per split-confidence
bucket. Ship when corpus-compound parity clears a documented threshold (target ≥ 98%, same
bar as the simple-noun rule engine), and never regress it.

## Rollout

1. Heuristic final-component fallback — **shipped** (this PR).
2. Ingest: keep Voikko-analysed compound forms as gold; build the corpus-compound parity
   harness (data + metric, no engine change).
3. Synthetic compound matrix via Voikko; raise coverage; tune segmentation scoring.
4. Modifier-inflection class + head-paradigm disambiguation from the 50/51 Kotus rows.
5. Flip remaining "Out" wording once parity clears the gate.
