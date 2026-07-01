#!/usr/bin/env bash
# Build the keinontolibrary Python wheel (native PyO3 extension + bundled data).
#
#   bindings/python/build-wheel.sh            # build a wheel into dist/
#   bindings/python/build-wheel.sh develop    # editable install into the active venv
#
# The data-backed artifact + overlay are copied in from the repo's data/ (they are build
# products, gitignored here). maturin is run via uv, so no global install is needed.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
PKG="$HERE/keinontolibrary"

ARTIFACT="$REPO/data/artifact/keinontolibrary.bin"
OVERLAY="$REPO/data/overlay.jsonl"
[ -f "$ARTIFACT" ] || { echo "missing $ARTIFACT — build it first (see docs/guides/build-artifact.md)" >&2; exit 1; }
[ -f "$OVERLAY" ]  || { echo "missing $OVERLAY" >&2; exit 1; }

echo "bundling data into the package ..."
cp "$ARTIFACT" "$PKG/keinontolibrary.bin"
cp "$OVERLAY"  "$PKG/overlay.jsonl"

cd "$HERE"
if [ "${1:-}" = "develop" ]; then
  uv run --with maturin maturin develop --release
else
  uv run --with maturin maturin build --release --out dist
  echo "wheel(s):"; ls -1 dist/*.whl
fi
