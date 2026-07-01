#!/usr/bin/env bash
# Publish ./_site to the `gh-pages` branch (GitHub Pages "deploy from a branch").
#
# GitHub Pages serves the branch contents directly. Run after a successful build.
#
#   scripts/publish-site.sh                 # build (if needed) + publish
#   scripts/publish-site.sh --no-build      # publish the existing ./_site as-is
#
# One-time GitHub setup: repo Settings -> Pages -> Source = "Deploy from a branch",
# Branch = gh-pages / (root). The CNAME file in _site sets the custom domain.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/_site"
BRANCH="gh-pages"
REMOTE="origin"

if [ "${1:-}" != "--no-build" ]; then
  "$ROOT/scripts/build-site.sh"
fi
[ -d "$OUT" ] || { echo "no $OUT — run scripts/build-site.sh first" >&2; exit 1; }

WT="$(mktemp -d)"
cleanup() { git -C "$ROOT" worktree remove --force "$WT" >/dev/null 2>&1 || true; rm -rf "$WT"; }
trap cleanup EXIT

git -C "$ROOT" fetch "$REMOTE" "$BRANCH" 2>/dev/null || true
if git -C "$ROOT" show-ref --verify --quiet "refs/remotes/$REMOTE/$BRANCH"; then
  git -C "$ROOT" worktree add --force -B "$BRANCH" "$WT" "$REMOTE/$BRANCH"
else
  echo "creating orphan $BRANCH branch ..."
  git -C "$ROOT" worktree add --force --detach "$WT"
  git -C "$WT" checkout --orphan "$BRANCH"
  git -C "$WT" reset --hard >/dev/null 2>&1 || true
fi

# Replace working tree of the branch with the freshly built site.
find "$WT" -mindepth 1 -maxdepth 1 ! -name '.git' -exec rm -rf {} +
cp -R "$OUT/." "$WT/"

git -C "$WT" add -A
if git -C "$WT" diff --cached --quiet; then
  echo "no changes to publish."
  exit 0
fi
git -C "$WT" commit -q -m "Publish site $(date -u +%Y-%m-%dT%H:%M:%SZ)"
git -C "$WT" push "$REMOTE" "$BRANCH"
echo "published to $REMOTE/$BRANCH — GitHub Pages will serve it shortly."
