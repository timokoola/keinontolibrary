# Road to 100% — closing the last 6,661 slots

State after #55: **total coverage 99.26%** (891,467/898,128), failing slots 0.
The gap decomposes into five buckets; this plans all of them, with the two
"nerd-test" buckets (pronouns, compound ordinals) first. Each bucket lands as
its own QA cycle: implement → full Voikko loop → 0 regressions → merge.

## Cycle 10a — pronouns (tn101, ~765 slots) ★ nerd-test priority — ✅ DONE

`hän`, `he`, `joka`, `joku`, `jokin`, `kuka`, `ken`, `mikä`, `kukin`,
`kumpikin`, `jompikumpi`, `mikään`, `kukaan`, `kenkään`, `muuan`, …

- **Mechanism: registry.** These are the most irregular words in the language
  and several move their clitic *inside* the inflection (`jokin → jonkin,
  jossakin`; `kukaan → kenenkään, keitään`) — no rule arm can carry that;
  fixed strings can. The core set (minä/tämä/se…) already lives there.
- Extend `.claude/skills/finnish-testgen/scripts/gen_pronouns.py` to the
  relative/interrogative/indefinite series; candidates hand-written per lemma,
  every form Voikko-verified before a row is written (the established
  pattern). Multiword `itse kukin` waits for bucket 5's space-compound route.
- Registry caps: ~15 lemmas × ~25 rows ≈ 375 rows on top of 500 → fits the
  1500/64 caps with room.
- Scope note: ✅ done — `kuka/mikä/kumpi/joka` are now registry-served (correct
  suppletive stems `kenen/minkä/kumman/jonka`, Voikko-verified) and listed under
  **In** in the README.

## Cycle 10b — compound ordinals (tn45 both-part, ~216 slots) ★ nerd-test priority — ✅ DONE

`kahdeskymmenes → kahdennenkymmenennen`, … (8 Kotus lemmas + productive).

- **Mechanism: engine both-parts route**, like tn51 but for ordinals: when a
  tn45 lemma ends in a known ordinal head (`kymmenes`, `sadas`, `tuhannes`)
  with a known ordinal prefix, decline BOTH parts through tn45 in the same
  slot and concatenate (`kahdennen` + `kymmenennen`). Reuses
  `compound_both_slot`'s shape; the ordinal split is deterministic.
- **Corpus cleanup**: the corpus carries mislabeled head-reading rows for
  these 8 lemmas (`…kymmeneksen`); drop them at ingest (lemma+tn denylist) so
  lookup stops serving junk, then remove the accepted-list entries.
- Comitative/instructive of ordinals stay accepted (outside Voikko entirely).

## Cycle 11 — plurale tantum citations (~3,500 slots, the biggest bucket)

`sakset`, `suitset`, `länget` (tn7) · `hohtimet`, `aterimet` (tn33) ·
`arpajaiset`, `avajaiset` (tn38) · `kaverukset` (tn39) · `rattaat`,
`tikkaat`, `valjaat` (tn41) · `isovanhemmat` (tn50 heads).

- **Mechanism: generalize the tn48 tantum trick.** A `-t` citation is the
  nominative plural, i.e. `{sg_weak}t` — stripping the `t` yields the weak
  singular stem directly (`sakset → sakse-`, `länget → länge-` →
  strengthen → `länke-`). When the class arm fails AND the lemma ends in
  `-t`: build Stems from that stem (per-class inverse where needed:
  `-set→-nen`, `-kset→-s`, `-aat→-as`), generate **plural slots only**,
  singulars return None (they don't exist — also fixes the denominator:
  these singulars move from "gap" to grammar-defective).
- tn50 tantum compounds (isovanhemmat) then work through the compound path
  once the head (`vanhemmat`) resolves via the same machinery.

## Cycle 12 — pronunciation citations (tn22 + letter-words, ~1,275 slots)

`parfait'n`, `bordeaux'ta` (tn22) · `cd:n`, `adhd:n`, `tv:ssä` (tn18-ish).

- **Mechanism: a third probe-minted sidecar**, same pattern as harmony and
  comitative overrides: `data/citation-overrides.jsonl` with
  `{lemma, sep (' or :), front, echo}` — minted by probing Voikko with both
  separators × both harmonies × echo candidates (`parfait'hen`, `cd:hen`).
  Plumbed as one `Option<ForeignCitation>` through LemmaRecord → ParadigmRef;
  a small rules arm attaches endings after the separator.
- Voikko's own coverage of colon forms is partial (it rejected `dna:n`), so
  unverifiable lemmas stay accepted-listed — but the *served* forms follow
  the standard orthography either way.

## Cycle 12b — odd spellings (~700 slots)

- **Accented vowels** (`csárdás`, `bébé`): map à/á/é/è/ê → base vowel in the
  stem/harmony helpers; then the normal arms apply.
- **Multiword citations** (`itse kukin`, `eau de cologne`): a space-compound
  route — freeze everything before the last space, decline the tail (after
  cycle 10a, `kukin` resolves). Mirrors the hyphen handling.

## Content pipeline (pSEO + social) — runs alongside

The QA artifacts are ready-made content:

- **`exceptions.toml`** — "the ~45 most irregular Finnish words", each with a
  human-written `reason` line (aika→ajan k:j, veli→velj-, meri→merta…).
- **`qa/accepted.jsonl`** — "words where the authorities disagree": Kotus vs
  Voikko class fights (vakaus, koiras), spell-checker blind spots
  (nonstop, hittolainen), pronunciation-vs-spelling loans (menu, jockey).
- **Cycle war stories** — taika is not t+aika; vaa'an but vaaoissa; ko'oissa
  but koon; countrya because y isn't always front.

Plan: `scripts/qa/gen_content.py` exports a single
`data/content/irregulars.json` (lemma, family, tricky forms, why, oracle
verdicts) consumed by keinonto-web's pSEO templates (one page per word/family
— "miksi 'ko'oissa' kirjoitetaan heittomerkillä?") and social snippets. The
keinonto-web side uses the pseo-* tooling; this repo only exports the data.

## Order & gates

1. 10a pronouns → 10b compound ordinals (one PR each, gate at 0).
2. 11 tantum (biggest coverage jump: ~99.26 → ~99.8%).
3. 12 + 12b (pronunciation + spellings → ~99.95%+; the rest documented).
4. Content export once 10a/10b land (the nerd-test words are the hook).

Coverage target line in every report; the gate stays the law: 0 regressions,
fixes never re-baseline over failures.
