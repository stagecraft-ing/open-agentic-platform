# 045 post-F001/F002 review

> Reviewed: 2026-03-29 by **claude**. Assesses state after cursor's F-001/F-002 fixes.

## Verified fixes

### F-001 — RESOLVED
`useClaudeMessages.ts:28-70` now branches on `system` (init → sets streaming, sessionId, sessionInfo), `assistant` (extracts usage/tool_calls), `result` (final token update, stops streaming), `error`, and `bridge_permission_request`. Legacy types (`start`, `partial`, `response`, `session_info`) are retained as fallback branches for CLI-direct consumers. Correct.

### F-002 — RESOLVED
`cli-adapter.ts:46` introduces `sessionCompleteTracker = { emitted: false }`, set to `true` at line 176 when `streamOutput` encounters a `result` line. The fallback at line 69 only fires when `!sessionCompleteTracker.emitted`. No double-emission possible.

### F-005 — RESOLVED
`spec.md:3` now reads `feature_branch: "045-claude-code-sdk-bridge"`. Correct.

### F-006 — RESOLVED
`ClaudeStreamMessage` is defined once in `AgentExecution.tsx:62-103` and imported everywhere else. The triple-definition is gone. The `[key: string]: any` index signature remains (line ~103) — tolerable for now since the hook does typed checks, but should eventually be removed when all consumers are migrated to typed fields.

## Open findings

### F-003 — PermissionBroker dead code (STILL OPEN, severity: MEDIUM)

`sdk-adapter.ts:35`: `new PermissionBroker()` is created. It is:
- Attached to the generator at line 73 via `(gen as any).__permissionBroker = broker`
- Passed to `queryImpl` at line 72
- Called in `queryImpl` only for `denyAll()` on error (lines 119, 147)

The `broker.request()` method is **never called**. The `canUseTool` wiring at lines 54-59 bypasses the broker entirely — it wraps the user callback directly without going through `broker.request()`.

**Why this matters:** The sidecar IPC plan (`.ai/findings/045-tauri-node-ipc-plan.md`) requires the broker pattern for the permission round-trip: sidecar emits `permission-request` on stdout → Rust forwards to UI → user responds → Rust writes `permission-response` on stdin → broker resolves the pending promise. Without wiring `broker.request()` into `canUseTool`, the sidecar has no way to emit permission events into the BridgeEvent stream.

**What needs to happen:**
1. When `canUseTool` is NOT provided (the IPC case), set `queryOptions.canUseTool` to delegate to `broker.request(toolName, toolInput)`.
2. Wire `broker.setEventSink()` to a callback that yields `permission-request` BridgeEvents from the generator. This requires a coordination pattern (e.g., an async queue that `queryImpl` drains alongside the SDK stream).
3. The sidecar (not yet implemented) will read `permission-response` messages from stdin and call `broker.respond(requestId, allowed)`.

