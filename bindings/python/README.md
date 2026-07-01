# keinontolibrary (Python)

Decline Finnish nouns from Python. A fast Rust engine (data-backed, Voikko-verified, with a
rule-based fallback over the Kotus declension classes) with native [PyO3](https://pyo3.rs)
bindings. The engine and its data ship inside the wheel — no build step, no external service.

```python
import keinontolibrary

keinontolibrary.decline("hevonen", "plural", "inessive")   # ['hevosissa']
keinontolibrary.paradigm("talo")["singular"]["inessive"]   # ['talossa']
```

From the terminal, without installing (via [uv](https://docs.astral.sh/uv/)):

```sh
uvx keinontolibrary decline hevonen --number plural --case inessive
uvx keinontolibrary table talo
```

Or install it:

```sh
uv pip install keinontolibrary      # or: pip install keinontolibrary
```

## API

- `decline(word, number, case) -> list[str]` — one slot. `number` is `"singular"`/`"plural"`;
  `case` is an English case name (`"nominative"`, `"genitive"`, `"inessive"`, …). Raises
  `KeyError` for an unknown word, `ValueError` for bad arguments / ambiguity.
- `paradigm(word) -> dict[str, dict[str, list[str]]]` — the full `{number: {case: [forms]}}`.
- `Inflector(artifact_path, overlay_path="")` — construct directly only to point at custom data.

Scope: all nominal Kotus classes 1–51. Verbs and the indeclinable tn 99/100 are out of scope.

Home & docs: <https://keinonto.com> · Source: <https://github.com/timokoola/keinontolibrary>
MIT-licensed. Contains Kotus *Nykysuomen sanalista* data (CC BY 4.0).
