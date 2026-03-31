// SPDX-License-Identifier: AGPL-3.0-or-later
// Feature 052 Phase 6: Cross-module integration tests (crash resume + SSE replay).

use orchestrator::{
    sqlite_db_path_for_run, ArtifactManager, EventBroadcaster, SqliteWorkflowStore,
    WorkflowManifest, WorkflowStep,
};
use orchestrator::{
    compute_resume_plan_from_state, state_file_path_for_run, write_workflow_state_atomic,
    StepExecutionStatus, WorkflowState,
};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Integration-style verification that a workflow which has partially
/// completed can be resumed from the last completed step using the JSON
/// state backend.
#[test]
fn integration_052_crash_resume_from_state_file() {
    let tmp = tempfile::tempdir().unwrap();
    let artifact_base = ArtifactManager::new(tmp.path());
    let wf_id = Uuid::new_v4();

    // Three-step manifest with linear dependencies.
    let manifest = WorkflowManifest {
        steps: vec![
            WorkflowStep {
                id: "step-1".into(),
                agent: "agent-a".into(),
                effort: orchestrator::EffortLevel::Quick,
                inputs: vec![],
                outputs: vec!["out1.md".into()],
                instruction: "do 1".into(),
                gate: None,
            },
            WorkflowStep {
                id: "step-2".into(),
                agent: "agent-b".into(),
                effort: orchestrator::EffortLevel::Quick,
                inputs: vec!["step-1/out1.md".into()],
                outputs: vec!["out2.md".into()],
                instruction: "do 2".into(),
                gate: None,
            },
            WorkflowStep {
                id: "step-3".into(),
                agent: "agent-c".into(),
                effort: orchestrator::EffortLevel::Quick,
                inputs: vec!["step-2/out2.md".into()],
                outputs: vec!["out3.md".into()],
                instruction: "do 3".into(),
                gate: None,
            },
        ],
    };

    // Simulate a run where the first two steps have completed and a crash occurs
    // before the third step executes.
    let mut meta = serde_json::Map::new();
    meta.insert("branch".to_string(), JsonValue::from("main"));
    let mut state = WorkflowState::new(
        wf_id,
        "integration-052",
        "2026-03-31T10:00:00Z".to_string(),
        manifest
            .steps
            .iter()
            .map(|s| (s.id.clone(), s.instruction.clone())),
        meta,
    );
    state.mark_step_started("step-1", "2026-03-31T10:00:01Z".to_string());
    state.mark_step_finished(
        "step-1",
        StepExecutionStatus::Completed,
        "2026-03-31T10:00:05Z".to_string(),
        Some(4000),
        None,
    );
    state.mark_step_started("step-2", "2026-03-31T10:00:06Z".to_string());
    state.mark_step_finished(
        "step-2",
        StepExecutionStatus::Completed,
        "2026-03-31T10:00:10Z".to_string(),
        Some(4000),
        None,
    );

    let path = state_file_path_for_run(&artifact_base, wf_id);
    write_workflow_state_atomic(&path, &state).unwrap();

    // "Crash": drop in-memory state, then reload only from JSON state file and recompute plan.
    let loaded = orchestrator::load_workflow_state(&path).unwrap();
    let plan = compute_resume_plan_from_state(&loaded, &manifest).expect("expected resume plan");

    assert_eq!(
        plan.completed_step_ids,
        vec!["step-1".to_string(), "step-2".to_string()]
    );
    assert_eq!(plan.first_non_completed_step_index, 2);
}

/// Integration-style verification that events written through the SQLite
/// backend can be replayed via the EventBroadcaster using the same workflow id.
#[test]
fn integration_052_sse_replay_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let artifact_base = ArtifactManager::new(tmp.path());
    let wf_id = Uuid::new_v4();

    // Seed a minimal workflow row so events satisfy the FK constraint.
    let state = WorkflowState::new(
        wf_id,
        "events-integration",
        "2026-03-31T00:00:00Z".to_string(),
        Vec::<(String, String)>::new(),
        serde_json::Map::new(),
    );
    let db_path = sqlite_db_path_for_run(&artifact_base, wf_id);
    let mut store = SqliteWorkflowStore::open(&db_path).expect("open sqlite store");
    store
        .write_workflow_state(&state)
        .expect("write initial workflow state");

    // Append a few events via the SQLite store.
    let e1 = store
        .append_event(
            wf_id,
            "step_started",
            &JsonValue::from("step-1"),
            Some("t1".to_string()),
        )
        .expect("append e1");
    let _e2 = store
        .append_event(
            wf_id,
            "step_completed",
            &JsonValue::from("step-1"),
            Some("t2".to_string()),
        )
        .expect("append e2");
    let e3 = store
        .append_event(
            wf_id,
            "step_started",
            &JsonValue::from("step-2"),
            Some("t3".to_string()),
        )
        .expect("append e3");

    assert!(e1 < e3);

    // Subscribe with replay from offset 0 and assert we see the full history.
    let broadcaster = EventBroadcaster::new();
    let sub = broadcaster
        .subscribe_with_replay(&store, wf_id, 0)
        .expect("subscribe with replay");

    assert_eq!(sub.replay.len(), 3);
    assert_eq!(sub.replay[0].event_type, "step_started");
    assert_eq!(sub.replay[0].event_id, e1);
    assert_eq!(sub.replay[2].event_id, e3);
    assert_eq!(sub.high_water_mark, e3);
}

