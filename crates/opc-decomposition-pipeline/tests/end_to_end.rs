// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/165-opc-decomposition-pipeline/spec.md
//
// End-to-end integration test for the decomposition pipeline.
// Asserts the success criteria SC-001 (≥1 draft spec emitted),
// SC-003 (`role: decomposition-origin` present), the FR-010 degraded
// behaviour (no knowledge bundle, no git, no embeddings), and the
// run-listing helper used by the Tauri command surface.

use std::fs;

use opc_decomposition_pipeline::{
    DegradedReason, PipelineConfig, PipelineRunner, StageId, StageStatus, list_runs, load_run,
};

fn fixture_project(root: &std::path::Path) {
    let crates_a = root.join("crates").join("alpha");
    let crates_b = root.join("crates").join("beta");
    let tools = root.join("tools");
    let docs = root.join("docs");
    fs::create_dir_all(&crates_a).unwrap();
    fs::create_dir_all(&crates_b).unwrap();
    fs::create_dir_all(&tools).unwrap();
    fs::create_dir_all(&docs).unwrap();

    fs::write(
        crates_a.join("Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(crates_a.join("lib.rs"), "fn alpha_one() { alpha_two(); }\nfn alpha_two() {}\n")
        .unwrap();
    fs::write(crates_b.join("Cargo.toml"), "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2024\"\n").unwrap();
    fs::write(crates_b.join("lib.rs"), "fn beta() {}\n").unwrap();
    fs::write(tools.join("main.rs"), "fn main() {}\n").unwrap();
    fs::write(docs.join("README.md"), "# fixture project").unwrap();
}

#[test]
fn full_pipeline_emits_drafts_and_records_manifest() {
    let project = tempfile::tempdir().unwrap();
    fixture_project(project.path());

    let output_root = project.path().join(".opc").join("decomposition");
    let cfg = PipelineConfig {
        project_root: project.path().to_path_buf(),
        knowledge_bundle: None,
        output_root: output_root.clone(),
        embeddings_enabled: false,
    };
    let run = PipelineRunner::new(cfg).run().unwrap();

    // 6 stage records persisted.
    assert_eq!(run.stages.len(), 6);

    // Stage IDs in the right order.
    let ids: Vec<StageId> = run.stages.iter().map(|s| s.id).collect();
    assert_eq!(
        ids,
        vec![
            StageId::Extraction,
            StageId::Fingerprint,
            StageId::Clustering,
            StageId::CallGraph,
            StageId::Lineage,
            StageId::Synthesis,
        ]
    );

    // FR-010 degraded markers: no bundle, no git, no embeddings.
    let extraction = &run.stages[0];
    assert_eq!(extraction.status, StageStatus::Degraded);
    assert_eq!(extraction.degraded, Some(DegradedReason::NoKnowledgeBundle));

    let lineage = &run.stages[4];
    assert_eq!(lineage.status, StageStatus::Degraded);
    assert_eq!(lineage.degraded, Some(DegradedReason::NoGitHistory));

    let clustering = &run.stages[2];
    assert_eq!(clustering.status, StageStatus::Degraded);
    assert_eq!(clustering.degraded, Some(DegradedReason::NoEmbeddingsBackend));

    // SC-001: ≥ 1 draft spec emitted.
    assert!(!run.emitted_specs.is_empty());

    // SC-003: emitted spec carries role: decomposition-origin.
    let run_dir = output_root.join(run.run_id.as_str());
    let first = run_dir
        .join("s6-synthesis")
        .join(&run.emitted_specs[0].relpath);
    let body = fs::read_to_string(&first).expect("emitted spec readable");
    assert!(body.contains("role: decomposition-origin"));
    assert!(body.contains("kind: code-fingerprint"));
    assert!(body.contains("kind: capability"));
    assert!(body.contains("retroactive: true"));
    assert!(body.contains("establishes:"));
    assert!(body.contains("- unit: { kind: file, path:"));

    // Manifest round-trips through the listing helper used by Tauri.
    let runs = list_runs(&output_root).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run_id, run.run_id);

    // load_run resolves by id.
    let direct = load_run(&output_root, &run.run_id).unwrap();
    assert!(direct.is_some());
}

#[test]
fn knowledge_bundle_drives_extraction_stage_to_complete() {
    let project = tempfile::tempdir().unwrap();
    fixture_project(project.path());
    let bundle = tempfile::tempdir().unwrap();
    fs::write(bundle.path().join("notes.md"), "# Notes\n\nbody.").unwrap();
    fs::write(bundle.path().join("data.json"), r#"{"x":1}"#).unwrap();

    let output_root = project.path().join(".opc").join("decomposition");
    let cfg = PipelineConfig {
        project_root: project.path().to_path_buf(),
        knowledge_bundle: Some(bundle.path().to_path_buf()),
        output_root: output_root.clone(),
        embeddings_enabled: false,
    };
    let run = PipelineRunner::new(cfg).run().unwrap();
    let extraction = &run.stages[0];
    assert_eq!(extraction.status, StageStatus::Complete);
}
