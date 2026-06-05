#!/usr/bin/env python3
"""Collect gold declension tables for Finnish compounds from fi.wiktionary, politely.

For each compound lemma (Kotus tn50 / tn51) it fetches the fi.wiktionary declension table,
parses every case x number form, and tags each with a Voikko confidence flag. Wiktionary is
the authority for compound forms (Voikko normalises compound baseforms inconsistently —
`isonveljen` -> `isonveli` — so it is only a soft check here). Output is the gold the
compound-parity harness measures the engine against.

Politeness:
  - one descriptive User-Agent with a contact URL
  - a delay between network fetches (default 2.5s; --delay to change)
  - an on-disk cache (.cache/wiktionary/<lemma>.json) so re-runs never refetch
  - 404 / no-table pages are skipped and logged, not retried

Usage (from the repo root):
  KEINONTO_KOTUS=path/to/nykysuomensanalista2024.txt \
  DYLD_LIBRARY_PATH=/opt/homebrew/lib python3 .claude/skills/finnish-testgen/scripts/gen_compound_gold.py \
      --classes 50,51 --out crates/keinontolibrary-rules/tests/data/compound_gold.json
  # add --limit N to sample, or pass lemmas positionally instead of --classes.

Deps: requests, beautifulsoup4, libvoikko.
"""
import argparse
import json
import os
import re
import sys
import time
from pathlib import Path

CASES = {
    "nominatiivi": ("nominative", "nimento"),
    "genetiivi": ("genitive", "omanto"),
    "partitiivi": ("partitive", "osanto"),
    "inessiivi": ("inessive", "sisaolento"),
    "elatiivi": ("elative", "sisaeronto"),
    "illatiivi": ("illative", "sisatulento"),
    "adessiivi": ("adessive", "ulkoolento"),
    "ablatiivi": ("ablative", "ulkoeronto"),
    "allatiivi": ("allative", "ulkotulento"),
    "essiivi": ("essive", "olento"),
    "translatiivi": ("translative", "tulento"),
}
SIJAMUOTO_EN = {v[1]: v[0] for v in CASES.values()}
UA = "keinontolibrary-compound-gold/1.0 (https://github.com/timokoola/keinontolibrary; test data)"
CACHE = Path(".cache/wiktionary")


def kotus_compounds(kotus_path, classes):
    """Yield (lemma, tn) for rows whose Taivutustiedot carries one of `classes` (50/51),
    keeping the same 'tn50 only when sole reading' rule the ingest applies."""
    want = set(classes)
    out = []
    with open(kotus_path, encoding="utf-8") as fh:
        for i, line in enumerate(fh):
            if i == 0 and line.startswith("Hakusana"):
                continue
            cols = line.rstrip("\n").split("\t")
            if len(cols) < 4 or not cols[0] or "substantiivi" not in cols[2] and "adjektiivi" not in cols[2]:
                continue
            toks = []
            for t in cols[3].split(","):
                m = re.match(r"\(?(\d{1,3})", t.strip())
                if m:
                    toks.append(int(m.group(1)))
            has_regular = any(1 <= t <= 49 for t in toks)
            for tn in (50, 51):
                if tn in want and tn in toks:
                    # tn50 only when it is the sole reading (mirrors ingest); tn51 always.
                    if tn == 50 and has_regular:
                        continue
                    out.append((cols[0], tn))
                    break
    return out


def fetch(lemma, delay):
    """Return the parsed wikitext HTML for `lemma`, or None. Cached; sleeps before network."""
    import requests

    CACHE.mkdir(parents=True, exist_ok=True)
    cf = CACHE / f"{lemma.replace('/', '_')}.json"
    if cf.exists():
        cached = json.loads(cf.read_text(encoding="utf-8"))
        return cached.get("html")
    time.sleep(delay)  # only before an actual network call
    try:
        resp = requests.get(
            "https://fi.wiktionary.org/w/api.php",
            params={"action": "parse", "format": "json", "prop": "text", "redirects": "1", "page": lemma},
            headers={"User-Agent": UA},
            timeout=20,
        )
        data = resp.json()
    except Exception as e:  # noqa: BLE001
        print(f"  ! {lemma}: fetch error {e}", file=sys.stderr)
        return None
    html = None if "error" in data else data.get("parse", {}).get("text", {}).get("*")
    cf.write_text(json.dumps({"html": html}, ensure_ascii=False), encoding="utf-8")
    return html


