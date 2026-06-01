# LinkedIn post — draft

> Attach `keinontolibrary-accuracy.png`. Numbers below are from `metrics/metrics.json`
> (regenerate with `keinontolibrary-metrics`). Edit freely — this is a starting point.

---

I taught a Rust program to decline every Finnish noun. Here's how it did. 🇫🇮🦀

Finnish nouns are a beautiful headache: 15 grammatical cases × singular/plural ≈ 30 forms per word, 49 declension classes, and consonant gradation that turns *katu* into *kadun*, *kauppa* into *kaupan*, *kenkä* into *kengän*. Getting it right by hand is hard; doing it for the **whole language** is a systems problem.

So I built **keinontolibrary** — a small, embeddable declension engine:

• A **data-backed lookup** over a reference corpus of inflected forms (which I generated using the excellent Voikko tool), and
• A **rule engine** that derives forms from the Kotus declension class + gradation pattern when the lookup doesn't have them.

Then I tested it the honest way: take the **25 694 simple nouns** from the Kotus word list, generate **all singular and plural cases**, and check every form against the reference corpus.

The results:

✅ **98.0%** of generated forms agree with the reference corpus
✅ **166k** forms generated and checked, across **34 declension types**
✅ **99.5%** slot coverage
✅ Ships as a **< 10 MB** static container, with **microsecond** lookups

The corpus made a brutally effective oracle. It caught a bug where I'd swapped two gradation letters (E = p:v, F = t:d) — one fix lifted accuracy by ~2.4 points across the board. The single hardest case to get right? The **plural partitive** (*kissoja*, *omenoita*, *ristejä*) — its `-ja / -ita / -a` variation is where most of the remaining misses live.

Stack: Rust workspace (core + rules + ingest + axum HTTP server + CLI + FFI), clippy-pedantic and `-D warnings` throughout, the corpus round-trip as a CI gate.

Data: Kotus *Nykysuomen sanalista 2024* (CC BY 4.0); reference forms generated with Voikko (voikko.puimula.org).

Code: github.com/timokoola/keinontolibrary

What would you want a fast, embeddable Finnish morphology engine for?

#Rust #NLP #Finnish #ComputationalLinguistics #OpenSource #SoftwareEngineering
