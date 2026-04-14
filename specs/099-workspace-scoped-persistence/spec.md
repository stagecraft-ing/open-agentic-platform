---
id: "099-workspace-scoped-persistence"
title: "Workspace-Scoped Persistence"
status: approved
implementation: pending
owner: bart
created: "2026-04-11"
risk: medium
depends_on:
  - "092"
  - "094"
  - "097"
code_aliases: ["WORKSPACE_SCOPED_PERSISTENCE"]
summary: >
  Make persistence workspace-native and sync-trackable. Replace fire-and-forget
  Stagecraft calls with tracked acknowledgement, update SyncStatus in the
  promotion check, and add workspace_id columns to workflow persistence stores.
---

# 099 — Workspace-Scoped Persistence

Parent plan: [089 Governed Convergence](../089-governed-convergence-plan/plan.md) — Phase 5

## Problem

Two persistence subsystems fail to serve the governed execution model:

1. **SyncStatus is always default.** `lib.rs` line 1804 hardcodes `SyncStatus::default()` (all
   `false`) with a `TODO(097)` comment. All Stagecraft calls in `factory.rs` use `tokio::spawn`
   fire-and-forget — the `EventIngestionResponse { ingested }` and
   `RecordArtifactsResponse { recorded }` return values are discarded inside spawned tasks.
   Consequently, all workflows complete as `CompletedLocal` regardless of actual sync state.

2. **Workflow stores lack workspace_id.** The `workflows` table in both `sqlite_state.rs` and
   `hiqlite_store.rs` has no `workspace_id` column. Workspace_id lives only in the
   `WorkflowState.metadata` JSON bag, making it impossible to efficiently query "all runs for
   workspace X" at the storage layer. The artifact store at `artifact.rs` already has
   `workspace_id TEXT` + index, proving the pattern works.

## Solution

Replace fire-and-forget with tracked acknowledgement, plumb real SyncStatus into the promotion
check, and add workspace_id as a first-class column in both persistence stores.

### Slice 1: SyncTracker struct + factory integration

Add `SyncTracker` to `crates/orchestrator/src/promotion.rs`:

```rust
#[derive(Debug, Clone, Default)]
pub struct SyncTracker {
    inner: Arc<Mutex<SyncTrackerInner>>,
}

#[derive(Debug, Default)]
struct SyncTrackerInner {
    events_acked: u32,
    events_failed: u32,
    artifacts_acked: u32,
    artifacts_failed: u32,
    last_error: Option<String>,
}
```

Methods: `record_events_ack(count)`, `record_events_fail(err)`, `record_artifacts_ack(count)`,
`record_artifacts_fail(err)`, `to_sync_status() -> SyncStatus`.

In `factory.rs`, modify fire-and-forget calls to capture the `SyncTracker` reference inside
spawned tasks. On success: `tracker.record_events_ack(resp.ingested)`. On failure:
`tracker.record_events_fail(e.to_string())`.

**Files**: `crates/orchestrator/src/promotion.rs`, `apps/desktop/src-tauri/src/commands/factory.rs`

### Slice 2: SyncStatus plumbing into promotion

Add `pub sync_tracker: Option<SyncTracker>` to `DispatchOptions`.

Replace `lib.rs` line 1804:
```rust
let sync_status = match &options.sync_tracker {
    Some(tracker) => tracker.to_sync_status(),
    None => promotion::SyncStatus::default(),
};
```

In `factory.rs`, pass the per-run `SyncTracker` through `DispatchOptions.sync_tracker`.

**Files**: `crates/orchestrator/src/lib.rs`, `apps/desktop/src-tauri/src/commands/factory.rs`

### Slice 3: workspace_id column in SQLite store

Follow the proven `PRAGMA table_info` migration pattern from `sqlite_state.rs` lines 130-156:

```rust
let has_ws_id: bool = /* PRAGMA table_info(workflows) check */;
if !has_ws_id {
    conn.execute_batch(
        "ALTER TABLE workflows ADD COLUMN workspace_id TEXT;
         CREATE INDEX IF NOT EXISTS idx_workflows_workspace ON workflows(workspace_id);"
    );
}
```

In `write_workflow_state_sync`, extract `workspace_id` from `state.metadata["workspace_id"]`
and populate the column. Update INSERT/UPSERT SQL to include `workspace_id`.

**Files**: `crates/orchestrator/src/sqlite_state.rs`

### Slice 4: workspace_id column in Hiqlite store

Apply idempotent `ALTER TABLE` in `migrate()` (following pattern from
`axiomregent/src/db/mod.rs:139`):

```rust
let _ = self.client.execute(
    Cow::Borrowed("ALTER TABLE workflows ADD COLUMN workspace_id TEXT"),
    vec![],
).await;
```

Update `write_workflow_state` INSERT SQL to include `workspace_id`.

**Files**: `crates/orchestrator/src/hiqlite_store.rs`

### Slice 5: list_workflows_by_workspace query + Tauri command

Add query method to both stores:
```rust
async fn list_workflows_by_workspace(
    &self, workspace_id: &str, limit: Option<u32>,
) -> Result<Vec<WorkflowStateSummary>, OrchestratorError>;
```

Add `WorkflowStateSummary` struct (workflow_id, name, status, started_at, workspace_id) to
avoid loading all steps per workflow.

Add Tauri command `list_workspace_workflows` and register in `lib.rs` invoke_handler.

**Files**: `crates/orchestrator/src/sqlite_state.rs`, `crates/orchestrator/src/hiqlite_store.rs`,
`apps/desktop/src-tauri/src/commands/orchestrator.rs`, `apps/desktop/src-tauri/src/lib.rs`

## Acceptance Criteria

- **SC-099-1**: `SyncTracker` collects event ingestion and artifact recording acknowledgements from Stagecraft responses
- **SC-099-2**: `SyncStatus` passed to `check_promotion_eligibility` reflects actual Stagecraft sync state, not hardcoded defaults
- **SC-099-3**: Workflows that successfully sync events and artifacts are marked `Completed` (not `CompletedLocal`)
- **SC-099-4**: `workflows` table in SQLite store has `workspace_id TEXT` column with index, populated on write
- **SC-099-5**: `workflows` table in Hiqlite store has `workspace_id TEXT` column, populated on write
- **SC-099-6**: `list_workflows_by_workspace()` returns workflows filtered by workspace_id
