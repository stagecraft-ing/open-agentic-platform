//! Integration tests for spec-code-coupling-check (spec 127 FR-007).
//!
//! These tests build a synthetic codebase-index JSON file on disk and feed
//! both the diff path list and the PR body through CLI flags, exercising the
//! same code paths the CI workflow runs.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn cli_bin() -> PathBuf {
    // CARGO_BIN_EXE_<name> points at the just-built binary.
    PathBuf::from(env!("CARGO_BIN_EXE_spec-code-coupling-check"))
}

fn write_synthetic_index(dir: &std::path::Path, mappings_json: &str) -> PathBuf {
    let path = dir.join("index.json");
    let body = format!(
        r#"{{
  "schemaVersion": "1.1.0",
  "build": {{
    "indexerId": "codebase-indexer",
    "indexerVersion": "test",
    "repoRoot": ".",
    "contentHash": "test"
  }},
  "inventory": [],
  "traceability": {{
    "mappings": {mappings_json},
    "orphanedSpecs": [],
    "untracedCode": []
  }},
  "factory": [],
  "infrastructure": {{
    "tools": [], "agents": [], "commands": [], "rules": [], "schemas": []
  }},
  "diagnostics": {{ "warnings": [], "errors": [] }}
}}"#
    );
    fs::write(&path, body).unwrap();
    path
}

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(cli_bin()).args(args).output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[test]
fn ac1_violation_block_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let mappings = r#"[{
        "specId": "044-multi-agent-orchestration",
        "specStatus": "approved",
        "dependsOn": [],
        "implementingPaths": [{"path": "crates/orchestrator", "source": "spec-implements"}]
    }]"#;
    let index_path = write_synthetic_index(dir.path(), mappings);

    let paths_file = dir.path().join("paths.txt");
    fs::write(&paths_file, "crates/orchestrator/src/lib.rs\n").unwrap();

    let (code, _stdout, stderr) = run(&[
        "--index",
        index_path.to_str().unwrap(),
        "--paths-from",
        paths_file.to_str().unwrap(),
    ]);
    assert_eq!(code, 1, "expected exit 1; stderr=\n{stderr}");
    assert!(stderr.contains("044-multi-agent-orchestration"));
    assert!(stderr.contains("crates/orchestrator/src/lib.rs"));
    assert!(stderr.contains("Spec-Drift-Waiver"));
}

#[test]
fn ac2_spec_edit_clears_violation_exit_0() {
    let dir = tempfile::tempdir().unwrap();
    let mappings = r#"[{
        "specId": "044-multi-agent-orchestration",
        "specStatus": "approved",
        "dependsOn": [],
        "implementingPaths": [{"path": "crates/orchestrator", "source": "spec-implements"}]
    }]"#;
    let index_path = write_synthetic_index(dir.path(), mappings);

    let paths_file = dir.path().join("paths.txt");
    fs::write(
        &paths_file,
        "crates/orchestrator/src/lib.rs\nspecs/044-multi-agent-orchestration/spec.md\n",
    )
    .unwrap();

    let (code, stdout, stderr) = run(&[
        "--index",
        index_path.to_str().unwrap(),
        "--paths-from",
        paths_file.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "expected exit 0; stderr=\n{stderr}");
    assert!(stdout.contains("OK") || stdout.is_empty(), "stdout was: {stdout}");
}

#[test]
fn ac3_bypass_paths_silent_exit_0() {
    let dir = tempfile::tempdir().unwrap();
    let mappings = r#"[]"#;
    let index_path = write_synthetic_index(dir.path(), mappings);

    let paths_file = dir.path().join("paths.txt");
    fs::write(
        &paths_file,
        ".github/workflows/ci.yml\ndocs/ARCHITECTURE.md\nREADME.md\n",
    )
    .unwrap();

    let (code, _stdout, stderr) = run(&[
        "--index",
        index_path.to_str().unwrap(),
        "--paths-from",
        paths_file.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr=\n{stderr}");
}

#[test]
fn ac4_waiver_in_body_suppresses_failure() {
    let dir = tempfile::tempdir().unwrap();
    let mappings = r#"[{
        "specId": "044-multi-agent-orchestration",
        "specStatus": "approved",
        "dependsOn": [],
        "implementingPaths": [{"path": "crates/orchestrator", "source": "spec-implements"}]
    }]"#;
    let index_path = write_synthetic_index(dir.path(), mappings);

    let paths_file = dir.path().join("paths.txt");
    fs::write(&paths_file, "crates/orchestrator/src/lib.rs\n").unwrap();

    let pr_body = dir.path().join("body.txt");
    fs::write(
        &pr_body,
        "title\n\nSpec-Drift-Waiver: emergency hotfix per OPS-123\n",
    )
    .unwrap();

    let (code, stdout, stderr) = run(&[
        "--index",
        index_path.to_str().unwrap(),
        "--paths-from",
        paths_file.to_str().unwrap(),
        "--pr-body",
        pr_body.to_str().unwrap(),
    ]);
    let combined = format!("{stdout}\n{stderr}");
    assert_eq!(code, 0, "stderr=\n{stderr}");
    assert!(combined.contains("::warning::"));
    assert!(combined.contains("emergency hotfix per OPS-123"));
}
