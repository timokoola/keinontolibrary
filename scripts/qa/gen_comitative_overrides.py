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

    # Sticky like the harmony minting: probe BOTH styles symmetrically; an existing
    # override is kept when the probe has no signal (Voikko silent on both), so the
    # set is stable regardless of which style the current dump happens to serve.
    existing = {}
    try:
        with open(args.out, encoding="utf-8") as fh:
            for line in fh:
                if line.strip():
                    row = json.loads(line)
                    existing[row["lemma"]] = row["bare"]
    except FileNotFoundError:
        pass

    decided = dict(existing)
    with open(args.dump, encoding="utf-8") as fh:
        for line in fh:
            row = json.loads(line)
            if row["case"] != "comitative" or row["number"] != "plural":
                continue
            rules = row.get("rules")
            if not rules or not rules["variants"]:
                continue
            lemma, primary = row["lemma"], rules["variants"][0]
            if primary.endswith("neen"):
                bare_form, poss_form = primary[:-2], primary  # taloine, taloineen
            elif primary.endswith("ne"):
                bare_form, poss_form = primary, primary + "en"
            else:
                continue
            bare_ok = comitative_ok(bare_form, lemma)
            poss_ok = comitative_ok(poss_form, lemma)
            if bare_ok == poss_ok:
                if bare_ok:
                    decided.pop(lemma, None)  # both fine: the Kotus default is safe
                continue  # neither known: no signal, keep any existing override
            decided[lemma] = bare_ok

    with open(args.out, "w", encoding="utf-8") as out:
        for lemma in sorted(decided):
            out.write(
                json.dumps({"lemma": lemma, "bare": decided[lemma]}, ensure_ascii=False) + "\n"
            )
    print(
        f"{len(decided)} comitative overrides ({len(existing)} carried) -> {args.out}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
