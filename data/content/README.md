# Content export — `irregulars.json`

Structured data about the Finnish nominals that decline surprisingly, for keinonto-web's
programmatic-SEO pages (one page per word/family — "miksi *ko'oissa* kirjoitetaan
heittomerkillä?") and social snippets. Regenerate with:

```sh
scripts/qa/run.sh content     # or: python3 scripts/qa/gen_content.py
```

It is **derived** (from `exceptions.toml`, the `data/*-overrides.jsonl` sidecars, and
`qa/accepted.jsonl`, with forms from the rule/registry leg of `qa/generated.jsonl`) but
**committed** so keinonto-web can consume it without this repo's QA pipeline. All forms
are rule/registry-generated — no reference-corpus data — so it carries none of the
licensing weight of the artifact (see [`LICENSING.md`](../../LICENSING.md)).

## Shape

```jsonc
{
  "schema": 1,
  "families": [ { "id": "kj-gradation", "title": "k:j-astevaihtelu", "count": 2 }, … ],
  "stories":  [ { "id": "taika-not-compound", "title": …, "body": …, "examples": [...] }, … ],
  "words": [
    {
      "lemma": "aika", "tn": 9, "family": "kj-gradation", "source": "registry",
      "why": "k:j gradation aika->aja- (Kotus marks D)",
      "nominative": "aika",
      "headline": { "number": "singular", "case": "genitive", "form": "ajan" },
      "forms": [ { "number": "singular", "case": "genitive", "forms": ["ajan"] }, … ]
    }, …
  ]
}
```

- **`words`** — one entry per genuinely irregular lemma (registry, harmony/citation
  sidecars, plurale tantums, and Kotus↔Voikko disagreements). `headline` is the single
  most illustrative form for a card/title; `forms` is the full rule-generated table.
- **`families`** — hub-page topics (k:j gradation, loanword harmony, foreign citations,
  pronouns, plurale tantums, …), with counts.
- **`stories`** — cross-cutting rules behind the surprises (taika ≠ t+aika; vaa'an but
  vaaoissa; afääriä but countrya; antigeenissä; jossakin) — the strongest social hooks.

`source` is provenance: `registry`, `harmony`, `citation`, `tantum`, `comitative`,
`accepted` (the last = words where Kotus and Voikko disagree).
