#!/usr/bin/env python3
"""Mint Voikko-verified test material for a Finnish noun from fi.wiktionary.

Pipeline:
  1. fetch the fi.wiktionary declension table for a lemma (all 15 cases x sg/pl)
  2. look up the lemma's Kotus class (tn) + gradation (av) from the Kotus list
  3. validate every surface form with Voikko (baseform + case + number must agree)
  4. emit a verification table and ingest-compatible corpus JSONL for the verified forms

The JSONL plugs straight into keinontolibrary-ingest (one shard) so the forms become
Lookup data and parity-test material. See SKILL.md for the full workflow.

Usage:
  python3 mint_testdata.py aika
  python3 mint_testdata.py aika --kotus /path/to/nykysuomensanalista2024.txt --jsonl out.jsonl
  python3 mint_testdata.py aika --no-voikko        # skip validation (forms marked UNVERIFIED)

Deps: requests + beautifulsoup4 (parsing), libvoikko (validation, optional).
  pip install requests beautifulsoup4 libvoikko
"""
import argparse
import json
import os
import re
import sys

# Finnish case name (as it appears in the wiktionary table) -> (english, Voikko SIJAMUOTO)
CASES = {
    "nominatiivi": ("nominative", "nimento"),
    "genetiivi": ("genitive", "omanto"),
    "partitiivi": ("partitive", "osanto"),
    "akkusatiivi": ("accusative", "kohdanto"),
    "inessiivi": ("inessive", "sisaolento"),
    "elatiivi": ("elative", "sisaeronto"),
    "illatiivi": ("illative", "sisatulento"),
    "adessiivi": ("adessive", "ulkoolento"),
    "ablatiivi": ("ablative", "ulkoeronto"),
    "allatiivi": ("allative", "ulkotulento"),
    "essiivi": ("essive", "olento"),
    "translatiivi": ("translative", "tulento"),
    "abessiivi": ("abessive", "vajanto"),
    "instruktiivi": ("instructive", "keinonto"),
    "komitatiivi": ("comitative", "seuranto"),
}
# Voikko SIJAMUOTO -> english (for reading Voikko analyses)
SIJAMUOTO_EN = {v[1]: v[0] for v in CASES.values()}
# english -> Voikko SIJAMUOTO (for emitting JSONL)
CASE_EN_TO_SIJAMUOTO = {v[0]: v[1] for v in CASES.values()}
# The corpus derives accusative and uses a possessive citation form for comitative, so we
# do NOT emit those two as plain JSONL (they would not round-trip through the ingest filter).
JSONL_SKIP = {"accusative", "comitative"}


def fetch_wiktionary_html(lemma):
    import requests

    resp = requests.get(
        "https://fi.wiktionary.org/w/api.php",
        params={
            "action": "parse",
            "format": "json",
            "prop": "text",
            "redirects": "1",
            "page": lemma,
        },
        headers={"User-Agent": "keinontolibrary-testgen/1.0"},
        timeout=20,
    )
    resp.raise_for_status()
    data = resp.json()
    if "error" in data:
        raise SystemExit(f"wiktionary: {data['error'].get('info', 'page not found')}")
    return data["parse"]["text"]["*"]


def clean_cell(cell):
    """Extract surface form(s) from a table cell: drop footnotes, split alternatives."""
    text = cell.get_text("\n")
    forms = []
    for chunk in re.split(r"[\n,/]|\btai\b", text):
        w = chunk.strip().strip("–-").strip()
        # drop footnote markers, parentheticals, possessive-suffix notes
        w = re.sub(r"\[[^\]]*\]", "", w).strip()
        if w and re.fullmatch(r"[a-zåäö’'\- ]+", w, re.IGNORECASE) and " " not in w:
            forms.append(w)
    # de-dup, preserving order (dict.fromkeys keeps insertion order)
    return list(dict.fromkeys(forms))


def parse_declension(html):
    """Return {(english_case, 'singular'|'plural'): [forms]} from the declension table."""
    from bs4 import BeautifulSoup

    soup = BeautifulSoup(html, "html.parser")
    result = {}
    for table in soup.find_all("table"):
        head = table.get_text(" ", strip=True).lower()
        if "yksikkö" not in head or "monikko" not in head:
            continue
        for row in table.find_all("tr"):
            cells = row.find_all(["th", "td"])
            if len(cells) < 3:
                continue
            name = cells[0].get_text(" ", strip=True).lower().split()[0] if cells[0].get_text(strip=True) else ""
            if name not in CASES:
                continue
            en = CASES[name][0]
            sg = clean_cell(cells[-2])
            pl = clean_cell(cells[-1])
            if sg:
                result[(en, "singular")] = sg
            if pl:
                result[(en, "plural")] = pl
    if not result:
        raise SystemExit("no declension table found (is this a Finnish noun page?)")
    return result


