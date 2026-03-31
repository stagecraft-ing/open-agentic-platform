# 051 — Worktree Agents phased plan

Date: 2026-03-30
Owner: cursor
Spec: `specs/051-worktree-agents/spec.md`

## Slice selection rationale

Choose **051 Worktree Agents** as the next P1 implementation slice because it unlocks safe parallel execution capacity for follow-on P1 work (especially 052 state persistence and 049 permission system integration) while reusing established governed execution paths from 035/042/044.

## Pre-implementation decisions

- **W-001 (runtime topology):** Implement as new package `packages/worktree-agents/` (TypeScript) with a thin Tauri command surface in `apps/desktop/src-tauri/` for desktop integration.
- **W-002 (agent execution path):** Reuse existing governed execution dispatch primitives; worktree agents provide isolation/lifecycle and call into existing execution adapters rather than introducing another execution runtime.
- **W-003 (queue semantics):** Enforce strict FIFO queue when concurrency limit is reached; dequeue only on terminal states (`completed`, `failed`, `timed_out`, `discarded`).
- **W-004 (state durability):** Persist active agent registry in-memory for first implementation phase set; add stable on-disk resume support later via spec 052 integration.
- **W-005 (merge safety):** Restrict merge operations to explicit strategies from spec (`fast-forward`, `squash`, `cherry-pick`) and require clean preconditions before branch mutation.
- **W-006 (cleanup semantics):** Make cleanup idempotent and resilient to partial failures by best-effort worktree removal followed by branch deletion reconciliation.

## Phase plan

### Phase 1 — Worktree manager core (FR-001, FR-010, NF-003 baseline)

Deliverables:
- `worktree-manager.ts` with create/list/remove helpers.
- Branch naming helper (`agent/<id>-<slug>`), deterministic path layout under `.worktrees/`.
- Idempotent cleanup primitives and existence checks.
- Unit tests for create/remove/list/idempotent removal semantics.

Validation:
- Spawned worktree is isolated from primary working tree.
- Duplicate remove calls are no-op success.

### Phase 2 — Concurrency + queue control (FR-002, SC-002)

Deliverables:
- `concurrency.ts` semaphore with configurable max slots (default 4).
- FIFO pending queue with dequeue on slot release.
- Queue/active status metrics exposed to API layer.

Validation:
- With max=2, 3rd request waits until one active agent exits.
- FIFO order preserved under concurrent completions.

### Phase 3 — Agent runner lifecycle + timeout (FR-003, FR-004, NF-002, SC-001, SC-003)

Deliverables:
- `agent-runner.ts` process lifecycle (spawn, monitor, terminate).
- Inactivity timer reset on agent output/lifecycle events.
- Timeout path emits `timed_out` and preserves worktree.
- Spawn options accept pre-approved permissions contract (integration point for 049).

Validation:
- Timed-out agent transitions to terminal state with preserved worktree.
- Background process isolation keeps interactive session responsive.

### Phase 4 — Lifecycle event bus + listing API (FR-005, FR-009)

Deliverables:
- `lifecycle-events.ts` typed emitter/subscriber API.
- `listAgents()` status projection (active + recent terminal agents).
- Event payloads for `spawned`, `running`, `tool_use`, `completed`, `failed`, `timed_out`.

Validation:
- Event stream order consistent with state transitions.
- `listAgents()` reflects latest lifecycle event and elapsed time.

### Phase 5 — Diff + merge + discard workflow (FR-006, FR-007, FR-008, SC-004, SC-005, SC-006)

Deliverables:
- `diff.ts` unified diff + commit summary between parent and agent branch.
- `merge.ts` support for `fast-forward`, `squash`, and `cherry-pick`.
- `discardAgent()` branch/worktree cleanup wiring.
- Conflict/error surface with actionable diagnostics.

Validation:
- Diff output includes all branch changes.
- Squash merge applies as a single parent-branch commit.
- Discard fully removes worktree/branch artifacts.

### Phase 6 — Desktop/Tauri integration + verification artifacts (NF-001, full SC pass)

Deliverables:
- Tauri command wiring for spawn/list/diff/merge/discard.
- End-to-end integration tests for spawn → complete → diff → merge/discard.
- `specs/051-worktree-agents/execution/verification.md` with FR/NF/SC evidence.

Validation:
- Worktree creation under 2s on representative repo fixture.
- End-to-end flows pass for merge and discard paths.

## Risks and mitigations

- **R-001 (`.git` contention):** Keep git operations short-lived and serialized per agent id where needed.
- **R-002 (parent branch drift before merge):** Recompute merge preconditions at merge time and surface clear conflict guidance.
- **R-003 (orphaned worktrees):** Add startup reconciliation sweep for non-running agents with stale worktrees.
- **R-004 (permission overreach in skip mode):** Restrict pre-approved permission set at spawn, defer richer policy checks to 049 integration.

## Proposed next baton

- Next reviewer: **claude**
- Requested output: Phase-plan review against `specs/051-worktree-agents/spec.md` with findings and approval/blockers for Phase 1 start.
