# LinkedIn post — draft

> Two images: (1) `keinontolibrary-accuracy.png`, (2) your screenshot of one of the project
> issues. Alt-text for both is in the "Image alt-text" section below.
> Written in your voice — edit freely. A longer "capability note" follows for the comments.

## Image alt-text

**Image 1 — accuracy chart (`keinontolibrary-accuracy.png`):**
> Bar chart titled "Declining every Finnish noun — in Rust." For each of the 15 Finnish
> grammatical cases, two bars show the engine's agreement with the reference corpus in
> singular and plural — all between 92% and 100%. A side panel highlights 98.0% overall
> agreement, 25 694 nouns, 166k forms checked, 99.5% coverage, 34 declension types, and a
> sub-10 MB container.

**Image 2 — GitHub issue screenshot:**
> Screenshot of one of the project's issues, tagged "rule-engine", "needs-examples" and
> "linguistics", documenting a Finnish declension edge case the engine doesn't yet cover and
> asking a native speaker for reference paradigms with consonant-gradation examples.

---

I spent **3 years** collecting a 400 000-form test corpus for Finnish.
The program it was waiting for took an **afternoon**.

Here's the story — and why I think it says something about the last 12 months.

Finnish nouns are gloriously hard: 15 grammatical cases × singular/plural ≈ 30 forms per word, 49 declension classes, and consonant gradation that bends *katu → kadun*, *kauppa → kaupan*, *kenkä → kengän*. I'd slowly built a ~400k-form corpus to *test* an inflection engine — but the engine itself never got built.

**A year ago I tried to generate it with an LLM. No model could.** They'd write a plausible single function, then fall apart on the real thing: 49 interacting classes, gradation, vowel harmony, a multi-crate Rust workspace, and a 400k-row oracle to validate against. Too big to hold in its head, too stateful to iterate on.

**This week I tried again with Opus 4.8. It was a matter of hours.** Not autocomplete — an agent that planned the architecture, wrote the whole workspace (engine + rules + ingest + HTTP server + CLI), ran the test suite, read the failures, and *fixed itself*. At one point the corpus caught a bug where two gradation letters were swapped (E = p:v, F = t:d); the model diagnosed it from the failing forms and the fix lifted accuracy ~2.4 points.

The result — **keinontolibrary**, validated by declining all **25 694** Kotus nouns into every case and checking against my corpus:

✅ **98.0%** agreement with the reference corpus
✅ **166k** forms produced & checked across **34 declension types**
✅ **99.5%** slot coverage · **< 10 MB** static container · **µs** lookups

**What actually changed in a year?** Three things, together: context windows big enough to hold a whole project *and* its test data; genuine long-horizon agency (plan → build → test → debug → repeat for hours without losing the thread); and reliability on multi-file, stateful code instead of just snippets. Independent coding benchmarks tell the same story — on real-world "fix this GitHub issue" tasks, frontier models went from solving a small fraction to solving the majority. But the qualitative jump is the headline: work that was "no model can do this" became "done before lunch."

The corpus was the hard, human part — 3 years of judgment. The implementation became the easy part. That inversion is new.

Data: Kotus *Nykysuomen sanalista 2024* (CC BY 4.0). The ~400k-form corpus was collected over 3 years and morphologically labeled with Voikko (voikko.puimula.org).

What's the project *you've* been waiting to build because the implementation was the bottleneck?

---

## Capability note (longer cut — for the comments or a follow-up)

A year ago, frontier LLMs were excellent at *local* code: a function, a regex, a class, a tricky algorithm you could describe in a paragraph. Where they fell down was **scale and statefulness** — anything that needed to (a) hold a large codebase plus its data in working memory, (b) make many coordinated edits across files, and (c) keep going through a long plan-build-test-debug loop without drifting.

Over the last ~12 months three capabilities crossed a threshold at roughly the same time:

1. **Context** — windows grew to hundreds of thousands of tokens (Opus 4.8 ran this in a 1M-token context), enough to keep an entire multi-crate workspace *and* the linguistic reference data in view at once.
2. **Long-horizon agency** — models became dependable across long tool-using sessions: read files, run the compiler and tests, interpret failures, edit, re-run — for hours, toward a measurable target (here: ≥ a parity threshold against the corpus).
3. **Reliability on real software** — fewer hallucinated APIs, better at large refactors and at *finding their own bugs* (the swapped gradation letters were caught by the data, then fixed).

The honest framing: this isn't "AI writes code now" — it's that the **bottleneck moved**. The scarce, valuable thing was the 3-year corpus and the judgment in it. The 49-class engine — previously a multi-week specialist project — became the cheap part. When implementation gets cheap, *what you choose to build and how you validate it* becomes the whole game.
