# 052 Phase 5 Review — Append-Only Event Logging API

**Reviewer:** claude
**Date:** 2026-03-31
**Commit:** c5508ed
**Verdict:** ✅ Phase 5 data layer approved — SSE endpoint/broadcaster deferred

## Scope

Phase 5 per spec: "SSE stream server — Implement the append-only event broadcaster with offset-based replay for late-joining clients." This delivery covers the SQLite data layer (append + query) that underpins the SSE server. The HTTP SSE endpoint and multi-subscriber broadcaster remain pending.

## Files reviewed

- `crates/orchestrator/src/sqlite_state.rs` (lines 33–577 — `PersistedEvent`, `sqlite_now_ts`, `append_event`, `load_events_since`, 1 new test)
- `crates/orchestrator/src/lib.rs` (re-exports — `PersistedEvent` NOT re-exported)

## What was delivered

1. **`PersistedEvent` struct** (line 38) — `event_id: i64`, `workflow_id: Uuid`, `timestamp: String`, `event_type: String`, `payload: JsonValue`. Clean, `Clone + Debug` derived.
2. **`SqliteWorkflowStore::append_event()`** (line 428) — inserts into `events` table, returns auto-increment `event_id`. Takes optional timestamp (falls back to `sqlite_now_ts()`). Enforces FK constraint (workflow must exist).
3. **`SqliteWorkflowStore::load_events_since()`** (line 466) — loads events with `event_id > from_event_id`, ordered ASC, optional `LIMIT`. This is the offset-based replay query that SSE servers will use.
4. **1 new test** (`append_and_load_events_since_respects_offset_and_limit`) — seeds workflow, appends 3 events, verifies monotonic IDs, tests offset filtering, tests limit.

## Requirement satisfaction

### FR-006 (SSE endpoint with offset-based replay)
⚠️ **Partially satisfied.** The data layer for offset-based replay is complete: `append_event` writes to the events table, `load_events_since(workflow_id, from_event_id, limit)` provides the replay query. The HTTP SSE endpoint and broadcaster that consume this API are not yet implemented.

### SC-004 (SSE client with `?offset=0` receives all historical + live events)
⚠️ **Data layer ready, endpoint not wired.** `load_events_since(wf_id, 0, None)` returns all events — verified in test. Live streaming requires the broadcaster (pending).

### NF-002 (50 concurrent SSE subscribers)
❌ **Not addressed.** No broadcaster or connection management implemented.

## Code quality assessment

### `append_event` (lines 428–460)
Sound. Serializes payload to JSON string, inserts with parameterized query, returns `last_insert_rowid()`. The optional `timestamp` parameter is a good design — allows deterministic timestamps in tests while defaulting to wall-clock in production.

### `load_events_since` (lines 466–577)
Functionally correct. The offset-based query (`event_id > ?2`) is the right approach for SSE replay — avoids OFFSET/LIMIT pagination pitfalls. However, the limit/no-limit branches duplicate ~50 lines of identical row-mapping code (see P5-002).

### `PersistedEvent` (lines 37–44)
Clean struct. `event_id: i64` matches SQLite `INTEGER PRIMARY KEY AUTOINCREMENT`. `payload: JsonValue` parsed eagerly on load — reasonable for the expected event sizes.

### `sqlite_now_ts` (lines 48–56)
Epoch-based format (`{secs}.{millis:03}`) — carries forward P3-001 (not ISO-8601). Functional for ordering but not human-readable in the events table.

## Test verification

39/39 orchestrator tests pass. 0 new compiler warnings from Phase 5 code. 2 pre-existing clippy warnings on sqlite_state.rs (lines 73, 184) are from Phase 4.

The new test (`append_and_load_events_since_respects_offset_and_limit`) covers:
- FK constraint satisfaction (seeds workflow before appending events)
- Monotonic event ID ordering
- Full replay from offset 0
- Partial replay from a specific event ID
- Limit parameter

## Findings

### P5-001: `PersistedEvent` not re-exported from `lib.rs` (LOW)
`lib.rs:23-25` re-exports `SqliteWorkflowStore`, `sqlite_db_path_for_run`, `sqlite_db_path_for_run_dir` but not `PersistedEvent`. External crates consuming the events API must use `orchestrator::sqlite_state::PersistedEvent` directly. Should be added to the re-export block.

### P5-002: Duplicated row-mapping in `load_events_since` (LOW)
Lines 494–527 and 535–573 contain identical row-mapping logic — the only difference is whether `limit` is bound as `?3`. Could be unified by always using `LIMIT ?3` with a very large sentinel (e.g., `i64::MAX`) when no limit is specified, or by building the parameter list conditionally.

### P5-003: SSE endpoint and broadcaster not implemented (INFO)
Explicitly acknowledged in baton as pending. FR-006 data layer is ready; the HTTP/SSE server that streams events to subscribers is the remaining deliverable. This should be completed in a Phase 5 continuation or early Phase 6.

### P5-004: `append_event` requires `&mut self` (INFO)
The `&mut self` borrow means a single `SqliteWorkflowStore` can't serve concurrent append + read. For the SSE broadcaster pattern (one writer, many readers), this will need either `Arc<Mutex<SqliteWorkflowStore>>` or separate read-only connections. WAL mode supports concurrent readers, so a pool of read-only connections + one write connection would be the natural architecture.

### P5-005: `sqlite_now_ts` not ISO-8601 (INFO)
Carries forward P3-001. The `{secs}.{millis:03}` format is monotonic and correct for ordering but not human-readable when inspecting the events table directly (NF-003 spirit).

### P5-006: Limited test coverage for edge cases (LOW)
Only 1 new test. Missing coverage for: empty event stream (`load_events_since` with no events), cross-workflow isolation (events from workflow A not returned for workflow B), `append_event` with `None` timestamp (default path), and large payload handling.

## Summary

The append-only event logging API is well-implemented and provides the correct foundation for SSE replay. `append_event` + `load_events_since` with offset-based querying is the right pattern. The code is clean, correctly handles FK constraints, and the test validates the core contract. The main gap is that Phase 5's primary deliverable — the SSE endpoint and broadcaster — is not yet implemented. The data layer is approved; the SSE server should follow.

**Approved items:** `PersistedEvent`, `append_event`, `load_events_since`, `sqlite_now_ts`, events test
**Pending for FR-006 completion:** HTTP SSE endpoint, multi-subscriber broadcaster, NF-002 (50 concurrent subscribers)