def clean_cell(cell):
    forms = []
    for chunk in re.split(r"[\n,/]|\btai\b", cell.get_text("\n")):
        w = re.sub(r"\[[^\]]*\]", "", chunk).strip().strip("–-").strip()
        if w and re.fullmatch(r"[a-zåäö’'\- ]+", w, re.IGNORECASE) and " " not in w:
            forms.append(w)
    return list(dict.fromkeys(forms))


def parse_declension(html):
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
            for number, cell in (("singular", cells[-2]), ("plural", cells[-1])):
                fs = clean_cell(cell)
                if fs:
                    result[(en, number)] = fs
    return result


def make_voikko():
    try:
        from libvoikko import Voikko

        for p in ("/opt/homebrew/lib/voikko", "/usr/local/lib/voikko", None):
            if p is None or os.path.isdir(p):
                try:
                    return Voikko("fi", p) if p else Voikko("fi")
                except Exception:  # noqa: BLE001
                    continue
    except Exception as e:  # noqa: BLE001
        print(f"# voikko unavailable ({e}); forms get voikko=null", file=sys.stderr)
    return None


def voikko_ok(v, form, en, number):
    """Soft check: does Voikko recognise `form` as this case+number? (baseform ignored —
    Voikko's compound baseforms are unreliable)."""
    if v is None:
        return None
    for a in v.analyze(form):
        if SIJAMUOTO_EN.get(a.get("SIJAMUOTO", "")) == en and a.get("NUMBER") == number:
            return True
    return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("lemmas", nargs="*")
    ap.add_argument("--classes", default="", help="comma list, e.g. 50,51 (reads Kotus)")
    ap.add_argument("--kotus", default=os.environ.get("KEINONTO_KOTUS", ""))
    ap.add_argument("--out", default="crates/keinontolibrary-rules/tests/data/compound_gold.json")
    ap.add_argument("--delay", type=float, default=2.5)
    ap.add_argument("--limit", type=int, default=0)
    args = ap.parse_args()

    if args.classes:
        classes = [int(x) for x in args.classes.split(",")]
        targets = kotus_compounds(args.kotus, classes)
    else:
        targets = [(l, 0) for l in args.lemmas]
    if args.limit:
        targets = targets[: args.limit]

    v = make_voikko()
    records, pages, no_page = [], 0, 0
    print(f"fetching {len(targets)} compound(s), delay={args.delay}s (cached fetches are instant)…", file=sys.stderr)
    for idx, (lemma, tn) in enumerate(targets, 1):
        html = fetch(lemma, args.delay)
        if not html:
            no_page += 1
            continue
        table = parse_declension(html)
        if not table:
            no_page += 1
            print(f"  - {lemma}: page but no declension table", file=sys.stderr)
            continue
        pages += 1
        slots = 0
        for (en, number), forms in sorted(table.items()):
            forms = [f for f in forms if f != lemma or en == "nominative"]
            if not forms:
                continue
            records.append({
                "lemma": lemma, "tn": tn, "case": en, "number": number,
                "forms": forms, "voikko": all(voikko_ok(v, f, en, number) for f in forms),
            })
            slots += 1
        if idx % 25 == 0 or idx == len(targets):
            print(f"  …{idx}/{len(targets)} ({pages} with tables)", file=sys.stderr)

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(records, ensure_ascii=False, indent=0), encoding="utf-8")
    vok = sum(1 for r in records if r["voikko"])
    print(
        f"\nwrote {len(records)} gold slots for {pages} compounds -> {args.out}\n"
        f"  pages without a usable table: {no_page}\n"
        f"  slots Voikko-confirmed: {vok}/{len(records)}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
