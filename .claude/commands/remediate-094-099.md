---
name: remediate-094-099
description: Quality remediation for specs 094–099 — fix 14 wiring gaps found in validation pass
allowed-tools: Bash, Read, Edit, Write, Glob, Grep, Agent
---

# Quality Remediation: Specs 094–099

A validation pass found that specs 094–099 have correct types, schemas, and APIs but **wiring between subsystems was left incomplete**. This command implements 14 fixes across 4 phases.

## Orchestrator Rules

- Execute phases in order. Do not skip ahead.
- Run `cargo check` or `cargo build` after each phase to verify compilation.
- Run `cargo test` for affected crates at each checkpoint.
- If a test fails, fix it before proceeding. Do not disable tests.
- Do not commit unless explicitly asked.

---

## Phase A — Quick Wins (6 fixes, no cross-file dependencies)

### Fix 1 — [CRITICAL] 095: `list_checkpoints` SQL missing columns

**File:** `crates/axiomregent/src/checkpoint/store.rs`

The `list_checkpoints` function has two SELECT statements that omit `branch_name` and `run_id` columns. `CheckpointInfo` has 15 fields but only 13 columns are selected, causing `branch_name` and `run_id` to always deserialize as `None`. This silently breaks all branch/run filtering in `do_list` and `do_timeline`.

The correct column list is already in `get_checkpoint` (lines 163–165):
```sql
SELECT checkpoint_id, repo_root, parent_id, label, head_sha, fingerprint,
       state_hash, merkle_root, file_count, total_bytes, created_at, metadata,
       workspace_id, branch_name, run_id
```

**Action:** In `list_checkpoints`, find both SELECT statements:

1. **Lines 186–188** (the `Some(wid)` branch) — change:
```
workspace_id \
```
to:
```
workspace_id, branch_name, run_id \
```

2. **Lines 201–203** (the `None` branch) — same change:
```
workspace_id \
```
to:
```
workspace_id, branch_name, run_id \
```

Both SELECT strings end with `workspace_id \` before the FROM clause. Append `, branch_name, run_id` after `workspace_id` in each.

---

### Fix 9 — [HIGH] 098: Missing `repo_root` silently bypasses mutation preflight

**File:** `crates/axiomregent/src/router/mod.rs`

In `run_mutation_preflight` (line 282), missing `repo_root` in tool args causes `?` on `Option` to return `None`, which means "allow the mutation." This lets any agent bypass governance preflight by simply omitting `repo_root`.

**Action:** Replace line 282:
```rust
let repo_root = args.get("repo_root").and_then(|v| v.as_str())?;
```
with:
```rust
let repo_root = match args.get("repo_root").and_then(|v| v.as_str()) {
    Some(r) => r,
    None => {
        // Mutation tools require repo_root for governance preflight (098 Slice 4).
        // Missing repo_root must not silently bypass — return error.
        return Some(json_rpc_error(
            id,
            -32602, // Invalid params
            "Mutation preflight requires 'repo_root' parameter",
        ));
    }
};
```

Note: `json_rpc_error` is already used elsewhere in this function (e.g., line 323). The `id` parameter is available from the function signature.

---

### Fix 10 — [HIGH] 099: Hiqlite `workspace_id` column missing index

**File:** `crates/orchestrator/src/hiqlite_store.rs`

The SQLite migration creates `idx_workflows_workspace` but the hiqlite migration at lines 101–110 only adds the column without an index. `list_workflows_by_workspace` on hiqlite will do full table scans.

**Action:** In `migrate()`, after the existing `ALTER TABLE` block (line 110), add:

```rust
        // Index for workspace-scoped queries (099 Slice 4).
        let _ = self
            .client
            .execute(
                Cow::Borrowed(
                    "CREATE INDEX IF NOT EXISTS idx_workflows_workspace ON workflows(workspace_id)",
                ),
                vec![],
            )
            .await;
