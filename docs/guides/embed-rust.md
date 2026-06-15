# Embed in a Rust program

Decline Finnish nominals from your own crate.

## Add the dependency

```toml
[dependencies]
keinontolibrary-core = "0.1"
keinontolibrary-data = "0.1"   # the artifact-backed lookup store
```

`keinontolibrary-core` is the API (enums, `Engine`, `decline`, `paradigm`).
`keinontolibrary-data` loads the packed artifact and wires the rule fallback behind it.

## Build an engine and decline a word

```rust
use keinontolibrary_core::{Case, Number};
use keinontolibrary_data::build_engine;

// Loads the packed artifact + (optional) overlay, with the rule engine as fallback.
let bundle = build_engine("data/artifact/keinontolibrary.bin", "data/overlay.jsonl")?;
let engine = &bundle.engine;

let forms = engine.decline("hevonen", Number::Plural, Case::Inessive)?;
assert_eq!(forms.primary(), Some("hevosissa"));
assert_eq!(forms.variants, ["hevosissa"]);   // primary first; some slots have several
```

`decline` returns `Forms { variants, status, source, coincides_with }`:
- `variants` — surface forms, primary first (genitive plural and illative often have more
  than one valid form).
- `source` — `Lookup` (corpus), `Generated` (rules/registry), or `Overlay`.
- `coincides_with` — set on the accusative (singular = genitive, plural = nominative).

## The whole paradigm

```rust
let p = engine.paradigm("talo")?;
for (number, case, forms) in p.iter() {
    println!("{number} {case}: {}", forms.variants.join(", "));
}
```

## Handle the three error cases

`decline`/`paradigm` return `Result<_, Error>`:

```rust
use keinontolibrary_core::Error;

match engine.decline("kuusi", Number::Singular, Case::Inessive) {
    Ok(f) => println!("{:?}", f.primary()),
    Err(Error::UnknownWord(w)) => eprintln!("not a known word: {w}"),
    Err(Error::Ambiguous { lemma, paradigms }) => {
        // Homonyms: kuusi is "six" (tn24) and "spruce" (tn27). Pick one and retry.
        eprintln!("{lemma} is ambiguous: {paradigms:?}");
    }
    Err(Error::DefectiveForm { lemma, number, case }) => {
        // The slot genuinely does not exist (e.g. sakset has no singular).
        eprintln!("{lemma} has no {number} {case}");
    }
}
```

## Disambiguate homonyms

Pass an explicit paradigm with `decline_with` / `paradigm_with`:

```rust
use keinontolibrary_core::ParadigmRef;

// "kuusi" → spruce (Kotus class tn27)
let f = engine.decline_with("kuusi", Number::Singular, Case::Inessive,
                            &ParadigmRef::new(None, 27))?;
assert_eq!(f.primary(), Some("kuusessa"));
```

`ParadigmRef::new(hn, tn)` — a `None` field is a wildcard (matches any homonym number /
any class). To require a specific reading, pass `Some(_)`.

## Notes

- All inputs are normalized (trimmed, NFC, lowercased) — `"Talo"`, `" talo "`, and
  `"talo"` are equivalent.
- Resolution order is **overlay → lookup → rule fallback**: an overlay entry wins, then
  the corpus, then the rules.
- The artifact is not committed (it embeds corpus-derived data; see `LICENSING.md`).
  Build it once with `cargo run -p keinontolibrary-ingest` — see
  [build-artifact](build-artifact.md) — or embed your own via `LookupData::from_bytes`.
