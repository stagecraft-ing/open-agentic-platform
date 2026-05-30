#!/usr/bin/env bash
# Spec: 188-derived-index-merge-serialization
#
# Git merge driver `oap-index-regen` for the one committed derived
# artifact, `.derived/codebase-index/index.json`. When two branches both
# regenerated the index, the textual merge conflicts on the content hash
# and the inventory block. This driver resolves that conflict
# deterministically: regenerate the index from the merged working tree and
# hand the fresh artifact back to git as the resolution — so a rebase onto
# a merged PR no longer leaves an index conflict to resolve by hand
# (spec 188 Phase 1, FR-001/FR-002).
#
# Enable in this clone (opt-in, mirrors `git config core.hooksPath
# .githooks` for the pre-commit hook):
#   git config merge.oap-index-regen.name "regenerate codebase index on conflict"
#   git config merge.oap-index-regen.driver ".githooks/merge-derived-index.sh %O %A %B %P"
#
# Disable:
#   git config --unset merge.oap-index-regen.driver
#   git config --unset merge.oap-index-regen.name
#
# Assignment lives in committed .gitattributes:
#   .derived/codebase-index/index.json merge=oap-index-regen
#
# Git invokes:  <driver> %O %A %B %P
#   $1 = %O  ancestor version  (unused — the index is fully derived)
#   $2 = %A  current/ours temp file — the driver MUST leave the merged
#            result here and exit 0 on success
#   $3 = %B  other/theirs version  (unused)
#   $4 = %P  pathname being merged (for logging)
#
# Fail-closed: if the indexer is not built or `compile` fails, exit 1 and
# leave the conflict in place. The CI staleness gate
# (`codebase-indexer check`, spec 101/184) remains the source of truth;
# this driver is a convenience over the conflict, never a replacement for
# the gate.

set -eu

OURS="${2:?merge driver expects %A as \$2}"
PATHNAME="${4:-.derived/codebase-index/index.json}"

INDEXER=tools/spec-spine/codebase-indexer/target/release/codebase-indexer
INDEX=.derived/codebase-index/index.json

if [ ! -x "$INDEXER" ]; then
  cat >&2 <<EOF
[merge-derived-index] codebase-indexer binary not built — cannot auto-resolve $PATHNAME.
            Build it (\`make setup\` one-time, or \`make index\`), then re-run the
            rebase/merge, or resolve manually:
                make registry && git add $INDEX
EOF
  exit 1
fi

# Regenerate from the merged working tree. The indexer is deterministic
# for a given committed input set, so the regenerated index is the correct
# union of both branches' input changes.
if ! "$INDEXER" compile >/dev/null 2>&1; then
  cat >&2 <<EOF
[merge-derived-index] \`codebase-indexer compile\` failed; leaving conflict in $PATHNAME
            for manual resolution (\`make registry && git add $INDEX\`).
EOF
  exit 1
fi

# Hand the freshly-compiled artifact back to git as the merge result.
cp "$INDEX" "$OURS"
echo "[merge-derived-index] regenerated $PATHNAME from the merged tree." >&2
exit 0
