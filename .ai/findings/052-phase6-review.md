# 052 Phase 6 Review — HTTP SSE Endpoint (Partial)

**Reviewer:** claude
**Date:** 2026-03-31
**Base commit:** 62d306f
**Verdict:** Phase 6A (HTTP SSE endpoint) and 6B (broadcaster registry) approved. Phase 6C (integration tests) and 6D (dispatch loop wiring) not yet implemented.

## What Was Delivered

The latest commit (`62d306f`) adds `src/http.rs` — an Axum-based HTTP SSE endpoint satisfying the first two of the four Phase 6 sub-tasks defined in the readiness review:

### 6A — HTTP SSE Endpoint: **APPROVED**

`http.rs:56–98` implements `GET /workflows/:id/events?offset=N`:
- Parses `workflow_id` from path (Uuid), `offset` from query (default 0) — **correct**.
- Looks up broadcaster from `DashMap` registry, returns 404 if missing — **satisfies spec error handling**.
- Locks store (`Arc<Mutex<SqliteWorkflowStore>>`), calls `subscribe_with_replay` — **correct R-002 mitigation**.
- Streams replay events first, then live events via `build_sse_stream` — **satisfies FR-006**.
- Sets `Content-Type: text/event-stream` via Axum's `Sse` wrapper with 15s keep-alive — **correct SSE framing**.
- Each event carries `id:` field (event_id) and `data:` (JSON body) — **enables `Last-Event-ID` reconnection**.

`http.rs:100–145` (`build_sse_stream`):
- Drains replay vec as `stream::iter`, then chains `async_stream` for live events — **correct ordering per SC-004**.
- Live dedup: skips events with `event_id <= current_hwm`, updates `current_hwm` on each accepted event — **correct, matches dedup pattern proven in sse.rs tests**.
- Lagged subscriber: `Err(_lagged) => continue` — silently drops gap. Acceptable for Phase 6; clients can reconnect with a higher offset. Matches P5-008 carry-forward.

### 6B — Per-Workflow Broadcaster Registry: **APPROVED**

`http.rs:33–36` (`HttpState`):
- `store: Arc<Mutex<SqliteWorkflowStore>>` — write serialization via `Mutex`, reads concurrent via WAL. **R-002 resolved.**
- `broadcasters: Arc<DashMap<Uuid, EventBroadcaster>>` — lock-free per-workflow lookup, no contention between independent workflows. **Matches readiness guidance exactly.**

`http.rs:47–54` (`router`):
- Clean Axum router construction with shared state. Single endpoint mounted at `/workflows/:id/events`.

### Dependencies: **CORRECT**

`Cargo.toml`:
- `axum = "0.7"` with `macros` feature — correct for route macros.
- `tokio` now has `rt-multi-thread` and `macros` in main `[dependencies]` — **R-001 resolved**.
- `dashmap = "5"`, `futures-util = "0.3"`, `async-stream = "0.3"` — all appropriate.

## Build and Test Results

- **Build:** `cargo build` succeeds, zero errors.
- **Tests:** 48/48 pass (unchanged from Phase 5 — no new tests added in this commit, expected since this is 6A/6B only).
- **Clippy:** 9 warnings total. 2 are new from `http.rs`:
  - `unnecessary_mut_passed` at `http.rs:70` — `&mut *guard` where `&*guard` suffices (since `subscribe_with_replay` takes `&SqliteWorkflowStore`).
  - `explicit_auto_deref` at `http.rs:70` — `&mut *guard` should be `&guard`.
  - Both are cosmetic. The remaining 7 warnings are pre-existing from earlier phases.

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| P6-001 | LOW | `http.rs:70` — `&mut *guard` triggers two clippy warnings; should be `&*guard` or `&guard` since `subscribe_with_replay` takes `&SqliteWorkflowStore` not `&mut`. |
| P6-002 | LOW | No HTTP-level tests — the SSE endpoint has no test coverage. Unit tests exist for the underlying `EventBroadcaster` and `subscribe_with_replay`, but the Axum handler, 404 path, and SSE framing are untested. |
| P6-003 | LOW | `build_sse_stream` uses `json_data()` which double-serializes: `serde_json::to_string(&event)` produces a JSON string, then `json_data(json)` wraps it again. Should use `.data(json)` instead of `.json_data(json)` to avoid double-encoding. |
| P6-004 | INFO | No lifecycle management for broadcaster registry — broadcasters are never removed from `DashMap` when workflows complete. Acceptable for Phase 6 scope but will leak memory for long-running servers. |
| P6-005 | INFO | Lagged subscriber handling is silent (`continue`). A production SSE server would benefit from sending a reconnect hint or error event, but this is acceptable for Phase 6. |
| P6-006 | INFO | `router()` returns a bare `Router` — no middleware (CORS, auth, logging). Expected for Phase 6; production hardening is a follow-on concern. |

## Phase 6 Completion Status

| Sub-task | Status | Notes |
|----------|--------|-------|
| 6A — HTTP SSE endpoint | **Done** | `http.rs` with Axum SSE, replay+live stream, keep-alive |
| 6B — Broadcaster registry | **Done** | `HttpState` with `Arc<DashMap<Uuid, EventBroadcaster>>` |
| 6C — Integration tests | **Not started** | No `tests/` directory, no cross-module integration tests |
| 6D — Dispatch loop wiring | **Not started** | `dispatch_manifest_noop` still does not persist state or emit events |

## Carry-Forward from Prior Phases

All prior carry-forward findings remain open (P3-001 epoch timestamps, P4-001/P4-002/P4-003 minor schema gaps). None block Phase 6 completion.

## Recommendation

6A and 6B are solid and spec-faithful. The implementer should proceed with:
1. Fix P6-001 (clippy) and P6-003 (double-serialization) — both are one-line fixes.
2. Implement 6C: create `crates/orchestrator/tests/integration_052.rs` with crash-resume and SSE replay tests per readiness guidance.
3. Implement 6D: wire state persistence into the dispatch loop (extend `dispatch_manifest_noop` or create `dispatch_manifest_persisted`).
4. After 6C+6D: create `specs/052-state-persistence/execution/verification.md` documenting SC-001 through SC-006 evidence.

**No blockers for continuing Phase 6.** Hand to **cursor** for 6C + 6D implementation.
