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
  "schemaVersion": "1.2.0",
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
    let out = Command::new(cli_bin())
        .args(args)
        .env_remove("GITHUB_PR_BODY")
        .output()
        .unwrap();
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

// ── Spec 133 — amends-aware coupling ────────────────────────────────────────

/// T010 / spec 133 AC-1. A diff containing the amended spec's `spec.md`
/// (specs/A/spec.md) plus the amender's `spec.md` (specs/B/spec.md, with
/// `amends: ["A"]`) clears the path even when no `implements:` claimant of
/// A is in the diff.
#[test]
fn amends_link_clears_amended_spec_path() {
    let dir = tempfile::tempdir().unwrap();
    // Spec C claims `specs/200-amended-fixture` via `implements:` (this
    // mirrors how spec 119 claims `specs/000-bootstrap-spec-system`).
    // Spec B amends spec 200 via the spec 119 protocol.
    let mappings = r#"[
        {
            "specId": "200-amended-fixture",
            "specStatus": "approved",
            "dependsOn": [],
            "amends": [],
            "implementingPaths": []
        },
        {
            "specId": "201-amender",
            "specStatus": "approved",
            "dependsOn": [],
            "amends": ["200-amended-fixture"],
            "implementingPaths": []
        },
        {
            "specId": "202-implements-claimant",
            "specStatus": "approved",
            "dependsOn": [],
            "amends": [],
            "implementingPaths": [
                {"path": "specs/200-amended-fixture", "source": "spec-implements"}
            ]
        }
    ]"#;
    let index_path = write_synthetic_index(dir.path(), mappings);

    let paths_file = dir.path().join("paths.txt");
    fs::write(
        &paths_file,
        "specs/200-amended-fixture/spec.md\nspecs/201-amender/spec.md\n",
    )
    .unwrap();

    let (code, stdout, stderr) = run(&[
        "--index",
        index_path.to_str().unwrap(),
        "--paths-from",
        paths_file.to_str().unwrap(),
    ]);
    assert_eq!(
        code, 0,
        "amender's spec.md edit must clear the amended spec's path; stdout={stdout} stderr={stderr}"
    );
}

/// T011 / spec 133 AC-3. A diff containing the amended spec's `spec.md`
/// (with `amendmentRecord: "D"` in its mapping) plus `specs/D/spec.md`
/// clears the path via the amendment_record back-link, even when D's own
/// frontmatter does not yet carry `amends: ["A"]`.
#[test]
fn amendment_record_clears_amended_spec_path() {
    let dir = tempfile::tempdir().unwrap();
    let mappings = r#"[
        {
            "specId": "200-amended-fixture",
            "specStatus": "approved",
            "dependsOn": [],
            "amends": [],
            "amendmentRecord": "203-amendment-target",
            "implementingPaths": []
        },
        {
            "specId": "203-amendment-target",
            "specStatus": "approved",
            "dependsOn": [],
            "amends": [],
            "implementingPaths": []
        },
        {
            "specId": "202-implements-claimant",
            "specStatus": "approved",
            "dependsOn": [],
            "amends": [],
            "implementingPaths": [
                {"path": "specs/200-amended-fixture", "source": "spec-implements"}
            ]
        }
    ]"#;
    let index_path = write_synthetic_index(dir.path(), mappings);

    let paths_file = dir.path().join("paths.txt");
    fs::write(
        &paths_file,
        "specs/200-amended-fixture/spec.md\nspecs/203-amendment-target/spec.md\n",
    )
    .unwrap();

    let (code, stdout, stderr) = run(&[
        "--index",
        index_path.to_str().unwrap(),
        "--paths-from",
        paths_file.to_str().unwrap(),
    ]);
    assert_eq!(
        code, 0,
        "amendmentRecord target's spec.md edit must clear the amended path; stdout={stdout} stderr={stderr}"
    );
}

/// T012 / spec 133 AC-4. A diff with no `specs/*/spec.md` paths is
/// unaffected — the new amend-aware resolver paths only fire when the
/// path being checked is itself a `specs/X/spec.md` path. A code-only
/// diff that already passes today (spec 130 path) continues to pass with
/// the amend extension active, and an amender's `spec.md` edit alone
/// does NOT silently clear an unrelated crate path.
#[test]
fn non_spec_path_unaffected_by_amends_extension() {
    let dir = tempfile::tempdir().unwrap();
    // Spec 044 claims crates/orchestrator. Spec 200 amends 044 (noise:
    // amends links should not project onto crate paths).
    let mappings = r#"[
        {
            "specId": "044-multi-agent-orchestration",
            "specStatus": "approved",
            "dependsOn": [],
            "amends": [],
            "implementingPaths": [
                {"path": "crates/orchestrator", "source": "spec-implements"}
            ]
        },
        {
            "specId": "200-amender-noise",
            "specStatus": "approved",
            "dependsOn": [],
            "amends": ["044-multi-agent-orchestration"],
            "implementingPaths": []
        }
    ]"#;
    let index_path = write_synthetic_index(dir.path(), mappings);

    // Case A: diff edits the crate file plus the amender's spec.md only —
    // the amender's edit must NOT cover the crate path; the gate must
    // still demand spec 044's spec.md (or any other implements claimant).
    let paths_file = dir.path().join("paths_a.txt");
    fs::write(
        &paths_file,
        "crates/orchestrator/src/lib.rs\nspecs/200-amender-noise/spec.md\n",
    )
    .unwrap();
    let (code, stdout, stderr) = run(&[
        "--index",
        index_path.to_str().unwrap(),
        "--paths-from",
        paths_file.to_str().unwrap(),
    ]);
    assert_eq!(
        code, 1,
        "amender spec.md edit must NOT cover an unrelated crate path; stdout={stdout} stderr={stderr}"
    );
    assert!(stderr.contains("044-multi-agent-orchestration"));

    // Case B: same crate file diff plus the implements claimant's spec.md —
    // continues to pass identically to today (spec 130 heuristic).
    let paths_file = dir.path().join("paths_b.txt");
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
    assert_eq!(
        code, 0,
        "implements-claimant edit must continue to clear crate paths; stdout={stdout} stderr={stderr}"
    );
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
