#!/usr/bin/env sh
# Print branch status, baton summary, .ai/ changes, and a preview of current.md
set -e
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "=== Branch ==="
git branch --show-current 2>/dev/null || git rev-parse --abbrev-ref HEAD

echo ""
echo "=== Git status (short) ==="
git status -sb

echo ""
echo "=== Baton (from .ai/handoff/current.md) ==="
if [ -f .ai/handoff/current.md ]; then
  awk '/^## Baton$/{p=1;next} p && /^## /{exit} p' .ai/handoff/current.md
else
  echo "(missing .ai/handoff/current.md)"
fi

echo ""
echo "=== Working tree: .ai/ (git status --short) ==="
git status --short .ai 2>/dev/null || true

echo ""
echo "=== First lines of .ai/handoff/current.md (up to 60) ==="
if [ -f .ai/handoff/current.md ]; then
  head -n 60 .ai/handoff/current.md
else
  echo "(missing)"
fi
