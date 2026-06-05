#!/usr/bin/env python3
"""Emit Voikko-verified pronoun (Kotus tn 101) entries for exceptions.toml.

Pronouns are irregular and outside the rule classes. Each candidate form is verified against
Voikko (matching baseform + case + number) before it is emitted; unverified forms are skipped
with a stderr note rather than guessed. Run:
  DYLD_LIBRARY_PATH=/opt/homebrew/lib python3 gen_pronouns.py > /tmp/pronouns.toml
"""
import sys
from libvoikko import Voikko

v = Voikko("fi", "/opt/homebrew/lib/voikko")

# English case -> Voikko SIJAMUOTO
SJ = {
    "nominative": "nimento", "genitive": "omanto", "partitive": "osanto",
    "inessive": "sisaolento", "elative": "sisaeronto", "illative": "sisatulento",
    "adessive": "ulkoolento", "ablative": "ulkoeronto", "allative": "ulkotulento",
    "essive": "olento", "translative": "tulento", "abessive": "vajanto",
}

# (lemma, number, {case: candidate form}) — personal + demonstrative pronouns.
PRON = [
    ("minä", "singular", dict(nominative="minä", genitive="minun", partitive="minua", inessive="minussa", elative="minusta", illative="minuun", adessive="minulla", ablative="minulta", allative="minulle", essive="minuna", translative="minuksi", abessive="minutta")),
    ("sinä", "singular", dict(nominative="sinä", genitive="sinun", partitive="sinua", inessive="sinussa", elative="sinusta", illative="sinuun", adessive="sinulla", ablative="sinulta", allative="sinulle", essive="sinuna", translative="sinuksi", abessive="sinutta")),
    ("hän", "singular", dict(nominative="hän", genitive="hänen", partitive="häntä", inessive="hänessä", elative="hänestä", illative="häneen", adessive="hänellä", ablative="häneltä", allative="hänelle", essive="hänenä", translative="häneksi", abessive="hänettä")),
    ("me", "plural", dict(nominative="me", genitive="meidän", partitive="meitä", inessive="meissä", elative="meistä", illative="meihin", adessive="meillä", ablative="meiltä", allative="meille", essive="meinä", translative="meiksi", abessive="meittä")),
    ("te", "plural", dict(nominative="te", genitive="teidän", partitive="teitä", inessive="teissä", elative="teistä", illative="teihin", adessive="teillä", ablative="teiltä", allative="teille", essive="teinä", translative="teiksi", abessive="teittä")),
    ("he", "plural", dict(nominative="he", genitive="heidän", partitive="heitä", inessive="heissä", elative="heistä", illative="heihin", adessive="heillä", ablative="heiltä", allative="heille", essive="heinä", translative="heiksi", abessive="heittä")),
    ("se", "singular", dict(nominative="se", genitive="sen", partitive="sitä", inessive="siinä", elative="siitä", illative="siihen", adessive="sillä", ablative="siltä", allative="sille", translative="siksi")),
    ("tämä", "singular", dict(nominative="tämä", genitive="tämän", partitive="tätä", inessive="tässä", elative="tästä", illative="tähän", adessive="tällä", ablative="tältä", allative="tälle", essive="tänä", translative="täksi")),
    ("tuo", "singular", dict(nominative="tuo", genitive="tuon", partitive="tuota", inessive="tuossa", elative="tuosta", illative="tuohon", adessive="tuolla", ablative="tuolta", allative="tuolle", essive="tuona", translative="tuoksi")),
    ("nämä", "plural", dict(nominative="nämä", genitive="näiden", partitive="näitä", inessive="näissä", elative="näistä", illative="näihin", adessive="näillä", ablative="näiltä", allative="näille", essive="näinä", translative="näiksi")),
    ("nuo", "plural", dict(nominative="nuo", genitive="noiden", partitive="noita", inessive="noissa", elative="noista", illative="noihin", adessive="noilla", ablative="noilta", allative="noille", essive="noina", translative="noiksi")),
    ("ne", "plural", dict(nominative="ne", genitive="niiden", partitive="niitä", inessive="niissä", elative="niistä", illative="niihin", adessive="niillä", ablative="niiltä", allative="niille", essive="niinä", translative="niiksi")),
]

def verified(lemma, number, case, form):
    return any(
        a.get("BASEFORM") == lemma and a.get("SIJAMUOTO") == SJ[case] and a.get("NUMBER") == number
        for a in v.analyze(form)
    )

print("# --- pronouns (Kotus tn 101) — irregular nominals, Voikko-verified ---------------------")
ok = bad = 0
for lemma, number, cases in PRON:
    for case, form in cases.items():
        if not verified(lemma, number, case, form):
            print(f"# SKIP unverified: {lemma} {number} {case} {form}", file=sys.stderr)
            bad += 1
            continue
        ok += 1
        print(
            f'[[exception]]\nlemma = "{lemma}"\ntn = 101\nnumber = "{number}"\n'
            f'case = "{case}"\nforms = ["{form}"]\nreason = "pronoun (tn 101), irregular"\n'
        )
print(f"# emitted {ok}, skipped {bad}", file=sys.stderr)