```

This follows the same error-suppression pattern as the `ALTER TABLE` above it (`let _ =`).

---

### Fix 7 — [HIGH] 097: `CompletedLocal` serde serialization mismatch

**File:** `crates/orchestrator/src/state.rs`

`WorkflowStatus` has `#[serde(rename_all = "lowercase")]` which produces `"completedlocal"` in JSON, but the SQLite/Hiqlite stores use manual conversion functions that produce `"completed_local"`. This creates a format mismatch between `state.json` and DB columns.

**Action:** Add explicit `#[serde(rename = ...)]` on the three multi-word variants (lines 20–24):

Change:
```rust
    /// Finished locally but platform sync incomplete. Not promotion-eligible (spec 097).
    CompletedLocal,
    Failed,
    TimedOut,
    AwaitingCheckpoint,
```
to:
```rust
    /// Finished locally but platform sync incomplete. Not promotion-eligible (spec 097).
    #[serde(rename = "completed_local")]
    CompletedLocal,
    Failed,
    #[serde(rename = "timed_out")]
    TimedOut,
    #[serde(rename = "awaiting_checkpoint")]
    AwaitingCheckpoint,
```

This makes serde match the manual `workflow_status_to_str` at `sqlite_state.rs:946-954` and `hiqlite_store.rs:571-579` which already use `"completed_local"`, `"timed_out"`, `"awaiting_checkpoint"`.

---

### Fix 11 — [MEDIUM] 096: `risk` field missing from `EnrichedFeature`

**File:** `crates/featuregraph/src/enrichment.rs`

The spec declares `pub risk: String` in `EnrichedFeature`. The data IS available: `RegistryFeatureRecord.risk` flows through `FeatureEntry.governance` (scanner.rs:159) into `FeatureNode.governance` (scanner.rs:249). It just isn't propagated to `EnrichedFeature`.

**Action 1:** Add `risk` field to the `EnrichedFeature` struct (after `owner` around line 17):
```rust
    /// Spec risk level from registry (low/medium/high/critical).
    pub risk: String,
```

**Action 2:** In `enrich_one()` function, in the `EnrichedFeature` construction block (around line 97), add:
```rust
        risk: node.governance.clone(),
```
Place it after `owner: node.owner.clone(),`.

**Action 3:** Update the test helper `make_feature()` in the same file (around line 122). The `governance` field is already set to `String::new()`, which means `risk` will be empty string in tests. The test assertions don't check `risk`, so no assertion changes needed — the struct construction will compile once the field is added.

---

### Fix 12 — [MEDIUM] 095: `CheckpointCompare` TS type missing fingerprint fields

**File:** `apps/desktop/src/features/checkpoint/types.ts`

The Rust `do_compare` returns `fingerprint_a` and `fingerprint_b` in the JSON response, but the TypeScript `CheckpointCompare` interface doesn't include these fields.

**Action:** Add two fields to the `CheckpointCompare` interface, after `branch_b` (line 37):

```typescript
  /** Fingerprint of checkpoint A ({file_count, total_size, top_extensions}). */
  fingerprint_a?: Record<string, unknown> | null;
  /** Fingerprint of checkpoint B. */
  fingerprint_b?: Record<string, unknown> | null;
```

---

### Phase A Checkpoint

Run these verification commands:

```bash
# Build all affected crates
for crate in orchestrator axiomregent featuregraph; do
  echo "=== Building $crate ===" && \
  cargo build --manifest-path crates/$crate/Cargo.toml 2>&1 | tail -5
done

# Run tests
for crate in orchestrator axiomregent featuregraph; do
  echo "=== Testing $crate ===" && \
  cargo test --manifest-path crates/$crate/Cargo.toml 2>&1 | tail -10
done
```

All crates must build. The featuregraph golden test WILL still fail (fixed in Phase D). All other tests must pass.

**Stop and report results before proceeding to Phase B.**

---

## Phase B — RunSummary + Promotion (2 fixes, interconnected)

### Fix 3 — [CRITICAL] 097: Add promotion check to `dispatch_manifest`

**File:** `crates/orchestrator/src/lib.rs`

`dispatch_manifest` (non-persisted path) returns at line ~1278 without any promotion check. Only `dispatch_manifest_persisted` runs `check_promotion_eligibility` and sets `Completed` vs `CompletedLocal`. This means the non-persisted path never populates promotion metadata.

