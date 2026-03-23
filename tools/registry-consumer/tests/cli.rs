//! Integration tests: exit codes 0 / 1 / 3 per specs/002-registry-consumer-mvp.

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn registry_consumer_exe() -> PathBuf {
    if let Some(e) = std::env::var_os("CARGO_BIN_EXE_registry_consumer") {
        return PathBuf::from(e);
    }
    #[cfg(windows)]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/registry-consumer.exe")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/registry-consumer")
    }
}

fn write_registry(path: &Path, v: &serde_json::Value) {
    fs::write(path, serde_json::to_vec_pretty(v).unwrap()).unwrap();
}

fn fixture_registry_ok() -> serde_json::Value {
    json!({
        "specVersion": "1.0.0",
        "build": {
            "compilerId": "test",
            "compilerVersion": "0.1.0",
            "inputRoot": ".",
            "contentHash": "0000000000000000000000000000000000000000000000000000000000000000"
        },
        "features": [
            {
                "id": "002-b",
                "title": "Second",
                "status": "draft",
                "created": "2026-03-22",
                "summary": "sum",
                "specPath": "specs/002-b/spec.md",
                "sectionHeadings": ["H"]
            },
            {
                "id": "001-a",
                "title": "First",
                "status": "active",
                "created": "2026-03-22",
                "summary": "sum",
                "specPath": "specs/001-a/spec.md",
                "sectionHeadings": ["H"]
            }
        ],
        "validation": { "passed": true, "violations": [] }
    })
}

fn fixture_registry_statuses() -> serde_json::Value {
    json!({
        "specVersion": "1.0.0",
        "build": {
            "compilerId": "test",
            "compilerVersion": "0.1.0",
            "inputRoot": ".",
            "contentHash": "0000000000000000000000000000000000000000000000000000000000000000"
        },
        "features": [
            { "id": "004-d", "title": "D", "status": "draft", "created": "2026-03-22", "summary": "s", "specPath": "specs/004-d/spec.md", "sectionHeadings": ["H"] },
            { "id": "001-a", "title": "A", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/001-a/spec.md", "sectionHeadings": ["H"] },
            { "id": "003-c", "title": "C", "status": "superseded", "created": "2026-03-22", "summary": "s", "specPath": "specs/003-c/spec.md", "sectionHeadings": ["H"] },
            { "id": "002-b", "title": "B", "status": "retired", "created": "2026-03-22", "summary": "s", "specPath": "specs/002-b/spec.md", "sectionHeadings": ["H"] }
        ],
        "validation": { "passed": true, "violations": [] }
    })
}

#[test]
fn exit_code_zero_list_and_show() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("list")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0), "stderr={}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("001-a"));
    assert!(stdout.contains("002-b"));
    let pos_001 = stdout.find("001-a").unwrap();
    let pos_002 = stdout.find("002-b").unwrap();
    assert!(
        pos_001 < pos_002,
        "list must be sorted by id lexicographically"
    );

    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let shown: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("show emits JSON");
    assert_eq!(shown["id"], "001-a");
    assert_eq!(shown["title"], "First");
}

#[test]
fn list_filter_status_and_id_prefix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--status", "draft"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("002-b"));
    assert!(
        !stdout.contains("001-a"),
        "draft filter must exclude 001-a row"
    );

    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--id-prefix", "001"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("001-a"));
    assert!(!stdout.contains("002-b"));
}

#[test]
fn exit_code_one_validation_failed_without_allow_invalid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    let mut v = fixture_registry_ok();
    v["validation"]["passed"] = json!(false);
    v["validation"]["violations"] = json!([{
        "code": "V-001",
        "severity": "error",
        "message": "test"
    }]);
    write_registry(&reg, &v);

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("list")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn allow_invalid_permits_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    let mut v = fixture_registry_ok();
    v["validation"]["passed"] = json!(false);
    v["validation"]["violations"] = json!([]);
    write_registry(&reg, &v);

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("--allow-invalid")
        .arg("list")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn exit_code_one_show_not_found() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "999-nope"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn exit_code_three_missing_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("does-not-exist.json");

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("list")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(3));
}

#[test]
fn exit_code_three_invalid_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    fs::write(&reg, "not json {{{").unwrap();

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("list")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(3));
}

#[test]
fn exit_code_three_missing_features_array() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    let v = json!({
        "specVersion": "1.0.0",
        "build": {
            "compilerId": "t",
            "compilerVersion": "0.1.0",
            "inputRoot": ".",
            "contentHash": "0000000000000000000000000000000000000000000000000000000000000000"
        },
        "validation": { "passed": true, "violations": [] }
    });
    write_registry(&reg, &v);

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("list")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(3));
}

#[test]
fn status_report_has_deterministic_status_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_statuses());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("status-report")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));

    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    let status_lines: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| {
            line.starts_with("draft")
                || line.starts_with("active")
                || line.starts_with("superseded")
                || line.starts_with("retired")
        })
        .collect();
    assert_eq!(
        status_lines,
        vec!["draft      1", "active     1", "superseded 1", "retired    1"]
    );
}

