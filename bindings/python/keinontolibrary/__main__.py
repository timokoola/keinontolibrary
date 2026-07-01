"""Command-line entry point: ``keinontolibrary`` (and ``python -m keinontolibrary``).

Usable via uv without an install:

    uvx keinontolibrary decline hevonen --number plural --case inessive
    uvx keinontolibrary table talo
"""

from __future__ import annotations

import argparse
import sys

from . import __version__, decline, paradigm


def main(argv: "list[str] | None" = None) -> int:
    parser = argparse.ArgumentParser(
        prog="keinontolibrary",
        description="Decline Finnish nouns (Kotus classes 1–51, Voikko-verified).",
    )
    parser.add_argument("--version", action="version", version=f"keinontolibrary {__version__}")
    sub = parser.add_subparsers(dest="command", required=True)

    d = sub.add_parser("decline", help="decline one word into a single (number, case) slot")
    d.add_argument("word")
    d.add_argument("--number", default="singular", help="singular | plural (default: singular)")
    d.add_argument("--case", default="nominative", help="e.g. nominative, genitive, inessive")

    t = sub.add_parser("table", help="print the full paradigm for one or more words")
    t.add_argument("words", nargs="+")

    args = parser.parse_args(argv)

    try:
        if args.command == "decline":
            forms = decline(args.word, args.number, args.case)
            print(", ".join(forms))
        elif args.command == "table":
            for i, word in enumerate(args.words):
                if i:
                    print()
                print(word)
                para = paradigm(word)
                for case in _CASE_ORDER:
                    sg = ", ".join(para["singular"].get(case, []))
                    pl = ", ".join(para["plural"].get(case, []))
                    print(f"  {case:<14} {sg:<28} {pl}")
    except KeyError as e:
        print(f"error: {e}", file=sys.stderr)
        return 3
    except ValueError as e:
        print(f"error: {e}", file=sys.stderr)
        return 2
    return 0


_CASE_ORDER = [
    "nominative", "genitive", "partitive", "accusative", "inessive", "elative",
    "illative", "adessive", "ablative", "allative", "essive", "translative",
    "abessive", "instructive", "comitative",
]


if __name__ == "__main__":
    raise SystemExit(main())