**Context:**
- `RunSummary` is at lines 118–122 — it has `run_id` and `steps` only, no metadata
- `build_summary` is at lines 929–967 — constructs RunSummary
- `dispatch_manifest` ends at line 1278 with `Ok(summary)`
- The promotion check in `dispatch_manifest_persisted` is at lines 1848–1879

**Action 1 — Extend `RunSummary` with metadata:**

At line ~118, change:
```rust
pub struct RunSummary {
    pub run_id: Uuid,
    pub steps: Vec<StepSummaryEntry>,
}
```
to:
```rust
pub struct RunSummary {
    pub run_id: Uuid,
    pub steps: Vec<StepSummaryEntry>,
    /// Workflow-level metadata: governance_mode, promotion result (097/098).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}
```

**Action 2 — Update `build_summary` to initialize metadata:**

In `build_summary` (line ~963), change:
```rust
    RunSummary {
        run_id,
        steps: summary_entries,
    }
```
to:
```rust
    RunSummary {
        run_id,
        steps: summary_entries,
        metadata: HashMap::new(),
    }
```

**Action 3 — Extract a lightweight promotion helper:**

Add a new function near the promotion check code (before or after `dispatch_manifest_persisted`):

```rust
/// Run promotion eligibility check using metadata fields + sync tracker.
/// Returns (final_status, promotion_check) without requiring a full WorkflowState.
fn check_promotion_from_metadata(
    metadata: &HashMap<String, serde_json::Value>,
    sync_tracker: &Option<promotion::SyncTracker>,
    steps_all_succeeded: bool,
) -> promotion::PromotionCheck {
    let sync_status = match sync_tracker {
        Some(tracker) => tracker.to_sync_status(),
        None => promotion::SyncStatus::default(),
    };

    let workspace_id = metadata
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let governance_mode = metadata
        .get("governance_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let mut reasons = Vec::new();
    let has_workspace = !workspace_id.is_empty();
    let governance_active = governance_mode == "governed";

    if !has_workspace {
        reasons.push("no workspace_id".to_string());
    }
    if !governance_active {
        reasons.push(format!("governance mode is '{governance_mode}', expected 'governed'"));
    }
    if !sync_status.events_synced {
        reasons.push("not all events were synced to platform".to_string());
    }
    if !sync_status.artifacts_recorded {
        reasons.push("not all artifacts were recorded".to_string());
    }
    if !steps_all_succeeded {
        reasons.push("not all steps completed successfully".to_string());
    }

    let eligibility = if reasons.is_empty() {
        promotion::PromotionEligibility::Eligible
    } else {
        promotion::PromotionEligibility::Ineligible { reasons: reasons.clone() }
    };

    promotion::PromotionCheck {
        workspace_id: has_workspace,
        governance_active,
        events_synced: sync_status.events_synced,
        artifacts_recorded: sync_status.artifacts_recorded,
        all_steps_completed: steps_all_succeeded,
        eligibility,
    }
}
```

Check the actual `PromotionCheck` struct fields in `crates/orchestrator/src/promotion.rs` first to make sure the field names match. Read the struct definition and adjust accordingly.

**Action 4 — Wire into `dispatch_manifest` before return:**

Before line ~1278 (`Ok(summary)`), add:

```rust
    // --- Promotion eligibility check (097 Slice 4, 098 Slice 2) ---
    let all_succeeded = statuses.iter().all(|s| matches!(s, StepStatus::Success));
    let mut run_metadata = HashMap::new();
    if let Some(ref gm) = options.governance_mode {
        run_metadata.insert("governance_mode".to_string(), serde_json::json!(gm));
    }
    let promo_check = check_promotion_from_metadata(&run_metadata, &options.sync_tracker, all_succeeded);
    if let Ok(v) = serde_json::to_value(&promo_check) {
        summary.metadata.insert("promotion".to_string(), v);
    }
    if let Some(ref gm) = options.governance_mode {
        summary.metadata.insert("governance_mode".to_string(), serde_json::json!(gm));
    }
```

This also resolves **Fix 8** (governance_mode carried in RunSummary metadata).

---

