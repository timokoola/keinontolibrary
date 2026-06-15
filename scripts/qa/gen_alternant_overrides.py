#!/usr/bin/env python3
"""Mint slot-level *alternant-completion* overrides from the QA dump.

The artifact's lookup layer holds whatever the reference corpus attested. Natural text
under-attests the rarer of two legitimate alternants — e.g. `omena` (tn 11) plural
ablative is attested only as `omenilta`, though `omenoilta` is equally correct (the
parallel `-oi-` plural stem). Because the engine resolves a slot lookup-first and stops,
the rule generator's extra alternant is shadowed and never served.

This closes that gap WITHOUT diluting the corpus's trust: for every slot the engine
answers from lookup, take the alternants the rule engine would additionally produce,
and keep only those Voikko independently confirms as `lemma + case + number` (the same
analysis gate the verifier uses). The kept forms are emitted as a per-slot override that
ingest unions into the corpus slot (corpus form stays primary). A blind ingest-time union
of rule output would inject the generator's ~2% wrong forms; the Voikko gate is what makes
this safe.

Sticky by construction: after a re-ingest the slot already serves both forms, so the
candidate set is empty on the next run — existing overrides are carried, not dropped.

Usage:
  .venv/bin/python scripts/qa/gen_alternant_overrides.py \
      [--dump qa/generated.jsonl] [--out data/alternant-overrides.jsonl]
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from verify_voikko import CASE_TO_SIJAMUOTO, analysis_matches, make_voikko  # noqa: E402

# Deterministic slot ordering for the emitted file.
CASE_ORDER = {c: i for i, c in enumerate(CASE_TO_SIJAMUOTO)}
NUMBER_ORDER = {"singular": 0, "plural": 1}


def verified_alternants(v, lemma, case, number, candidates):
    """The subset of `candidates` Voikko confirms as this exact slot of `lemma`."""
    return [c for c in candidates if any(analysis_matches(a, lemma, case, number) for a in v.analyze(c))]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dump", default="qa/generated.jsonl")
    ap.add_argument("--out", default="data/alternant-overrides.jsonl")
    args = ap.parse_args()

    v = make_voikko()

    # Carry existing overrides (stickiness): the next dump, built WITH them, serves both
    # forms from lookup, so it surfaces no new candidate — without this they would vanish.
    decided = {}  # (lemma, tn, number, case) -> variants
    try:
        with open(args.out, encoding="utf-8") as fh:
            for line in fh:
                row = json.loads(line)
                decided[(row["lemma"], row["tn"], row["number"], row["case"])] = row["variants"]
    except FileNotFoundError:
        pass
    carried = len(decided)

    minted = 0
    with open(args.dump, encoding="utf-8") as fh:
        for line in fh:
            row = json.loads(line)
            case = row["case"]
            # The accusative is derived at ingest from genitive-sg / nominative-pl, so an
            # override there would be redundant (and Voikko reports it as nimento/omanto).
            if case == "accusative":
                continue
            engine = row.get("engine")
            rules = row.get("rules")
            # Only lookup-answered slots are shadowed; generated/overlay slots already
            # serve the rule alternants (or are deliberate).
            if not engine or engine.get("source") != "lookup" or not rules or not rules.get("variants"):
                continue
            served = engine["variants"]
            candidates = [f for f in rules["variants"] if f not in served]
            if not candidates:
                continue
            lemma, number = row["lemma"], row["number"]
            kept = verified_alternants(v, lemma, case, number, candidates)
            if not kept:
                continue
            # Corpus form(s) stay first (primary); append the verified alternants.
            full = list(served) + [f for f in kept if f not in served]
            decided[(lemma, row["tn"], number, case)] = full
            minted += 1

    with open(args.out, "w", encoding="utf-8") as out:
        for key in sorted(
            decided, key=lambda k: (k[0], k[1], NUMBER_ORDER.get(k[2], 9), CASE_ORDER.get(k[3], 99))
        ):
            lemma, tn, number, case = key
            out.write(
                json.dumps(
                    {"lemma": lemma, "tn": tn, "number": number, "case": case, "variants": decided[key]},
                    ensure_ascii=False,
                )
                + "\n"
            )
    print(
        f"minted {minted} slot overrides ({carried} carried) -> {len(decided)} total -> {args.out}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
