# 045 Bridge ↔ stream-json parity gaps

> Reviewed: 2026-03-29 by **claude**. Evidence is file-path:line referenced.

## F-001: useClaudeMessages checks obsolete message types

**Severity: HIGH — the hook will silently ignore all bridge-sourced messages**

`useClaudeMessages.ts:26-60` checks for types `start`, `partial`, `response`, `error`, `output`, `session_info`. None of these match what `--output-format stream-json` or the SDK actually emit, which are `system`, `assistant`, `user`, `result`.

The `ClaudeStreamMessage` type (defined identically in 3 places — see F-006) correctly declares `type: "system" | "assistant" | "user" | "result"`, but the runtime `handleMessage()` never branches on these values. Everything falls through to the `else` branch (line 58: "Unknown message type") and is appended to state without processing.

**Impact:** When the bridge is wired, the hook will:
- Never set `isStreaming = true` (looks for `type === "start"`, bridge emits `kind: "start"` → mapper returns `[]`)
- Never extract token counts (looks for `type === "response"`, bridge emits `type: "assistant"` with usage)
- Never extract session info (looks for `type === "session_info"`, bridge emits `type: "system"` with `session_id`)
- Never set `isStreaming = false` (looks for `type === "error"` or `"response"`, bridge emits `type: "result"`)

**Fix:** Rewrite `handleMessage()` to branch on `system`, `assistant`, `user`, `result`, plus the new `bridge_permission_request` and `error` types from the mapper.

## F-002: CLI adapter emits duplicate session-complete

`cli-adapter.ts:159-175` — when the CLI emits a `result` JSONL line, `streamOutput()` yields a `session-complete` event with full cost data. Then `queryViaCli()` (lines 60-71) unconditionally yields a **second** `session-complete` after `streamOutput` finishes.

Consumers that track "done" state via `session-complete` will fire twice. The second event has zeroed-out cost fields, potentially overwriting the real data.

**Fix:** Track whether a `session-complete` was already emitted inside `streamOutput`; skip the fallback emission in `queryViaCli` if so.

## F-003: PermissionBroker instantiated but not connected

`sdk-adapter.ts:35` creates `new PermissionBroker()` and attaches it to the generator at line 73 (`(gen as any).__permissionBroker = broker`), but:
- The broker's `request()` method is never called — the `canUseTool` callback passed to queryOptions (lines 56-58) directly forwards to the user callback, bypassing the broker entirely.
- The broker's `setEventSink()` is never called, so even if `request()` were called it would deny by default (NF-003 fallback).

The broker pattern is needed for the IPC round-trip (bridge → Tauri → UI → Tauri → bridge) described in FR-004 and the spec's "Permission handling" architecture. Currently dead code.

**Fix:** When `canUseTool` is NOT provided but the caller wants IPC-based permissions, wire the broker: set its event sink to yield `permission-request` events into the generator, and have `canUseTool` delegate to `broker.request()`.

## F-004: bridgeEventToClaudeOutput maps `session-complete` with hardcoded `duration_api_ms: 0`

`bridgeEventToClaudeOutput.ts:39` — `SessionCostSummary` doesn't carry `duration_api_ms` (it's not in the bridge types). The mapper hardcodes `0`. The `SDKResultMessage` type in `types.ts:57` does have `duration_api_ms`. This field is lost during the SDK-adapter → BridgeEvent → JSONL round-trip.

**Impact:** Minor. The frontend doesn't currently use this field. But if cost dashboards are built later, API-time vs wall-time will be indistinguishable.

**Fix:** Add `durationApiMs` to `SessionCostSummary` and propagate it through the mapper.

## F-005: Spec frontmatter has wrong feature_branch

`specs/045-claude-code-sdk-bridge/spec.md:4` — `feature_branch: "044-claude-code-sdk-bridge"` should be `045-claude-code-sdk-bridge`.

## F-006: ClaudeStreamMessage defined identically in 3 files

- `AgentExecution.tsx:61`
- `SessionOutputViewer.tsx:23`
- `outputCache.tsx:5`

All three are `{ type: "system" | "assistant" | "user" | "result"; ... [key: string]: any }`. The `[key: string]: any` index signature masks every type error.

**Promotion candidate:** Extract to a shared `types/claude-stream.ts` and import everywhere. Drop the index signature once the hook handles all real message types.

## F-007: `cost-tracker.ts` listed in spec but not implemented

The spec's architecture section lists `cost-tracker.ts` as a separate module. Cost extraction is done inline in `sdk-adapter.ts:106-114`. Not a bug — the spec is aspirational — but the spec should be updated to match reality or the module should be extracted.