def kotus_lookup(lemma, kotus_path):
    """Return (tn, av) for the noun reading of `lemma`, or (None, None)."""
    if not kotus_path or not os.path.exists(kotus_path):
        return None, None
    with open(kotus_path, encoding="utf-8") as fh:
        for line in fh:
            cols = line.rstrip("\n").split("\t")
            if len(cols) < 4 or cols[0] != lemma:
                continue
            if "substantiivi" not in cols[2]:
                continue
            m = re.match(r"\(?(\d{1,2})\*?([A-Z])?", cols[3])
            if m and 1 <= int(m.group(1)) <= 49:
                return int(m.group(1)), m.group(2)
    return None, None


def make_voikko():
    try:
        from libvoikko import Voikko
    except OSError as e:  # native libvoikko dylib/so not found
        print(
            f"# libvoikko native library not found ({e}).\n"
            "# On Homebrew macOS run with: DYLD_LIBRARY_PATH=/opt/homebrew/lib  (forms UNVERIFIED for now)",
            file=sys.stderr,
        )
        return None
    except Exception as e:  # noqa: BLE001
        print(f"# libvoikko not importable ({e}); forms UNVERIFIED", file=sys.stderr)
        return None
    # dylib loaded; now find a dictionary: default path, then env, then common install dirs.
    candidates = [None, os.environ.get("VOIKKO_DICTIONARY_PATH")]
    candidates += ["/opt/homebrew/lib/voikko", "/usr/local/lib/voikko", "/usr/lib/voikko"]
    last = None
    for path in (c for c in candidates if c is None or os.path.isdir(c)):
        try:
            return Voikko("fi", path) if path else Voikko("fi")
        except Exception as e:  # noqa: BLE001
            last = e
    print(f"# voikko dictionary not found ({last}); forms UNVERIFIED", file=sys.stderr)
    return None


def voikko_verify(v, form, lemma, english_case, number):
    """True if Voikko has an analysis with matching baseform + case + number."""
    for a in v.analyze(form):
        if a.get("BASEFORM", "").lower() != lemma.lower():
            continue
        if SIJAMUOTO_EN.get(a.get("SIJAMUOTO", "")) != english_case:
            continue
        if a.get("NUMBER") == number:
            return True
    return False


CASE_ORDER = [c[0] for c in CASES.values()]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("lemma")
    ap.add_argument("--kotus", default=os.environ.get("KEINONTO_KOTUS", ""))
    ap.add_argument("--jsonl", default="", help="write corpus JSONL here (else stdout)")
    ap.add_argument("--no-voikko", action="store_true")
    args = ap.parse_args()

    forms = parse_declension(fetch_wiktionary_html(args.lemma))
    tn, av = kotus_lookup(args.lemma, args.kotus)
    v = None if args.no_voikko else make_voikko()

    print(f"\n{args.lemma}  (Kotus tn={tn} av={av or '-'})\n")
    print(f"{'case':<12} {'number':<9} {'form':<16} {'voikko'}")
    print("-" * 48)
    jsonl = []
    for en in CASE_ORDER:
        for number in ("singular", "plural"):
            fs = forms.get((en, number))
            if not fs:
                continue
            for i, form in enumerate(fs):
                if v is not None:
                    ok = voikko_verify(v, form, args.lemma, en, number)
                    verdict = "ok" if ok else "MISMATCH"
                else:
                    ok, verdict = None, "UNVERIFIED"
                print(f"{en:<12} {number:<9} {form:<16} {verdict}")
                # emit only the primary form of corpus-eligible cases, when verified-or-unchecked
                if i == 0 and en not in JSONL_SKIP and tn is not None and ok is not False:
                    row = {
                        "BASEFORM": args.lemma,
                        "tn": tn,
                        "av": av or "_",
                        "CLASS": "nimisana",
                        "NUMBER": number,
                        "SIJAMUOTO": CASE_EN_TO_SIJAMUOTO[en],
                        "BOOKWORD": form,
                    }
                    jsonl.append(json.dumps(row, ensure_ascii=False))

    body = "\n".join(jsonl) + "\n"
    if args.jsonl:
        with open(args.jsonl, "w", encoding="utf-8") as fh:
            fh.write(body)
        print(f"\nwrote {len(jsonl)} corpus rows -> {args.jsonl}")
    else:
        print(f"\n# corpus JSONL ({len(jsonl)} rows):")
        sys.stdout.write(body)


if __name__ == "__main__":
    main()
