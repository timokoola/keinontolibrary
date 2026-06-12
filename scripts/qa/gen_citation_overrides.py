#!/usr/bin/env python3
"""Mint foreign-citation styles: how parfait'n / cd:n words attach their endings.

A tn22 citation (silent foreign letters: parfait, bordeaux, show) declines behind an
apostrophe; a letter-word (cd, tv, dna, adhd) behind a colon. Harmony and the illative
echo vowel come from the PRONUNCIATION, which the spelling cannot give:

  parfait'ta (back, echo e is not even used — Voikko-verified)
  show'hun  (back, echo u)         cd:tä, cd:hen (front, echo e)
  dna:ta, dna:han (back, echo a — from the letter name "aa")

Minting: tn22 lemmas are probed against Voikko (partitive both harmonies, illative
echo candidates); where Voikko is silent, a hand pronunciation table fills in. Letter
words are deterministic from the Finnish name of the LAST letter (d = "dee" -> front,
echo e; a = "aa" -> back, echo a) — Voikko cross-checks the few it knows (cd, tv).

Output: data/citation-overrides.jsonl {"lemma", "sep", "front", "echo"}.
"""
import argparse
import json
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from verify_voikko import make_voikko  # noqa: E402

NOMINALS = ("substantiivi", "adjektiivi", "numeraali")

# Finnish letter names: (front-harmonic?, echo vowel = the name's last vowel).
LETTER_NAMES = {
    'a': (False, 'a'), 'b': (True, 'e'), 'c': (True, 'e'), 'd': (True, 'e'),
    'e': (True, 'e'), 'f': (True, 'ä'), 'g': (True, 'e'), 'h': (False, 'o'),
    'i': (True, 'i'), 'j': (True, 'i'), 'k': (False, 'o'), 'l': (True, 'ä'),
    'm': (True, 'ä'), 'n': (True, 'ä'), 'o': (False, 'o'), 'p': (True, 'e'),
    'q': (False, 'u'), 'r': (True, 'ä'), 's': (True, 'ä'), 't': (True, 'e'),
    'u': (False, 'u'), 'v': (True, 'e'), 'w': (True, 'e'), 'x': (True, 'ä'),
    'y': (True, 'y'), 'z': (False, 'a'), 'å': (False, 'o'), 'ä': (True, 'ä'),
    'ö': (True, 'ö'),
}

# Hand pronunciation table for tn22 lemmas Voikko does not carry (final pronounced
# vowel + harmony).
TN22_FALLBACK = {
    'bavarois': (False, 'a'), 'beaujolais': (True, 'e'), 'beignet': (True, 'e'),
    'bordeaux': (False, 'o'), 'bouquet': (True, 'e'), 'buffet': (True, 'e'),
    'café au lait': (True, 'e'), 'clafoutis': (True, 'i'), 'coulis': (True, 'i'),
    'flow': (False, 'u'), 'gourmet': (True, 'e'), 'know-how': (False, 'u'),
    'nougat': (False, 'a'), 'parfait': (False, 'e'), 'passepartout': (False, 'u'),
    'port salut': (False, 'y'), 'ragoût': (False, 'u'), 'roux': (False, 'u'),
    'show': (False, 'u'), 'sioux': (False, 'u'), 'tournedos': (False, 'o'),
}
VOWELS = set('aeiouyäö')


def probe_tn22(v, lemma):
    """Voikko-probe harmony and echo for one tn22 lemma; None if Voikko is silent."""
    back = v.spell(f"{lemma}'ta")
    front = v.spell(f"{lemma}'tä")
    if not back and not front:
        return None
    is_front = front and not back
    # Try the pronunciation-table echo first: Voikko can be lenient about the echo
    # vowel (it accepts both show'hun and show'hen), and the table has the standard.
    preferred = TN22_FALLBACK.get(lemma, (is_front, 'e'))[1]
    for echo in preferred + 'eouayiäö':
        if v.spell(f"{lemma}'h{echo}n"):
            return (is_front, echo)
    return (is_front, preferred)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--kotus", default="data/sources/nykysuomensanalista2024.txt")
    ap.add_argument("--out", default="data/citation-overrides.jsonl")
    args = ap.parse_args()

    v = make_voikko()
    rows = []
    seen = set()
    with open(args.kotus, encoding="utf-8") as fh:
        for line in fh:
            cols = line.rstrip("\n").split("\t")
            if len(cols) < 4 or not any(n in cols[2] for n in NOMINALS):
                continue
            lemma, taiv = cols[0].lower(), cols[3]
            if lemma in seen:
                continue
            # tn22: apostrophe citations.
            if '22' in [t.strip().strip('()').split('*')[0] for t in taiv.split(',')]:
                seen.add(lemma)
                got = probe_tn22(v, lemma) or TN22_FALLBACK.get(lemma)
                if not got:
                    print(f"  no style for tn22 {lemma!r}", file=sys.stderr)
                    continue
                front, echo = got
                rows.append({"lemma": lemma, "sep": "'", "front": front, "echo": echo})
                continue
            # Letter-words decline on the name of their final letter behind a colon.
            # Three shapes: pure abbreviations (cd, adhd — short, vowel-less or
            # consonant-final in classes 17–20), and the linguistics terms written
            # name+hyphen+letter (sora-r, kaksois-v) where the final letter is the head.
            tns = {t.strip().strip('()').split('*')[0] for t in taiv.split(',')}
            last = lemma[-1]
            hyphen_letter = "-" in lemma and len(lemma.rsplit("-", 1)[1]) == 1
            no_vowel = not (set(lemma) & VOWELS)
            cons_final_18 = (
                tns & {'17', '18', '19', '20'}
                and last not in VOWELS
                and lemma.isascii()
            )
            short_abbrev = len(lemma) <= 5 and lemma.isalpha() and (no_vowel or cons_final_18)
            if (short_abbrev or hyphen_letter) and last in LETTER_NAMES:
                seen.add(lemma)
                front, echo = LETTER_NAMES[last]
                rows.append({"lemma": lemma, "sep": ":", "front": front, "echo": echo})

    with open(args.out, "w", encoding="utf-8") as out:
        for r in sorted(rows, key=lambda r: r["lemma"]):
            out.write(json.dumps(r, ensure_ascii=False) + "\n")
    print(f"{len(rows)} citation styles -> {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