### Phase B Checkpoint

```bash
cargo test --manifest-path crates/orchestrator/Cargo.toml 2>&1 | tail -20
```

All orchestrator tests must pass. Check that existing tests still compile — `RunSummary` is used in several test files, so the new `metadata` field (with `#[serde(default)]`) should be backward-compatible for deserialization.

**Stop and report results before proceeding to Phase C.**

---

## Phase C — Artifact Metadata Wiring (2 fixes)

### Fix 2 — [CRITICAL] 094: Wire `ArtifactMetadataStore` into dispatch paths

**Files:**
- `crates/orchestrator/src/lib.rs`
- `crates/orchestrator/src/artifact.rs`

`ArtifactMetadataStore::record_artifact()` and `record_lineage()` are fully implemented but never called from dispatch. The store exists only in tests.

**Action 1 — Add `artifact_metadata` to `DispatchOptions`** (lib.rs around line 208):

After the `cas` field, add:
```rust
    /// Artifact metadata store for provenance recording (094 Slices 3-4).
    /// When `Some`, artifact records and lineage are persisted after CAS promotion.
    #[cfg(feature = "local-sqlite")]
    pub artifact_metadata: Option<std::sync::Arc<artifact::ArtifactMetadataStore>>,
```

The `DispatchOptions` has a manual `Default` impl or uses `#[derive(Default)]`. If it's `#[derive(Default)]`, `Option` defaults to `None` which is correct. If manual, add `artifact_metadata: None` to the impl. You need to check by reading the current `Default` implementation.

Note: the `#[cfg(feature = "local-sqlite")]` gate matches `ArtifactMetadataStore` which is also gated. The `local-sqlite` feature is in `default` features in `Cargo.toml`, so it compiles unconditionally.

**Action 2 — Add recording helper function** (lib.rs, near the `promote_to_cas` call sites):

```rust
/// Record artifact metadata and provenance lineage after CAS promotion (094 Slices 3-4).
#[cfg(feature = "local-sqlite")]
fn record_artifact_metadata(
    store: &artifact::ArtifactMetadataStore,
    output_hashes: &HashMap<String, String>,
    step: &manifest::WorkflowStep,
    run_id: Uuid,
    workspace_id: Option<&str>,
    output_paths: &[PathBuf],
) {
    let now = now_ts();
    for (filename, hash) in output_hashes {
        let size = output_paths
            .iter()
            .find(|p| p.file_name().map(|f| f.to_string_lossy().as_ref() == filename.as_str()).unwrap_or(false))
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| m.len())
            .unwrap_or(0);

        let record = artifact::ArtifactRecord {
            content_hash: hash.clone(),
            filename: filename.clone(),
            step_id: step.id.clone(),
            workflow_id: run_id.to_string(),
            workspace_id: workspace_id.map(String::from),
            created_at: now.clone(),
            size_bytes: size,
            content_type: None,
            producer_agent: Some(step.agent.clone()),
        };
        if let Err(e) = store.record_artifact(&record) {
            eprintln!("[094] Artifact metadata warning for {}: {e}", filename);
        }

        let lineage = artifact::ArtifactLineage {
            content_hash: hash.clone(),
            relationship: artifact::LineageRelation::ProducedBy,
            workflow_id: run_id.to_string(),
            step_id: step.id.clone(),
            agent_id: Some(step.agent.clone()),
            created_at: now.clone(),
        };
        if let Err(e) = store.record_lineage(&lineage) {
            eprintln!("[094] Lineage recording warning for {}: {e}", filename);
        }
    }
}
```

**Action 3 — Call from both dispatch paths:**

In `dispatch_manifest` after CAS promotion (~line 1206), add:
```rust
                    // Record artifact metadata and provenance (094 Slices 3-4).
                    #[cfg(feature = "local-sqlite")]
                    if let Some(ref meta_store) = options.artifact_metadata {
                        record_artifact_metadata(
                            meta_store,
                            &step_output_hashes[idx],
                            step,
                            run_id,
                            None, // workspace_id not directly in DispatchOptions for non-persisted
                            &output_paths,
                        );
                    }
```