#[test]
fn status_report_show_ids_lists_sorted_ids() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    let mut v = fixture_registry_statuses();
    v["features"] = json!([
        { "id": "010-z", "title": "Z", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/010-z/spec.md", "sectionHeadings": ["H"] },
        { "id": "001-a", "title": "A", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/001-a/spec.md", "sectionHeadings": ["H"] }
    ]);
    write_registry(&reg, &v);

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--show-ids"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("active"));
    assert!(stdout.contains("ids: 001-a, 010-z"));
}

#[test]
fn status_report_json_emits_machine_readable_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_statuses());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));

    let rows: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("status-report --json emits valid JSON");
    let arr = rows.as_array().expect("json output must be an array");
    assert_eq!(arr.len(), 4, "one row per known status");
    assert_eq!(arr[0]["status"], "draft");
    assert_eq!(arr[1]["status"], "active");
    assert_eq!(arr[2]["status"], "superseded");
    assert_eq!(arr[3]["status"], "retired");
    assert_eq!(arr[0]["count"], 1);
    assert_eq!(arr[1]["count"], 1);
    assert_eq!(arr[2]["count"], 1);
    assert_eq!(arr[3]["count"], 1);
}

#[test]
fn status_report_json_ids_are_sorted_for_each_status() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    let mut v = fixture_registry_statuses();
    v["features"] = json!([
        { "id": "010-z", "title": "Z", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/010-z/spec.md", "sectionHeadings": ["H"] },
        { "id": "001-a", "title": "A", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/001-a/spec.md", "sectionHeadings": ["H"] }
    ]);
    write_registry(&reg, &v);

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));

    let rows: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("status-report --json emits valid JSON");
    let arr = rows.as_array().expect("json output must be an array");
    let active = arr
        .iter()
        .find(|row| row.get("status").and_then(|x| x.as_str()) == Some("active"))
        .expect("active row present");
    assert_eq!(active["ids"], json!(["001-a", "010-z"]));
}

#[test]
fn status_report_nonzero_only_omits_zero_rows_in_text_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    let v = json!({
        "specVersion": "1.0.0",
        "build": {
            "compilerId": "test",
            "compilerVersion": "0.1.0",
            "inputRoot": ".",
            "contentHash": "0000000000000000000000000000000000000000000000000000000000000000"
        },
        "features": [
            { "id": "002-b", "title": "B", "status": "draft", "created": "2026-03-22", "summary": "s", "specPath": "specs/002-b/spec.md", "sectionHeadings": ["H"] },
            { "id": "001-a", "title": "A", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/001-a/spec.md", "sectionHeadings": ["H"] }
        ],
        "validation": { "passed": true, "violations": [] }
    });
    write_registry(&reg, &v);

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--nonzero-only"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("draft      1"));
    assert!(stdout.contains("active     1"));
    assert!(!stdout.contains("superseded"));
    assert!(!stdout.contains("retired"));
    let draft_idx = stdout.find("draft").expect("draft row present");
    let active_idx = stdout.find("active").expect("active row present");
    assert!(draft_idx < active_idx, "remaining rows keep deterministic order");
}

#[test]
fn status_report_json_nonzero_only_omits_zero_rows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    let v = json!({
        "specVersion": "1.0.0",
        "build": {
            "compilerId": "test",
            "compilerVersion": "0.1.0",
            "inputRoot": ".",
            "contentHash": "0000000000000000000000000000000000000000000000000000000000000000"
        },
        "features": [
            { "id": "009-a", "title": "A", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/009-a/spec.md", "sectionHeadings": ["H"] }
        ],
        "validation": { "passed": true, "violations": [] }
    });
    write_registry(&reg, &v);

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--json", "--nonzero-only"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));

    let rows: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("status-report --json emits valid JSON");
    let arr = rows.as_array().expect("json output must be an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["status"], "active");
    assert_eq!(arr[0]["count"], 1);
    assert_eq!(arr[0]["ids"], json!(["009-a"]));
}

#[test]
fn status_report_default_behavior_unchanged_without_nonzero_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    let v = json!({
        "specVersion": "1.0.0",
        "build": {
            "compilerId": "test",
            "compilerVersion": "0.1.0",
            "inputRoot": ".",
            "contentHash": "0000000000000000000000000000000000000000000000000000000000000000"
        },
        "features": [
            { "id": "009-a", "title": "A", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/009-a/spec.md", "sectionHeadings": ["H"] }
        ],
        "validation": { "passed": true, "violations": [] }
    });
    write_registry(&reg, &v);

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("status-report")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("draft      0"));
    assert!(stdout.contains("active     1"));
    assert!(stdout.contains("superseded 0"));
    assert!(stdout.contains("retired    0"));
}
