#!/usr/bin/env bash
# Build the keinonto.com static site (landing page + mdBook docs) into ./_site.
#
# Output is a self-contained directory ready to serve or publish.
#
#   scripts/build-site.sh            # build into _site/
#   MDBOOK_BIN=/path/to/mdbook ...   # use a specific mdbook
#
# If mdbook isn't on PATH, a pinned release binary is fetched into .cache/mdbook.
set -euo pipefail

MDBOOK_VERSION="0.5.3"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/_site"
CACHE="$ROOT/.cache"

# --- locate or fetch mdbook ---------------------------------------------------
mdbook_bin() {
  if [ -n "${MDBOOK_BIN:-}" ]; then echo "$MDBOOK_BIN"; return; fi
  if command -v mdbook >/dev/null 2>&1; then command -v mdbook; return; fi
  if [ -x "$CACHE/mdbook/mdbook" ]; then echo "$CACHE/mdbook/mdbook"; return; fi

  echo "mdbook not found; fetching v$MDBOOK_VERSION into .cache/mdbook ..." >&2
  local os arch target tgz url
  os="$(uname -s)"; arch="$(uname -m)"
  case "$os" in
    Darwin) case "$arch" in arm64|aarch64) target="aarch64-apple-darwin";; *) target="x86_64-apple-darwin";; esac;;
    Linux)  target="x86_64-unknown-linux-gnu";;
    *) echo "unsupported OS '$os'; install mdbook manually (cargo install mdbook) and re-run" >&2; exit 1;;
  esac
  tgz="mdbook-v${MDBOOK_VERSION}-${target}.tar.gz"
  url="https://github.com/rust-lang/mdBook/releases/download/v${MDBOOK_VERSION}/${tgz}"
  mkdir -p "$CACHE/mdbook"
  curl -fsSL "$url" | tar -xz -C "$CACHE/mdbook"
  echo "$CACHE/mdbook/mdbook"
}

MDBOOK="$(mdbook_bin)"
echo "using mdbook: $MDBOOK ($("$MDBOOK" --version))"

# --- build --------------------------------------------------------------------
echo "building mdBook docs ..."
"$MDBOOK" build "$ROOT/docs"

echo "assembling $OUT ..."
rm -rf "$OUT"
mkdir -p "$OUT"
cp -R "$ROOT/site/." "$OUT/"
# Docs live under /guide/ (matches docs/book.toml site-url and the landing-page links).
mkdir -p "$OUT/guide"
cp -R "$ROOT/docs/book/." "$OUT/guide/"
# src="." makes mdBook copy book.toml + the theme dir into the output; don't publish them.
rm -f "$OUT/guide/book.toml"
rm -rf "$OUT/guide/theme"
# GitHub Pages: serve files as-is (no Jekyll) + custom domain.
touch "$OUT/.nojekyll"
echo "keinonto.com" > "$OUT/CNAME"

echo "done -> $OUT"
echo "preview: (cd $OUT && python3 -m http.server 8000)  then open http://localhost:8000"
