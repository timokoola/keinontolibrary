#!/usr/bin/env bash
# QA loop orchestrator — runs locally (by design: no GitHub Actions; later its own service).
#
#   scripts/qa/run.sh setup    one-time: venv + libvoikko (native lib via Homebrew)
#   scripts/qa/run.sh sync     fetch sources: Kotus list (curl) + corpus (gsutil rsync)
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
CORPUS_BUCKET="gs://REDACTED-CORPUS-BUCKET/"

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
  gsutil -m -q rsync "$CORPUS_BUCKET" data/sources/voikko/
  echo "sources: $(wc -l < data/sources/nykysuomensanalista2024.txt) Kotus lines, $(ls data/sources/voikko | wc -l | tr -d ' ') shards"
}

harmony() {
  # Voikko-probed per-lemma overrides (harmony + comitative style), minted from the
  # QA dump; committed in data/. Regenerate after rule changes, then re-ingest.
  $PY scripts/qa/gen_harmony_overrides.py
  $PY scripts/qa/gen_comitative_overrides.py
  $PY scripts/qa/gen_citation_overrides.py
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