In `dispatch_manifest_persisted` after CAS promotion (~line 1769), add the same block but extract `workspace_id` from `wf_state.metadata`:
```rust
                    // Record artifact metadata and provenance (094 Slices 3-4).
                    #[cfg(feature = "local-sqlite")]
                    if let Some(ref meta_store) = options.artifact_metadata {
                        let ws_id = wf_state.metadata.get("workspace_id")
                            .and_then(|v| v.as_str());
                        record_artifact_metadata(
                            meta_store,
                            &step_output_hashes[idx],
                            step,
                            run_id,
                            ws_id,
                            &output_paths,
                        );
                    }
```

**Action 4 — Add `find_by_workspace` to `ArtifactMetadataStore`** (artifact.rs):

After `find_by_workflow` (~line 383), add a new method following the exact same pattern:

```rust
    /// Look up artifacts by workspace ID.
    pub fn find_by_workspace(&self, workspace_id: &str) -> rusqlite::Result<Vec<ArtifactRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_hash, filename, step_id, workflow_id, workspace_id,
                    created_at, size_bytes, content_type, producer_agent
             FROM artifact_records WHERE workspace_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map([workspace_id], |row| {
            Ok(ArtifactRecord {
                content_hash: row.get(0)?,
                filename: row.get(1)?,
                step_id: row.get(2)?,
                workflow_id: row.get(3)?,
                workspace_id: row.get(4)?,
                created_at: row.get(5)?,
                size_bytes: row.get::<_, i64>(6)? as u64,
                content_type: row.get(7)?,
                producer_agent: row.get(8)?,
            })
        })?;
        rows.collect()
    }
```

---

### Fix 4 — [CRITICAL] 099: Wire artifact recording in factory.rs

**File:** `apps/desktop/src-tauri/src/commands/factory.rs`

`StagecraftClient::record_artifacts` (defined in `stagecraft_client.rs` around line 411) is never called from `factory.rs`. `artifacts_acked` is always 0, so `to_sync_status()` always returns `artifacts_recorded: false`, making every workflow `CompletedLocal`.

**Context:**
- The existing `sc_ingest_step_events` function (~line 300) shows the pattern for spawning async tasks with `SyncTracker`
- `StagecraftClient::record_artifacts` takes `(project_id, pipeline_id, stage_id, &[ArtifactRecord])` and returns `Result<RecordArtifactsResponse, StagecraftError>`
- The `stagecraft_client::ArtifactRecord` struct has fields: `artifact_type: String`, `content_hash: String`, `storage_path: String`, `size_bytes: u64`
- Phase 1 completion is at ~line 797 (after `sc_ingest_step_events` for "process")
- Phase 2 completion is at ~line 1032 (after `sc_ingest_step_events` for "scaffold")
- `SyncTracker` is created at ~line 691

**Action 1 — Read and understand** `sc_ingest_step_events` pattern first (around line 300–354). Note how it:
1. Clones `sc`, `project_id`, `pipeline_id`, and `sync_tracker`
2. Collects data from `summary.steps`
3. Spawns a `tokio::spawn` with the async call
4. Calls `tracker.record_events_ack(count)` on success, `tracker.record_events_fail(err)` on failure

**Action 2 — Create `sc_record_artifacts` helper function:**

Add near `sc_ingest_step_events`:

