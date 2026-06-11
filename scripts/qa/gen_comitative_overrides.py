#!/usr/bin/env python3
"""Mint per-lemma comitative-style overrides by probing Voikko.

The plural comitative has two citation shapes: bare `-ine` for modifiers
(`punaisine`) and possessive `-ineen`/`-inensA` for head nouns (`taloineen`). The
Kotus word class decides the default (any `substantiivi` reading → possessive), but
Voikko's lexicon disagrees for a set of dual-class words: `aateliton` is
"adjektiivi, substantiivi" in Kotus, yet Voikko rejects `*aatelittomineen` and accepts
only `aatelittomine`. For each lemma whose generated comitative Voikko rejects, probe
the opposite style; when exactly one style is accepted, emit
{"lemma": ..., "bare": bool}. The ingest applies it on top of the Kotus-derived
adjective flag.

Usage:
  .venv/bin/python scripts/qa/gen_comitative_overrides.py \
      [--dump qa/generated.jsonl] [--out data/comitative-overrides.jsonl]
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from verify_voikko import make_voikko  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dump", default="qa/generated.jsonl")
    ap.add_argument("--out", default="data/comitative-overrides.jsonl")
    args = ap.parse_args()

    v = make_voikko()

    def comitative_ok(form, lemma):
        return any(
            a.get("BASEFORM", "").lower() == lemma and a.get("SIJAMUOTO") == "seuranto"
            for a in v.analyze(form)
        )

    n = 0
    with open(args.dump, encoding="utf-8") as fh, open(args.out, "w", encoding="utf-8") as out:
        for line in fh:
            row = json.loads(line)
            if row["case"] != "comitative" or row["number"] != "plural":
                continue
            rules = row.get("rules")
            if not rules or not rules["variants"]:
                continue
            lemma, primary = row["lemma"], rules["variants"][0]
            # Derive the opposite style from the served shape.
            if primary.endswith("neen"):
                served_bare, other = False, primary[:-2]  # taloineen -> taloine
            elif primary.endswith("ne"):
                served_bare, other = True, primary + "en"  # punaisine -> punaisineen
            else:
                continue
            if comitative_ok(primary, lemma):
                continue  # served style is fine
            if comitative_ok(other, lemma):
                out.write(
                    json.dumps({"lemma": lemma, "bare": not served_bare}, ensure_ascii=False)
                    + "\n"
                )
                n += 1
    print(f"{n} comitative overrides -> {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