**Recommendation:** Defer full wiring until the sidecar is implemented (it's the only consumer). But the current code should at minimum have a `// TODO: wire broker.request for sidecar IPC` comment so the intent is clear, and the dead `__permissionBroker` attachment should be documented.

### F-004 — `duration_api_ms` lost in round-trip (STILL OPEN, severity: LOW)

`bridgeEventToClaudeOutput.ts:39` hardcodes `duration_api_ms: 0`. The `SDKResultMessage` type includes `duration_api_ms` (types.ts:57), and `sdk-adapter.ts:113` has it available on the result, but `SessionCostSummary` doesn't carry it.

No current consumer uses this field. Track as tech debt.

### F-007 — `cost-tracker.ts` not implemented (STILL OPEN, severity: LOW)

Cost extraction remains inline in `sdk-adapter.ts:106-114`. The spec lists `cost-tracker.ts` as a separate module. Either update the spec or extract. Not blocking.

### F-008 — NEW: `start` event dropped by mapper

`bridgeEventToClaudeOutput.ts:13-14` returns `[]` for `kind: "start"`. This means when the bridge is used via the Tauri sidecar path, the frontend never receives the `system` init message that would come from a `start` event.

However, looking more carefully: the `start` event is emitted by `sdk-adapter.ts:94` *in addition to* the `message` event at line 99 (both fire for `type === "system"`). So the init message does reach the frontend via the `kind: "message"` path. The `start` event is purely a bridge-level signal with just the sessionId — no message payload. The mapper correctly drops it since the `message` event already carries the `system` init. **Not a bug, but the empty return could use a comment.**

### F-009 — NEW: handleMessage stale-closure risk on `currentSessionId`

`useClaudeMessages.ts:119`: `handleMessage` has `currentSessionId` in its dependency array. This is a React state variable. Inside the callback (line 70), `options.onStreamingChange?.(false, m.session_id ?? currentSessionId)` uses `currentSessionId` which could be stale if multiple messages arrive in the same render cycle. The `result` handler at line 70 also uses it.

**Severity: LOW.** In practice, the `result` message always carries `session_id` (the `??` fallback rarely fires). But the `error` branch at line 72 relies solely on `currentSessionId`, which is guaranteed stale during rapid message bursts. Could use a ref instead.

### F-010 — NEW: ClaudeStreamMessage index signature masks type errors

`AgentExecution.tsx:~103`: `[key: string]: any` on `ClaudeStreamMessage` means any property access is valid, defeating TypeScript's type checking. The hook's typed branches work correctly, but any consumer using `message.foo` won't get a compile error even if `foo` doesn't exist.

**Recommendation (SC-007):** Once all consumers handle the typed fields properly, remove the index signature. This is blocked on auditing all consumers of `ClaudeStreamMessage`.

## Spec parity assessment

| Requirement | Status | Evidence |
|------------|--------|----------|
| FR-001 queryClaudeCode async generator | ✅ Done | `index.ts:32` |
| FR-002 BridgeQueryOptions | ✅ Done | `types.ts:124-137` |
| FR-003 Session resumption | ✅ Done | `sdk-adapter.ts:47`, `cli-adapter.ts:93` |
| FR-004 canUseTool + broker | ⚠️ Partial | Direct callback works; broker IPC path dead (F-003) |
| FR-005 permissionMode fallback | ✅ Done | `sdk-adapter.ts:43-44`, `cli-adapter.ts:95-101` |
| FR-006 AbortController | ✅ Done | `sdk-adapter.ts:41`, `cli-adapter.ts:31-39` |
| FR-007 session-complete cost data | ✅ Done | `sdk-adapter.ts:103-116`, `cli-adapter.ts:172-189` |
| FR-008 CLI fallback | ✅ Done | `index.ts:39-49` |
| FR-009 Discriminated BridgeEvent union | ✅ Done | `types.ts:97-102` |
| SC-007 No `as any` in hook | ⚠️ Partial | Hook uses typed branches but `ClaudeStreamMessage` still has index sig |

## Summary

**F-001 and F-002 are cleanly fixed.** The bridge package is solid — types, both adapters, fallback routing, and the mapper all work correctly. The main open item is **F-003 (PermissionBroker wiring)** which blocks the sidecar IPC permission round-trip but doesn't block the current integration where `canUseTool` is passed directly.

**Next recommended actions (priority order):**
1. Implement `sidecar.ts` per `.ai/findings/045-tauri-node-ipc-plan.md` — this is the next vertical slice
2. Wire PermissionBroker into the sidecar's stdin/stdout protocol (F-003 resolution)
3. Add `durationApiMs` to `SessionCostSummary` (F-004, minor)
4. Remove `[key: string]: any` from `ClaudeStreamMessage` after consumer audit (F-010)

**Baton:** Hand to **cursor** for sidecar implementation (`packages/claude-code-bridge/src/sidecar.ts` + Rust spawn in `commands/claude.rs`).
