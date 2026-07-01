# How it works

Finnish nouns inflect across 15 cases and two numbers — roughly 30 forms per word — with
consonant gradation, vowel harmony, and a long tail of irregulars that break any naive lookup
table. keinontolibrary produces the forms through several cooperating layers, and then checks
every one of them against an independent oracle.

## The layers

1. **Rule generator (Kotus classes 1–49).** Implements the Kotus declension types with
   consonant gradation (grades A–M) and vowel harmony. It handles the bulk of the language
   directly — about 98% agreement with the reference corpus.
2. **Compound classes (tn 50–51).** Compounds inflect through their parts: tn 50
   head-inflecting (`koirankeksi → koirankeksissä`), tn 51 both-parts-inflecting
   (`isoveli → isoissaveljissä`). A segmenter splits compounds; a plural-head reverse index
   and a combining-head registry reach the transparent compounds and derivations that Kotus
   leaves without a declension type.
3. **Corpus-backed lookup.** Precomputed surface forms from a reference corpus (collected over
   three years) are the primary source where available; the rule generator is the fallback.
4. **Exception registry + overrides.** Genuine irregulars the rules can't derive — `aika → ajan`,
   the suppletive `kuka → kenet`, `vaaka → vaa'an` — live in a registry and per-slot overrides.
5. **Productive inference & resolvers.** Class inference (`-nen`→38, `-uus`→40, `-ias`→41),
   nested compounds, and compound numerals extend reach to **100% of declinable Kotus nominal
   rows**.

## Verification

Correctness isn't asserted — it's checked. Every form the engine can produce is validated
against **[Voikko](https://voikko.puimula.org/)**, an open-source Finnish morphological
analyzer, used purely as an **independent oracle**: it analyzes each generated form, and a form
is accepted only when Voikko confirms it as the expected lemma + case + number. Where the rule
generator and the corpus disagree, the corpus is the witness and Voikko the tie-breaker. The
quality gate runs at zero wrong forms across the in-scope inventory.

That is why "verified" is a claim rather than a slogan: each form is checked by a tool that had
no hand in generating it.

## Data & provenance

- **Lemma inventory and inflection metadata** — the Kotus *Nykysuomen sanalista 2024*, licensed
  CC BY 4.0.
- **Surface forms** — our own reference corpus, collected over three years and labeled using
  Voikko as an analysis tool.

Scope: all nominal Kotus classes **1–51** — substantives, adjectives, numerals, and pronouns.
Verbs and the indeclinable tn 99/100 classes are out of scope.
