---
feature_id: "032-opc-inspect-governance-wiring-mvp"
---

# Changeset

Slice 1 implementation batch for OPC inspect + governance wiring.

**PR-1:** import + T000a baseline only — record evidence in [`verification.md`](./verification.md); no 032 product behavior except consolidation fixes.  
**PR-2:** T003 inspect shell only (see `plan.md` PR sequencing).

## In-scope tasks

- T003
- T004
- T005
- T006
- T007
- T008
- T009
- T010
- T011

## Planned touch targets (pending OPC tree import)

- Inspect: `apps/desktop/src/features/inspect/*`
- Git context: `apps/desktop/src/features/git/*`, `packages/mcp-client/src/index.ts`
- Governance: `apps/desktop/src/features/governance/*`, `apps/desktop/src-tauri/src/commands/analysis.rs`
- Action handoff: `apps/desktop/src/features/inspect/actions.ts`
- Tests: inspect/git/governance panel tests and backend command tests

## Approval and safety notes

- Any repository-mutating automation or destructive command path requires explicit review approval before execution.
- External runner invocations must be listed in `execution/verification.md` with purpose and preconditions.

## Governance evidence

- Fixtures touched: `none`
- Spec/doc touchpoints: `specs/032-opc-inspect-governance-wiring-mvp/tasks.md`, `specs/032-opc-inspect-governance-wiring-mvp/execution/verification.md`, `specs/032-opc-inspect-governance-wiring-mvp/execution/pr1-import-runbook.md`
