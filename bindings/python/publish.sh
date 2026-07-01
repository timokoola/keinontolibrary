#!/usr/bin/env bash
# Build the wheel matrix + sdist and upload to PyPI, cross-compiling Linux wheels with zig
# (no Docker needed).
#
#   MATURIN_PYPI_TOKEN=pypi-... bindings/python/publish.sh            # build + upload
#   bindings/python/publish.sh --dry-run                              # build only, no upload
#
# One-time prerequisites:
#   rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu \
#                     x86_64-apple-darwin aarch64-apple-darwin
#   (zig is pulled in per-invocation via `uv run --with 'maturin[zig]'`)
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
PKG="$HERE/keinontolibrary"
DRY=0; [ "${1:-}" = "--dry-run" ] && DRY=1

# Bundle the data-backed artifact + overlay (same as build-wheel.sh).
cp "$REPO/data/artifact/keinontolibrary.bin" "$PKG/keinontolibrary.bin"
cp "$REPO/data/overlay.jsonl" "$PKG/overlay.jsonl"

cd "$HERE"
rm -rf dist

echo "== macOS wheels (universal2: arm64 + x86_64) =="
uv run --with maturin maturin build --release --target universal2-apple-darwin --out dist

echo "== Linux manylinux wheels (via zig cross-compile) =="
for tgt in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
  uv run --with 'maturin[zig]' maturin build --release --zig --target "$tgt" --out dist
done

echo "== sdist =="
uv run --with maturin maturin sdist --out dist

echo "== built artifacts =="
ls -1 dist

if [ "$DRY" -eq 1 ]; then
  echo "dry run — not uploading."
  exit 0
fi
: "${MATURIN_PYPI_TOKEN:?set MATURIN_PYPI_TOKEN (a PyPI API token) to upload}"
echo "== uploading to PyPI =="
uv run --with maturin maturin upload --skip-existing dist/*
