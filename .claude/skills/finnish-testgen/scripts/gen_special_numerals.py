#!/usr/bin/env python3
"""Emit Voikko-verified entries for the irregular numeral singletons (Kotus tn 31, 46).

`kaksi`/`yksi` (tn 31) and `tuhat` (tn 46) have no rule arm — their stems (kah-/kahde-,
yh-/yhde-, tuhanne-/tuhante-) are one-off. The productive ordinals (tn 45) are handled by a
rule arm instead; only these singletons go in the registry. `haaksi` (archaic tn 31 noun) is
included if Voikko knows it. Each candidate is verified against Voikko (baseform + case +
number) before emission; unverified candidates are skipped with a stderr note, not guessed.
  DYLD_LIBRARY_PATH=/opt/homebrew/lib python3 gen_special_numerals.py > /tmp/numerals.toml
"""
import sys
from libvoikko import Voikko

v = Voikko("fi", "/opt/homebrew/lib/voikko")

SJ = {
    "nominative": "nimento", "genitive": "omanto", "partitive": "osanto",
    "inessive": "sisaolento", "elative": "sisaeronto", "illative": "sisatulento",
    "adessive": "ulkoolento", "ablative": "ulkoeronto", "allative": "ulkotulento",
    "essive": "olento", "translative": "tulento",
}

# (lemma, tn, {number: {case: candidate}}). Both numbers offered; Voikko keeps the real ones.
WORDS = [
    ("kaksi", 31, {
        "singular": dict(nominative="kaksi", genitive="kahden", partitive="kahta", inessive="kahdessa", elative="kahdesta", illative="kahteen", adessive="kahdella", ablative="kahdelta", allative="kahdelle", essive="kahtena", translative="kahdeksi"),
        "plural": dict(nominative="kakset", genitive="kaksien", partitive="kaksia", inessive="kaksissa", elative="kaksista", illative="kaksiin", adessive="kaksilla", ablative="kaksilta", allative="kaksille", essive="kaksina", translative="kaksiksi"),
    }),
    ("yksi", 31, {
        "singular": dict(nominative="yksi", genitive="yhden", partitive="yhtä", inessive="yhdessä", elative="yhdestä", illative="yhteen", adessive="yhdellä", ablative="yhdeltä", allative="yhdelle", essive="yhtenä", translative="yhdeksi"),
        "plural": dict(nominative="yhdet", genitive="yksien", partitive="yksiä", inessive="yksissä", elative="yksistä", illative="yksiin", adessive="yksillä", ablative="yksiltä", allative="yksille", essive="yksinä", translative="yksiksi"),
    }),
    ("tuhat", 46, {
        "singular": dict(nominative="tuhat", genitive="tuhannen", partitive="tuhatta", inessive="tuhannessa", elative="tuhannesta", illative="tuhanteen", adessive="tuhannella", ablative="tuhannelta", allative="tuhannelle", essive="tuhantena", translative="tuhanneksi"),
        "plural": dict(nominative="tuhannet", genitive="tuhansien", partitive="tuhansia", inessive="tuhansissa", elative="tuhansista", illative="tuhansiin", adessive="tuhansilla", ablative="tuhansilta", allative="tuhansille", essive="tuhansina", translative="tuhansiksi"),
    }),
    ("haaksi", 31, {
        "singular": dict(nominative="haaksi", genitive="haahden", partitive="haahta", inessive="haahdessa", elative="haahdesta", illative="haahteen", adessive="haahdella", ablative="haahdelta", allative="haahdelle", essive="haahtena", translative="haahdeksi"),
        "plural": dict(nominative="haahdet", genitive="haaksien", partitive="haaksia", inessive="haaksissa", elative="haaksista", illative="haaksiin", adessive="haaksilla", ablative="haaksilta", allative="haaksille", essive="haaksina", translative="haaksiksi"),
    }),
]


def verified(lemma, number, case, form):
    return any(
        a.get("BASEFORM") == lemma and a.get("SIJAMUOTO") == SJ[case] and a.get("NUMBER") == number
        for a in v.analyze(form)
    )


print("# --- irregular numeral singletons (Kotus tn 31 kaksi/yksi, tn 46 tuhat) — Voikko-verified")
ok = bad = 0
for lemma, tn, numbers in WORDS:
    for number, cases in numbers.items():
        for case, form in cases.items():
            if not verified(lemma, number, case, form):
                print(f"# SKIP unverified: {lemma} {number} {case} {form}", file=sys.stderr)
                bad += 1
                continue
            ok += 1
            print(
                f'[[exception]]\nlemma = "{lemma}"\ntn = {tn}\nnumber = "{number}"\n'
                f'case = "{case}"\nforms = ["{form}"]\nreason = "tn{tn} irregular numeral, Voikko-verified"\n'
            )
print(f"# emitted {ok}, skipped {bad}", file=sys.stderr)
