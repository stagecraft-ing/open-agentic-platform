# 051 Phase 3 Review — Agent Runner Lifecycle

**Reviewer:** claude
**Date:** 2026-03-30
**Verdict:** ✅ Approved — no blockers for Phase 4
**Validation:** `pnpm --filter @opc/worktree-agents test` → 9/9 pass

## Requirement coverage

### FR-003 — Skip-permissions mode with pre-approved permission set at spawn

**Satisfied.** `AgentRunnerSpawnOptions.permissions` is a required field accepting `PreApprovedPermissions` (`tools`, `readGlobs`, `writeGlobs`, `networkHosts`). The type is exported from `index.ts`. The permissions contract is structural — the runner stores them but does not enforce them itself, which is correct: enforcement belongs to the 049 integration layer that will consume these values when wiring the agent's execution environment. Both tests pass distinct permission sets (`{ tools: ["read", "write"] }` and `{ tools: ["read"] }`), confirming the contract is threaded through.

### FR-004 — Inactivity timeout with worktree preservation

**Satisfied.** `touchActivityTimer()` (line 158) resets the inactivity timer on every lifecycle emission where `shouldTouch=true`. Default timeout is 5 minutes (`DEFAULT_INACTIVITY_TIMEOUT_MS = 300_000`), configurable via `inactivityTimeoutMs`. On timeout:
1. `finalize("timed_out", ...)` is called, setting `state.terminal = true` and emitting the terminal lifecycle event.
2. `child.kill("SIGTERM")` is sent.
3. A kill escalation timer (`timeoutKillGraceMs`, default 1s) sends `SIGKILL` if the process hasn't exited.
4. The worktree is **not** removed — the runner has no reference to cleanup, so the worktree persists by design. Test at `agent-runner.test.ts:82` explicitly asserts `existsSync(worktreePath)` is `true` after timeout.

### NF-002 — Background agents do not degrade interactive responsiveness

**Satisfied.** The runner spawns a child process via `child_process.spawn()` (line 98) with `stdio: ["ignore", "pipe", "pipe"]`. The child runs in a separate OS process with its own `cwd` set to the worktree path. No shared-thread or in-process execution — the runner only reads stdout/stderr pipes and manages timers. This is a separate-process architecture by construction.

### SC-001 — Agent creates worktree/branch, runs in that directory, main tree unaffected

**Satisfied.** Test at `agent-runner.test.ts:25-54`:
- Creates a worktree via `createAgentWorktree`, then spawns a Node process that writes `from-runner.txt` in its cwd (the worktree).
- Asserts file exists in `worktreePath` but does **not** exist in `repo` root.
- Agent completes with `status: "completed"` and `timedOut: false`.

### SC-003 — Inactive agent is terminated with `timed_out` lifecycle event

**Satisfied.** Test at `agent-runner.test.ts:56-85`:
- Spawns a process that runs an infinite `setInterval` (produces no stdout/stderr after spawn).
- `inactivityTimeoutMs: 100` triggers timeout.
- Asserts `done.status === "timed_out"` and `done.timedOut === true`.
- Asserts worktree directory still exists.
- Asserts lifecycle events contain both `"spawned"` and `"timed_out"`.

## Findings

### P3-001 — `tool_use` emitted on every stdout/stderr chunk (LOW)

All stdout/stderr data events emit `"tool_use"` lifecycle status (lines 181-189). The spec defines `tool_use` as "tool invocation" (FR-005), but raw stdout chunks are not necessarily tool invocations — they include any process output. This is acceptable for Phase 3 since the runner has no structured protocol to distinguish tool calls from general output. Phase 4 (lifecycle events) or the 049 integration layer should refine the event classification if needed.

**Evidence:** `agent-runner.ts:184` — `emitLifecycle("tool_use", text.slice(0, 240))`

### P3-002 — No test for `stop()` function (LOW)

The `start()` return value includes a `stop()` callback (lines 201-207) that sends `SIGTERM` to the child if not already terminal. No test exercises this path. Manual stop is a useful escape hatch for the UI layer.

**Evidence:** `agent-runner.ts:201-207` — `stop` closure; no corresponding test case.

### P3-003 — `finalize` called before `SIGTERM` on timeout path (LOW)

On inactivity timeout (lines 162-169), `finalize()` is called first (setting terminal state and resolving the promise), then `SIGTERM` is sent. This means the `AgentRunResult` promise resolves before the child process is actually killed. The `close` event handler (line 194) correctly checks `state.terminal` and no-ops, so there is no double-resolution. However, a consumer awaiting `result` could proceed (e.g., reading the worktree) while the process is still running for up to `timeoutKillGraceMs`. This is a minor timing concern — the worktree files may still be written to briefly after the result resolves.

**Evidence:** `agent-runner.ts:162-164` — `finalize` → `child.kill("SIGTERM")` ordering.

### P3-004 — No validation of `inactivityTimeoutMs` / `timeoutKillGraceMs` (INFO)

Constructor validates `agentId`, `worktreePath`, and `command` for non-empty, but does not validate that timeout values are positive numbers. Negative or zero timeouts would cause immediate or undefined timer behavior. Low risk since callers control these values.

**Evidence:** `agent-runner.ts:77-94` — no numeric validation on timeout fields.

### P3-005 — Kill escalation timer not cleared on normal close (INFO)

If the process exits normally during the kill grace period (unlikely but possible with fast `SIGTERM` handling), the `close` handler returns early because `state.terminal` is already true (line 195). The `killEscalationHandle` timeout fires but the `child.exitCode !== null` check (line 166) prevents the redundant `SIGKILL`. Safe in practice.

**Evidence:** `agent-runner.ts:165-169` — escalation timer; `agent-runner.ts:194-195` — close handler early return.

### P3-006 — `error` event emitted after `finalize` on spawn failure (INFO)

On the `error` event (lines 190-193), `finalize("failed", ...)` is called first, then `this.emit("error", err)` is emitted. This means the lifecycle `"failed"` event and result resolution happen before the error details are emitted. Consumers that listen to both `lifecycle` and `error` events should handle the ordering. Standard Node.js EventEmitter pattern.

**Evidence:** `agent-runner.ts:190-193` — `finalize` before `this.emit("error", err)`.

## Exports verification

- `BackgroundAgentRunner` and `AgentRunnerError` exported as values from `index.ts:9-10`.
- `AgentLifecycleEvent`, `AgentLifecycleStatus`, `AgentRunResult`, `AgentRunnerSpawnOptions`, `PreApprovedPermissions` exported as types from `index.ts:12-18`.
- Package subpath `./agent-runner` registered in `package.json:14`.

## Summary

Phase 3 delivers a clean process-based agent runner with proper inactivity timeout semantics, terminal-state guards, and worktree preservation. All five target requirements (FR-003, FR-004, NF-002, SC-001, SC-003) are satisfied with direct test evidence. The `finalize-before-kill` ordering (P3-003) is a minor timing concern but not a blocker — consumers should be aware that the process may still be alive briefly after the result resolves. No blockers for Phase 4.
