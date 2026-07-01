#!/usr/bin/env bash
# Publish the data assets to the GitHub release and fill the formula's resource sha256s.
#
#   packaging/homebrew/update-shas.sh [TAG]     # default TAG: v0.1.0
#
# Prereqisites: `gh` authenticated; the LICENSING.md corpus-cleared change merged to main;
# and your explicit intent to publish the corpus artifact publicly (this uploads it).
set -euo pipefail

TAG="${1:-v0.1.0}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/../.." && pwd)"
FORMULA="$HERE/keinontolibrary.rb"
REPO="timokoola/keinontolibrary"

ARTIFACT="$REPO_ROOT/data/artifact/keinontolibrary.bin"
OVERLAY="$REPO_ROOT/data/overlay.jsonl"
[ -f "$ARTIFACT" ] && [ -f "$OVERLAY" ] || { echo "missing artifact/overlay under data/" >&2; exit 1; }

echo "uploading data assets to release $TAG ..."
gh release upload "$TAG" "$ARTIFACT" "$OVERLAY" --repo "$REPO" --clobber

art_sha="$(shasum -a 256 "$ARTIFACT" | cut -d' ' -f1)"
ovl_sha="$(shasum -a 256 "$OVERLAY"  | cut -d' ' -f1)"
echo "artifact sha256: $art_sha"
echo "overlay  sha256: $ovl_sha"

# Patch the formula in place.
sed -i '' -e "s/REPLACE_WITH_ARTIFACT_SHA256/$art_sha/" -e "s/REPLACE_WITH_OVERLAY_SHA256/$ovl_sha/" "$FORMULA"
echo "patched $FORMULA — review, then copy it into your tap (see README.md)."
