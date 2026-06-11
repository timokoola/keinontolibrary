#!/usr/bin/env python3
"""QA triage report: join the qa-dump with the Voikko verdicts, classify every slot,
diff against the committed baseline, and emit qa/report.json + qa/report.md.

Slot categories (rule-engine focus — lookup slots equal the corpus by construction):

  corpus attested:
    pass            rules primary matches the corpus (and Voikko doesn't object)
    oracle_conflict rules primary matches the corpus but Voikko rejects the form
    extra_variant   rules primary differs from corpus, but Voikko accepts it
                    (likely a legitimate alternant the corpus never attested)
    engine_bug      rules primary differs from corpus AND Voikko rejects it
    parity_fail     rules primary differs from corpus; Voikko can't judge the lemma
    rules_gap       corpus has the form but the rule engine generates nothing

  corpus silent:
    pass_voikko     Voikko accepts the rules primary (coverage beyond the corpus)
    suspect_misspelled / suspect_analysis
                    only Voikko as witness, and it rejects the form
    unverified      neither oracle can judge (lemma outside Voikko's lexicon)
    unsupported     no rules output and no corpus data

Separately, `served_misspelled` lists slots where a variant the ENGINE actually serves
(what users get) is a non-word per Voikko — the hard-fail tier.

Failing categories (tracked in the baseline, gate on regressions):
  engine_bug, parity_fail, oracle_conflict, suspect_misspelled, suspect_analysis,
  served_misspelled.

Usage:
  python3 scripts/qa/report.py [--dump ...] [--verdicts ...] [--baseline qa/baseline.json]
                               [--out-json qa/report.json] [--out-md qa/report.md]
                               [--update-baseline] [--gate]

--gate exits 1 when there are failures NOT in the baseline (regressions).
--update-baseline rewrites the baseline to the current failure set (do this in the same
  PR as a fix, never to bury a regression).
"""
import argparse
import json
import sys
from collections import Counter, defaultdict

FAILING = {
    "engine_bug",
    "parity_fail",
    "oracle_conflict",
    "suspect_misspelled",
    "suspect_analysis",
    "served_misspelled",
}
SAMPLES_PER_CATEGORY = 25


def vkey(lemma, form, case, number):
    return f"{lemma}\t{form}\t{case}\t{number}"


def load_verdicts(path):
    verdicts = {}
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            r = json.loads(line)
            verdicts[vkey(r["lemma"], r["form"], r["case"], r["number"])] = r["verdict"]
    return verdicts


