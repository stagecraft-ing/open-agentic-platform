#!/usr/bin/env sh
# Print a concise promotion checklist (canonical artifacts — not .ai/)
set -e

cat <<'EOF'
Promotion checklist — move durable outcomes out of .ai/ into canonical homes:

  [ ] specs/ — spec.md, plan.md, tasks.md for the active feature (facts, scope, checkboxes)
  [ ] specs/.../execution/changeset.md — what landed and where (implementation truth)
  [ ] specs/.../execution/verification.md — commands run, results, evidence
  [ ] Repo docs — README or operator docs if user-facing behavior changed
  [ ] Code — module/class comments only where they prevent repeated confusion (keep brief)

.ai/ is for temporary handoff and drafts. If a decision should survive the next PR, promote it.

EOF
