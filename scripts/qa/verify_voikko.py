#!/usr/bin/env python3
"""Batch Voikko oracle for the QA loop: verify every generated form.

Reads the qa-dump JSONL (`qa/generated.jsonl`, one row per lemma×paradigm×slot), collects
every distinct surface variant the engine or the rule generator produced, and asks Voikko
about each: does some analysis say this form IS <lemma> in <case> <number>?

Verdict per (lemma, form, case, number):
  ok                  — a matching clean analysis exists
  wrong_analysis      — Voikko analyzes the form, but never as this lemma/case/number
  misspelled          — Voikko has no analysis and the speller rejects it (a non-word)
  not_in_voikko       — no analysis, but the speller accepts it (oracle can't judge)
  lemma_not_in_voikko — Voikko doesn't know the lemma at all (Kotus ⊃ Voikko lexicon),
                        so no form of it can be judged

Cleanliness rules (the corpus carries enclitics in FOCUS/KYSYMYSLIITE/POSSESSIVE — see
voikko.rs): an accepted analysis must have no FOCUS, no KYSYMYSLIITE and no POSSESSIVE,
EXCEPT the comitative, whose citation form is the 3rd-person possessive (-ineen,
POSSESSIVE=3). Slot-semantics special cases mirror the ingest:
  accusative — Voikko has no noun accusative; sg must analyze as genitive, pl as
               nominative (the engine derives them the same way)
  comitative — formally plural morphology; NUMBER is not required to match

Usage:
  .venv/bin/python scripts/qa/verify_voikko.py \
      [--dump qa/generated.jsonl] [--out qa/voikko-verdicts.jsonl] \
      [--sample N] [--jobs N]

Deps: libvoikko (native lib: on Homebrew macOS run via
  DYLD_LIBRARY_PATH=/opt/homebrew/lib — scripts/qa/run.sh sets this).
"""
import argparse
import json
import multiprocessing as mp
import os
import sys
import time

CASE_TO_SIJAMUOTO = {
    "nominative": "nimento",
    "genitive": "omanto",
    "partitive": "osanto",
    "inessive": "sisaolento",
    "elative": "sisaeronto",
    "illative": "sisatulento",
    "adessive": "ulkoolento",
    "ablative": "ulkoeronto",
    "allative": "ulkotulento",
    "essive": "olento",
    "translative": "tulento",
    "abessive": "vajanto",
    "comitative": "seuranto",
    "instructive": "keinonto",
}

VOIKKO = None


def make_voikko():
    from libvoikko import Voikko

    candidates = [None, os.environ.get("VOIKKO_DICTIONARY_PATH")]
    candidates += ["/opt/homebrew/lib/voikko", "/usr/local/lib/voikko", "/usr/lib/voikko"]
    last = None
    for path in (c for c in candidates if c is None or os.path.isdir(c)):
        try:
            return Voikko("fi", path) if path else Voikko("fi")
        except Exception as e:  # noqa: BLE001
            last = e
    raise SystemExit(f"voikko dictionary not found ({last})")


def init_worker():
    global VOIKKO
    VOIKKO = make_voikko()


def analysis_matches(a, lemma, case, number):
    """One Voikko analysis vs one expected slot, with the slot-semantics special cases."""
    if a.get("BASEFORM", "").lower() != lemma.lower():
        return False
    # Clean forms only: enclitics/possessives disqualify, except the comitative citation.
    if a.get("FOCUS") or a.get("KYSYMYSLIITE"):
        return False
    if a.get("POSSESSIVE") and case != "comitative":
        return False
    sija = a.get("SIJAMUOTO", "")
    if case == "accusative":
        expected = "omanto" if number == "singular" else "nimento"
        return sija in (expected, "kohdanto") and a.get("NUMBER") == number
    if case == "comitative":
        return sija == "seuranto"  # formally plural; NUMBER not meaningful for the slot
    return sija == CASE_TO_SIJAMUOTO[case] and a.get("NUMBER") == number


def lemma_known(lemma):
    """Voikko knows the lemma if it analyzes its citation form as itself."""
    return any(
        a.get("BASEFORM", "").lower() == lemma.lower() for a in VOIKKO.analyze(lemma)
    )


def verify_group(item):
    """One lemma's jobs: [(form, case, number), ...] -> [(form, case, number, verdict)]."""
    lemma, jobs = item
    if not lemma_known(lemma):
        return lemma, [(f, c, n, "lemma_not_in_voikko") for f, c, n in jobs]
    out = []
    cache = {}
    for form, case, number in jobs:
        if form not in cache:
            cache[form] = VOIKKO.analyze(form)
        analyses = cache[form]
        if any(analysis_matches(a, lemma, case, number) for a in analyses):
            verdict = "ok"
        elif analyses:
            verdict = "wrong_analysis"
        elif VOIKKO.spell(form):
            verdict = "not_in_voikko"
        else:
            verdict = "misspelled"
        out.append((form, case, number, verdict))
    return lemma, out


def collect_jobs(dump_path, sample):
    """Group distinct (form, case, number) verification jobs by lemma."""
    jobs = {}
    with open(dump_path, encoding="utf-8") as fh:
        for line in fh:
            row = json.loads(line)
            lemma, case, number = row["lemma"], row["case"], row["number"]
            seen = jobs.setdefault(lemma, set())
            for leg in ("engine", "rules"):
                forms = row.get(leg)
                if forms:
                    for v in forms["variants"]:
                        seen.add((v, case, number))
    if sample and sample < len(jobs):
        # Deterministic stratified-ish sample: every k-th lemma in sorted order.
        keys = sorted(jobs)
        step = len(keys) / sample
        jobs = {keys[int(i * step)]: jobs[keys[int(i * step)]] for i in range(sample)}
    return {lemma: sorted(forms) for lemma, forms in sorted(jobs.items())}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dump", default="qa/generated.jsonl")
    ap.add_argument("--out", default="qa/voikko-verdicts.jsonl")
    ap.add_argument("--sample", type=int, default=0, help="verify only N lemmas (quick mode)")
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) - 1))
    args = ap.parse_args()

    t0 = time.time()
    groups = collect_jobs(args.dump, args.sample)
    n_jobs = sum(len(v) for v in groups.values())
    print(f"verifying {n_jobs} forms across {len(groups)} lemmas "
          f"({args.jobs} workers)", file=sys.stderr)

    counts = {}
    with open(args.out, "w", encoding="utf-8") as out, mp.Pool(
        args.jobs, initializer=init_worker
    ) as pool:
        done = 0
        for lemma, results in pool.imap_unordered(
            verify_group, groups.items(), chunksize=64
        ):
            for form, case, number, verdict in results:
                counts[verdict] = counts.get(verdict, 0) + 1
                out.write(json.dumps(
                    {"lemma": lemma, "form": form, "case": case,
                     "number": number, "verdict": verdict},
                    ensure_ascii=False) + "\n")
            done += 1
            if done % 2000 == 0:
                print(f"  {done}/{len(groups)} lemmas", file=sys.stderr)

    for verdict in sorted(counts):
        print(f"{verdict:<20} {counts[verdict]}", file=sys.stderr)
    print(f"wrote {args.out} in {time.time() - t0:.0f}s", file=sys.stderr)


if __name__ == "__main__":
    main()