def classify(row, verdicts):
    """Return (category, detail) for one dump row."""
    corpus = row.get("corpus")
    corpus_variants = corpus["variants"] if corpus else []
    rules = row.get("rules")
    rules_variants = rules["variants"] if rules and rules["status"] != "missing" else []

    if not rules_variants:
        if corpus_variants:
            return "rules_gap", None
        return "unsupported", None

    rp = rules_variants[0]
    v = verdicts.get(vkey(row["lemma"], rp, row["case"], row["number"]))

    if corpus_variants:
        if rp in corpus_variants:
            if v in ("wrong_analysis", "misspelled"):
                return "oracle_conflict", {"form": rp, "voikko": v}
            return "pass", None
        detail = {"form": rp, "corpus": corpus_variants, "voikko": v}
        if v == "ok":
            return "extra_variant", detail
        if v in ("wrong_analysis", "misspelled"):
            return "engine_bug", detail
        return "parity_fail", detail

    if v == "ok":
        return "pass_voikko", None
    # A rare secondary paradigm (parenthesized in Kotus, e.g. koiras "41, (39)") that
    # the corpus never attests is routinely absent from Voikko's lexicon too — the
    # oracle cannot judge it, so a rejection is not a finding.
    if row.get("rare") and v in ("misspelled", "wrong_analysis"):
        return "rare_unverified", None
    if v == "misspelled":
        return "suspect_misspelled", {"form": rp}
    if v == "wrong_analysis":
        return "suspect_analysis", {"form": rp}
    return "unverified", None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dump", default="qa/generated.jsonl")
    ap.add_argument("--verdicts", default="qa/voikko-verdicts.jsonl")
    ap.add_argument("--baseline", default="qa/baseline.json")
    ap.add_argument("--accepted", default="qa/accepted.jsonl")
    ap.add_argument("--out-json", default="qa/report.json")
    ap.add_argument("--out-md", default="qa/report.md")
    ap.add_argument("--update-baseline", action="store_true")
    ap.add_argument("--gate", action="store_true")
    args = ap.parse_args()

    verdicts = load_verdicts(args.verdicts)
    # Documented-accepted slots: per-lemma patterns ("lemma|tn" or "lemma|tn|number|case")
    # with a reason — for failures the oracle cannot judge fairly (Voikko lacks the
    # lemma's inflections, corpus casing, homonym paradigms outside Voikko's lexicon).
    accepted = {}
    try:
        with open(args.accepted, encoding="utf-8") as fh:
            for line in fh:
                if line.strip():
                    row = json.loads(line)
                    accepted[row["match"]] = row["reason"]
    except FileNotFoundError:
        pass
    # Lemmas the verifier actually covered (sample mode covers a subset); slots for other
    # lemmas get corpus-only classification, which is fine, but served_misspelled and the
    # suspect tiers only fire where Voikko looked.
    counts = Counter()
    by_tn = defaultdict(Counter)
    by_case = defaultdict(Counter)
    samples = defaultdict(list)
    failures = set()
    served_misspelled = []

    with open(args.dump, encoding="utf-8") as fh:
        for line in fh:
            row = json.loads(line)
            slot_id = f"{row['lemma']}|{row['tn']}|{row['number']}|{row['case']}"
            category, detail = classify(row, verdicts)
            if category in FAILING and (
                f"{row['lemma']}|{row['tn']}" in accepted or slot_id in accepted
            ):
                category, detail = "accepted", None
            counts[category] += 1
            by_tn[row["tn"]][category] += 1
            by_case[f"{row['number']} {row['case']}"][category] += 1
            if category in FAILING:
                # Baseline keys are SLOT identity, not slot+category: a fix that merely
                # shifts a slot's failure category (e.g. misspelled → wrong-analysis)
                # must not read as a regression.
                failures.add(slot_id)
                if len(samples[category]) < SAMPLES_PER_CATEGORY:
                    samples[category].append({"slot": slot_id, **(detail or {})})

            engine = row.get("engine")
            # The served-variant check honors the same exemptions as classify():
            # accepted slots (whatever category classify chose for them) and rare
            # secondary paradigms the oracle cannot judge.
            if (
                f"{row['lemma']}|{row['tn']}" in accepted
                or slot_id in accepted
                or row.get("rare")
            ):
                engine = None
            if engine and engine["variants"]:
                for variant in engine["variants"]:
                    if (
                        verdicts.get(
                            vkey(row["lemma"], variant, row["case"], row["number"])
                        )
                        == "misspelled"
                    ):
                        failures.add(slot_id)
                        counts["served_misspelled"] += 1
                        if len(served_misspelled) < SAMPLES_PER_CATEGORY:
                            served_misspelled.append(
                                {"slot": slot_id, "form": variant,
                                 "source": engine["source"]}
                            )
                        break

    # Baseline diff.
    try:
        with open(args.baseline, encoding="utf-8") as fh:
            baseline = set(json.load(fh)["failures"])
    except FileNotFoundError:
        baseline = set()
    regressions = sorted(failures - baseline)
    fixed = sorted(baseline - failures)

    report = {
        "totals": dict(counts),
        "n_failures": len(failures),
        "n_regressions": len(regressions),
        "n_fixed": len(fixed),
        "regressions": regressions[:200],
        "fixed": fixed[:200],
        "samples": {**samples, "served_misspelled": served_misspelled},
        "by_tn": {str(tn): dict(c) for tn, c in sorted(by_tn.items())},
        "by_case": {k: dict(c) for k, c in sorted(by_case.items())},
    }
    with open(args.out_json, "w", encoding="utf-8") as fh:
        json.dump(report, fh, ensure_ascii=False, indent=1)

    with open(args.out_md, "w", encoding="utf-8") as fh:
        fh.write("# QA report\n\n## Totals\n\n")
        for cat, n in counts.most_common():
            flag = " ⚠" if cat in FAILING else ""
            fh.write(f"- {cat}: {n}{flag}\n")
        fh.write(
            f"\n**failures: {len(failures)}** · regressions vs baseline: "
            f"{len(regressions)} · fixed vs baseline: {len(fixed)}\n"
        )
        if regressions:
            fh.write("\n## Regressions\n\n")
            for r in regressions[:50]:
                fh.write(f"- `{r}`\n")
        if fixed:
            fh.write("\n## Fixed (run with --update-baseline in the fixing PR)\n\n")
            for r in fixed[:50]:
                fh.write(f"- `{r}`\n")
        fh.write("\n## Worst classes (failing slots per tn)\n\n")
        worst = sorted(
            by_tn.items(),
            key=lambda kv: -sum(n for c, n in kv[1].items() if c in FAILING),
        )[:15]
        for tn, c in worst:
            bad = sum(n for cat, n in c.items() if cat in FAILING)
            total = sum(c.values())
            if bad:
                fh.write(f"- tn {tn}: {bad}/{total} failing\n")
        fh.write("\n## Samples\n\n")
        for cat in sorted(samples):
            fh.write(f"### {cat}\n\n")
            for s in samples[cat]:
                fh.write(f"- `{s['slot']}` {json.dumps({k: v for k, v in s.items() if k != 'slot'}, ensure_ascii=False)}\n")
            fh.write("\n")
        if served_misspelled:
            fh.write("### served_misspelled (user-facing non-words!)\n\n")
            for s in served_misspelled:
                fh.write(f"- `{s['slot']}` {s['form']} (source: {s['source']})\n")

    print(f"failures: {len(failures)}  regressions: {len(regressions)}  "
          f"fixed: {len(fixed)}", file=sys.stderr)
    print(f"wrote {args.out_json}, {args.out_md}", file=sys.stderr)

    if args.update_baseline:
        with open(args.baseline, "w", encoding="utf-8") as fh:
            json.dump({"version": 1, "failures": sorted(failures)}, fh, indent=0)
        print(f"baseline updated: {args.baseline} ({len(failures)})", file=sys.stderr)

    if args.gate and regressions:
        print(f"GATE FAILED: {len(regressions)} regression(s)", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
