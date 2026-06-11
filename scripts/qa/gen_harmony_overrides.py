#!/usr/bin/env python3
"""Mint per-lemma vowel-harmony overrides by probing Voikko with generated forms.

Finnish suffix harmony follows the FINAL component of a compound: antigeenissä (head
`geeni`), not *antigeenissa. Whether a word IS a compound is lexical knowledge — the
spelling alone cannot tell `ajanviete` (compound, front) from `harakiri` (simplex loan,
back; Voikko's STRUCTURE even marks a morpheme point in it, so segmentation is NOT a
reliable proxy). The reliable signal is Voikko's own spelling judgment on inflected
forms: for each lemma, take the rule engine's generated forms for harmony-bearing slots
(from the QA dump), spell-check each against its harmony-flipped twin, and let the
votes decide. An override is emitted only when the verdict contradicts the rule
engine's default (last strong vowel).

Because the dump was generated WITH the previous overrides (if any), both directions
are probed symmetrically — rerunning after a re-ingest converges.

Usage:
  .venv/bin/python scripts/qa/gen_harmony_overrides.py \
      [--dump qa/generated.jsonl] [--out data/harmony-overrides.jsonl]
"""
import argparse
import collections
import json
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from verify_voikko import make_voikko  # noqa: E402

# Slots whose endings carry the harmony vowel unambiguously.
PROBE_CASES = {"inessive", "elative", "adessive", "ablative", "essive", "abessive", "partitive"}
FLIP = str.maketrans("aouäöy", "äöyaou")


def is_back(word):
    """Mirror of the rule engine's default: last strong vowel decides, else front."""
    for c in reversed(word):
        if c in "aou":
            return True
        if c in "äö":
            return False
    return False


def flip_suffix(lemma, form):
    """Flip harmony vowels in the part of `form` beyond its common prefix with `lemma`."""
    i = 0
    while i < min(len(lemma), len(form)) and lemma[i] == form[i]:
        i += 1
    i = max(i - 1, 0)
    return form[:i] + form[i:].translate(FLIP)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dump", default="qa/generated.jsonl")
    ap.add_argument("--out", default="data/harmony-overrides.jsonl")
    args = ap.parse_args()

    v = make_voikko()
    votes = collections.defaultdict(collections.Counter)  # lemma -> {'front': n, 'back': n}
    with open(args.dump, encoding="utf-8") as fh:
        for line in fh:
            row = json.loads(line)
            if row["case"] not in PROBE_CASES:
                continue
            rules = row.get("rules")
            if not rules or not rules["variants"]:
                continue
            lemma = row["lemma"]
            form = rules["variants"][0]
            flipped = flip_suffix(lemma, form)
            if flipped == form:
                continue
            # analyze() with a BASEFORM match, not bare spell(): Voikko's compound
            # guesser accepts coincidental splits (talviössä as talvi+yö-ish), which
            # would vote the wrong way for an unrelated lemma like talvio.
            def is_lemma_form(f):
                return any(
                    a.get("BASEFORM", "").lower() == lemma for a in v.analyze(f)
                )

            ok, ok_flip = is_lemma_form(form), is_lemma_form(flipped)
            if ok == ok_flip:
                continue  # both accepted (variation) or both unknown: no signal
            accepted = form if ok else flipped
            # Which harmony does the accepted form's suffix show?
            tail = accepted[max(len(lemma) - 2, 0):]
            if any(c in "äö" for c in tail):
                votes[lemma]["front"] += 1
            elif any(c in "aou" for c in tail):
                votes[lemma]["back"] += 1

    # Sticky merge: an override earned with quorum stays until a quorum probe actively
    # contradicts it. (Once an override is applied, the next dump generates the corrected
    # forms, which can move a marginal lemma below quorum — without stickiness those
    # lemmas oscillate between regenerations.)
    existing = {}
    try:
        with open(args.out, encoding="utf-8") as fh:
            for line in fh:
                row = json.loads(line)
                existing[row["lemma"]] = row["front"]
    except FileNotFoundError:
        pass

    decided = dict(existing)
    for lemma, c in votes.items():
        # Strict quorum: at least 3 probe slots, unanimous. A single coincidental
        # spell() hit (rare words, homonymous flipped forms) must not flip a lemma.
        if c["front"] + c["back"] < 3 or (c["front"] and c["back"]):
            continue
        front = c["front"] > c["back"]
        if front == (not is_back(lemma)):
            decided.pop(lemma, None)  # probe says the default is right after all
        else:
            decided[lemma] = front

    with open(args.out, "w", encoding="utf-8") as out:
        for lemma in sorted(decided):
            out.write(
                json.dumps({"lemma": lemma, "front": decided[lemma]}, ensure_ascii=False) + "\n"
            )
    print(
        f"probed {len(votes)} lemmas with signal -> {len(decided)} overrides "
        f"({len(existing)} carried) -> {args.out}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
