# 051 Phase 4 Review — Lifecycle Event Bus + Listing API

**Reviewer:** claude
**Date:** 2026-03-30
**Verdict:** ✅ **APPROVED** — no blockers for Phase 5

## Requirements assessed

| Requirement | Status | Evidence |
|-------------|--------|----------|
| FR-005 (lifecycle events: spawned, running, tool_use, completed, failed, timed_out) | ✅ Satisfied | All 6 event types defined as typed payloads (`AgentSpawnedEvent` … `AgentTimedOutEvent`) with discriminated union via `status` field. `emit()` dispatches per-status + wildcard `event` channel. Test at lifecycle-events.test.ts:6–72 emits all 6 in sequence and verifies payload fields per type. |
| FR-009 (listAgents returns active + recent terminal with status, branch, elapsed, last event) | ✅ Satisfied | `listAgents(now)` iterates records, partitions into active (non-terminal) and recent terminal (capped by `recentTerminalLimit`). Each `AgentListItem` includes `agentId`, `status`, `branchName`, `elapsedMs`, `startedAt`, `lastEvent`. Test at lifecycle-events.test.ts:74–145 verifies ordering (active first, then recent terminal newest-first) and field correctness. |

## Architecture analysis

### Event payload completeness

All 6 statuses from FR-005 are covered with explicit typed payloads:

- `spawned` — `branchName`, `worktreePath`, `parentBranch` (context needed for UI display)
- `running` — minimal (status transition marker)
- `tool_use` — optional `detail` string
- `completed` — `exitCode`, `signal` (process termination info)
- `failed` — `exitCode`, `signal`, optional `detail`
- `timed_out` — `timeoutMs` (the configured timeout value)

All extend `BasePayload` (`agentId` + `timestamp`), providing consistent identification and ordering.

### Event/state ordering guarantees

- `emit()` calls `projectEvent()` synchronously before dispatching to listeners — internal state is always consistent when subscribers fire.
- Per-status `on()` and wildcard `onAny()` both supported, with typed unsubscribe functions returned.
- `AgentLifecyclePayloadByStatus` mapped type ensures compile-time safety: `emit("spawned", ...)` requires `AgentSpawnedEvent` fields.

### listAgents projection correctness

- Active agents sorted by `lastEvent.timestamp` descending (most recently active first).
- Terminal agents ordered by insertion into `terminalOrder` array (most recent last), iterated in reverse — newest terminal agents appear first.
- Window capping via `recentTerminalLimit` (default 25) with lazy GC at 3× threshold in `trimTerminalOrder()`.
- `elapsedMs` computed as `max(0, now - baseline)` where baseline is `startedAt` (set on `spawned`) or `lastEvent.timestamp` as fallback — correct for agents that may emit non-spawned events first.
- Agent resurrection (non-terminal event after terminal) handled via `pruneTerminalEntry()` — removes from terminal order and resets to active.

### Export surface

- `AgentLifecycleBus` class + all 6 event types + `AgentListItem` + `AgentLifecyclePayload` + `AgentLifecyclePayloadByStatus` exported from `index.ts`.
- Package subpath `./lifecycle-events` correctly wired in `package.json` exports.

### Test coverage

- 2 tests covering: (1) all 6 payload types emitted with correct fields, (2) `listAgents` projection with active-first ordering and terminal window cap.
- 11/11 total package tests pass.

## Findings

| ID | Severity | Finding |
|----|----------|---------|
| P4-001 | LOW | No `onAny()` test — wildcard subscription is implemented and exported but untested. Would catch regressions in the dual-emit path (`emit(status, ...)` + `emit("event", ...)`). |
| P4-002 | LOW | No unsubscribe test — both `on()` and `onAny()` return unsubscribe closures but neither is tested for actually removing the listener. |
| P4-003 | LOW | Agent resurrection (non-terminal after terminal) is handled in code (`pruneTerminalEntry`) but untested. Edge case: an agent that `timed_out` then re-emits `spawned` would re-enter active set — the pruning logic should be verified. |
| P4-004 | INFO | `recentTerminalLimit` validation rejects non-integer and < 1 values (good), but default 25 is reasonable for typical usage — no issue. |
| P4-005 | INFO | `trimTerminalOrder` uses a 3× threshold before GC — amortized cleanup avoids per-event overhead. Correct tradeoff for in-memory operation. |
| P4-006 | INFO | `AgentLifecycleStatus` type is imported from `agent-runner.ts` rather than co-located — creates a coupling direction (lifecycle-events depends on agent-runner types). Acceptable for now since both are in the same package, but worth noting if modules are ever split. |

## Decision

**Phase 4 approved.** FR-005 and FR-009 are fully satisfied. Event payloads are complete with appropriate typed fields per status. The `listAgents()` projection correctly partitions active vs. recent terminal agents with proper ordering and window capping. No blockers for Phase 5 (diff/merge/discard workflow).
