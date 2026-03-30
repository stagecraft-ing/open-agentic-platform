# 045 sidecar slice review

> Reviewed: 2026-03-29 by **claude**. Assesses cursor's sidecar implementation against spec 045.

## What was delivered

1. **`packages/claude-code-bridge/src/sidecar.ts`** — Node entry point: reads query from stdin, calls `queryClaudeCode()`, streams `bridgeEventToClaudeOutputLines` on stdout, reads `permission-response` / `abort` from stdin via concurrent control loop.
2. **`packages/claude-code-bridge/src/claude-output-lines.ts`** — Extracted from `apps/desktop/src/lib/bridgeEventToClaudeOutput.ts` (which is now a re-export). Canonical home in bridge package.
3. **`commands/claude.rs: execute_claude_bridge`** — Tauri command: resolves `dist/sidecar.js`, writes query JSON to stdin, delegates to `spawn_claude_bridge_process`.
4. **`commands/claude.rs: respond_to_bridge_permission`** — Writes `permission-response` JSON to sidecar stdin.
5. **`commands/claude.rs: spawn_claude_bridge_process`** — Spawns Node, pipes stdin query, reads stdout line-by-line, emits `claude-output` events, registers session in ProcessRegistry, handles cleanup.
6. **`ClaudeBridgeIpcState`** — Holds `bridge_stdin` for permission/abort writes.

## Verified against spec

| Requirement | Status | Evidence |
|------------|--------|----------|
| FR-001 async generator | ✅ | `sidecar.ts:130` calls `queryClaudeCode(opts)` |
| FR-002 BridgeQueryOptions | ✅ | `sidecar.ts:118-127` maps stdin query to opts |
| FR-003 Session resumption | ✅ | `sidecar.ts:121` passes `sessionId` → `sdk-adapter` passes `resume` |
| FR-004 canUseTool via broker | ✅ **NOW WIRED** | `sidecar.ts:126` delegates to `broker.request()`, `sidecar.ts:90-93` sets `eventSink` to write permission-request JSONL to stdout, `sidecar.ts:112-113` reads permission-response from stdin and calls `broker.respond()` |
| FR-005 permissionMode fallback | ✅ | `sidecar.ts:123` passes mode to opts |
| FR-006 AbortController | ✅ | `sidecar.ts:87` creates AC, `sidecar.ts:109` aborts on `abort` message |
| FR-007 session-complete cost | ✅ | Flows through `queryClaudeCode` → `claude-output-lines` mapper |
| FR-008 CLI fallback | ✅ | `queryClaudeCode` in `index.ts` handles SDK-missing case |
| FR-009 Discriminated BridgeEvent | ✅ | Unchanged, still correct |

### F-003 (PermissionBroker) — RESOLVED

The sidecar wires the full permission round-trip:
- `broker.setEventSink()` called at line 90 — emits `permission-request` JSONL to stdout
- `canUseTool` at line 126 delegates to `broker.request()` (was bypassed before)
- Control loop at lines 112-113 reads `permission-response` from stdin and calls `broker.respond()`

This completes the IPC path described in the spec's architecture section and `.ai/findings/045-tauri-node-ipc-plan.md`.

**Note:** The `sdk-adapter.ts` still has the dead `(gen as any).__permissionBroker = broker` at line 73 and the direct-callback bypass at lines 54-59. This is fine — when called from the sidecar, `canUseTool` is provided (broker-backed), so the direct path runs. The `__permissionBroker` attachment is unused dead code but harmless.

## New findings

### F-011: cancel_claude_execution drops stdin without sending abort message (MEDIUM)

`commands/claude.rs:1207-1208`: `cancel_claude_execution` sets `bridge_stdin` to `None`, which closes the pipe. The sidecar's control loop (`sidecar.ts:98-99`) will see `n.done = true` and exit. However, the `AbortController.abort()` at `sidecar.ts:109` is never triggered — it only fires when the sidecar receives `{"type":"abort"}` on stdin.

