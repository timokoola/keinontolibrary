#!/usr/bin/env bash
# QA loop orchestrator — runs locally (by design: no GitHub Actions; later its own service).
#
#   scripts/qa/run.sh setup    one-time: venv + libvoikko (native lib via Homebrew)
#   scripts/qa/run.sh sync     fetch the public Kotus list; pull the private reference
#                              corpus only if KEINONTO_CORPUS_URI is set (maintainer-only)
#   scripts/qa/run.sh all      ingest → dump → verify (full) → report --gate
#   scripts/qa/run.sh quick    dump → verify --sample 2000 → report (no gate)
#   scripts/qa/run.sh ingest|dump|verify|report   individual stages
#
# Extra args after `report` are passed through (e.g. `run.sh report --update-baseline`).
# The gate fails (exit 1) only on NEW failures vs the committed qa/baseline.json.
set -euo pipefail
cd "$(dirname "$0")/../.."

# libvoikko: the python package is a ctypes wrapper; macOS needs the dylib dir.
if [[ "$(uname)" == "Darwin" && -d /opt/homebrew/lib ]]; then
  export DYLD_LIBRARY_PATH="${DYLD_LIBRARY_PATH:-/opt/homebrew/lib}"
fi
PY=.venv/bin/python
KOTUS_URL="https://kaino.kotus.fi/lataa/nykysuomensanalista2024.txt"
# The reference corpus is private and access-controlled (not redistributed). Maintainers
# set KEINONTO_CORPUS_URI to their own location; the repo names nothing.
CORPUS_URI="${KEINONTO_CORPUS_URI:-}"

setup() {
  python3 -m venv .venv
  .venv/bin/pip install -q --upgrade libvoikko
  $PY -c "import libvoikko" && echo "libvoikko ok"
  if [[ "$(uname)" == "Darwin" ]] && ! ls /opt/homebrew/lib/libvoikko*.dylib >/dev/null 2>&1; then
    echo "warning: native libvoikko not found — brew install libvoikko" >&2
  fi
}

sync() {
  mkdir -p data/sources/voikko
  if [[ ! -s data/sources/nykysuomensanalista2024.txt ]]; then
    curl -sS -o data/sources/nykysuomensanalista2024.txt "$KOTUS_URL"
  fi
  if [[ -n "$CORPUS_URI" ]]; then
    gsutil -m -q rsync "$CORPUS_URI" data/sources/voikko/   # maintainer-only, private
  else
    echo "note: KEINONTO_CORPUS_URI unset — Kotus list only. The rule engine, registry," >&2
    echo "      and overrides give full coverage without the corpus; see DISTRIBUTION.md." >&2
  fi
  echo "sources: $(wc -l < data/sources/nykysuomensanalista2024.txt) Kotus lines, $(ls data/sources/voikko | wc -l | tr -d ' ') shards"
}

harmony() {
  # Voikko-probed overrides minted from the QA dump; committed in data/. Regenerate after
  # rule changes, then re-ingest. Per-lemma: harmony + comitative style + citation. Plus
  # slot-level alternant completions (rule alternants the corpus under-attested, e.g.
  # omenoilta) — needs a dump to read, so it is skipped if qa/generated.jsonl is absent.
  $PY scripts/qa/gen_harmony_overrides.py
  $PY scripts/qa/gen_comitative_overrides.py
  $PY scripts/qa/gen_citation_overrides.py
  if [[ -s qa/generated.jsonl ]]; then $PY scripts/qa/gen_alternant_overrides.py; fi
}
ingest()  {
  if [[ -x $PY && ! -s data/harmony-overrides.jsonl ]]; then harmony; fi
  cargo run --release -p keinontolibrary-ingest
}
dump()    { cargo run --release -p keinontolibrary-ingest --bin keinontolibrary-qa-dump; }
verify()  { $PY scripts/qa/verify_voikko.py "$@"; }
report()  { $PY scripts/qa/report.py "$@"; }
content() { $PY scripts/qa/gen_content.py; }

case "${1:-all}" in
  setup)  setup ;;
  sync)   sync ;;
  harmony) harmony ;;
  ingest) ingest ;;
  dump)   dump ;;
  verify) shift; verify "$@" ;;
  report) shift; report "$@" ;;
  content) content ;;
  all)    ingest; dump; verify; report --gate ;;
  quick)  dump; verify --sample 2000; report ;;
  *) echo "usage: $0 [setup|sync|harmony|ingest|dump|verify|report|content|all|quick]" >&2; exit 2 ;;
esac
