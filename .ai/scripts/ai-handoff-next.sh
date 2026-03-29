#!/usr/bin/env sh
# Branch, latest commit, diff vs origin/main when possible, inventories, baton
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "=== Branch ==="
git branch --show-current 2>/dev/null || git rev-parse --abbrev-ref HEAD

echo ""
echo "=== Latest commit ==="
git log -1 --oneline

echo ""
echo "=== Files changed vs origin/main (triple-dot) ==="
if git rev-parse --verify origin/main >/dev/null 2>&1; then
  git diff --name-only origin/main...HEAD 2>/dev/null || true
else
  echo "(origin/main not found — fetch remotes or compare manually)"
  if git rev-parse --verify main >/dev/null 2>&1; then
    echo "Fallback: diff vs local main..."
    git diff --name-only main...HEAD 2>/dev/null || true
  fi
fi

echo ""
echo "=== .ai/findings ==="
ls -1 .ai/findings 2>/dev/null || echo "(none)"

echo ""
echo "=== .ai/reviews ==="
ls -1 .ai/reviews 2>/dev/null || echo "(none)"

echo ""
echo "=== .ai/plans ==="
ls -1 .ai/plans 2>/dev/null || echo "(none)"

echo ""
echo "=== Baton (from .ai/handoff/current.md) ==="
if [ -f .ai/handoff/current.md ]; then
  awk '/^## Baton$/{p=1;next} p && /^## /{exit} p' .ai/handoff/current.md
else
  echo "(missing .ai/handoff/current.md)"
fi
