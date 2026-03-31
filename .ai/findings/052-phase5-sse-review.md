# 052 Phase 5 Completion Review — SSE Broadcaster + P5-002 Fix

**Reviewer:** claude
**Date:** 2026-03-31
**Base commit:** f3969ee
**Verdict:** Phase 5 SSE broadcaster implemented — FR-006, NF-002, SC-004 now satisfied at the library layer

## Scope

Phase 5 per spec: "SSE stream server -- Implement the append-only event broadcaster with offset-based replay for late-joining clients." This delivery completes the Phase 5 scope by adding the multi-subscriber broadcast layer on top of the approved data layer (`append_event`/`load_events_since`). Also resolves P5-002 (duplicated row-mapping).

## Files created/modified

- `crates/orchestrator/src/sse.rs` (NEW -- EventBroadcaster, EventSubscriber, ReplaySubscription)
- `crates/orchestrator/src/lib.rs` (added `pub mod sse` + re-exports)
- `crates/orchestrator/src/sqlite_state.rs` (P5-002 fix: unified row-mapping; added `Serialize` to `PersistedEvent`)
- `crates/orchestrator/Cargo.toml` (added tokio `sync` feature)

## What was delivered

1. **`EventBroadcaster`** — wraps `tokio::sync::broadcast::Sender<PersistedEvent>`. `Clone + Send + Sync` for sharing across async tasks. Configurable channel capacity (default 1024 for NF-002 headroom). Methods: `broadcast()`, `subscribe()`, `subscribe_with_replay()`, `subscriber_count()`.

2. **`EventSubscriber`** — wraps `tokio::sync::broadcast::Receiver<PersistedEvent>`. Async `recv()` returns live events or `Lagged`/`Closed` errors.

3. **`ReplaySubscription`** — combines `Vec<PersistedEvent>` (historical replay from SQLite) with an `EventSubscriber` (live stream) and a `high_water_mark` for deduplication. Subscribe-first-then-query pattern ensures no events are missed between SQLite read and broadcast subscription.

4. **P5-002 fix** — `load_events_since` unified from two 50-line branches to a single branch using `LIMIT i64::MAX` as sentinel when no limit is specified.

5. **`PersistedEvent` now derives `Serialize`** — required for SSE JSON serialization to clients.

6. **9 new tests** covering: no-subscriber broadcast, single subscriber recv, 50 concurrent subscribers (NF-002), replay from offset 0, partial offset replay, empty history replay, replay-then-live deduplication pattern, cross-workflow isolation, subscriber count tracking.

## Requirement satisfaction

### FR-006 (SSE endpoint with offset-based replay)
**Satisfied at library layer.** `EventBroadcaster::subscribe_with_replay(store, wf_id, offset)` provides the complete replay + live stream contract. The broadcaster pushes events to all subscribers via `tokio::sync::broadcast`. HTTP SSE framing (e.g., `text/event-stream` format) is left to the consuming HTTP server (Phase 6 integration scope).

### NF-002 (50 concurrent subscribers per workflow)
**Satisfied.** Test `multiple_subscribers_all_receive_events` verifies 50 concurrent subscribers receive the same event. `tokio::sync::broadcast` supports arbitrary subscriber counts limited only by memory.

### SC-004 (SSE client with `?offset=0` receives all historical + live events)
**Satisfied at library layer.** `subscribe_with_replay(store, wf_id, 0)` returns all historical events in `replay`, then the subscriber receives live events via `recv()`. The `high_water_mark` field enables deduplication of events that arrive on both paths. Test `replay_then_live_deduplication_pattern` validates this contract.

## Prior findings addressed

### P5-001: `PersistedEvent` not re-exported (LOW)
**Resolved** in prior commit (f3969ee).

### P5-002: Duplicated row-mapping in `load_events_since` (LOW)
**Resolved.** Unified to single branch using `LIMIT i64::MAX` sentinel.

### P5-004: `&mut self` on `append_event` (INFO)
**Addressed by architecture.** The broadcaster is a separate `Clone` object that doesn't hold the store. The intended pattern is: caller holds `Arc<Mutex<SqliteWorkflowStore>>` for writes, calls `broadcaster.broadcast()` after each append. Read-only replay in `subscribe_with_replay` takes `&SqliteWorkflowStore` (immutable borrow). WAL mode allows concurrent readers.

### P5-005: `sqlite_now_ts` not ISO-8601 (INFO)
**Unchanged.** Carries forward as P3-001. Functional for ordering.

### P5-006: Limited test coverage (LOW)
**Resolved.** 9 new SSE tests cover empty streams, cross-workflow isolation, and the deduplication pattern.

## New findings

### P5-007: No HTTP SSE framing layer (INFO)
The broadcaster provides the subscription + replay contract but does not format events as `text/event-stream`. This is intentional -- the HTTP layer belongs in Phase 6 integration where the consuming server (e.g., axum/actix) adds `Content-Type: text/event-stream`, `data:` framing, `id:` fields, and `retry:` hints. The library layer is framework-agnostic.

### P5-008: Broadcast channel lagged subscriber handling (INFO)
If a subscriber falls behind by more than `capacity` events, `tokio::sync::broadcast` returns `RecvError::Lagged(n)`. The subscriber can recover by calling `recv()` again (resumes from the next available event) but the skipped events are lost from the live path. For workflows with high event rates, consumers should handle `Lagged` by re-querying SQLite for the gap.

### P5-009: Broadcaster is per-workflow, not per-store (INFO)
The current design creates one `EventBroadcaster` per workflow. A workflow registry or factory that maps `workflow_id -> EventBroadcaster` is needed at the HTTP layer. This is Phase 6 integration scope.

## Test verification

48/48 orchestrator tests pass (9 new SSE tests + 39 existing). 0 new clippy warnings. 7 pre-existing clippy warnings from Phases 1-4 unchanged.

## Summary

Phase 5 is now complete at the library layer. The `EventBroadcaster` provides multi-subscriber broadcast with offset-based replay from SQLite, satisfying FR-006, NF-002, and SC-004. The subscribe-first-then-query pattern in `subscribe_with_replay` guarantees no events are missed. P5-002 (duplicated row-mapping) and P5-006 (limited tests) are resolved. HTTP SSE framing and the workflow broadcaster registry are Phase 6 integration scope.

**Approved items:** `EventBroadcaster`, `EventSubscriber`, `ReplaySubscription`, P5-002 fix, `PersistedEvent` Serialize derive, 9 new tests
**Phase 6 scope:** HTTP SSE endpoint with `text/event-stream` framing, workflow broadcaster registry, end-to-end crash resume verification