```rust
/// Record step output artifacts to Stagecraft for promotion eligibility (099).
fn sc_record_artifacts(
    sc: &StagecraftClient,
    project_id: &str,
    pipeline_id: &str,
    summary: &orchestrator::RunSummary,
    stage_id: &str,
    sync_tracker: Option<&orchestrator::promotion::SyncTracker>,
) {
    let mut artifacts = Vec::new();
    for step in &summary.steps {
        for (filename, hash) in &step.output_hashes {
            let size = step.output_artifacts.iter()
                .find(|p| p.file_name().map(|f| f.to_string_lossy() == filename.as_str()).unwrap_or(false))
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len())
                .unwrap_or(0);
            let ext = std::path::Path::new(filename)
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            artifacts.push(crate::commands::stagecraft_client::ArtifactRecord {
                artifact_type: ext,
                content_hash: hash.clone(),
                storage_path: filename.clone(),
                size_bytes: size,
            });
        }
    }
    if artifacts.is_empty() {
        // No artifacts to record — still count as ack so sync succeeds
        if let Some(t) = sync_tracker {
            t.record_artifacts_ack(0);
        }
        return;
    }

    let sc = sc.clone();
    let pid = project_id.to_string();
    let plid = pipeline_id.to_string();
    let sid = stage_id.to_string();
    let tracker = sync_tracker.cloned();
    let count = artifacts.len() as u32;

    tokio::spawn(async move {
        match sc.record_artifacts(&pid, &plid, &sid, &artifacts).await {
            Ok(_) => {
                if let Some(ref t) = tracker {
                    t.record_artifacts_ack(count);
                }
            }
            Err(e) => {
                log::warn!("Stagecraft artifact recording failed: {e}");
                if let Some(ref t) = tracker {
                    t.record_artifacts_fail(e.to_string());
                }
            }
        }
    });
}
```

Verify the `StagecraftClient::record_artifacts` signature and `ArtifactRecord` type by reading `stagecraft_client.rs` first. Adjust field names and paths as needed.

**Action 3 — Call after Phase 1 event ingestion** (~line 797):

After `sc_ingest_step_events(&sc, &pid, &plid, &summary1, "process", Some(&sync_tracker));`, add:
```rust
            sc_record_artifacts(&sc, &pid, &plid, &summary1, "process", Some(&sync_tracker));
```

**Action 4 — Call after Phase 2 event ingestion** (~line 1032):

After `sc_ingest_step_events(&sc, &pid, &plid, &summary2, "scaffold", Some(&sync_tracker));`, add:
```rust
            sc_record_artifacts(&sc, &pid, &plid, &summary2, "scaffold", Some(&sync_tracker));
```

---

### Phase C Checkpoint

```bash
# Build orchestrator (verifies lib.rs + artifact.rs changes)
cargo build --manifest-path crates/orchestrator/Cargo.toml 2>&1 | tail -5

# Build desktop app (verifies factory.rs changes)
cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml 2>&1 | tail -5

# Run orchestrator tests
cargo test --manifest-path crates/orchestrator/Cargo.toml 2>&1 | tail -20
```

**Stop and report results before proceeding to Phase D.**

---

## Phase D — Gate Binding + Golden Test (2 fixes)

### Fix 5 — [CRITICAL] 095: Implement approval-checkpoint binding

**File:** `crates/orchestrator/src/gates.rs`

`StepGateConfig::Approval` has `checkpoint_id: Option<String>` (manifest.rs:87–90) but the gate executor destructures it with a wildcard `checkpoint_id: _` at gates.rs:112, ignoring it entirely.

**Action:** Read `gates.rs` fully first. Then in the `Approval` arm of `evaluate_gate` (~line 109):

1. Replace `checkpoint_id: _` with `checkpoint_id`:
```rust
StepGateConfig::Approval {
    timeout_ms: timeout_ms_val,
    escalation,
    checkpoint_id,
}
```

2. Before calling `handler.await_approval()`, auto-generate a checkpoint_id if not already set:
```rust
let bound_checkpoint_id = checkpoint_id.clone().unwrap_or_else(|| {
    format!("gate-{}-{}", step_id, chrono_now_iso())
});
```

3. Include checkpoint_id in the step output metadata. The exact mechanism depends on how the gate communicates state back to the caller. Read the gate evaluation flow to determine where to record this.

At minimum, the checkpoint_id should appear in the event emitted to the persistence layer. Find where `persist_and_emit` or similar is called after gate evaluation in the dispatch loop, and include `"checkpoint_id": bound_checkpoint_id` in the event JSON.

**Note:** The orchestrator does not have access to axiomregent's checkpoint provider. The actual checkpoint creation happens at the Tauri layer when it receives the gate event. Our job here is to generate the ID and include it in the event payload so the desktop handler can bind it.

---

### Fix 14 — Featuregraph golden test regeneration

The golden snapshot test fails because new specs were added since the golden file was last updated, plus Fix 11 added the `risk` field to `EnrichedFeature`.