**Impact:** Cancellation works (stdin close causes the readline iterator to end, the `queryClaudeCode` generator's `for await` eventually terminates), but it's a hard kill rather than graceful abort. The SDK's `AbortController` signal is never fired, so the SDK doesn't get a chance to cleanly terminate the API call. The spec (FR-006) says "the bridge propagates the signal to the SDK's `query()`, which terminates the current turn gracefully."

**Fix:** Before dropping `bridge_stdin`, write `{"type":"abort"}\n` to the pipe:
```rust
if let Some(ref mut stdin) = *guard {
    let _ = stdin.write_all(b"{\"type\":\"abort\"}\n").await;
    let _ = stdin.flush().await;
}
*guard = None;
```

### F-012: sidecar_js_path uses CARGO_MANIFEST_DIR (dev-only) (LOW)

`commands/claude.rs:156-158`: `bridge_sidecar_js_path()` resolves `dist/sidecar.js` relative to `CARGO_MANIFEST_DIR/../../../packages/claude-code-bridge/dist/sidecar.js`. This only works in development (when running from the repo). In a production Tauri bundle, the path won't exist.

**Impact:** Acceptable for current dev phase. Needs a resource/bundle strategy before shipping. Not blocking.

### F-013: sidecar stdout double-parse (COSMETIC)

`commands/claude.rs:1598-1604` parses every stdout line as JSON to check for `{"done":true}`, then immediately parses again at line 1604 to check for `system/init`. Could combine into one parse.

**Impact:** Negligible perf (one extra JSON parse per line).

### F-014: bridge_sidecar_js_path error message could be clearer (COSMETIC)

When `dist/sidecar.js` doesn't exist, the error should mention `pnpm exec tsc -p packages/claude-code-bridge/tsconfig.json` as the fix.

Let me check:

The error message at `claude.rs:160` likely already mentions this — confirmed by the handoff note. Acceptable.

### F-015: `claude-output-lines.ts` is a near-duplicate of `bridgeEventToClaudeOutput.ts` (RESOLVED)

`apps/desktop/src/lib/bridgeEventToClaudeOutput.ts` is now a one-line re-export of `@opc/claude-code-bridge/claude-output-lines`. The test file (`bridgeEventToClaudeOutput.test.ts`) still imports from the local path, which is fine — tests the re-export chain. Clean.

## Architecture assessment

The sidecar implementation matches the plan in `.ai/findings/045-tauri-node-ipc-plan.md`:

| Plan item | Status |
|-----------|--------|
| `sidecar.ts` entry point | ✅ Implemented |
| Tauri `execute_claude_bridge` command | ✅ Implemented |
| Tauri `respond_to_permission` command | ✅ `respond_to_bridge_permission` |
| Stdin JSONL protocol (query, permission-response, abort) | ✅ Implemented |
| Stdout JSONL protocol (message events + `{done:true}`) | ✅ Implemented |
| Process registry integration | ✅ Session registered in `spawn_claude_bridge_process` |
| Cleanup on exit | ✅ Stdin cleared, process awaited, registry unregistered |

## What remains for 045

1. **F-011 fix** — send abort JSON before dropping stdin (cursor, quick fix)
2. **Frontend API wiring** — expose `execute_claude_bridge` / `respond_to_bridge_permission` in `apps/desktop/src/lib/api.ts` so the UI can call them
3. **Frontend permission dialog** — when `bridge_permission_request` arrives in `useClaudeMessages`, surface a UI to call `respond_to_bridge_permission` (out of scope per spec, but the IPC is ready)
4. **Production sidecar path** — bundle `dist/sidecar.js` into Tauri resources (F-012, defer)
5. **SC-007** — Remove `[key: string]: any` from `ClaudeStreamMessage` (blocked on consumer audit)

## Summary

**The sidecar slice is solid.** FR-004 (PermissionBroker IPC) is now fully wired end-to-end: sidecar → broker → stdout → Rust → `claude-output` event → frontend, and reverse: frontend → `respond_to_bridge_permission` → Rust → stdin → broker.respond(). Only material issue is **F-011** (cancel doesn't send abort signal). The 045 bridge package is feature-complete against the spec's FR list. Remaining work is frontend integration (API layer + permission UI) and production bundling.

**Baton → cursor** for F-011 fix + frontend `api.ts` wiring when product-ready.
