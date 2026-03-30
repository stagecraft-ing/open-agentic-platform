# 045 Tauri ↔ Node bridge IPC plan

> Authored: 2026-03-29 by **claude**. Recommendation for next implementation slice.

## Problem

The `@opc/claude-code-bridge` package is pure Node/TS. Tauri's backend is Rust. The bridge's `queryClaudeCode()` async generator cannot be called directly from Rust. We need an IPC boundary.

## Options evaluated

### Option A: Node sidecar process (recommended)

Tauri spawns a long-lived Node.js child process that hosts the bridge. Communication over stdin/stdout JSONL.

**Flow:**
```
Tauri command (Rust)
  → spawn node sidecar (packages/claude-code-bridge/sidecar.ts)
  → write JSON request to sidecar stdin
  → sidecar calls queryClaudeCode(), yields BridgeEvents
  → sidecar writes each BridgeEvent as JSONL to stdout
  → Rust reads stdout line-by-line
  → Rust emits "claude-output" events to frontend (via bridgeEventToClaudeOutput mapping)
```

**Permission round-trip (FR-004):**
```
sidecar stdout: {"kind":"permission-request","requestId":"...","toolName":"...","toolInput":{...}}
  → Rust emits "claude-permission-request" to frontend
  → Frontend shows dialog → calls Tauri command "respond_to_permission"
  → Rust writes {"requestId":"...","allowed":true} to sidecar stdin
  → sidecar calls broker.respond() → SDK continues
```

**Pros:**
- Clean process boundary — bridge crashes don't take down Tauri
- Works with optional `@anthropic-ai/claude-code` peer dep (sidecar just needs Node)
- Sidecar can be the same process that runs the CLI fallback
- No Rust-to-JS FFI complexity
- Cancellation: Rust sends `{"abort":true}` on stdin or kills sidecar

**Cons:**
- Extra process overhead (~30MB resident for Node)
- Slightly more latency on IPC (negligible vs Claude API latency)
- Must manage sidecar lifecycle (start, restart, health check)

**Implementation plan:**
1. Create `packages/claude-code-bridge/src/sidecar.ts` — reads JSON requests from stdin, calls `queryClaudeCode()`, writes BridgeEvent JSONL to stdout, reads permission responses from stdin
2. Add Tauri command `execute_claude_bridge` in `commands/claude.rs` — spawns `node sidecar.ts`, manages stdin/stdout, emits events
3. Add Tauri command `respond_to_permission` — writes to sidecar stdin
4. Frontend: call `execute_claude_bridge` instead of `execute_claude_code`; handle `claude-permission-request` events

### Option B: Tauri shell plugin with bundled script

Use `@tauri-apps/plugin-shell` to run the bridge as a one-shot script per query. Simpler but no stdin channel for permission responses — would need a separate IPC (e.g., temp file, Unix socket).

**Verdict:** More complex for permission round-trip. Rejected.

### Option C: Embedded JS runtime (Deno/QuickJS in Rust)

Embed a JS runtime in the Tauri process to run the bridge directly.

**Verdict:** Huge complexity, fragile FFI. Rejected.

### Option D: Web-server mode only

Run the bridge in the existing web-server backend (Node/Express) and use WebSocket for IPC.

**Verdict:** Only works in web mode, not native Tauri. Could be a secondary path but not the primary one.

## Recommendation

**Option A (Node sidecar)** with a thin sidecar entry point. The sidecar:
- Accepts a JSON request object on stdin (one line)
- Calls `queryClaudeCode(options)`
- Writes each `BridgeEvent` as a JSONL line to stdout
- Reads permission responses from stdin (interleaved with the initial request)
- Writes a final `{"done":true}` sentinel after the generator completes

The Rust side needs:
- `spawn` the sidecar (reuse Tauri's `Command` or raw `std::process::Command`)
- Async reader on stdout → parse JSONL → emit `claude-output` events
- Writer on stdin for permission responses and abort signals
- Process registry entry for cleanup on app shutdown

## Sidecar protocol (draft)

**Stdin → sidecar:**
```jsonl
{"type":"query","prompt":"...","workingDirectory":"...","model":"...","sessionId":"...","permissionMode":"default"}
{"type":"permission-response","requestId":"abc-123","allowed":true}
{"type":"abort"}
```

**Stdout ← sidecar:**
```jsonl
{"kind":"start","sessionId":"..."}
{"kind":"message","message":{...}}
{"kind":"permission-request","requestId":"abc-123","toolName":"Bash","toolInput":{...}}
{"kind":"session-complete","summary":{...}}
{"done":true}
```

## Priority

1. **Fix F-001 first** (useClaudeMessages parity) — without this, no bridge integration will render correctly
2. **Fix F-002** (duplicate session-complete) — quick fix, prevents consumer confusion
3. **Implement sidecar.ts** — the bridge entry point
4. **Implement Rust sidecar management** — spawn, read, write, cleanup
5. **Wire permission round-trip** — optional, can ship without initially
