# 051 — Worktree Agents plan review

Date: 2026-03-30
Reviewer: claude
Plan: `.ai/plans/051-worktree-agents-phased-plan.md`
Spec: `specs/051-worktree-agents/spec.md`

## Verdict

**Plan approved for Phase 1 start.** No blockers.

## Requirement coverage

All 19 requirements (10 FR + 3 NF + 6 SC) are mapped to phases:

| Requirement | Phase | Status |
|-------------|-------|--------|
| FR-001 (spawn + worktree + branch) | 1 | Covered |
| FR-002 (concurrency limit + FIFO queue) | 2 | Covered |
| FR-003 (skip-permissions mode) | 3 | Covered |
| FR-004 (inactivity timeout) | 3 | Covered |
| FR-005 (lifecycle events) | 4 | Covered |
| FR-006 (getAgentDiff) | 5 | Covered |
| FR-007 (mergeAgent with strategies) | 5 | Covered |
| FR-008 (discardAgent) | 5 | Covered |
| FR-009 (listAgents) | 4 | Covered |
| FR-010 (shared .git, isolated working tree) | 1 | Covered |
| NF-001 (worktree creation < 2s) | 6 | Covered |
| NF-002 (separate processes) | 3 | Covered |
| NF-003 (idempotent cleanup) | 1 | Covered |
| SC-001 (spawn isolation) | 3 | Covered |
| SC-002 (queue at max concurrency) | 2 | Covered |
| SC-003 (timeout + timed_out event) | 3 | Covered |
| SC-004 (correct unified diff) | 5 | Covered |
| SC-005 (squash = single commit) | 5 | Covered |
| SC-006 (discard removes all artifacts) | 5 | Covered |

## Phase ordering assessment

Worktree manager → concurrency → runner → events → diff/merge → integration. Sound dependency chain — each phase builds on the prior. Runner (Phase 3) needs worktrees (Phase 1) and concurrency (Phase 2). Events (Phase 4) need agent state transitions from runner. Diff/merge (Phase 5) need completed agents. Integration (Phase 6) needs all prior components.

## Decision review (W-001..W-006)

- **W-001 (TypeScript package + Tauri surface):** Matches spec architecture which shows `packages/worktree-agents/`. Consistent with 048/054 package topology.
- **W-002 (reuse governed execution):** Sound — avoids duplicating dispatch paths from 035/042/044.
- **W-003 (FIFO + dequeue on terminal):** Spec-faithful. Dequeue on `discarded` is a good addition beyond the 4 spec terminal states.
- **W-004 (in-memory first, disk via 052):** Acceptable for first implementation. Agent state loss on restart is a known limitation.
- **W-005 (restrict to spec strategies):** Matches FR-007 exactly.
- **W-006 (idempotent cleanup):** Matches NF-003. Best-effort approach is pragmatic for partial failures.

## Findings

### F-001 — `.worktrees/` directory management unspecified (LOW)

The spec shows `.worktrees/` under the repo root. The plan doesn't mention directory creation or `.gitignore` handling. The `.worktrees/` directory should be gitignored to prevent committing worktree contents. Phase 1 should ensure this directory is created on first spawn and a `.gitignore` entry exists (or the directory itself contains a `.gitignore` with `*`).

### F-002 — Cherry-pick commit selection detail missing (LOW)

FR-007 specifies cherry-pick "with commit selection." Phase 5 lists cherry-pick as a strategy but doesn't detail how commit selection is exposed in the API. At minimum, the `cherry-pick` strategy should accept a list of commit SHAs. Clarify during Phase 5 implementation.

### F-003 — Orphaned worktree reconciliation not assigned to a phase (LOW)

Risk R-003 mentions a "startup reconciliation sweep" but no phase owns this deliverable. Recommend adding it to Phase 1 (as part of worktree manager) or Phase 6 (integration). It's a resilience concern, not a blocker.

### F-004 — Permission set type shape undefined (INFO)

Phase 3 accepts "pre-approved permissions contract" as an integration point for 049. Since 049 is a separate spec, a minimal placeholder type (e.g., `PermissionSet` with allowed tools list) is sufficient. Not a blocker.

### F-005 — `tool_use` event payload shape unspecified (INFO)

FR-005 lists `tool_use` as a lifecycle event. Phase 4 should define what data it carries (at minimum: tool name, timestamp). Standard for Phase 4 implementation detail.

### F-006 — In-memory registry resilience (INFO)

W-004 means all agent state is lost on crash/restart. Orphaned worktrees from a crash would not be tracked. F-003's reconciliation sweep partially addresses this. Acceptable given 052 integration is planned.

## Summary

6 findings: F-001 LOW (`.worktrees/` gitignore), F-002 LOW (cherry-pick commit selection API), F-003 LOW (orphan sweep phase assignment), F-004–F-006 INFO. No blockers for Phase 1 start.
