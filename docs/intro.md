# keinontolibrary

A fast, embeddable Rust library that declines **Finnish nouns**. Give it a noun lemma and a
case; it returns the inflected form(s).

```text
decline("hevonen", Number::Plural, Case::Inessive) -> ["hevosissa"]
```

It is **data-backed** — precomputed forms from a reference corpus collected over three years
and labeled with [Voikko](https://voikko.puimula.org/) — with a **rule-based fallback** over
the [Kotus](https://kaino.kotus.fi/sanat/nykysuomi/) declension classes (taivutustyypit 1–49
with consonant gradation). It covers **100% of declinable Kotus 2024 nominal rows**; verbs and
the indeclinable tn 99/100 classes are out of scope.

These guides are task-oriented — pick the one that matches what you want to do:

- **[Embed in a Rust program](guides/embed-rust.md)** — add the crate and decline words in
  your own code.
- **[Decline from the command line](guides/cli.md)** — look up forms from a terminal or script.
- **[Run the HTTP service](guides/http-service.md)** — stand up the declension service /
  container.
- **[Build the data artifact](guides/build-artifact.md)** — rebuild the packed data from
  sources.
- **[Fix a wrong declension](guides/contributing.md)** — the right way to correct a form.

For background on how compounds are handled, see
**[Compound nouns](compound-nouns.md)**.

The source lives on [GitHub](https://github.com/timokoola/keinontolibrary). If you'd rather
browse declensions than embed them, try [humalapaikallissija.com](https://humalapaikallissija.com),
a toy built on this library.