**Action:** Regenerate the golden file:
```bash
cd /Users/bart/Dev2/open-agentic-platform && \
  UPDATE_GOLDEN=1 cargo test test_golden_graph --manifest-path crates/featuregraph/Cargo.toml
```

Then verify it passes without the env var:
```bash
cargo test test_golden_graph --manifest-path crates/featuregraph/Cargo.toml
```

---

### Phase D Checkpoint

```bash
# Run all affected crate tests
for crate in orchestrator axiomregent featuregraph agent policy-kernel factory-engine; do
  echo "=== Testing $crate ===" && \
  cargo test --manifest-path crates/$crate/Cargo.toml 2>&1 | tail -10
done
```

ALL tests must pass.

---

## Final Verification

Run the full verification suite:

```bash
# 1. All crates build
for crate in orchestrator axiomregent featuregraph agent policy-kernel factory-engine; do
  echo "=== Building $crate ===" && \
  cargo build --manifest-path crates/$crate/Cargo.toml 2>&1 | tail -3
done

# 2. All tests pass
for crate in orchestrator axiomregent featuregraph agent policy-kernel factory-engine; do
  echo "=== Testing $crate ===" && \
  cargo test --manifest-path crates/$crate/Cargo.toml 2>&1 | tail -5
done

# 3. Desktop app builds
cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml 2>&1 | tail -5

# 4. Clippy clean on key crates
for crate in orchestrator axiomregent featuregraph; do
  echo "=== Clippy $crate ===" && \
  cargo clippy --manifest-path crates/$crate/Cargo.toml -- -D warnings 2>&1 | tail -5
done
```

Report the full results as a summary table:

```
## Remediation Complete

| Fix | Spec | Description | Status |
|-----|------|-------------|--------|
| 1   | 095  | list_checkpoints SQL columns | ... |
| 2   | 094  | ArtifactMetadataStore wiring | ... |
| 3   | 097  | dispatch_manifest promotion | ... |
| 4   | 099  | factory.rs artifact recording | ... |
| 5   | 095  | Approval-checkpoint binding | ... |
| 7   | 097  | CompletedLocal serde rename | ... |
| 9   | 098  | repo_root preflight bypass | ... |
| 10  | 099  | Hiqlite workspace_id index | ... |
| 11  | 096  | EnrichedFeature risk field | ... |
| 12  | 095  | TS CheckpointCompare types | ... |
| 14  | ---  | Golden test regeneration | ... |
```

## Key Files Reference

| File | What's there |
|------|-------------|
| `crates/orchestrator/src/lib.rs` | DispatchOptions (~208), RunSummary (~118), build_summary (~929), dispatch_manifest (~972), dispatch_manifest_persisted (~1306), promotion check (~1848) |
| `crates/orchestrator/src/artifact.rs` | ArtifactRecord (~238), ArtifactLineage (~252), ArtifactMetadataStore (~278), record_artifact (~320), record_lineage (~342), find_by_workflow (~383) |
| `crates/orchestrator/src/promotion.rs` | SyncTracker (~34), SyncStatus (~19), PromotionCheck, check_promotion_eligibility (~126) |
| `crates/orchestrator/src/state.rs` | WorkflowStatus (~14), WorkflowStateSummary (~72) |
| `crates/orchestrator/src/gates.rs` | StepGateConfig::Approval (~109), evaluate_gate |
| `crates/orchestrator/src/hiqlite_store.rs` | migrate() (~91) |
| `crates/axiomregent/src/checkpoint/store.rs` | get_checkpoint (~160), list_checkpoints (~177) |
| `crates/axiomregent/src/router/mod.rs` | run_mutation_preflight (~268) |
| `crates/featuregraph/src/enrichment.rs` | EnrichedFeature (~12), enrich_one (~54) |
| `apps/desktop/src/features/checkpoint/types.ts` | CheckpointCompare (~23) |
| `apps/desktop/src-tauri/src/commands/factory.rs` | sc_ingest_step_events (~300), Phase 1 completion (~797), Phase 2 completion (~1032), SyncTracker creation (~691) |
