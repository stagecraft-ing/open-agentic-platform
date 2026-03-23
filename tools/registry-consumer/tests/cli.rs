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
fn show_json_emits_valid_object_matching_feature_record() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("show --json emits valid JSON");
    assert!(v.is_object());
    assert_eq!(v["id"], "001-a");
    assert_eq!(v["title"], "First");
    assert_eq!(v["status"], "active");
    assert_eq!(v["specPath"], "specs/001-a/spec.md");
    assert_eq!(v["summary"], "sum");
}

#[test]
fn show_json_stdout_matches_show_without_json_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let without = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a"])
        .output()
        .expect("spawn");
    let with_json = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(without.status.code(), Some(0));
    assert_eq!(with_json.status.code(), Some(0));
    assert_eq!(without.stdout, with_json.stdout, "default show must match show --json");
}

#[test]
fn show_json_not_found_exits_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "999-nope", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn show_json_validation_failed_exits_one_without_allow_invalid() {
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
        .args(["show", "001-a", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn show_json_invalid_registry_file_exits_three() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    fs::write(&reg, "not json {{{").unwrap();

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(3));
}

#[test]
fn show_compact_emits_valid_json_equal_to_pretty_when_parsed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let pretty = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a"])
        .output()
        .expect("spawn");
    let compact = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(pretty.status.code(), Some(0));
    assert_eq!(compact.status.code(), Some(0));

    let vp: serde_json::Value = serde_json::from_slice(&pretty.stdout).unwrap();
    let vc: serde_json::Value = serde_json::from_slice(&compact.stdout).unwrap();
    assert_eq!(vp, vc, "compact and pretty must parse to the same feature object");
}

#[test]
fn show_compact_output_is_single_line_of_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout.lines().count(),
        1,
        "compact show must print exactly one line (plus trailing newline from println)"
    );
    assert!(
        !stdout.trim_end().contains('\n'),
        "compact JSON body must not contain embedded newlines"
    );
}

#[test]
fn show_json_and_compact_are_mutually_exclusive() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a", "--json", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn list_compact_emits_valid_array_equal_to_pretty_when_parsed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let pretty = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--json"])
        .output()
        .expect("spawn");
    let compact = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(pretty.status.code(), Some(0));
    assert_eq!(compact.status.code(), Some(0));

    let vp: serde_json::Value = serde_json::from_slice(&pretty.stdout).unwrap();
    let vc: serde_json::Value = serde_json::from_slice(&compact.stdout).unwrap();
    assert_eq!(vp, vc);
}

#[test]
fn list_compact_output_is_single_line() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.trim_end().contains('\n'));
}

#[test]
fn list_json_and_compact_are_mutually_exclusive() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--json", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn list_compact_status_filter_matches_pretty_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let pretty = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--json", "--status", "draft"])
        .output()
        .expect("spawn");
    let compact = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--compact", "--status", "draft"])
        .output()
        .expect("spawn");
    assert_eq!(pretty.status.code(), Some(0));
    assert_eq!(compact.status.code(), Some(0));
    let vp: serde_json::Value = serde_json::from_slice(&pretty.stdout).unwrap();
    let vc: serde_json::Value = serde_json::from_slice(&compact.stdout).unwrap();
    assert_eq!(vp, vc);
    assert_eq!(vc.as_array().unwrap().len(), 1);
}

#[test]
fn list_compact_id_prefix_filter_matches_pretty_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let pretty = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--json", "--id-prefix", "001"])
        .output()
        .expect("spawn");
    let compact = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--compact", "--id-prefix", "001"])
        .output()
        .expect("spawn");
    assert_eq!(pretty.status.code(), Some(0));
    assert_eq!(compact.status.code(), Some(0));
    let vp: serde_json::Value = serde_json::from_slice(&pretty.stdout).unwrap();
    let vc: serde_json::Value = serde_json::from_slice(&compact.stdout).unwrap();
    assert_eq!(vp, vc);
}

#[test]
fn list_compact_validation_failed_exits_one_without_allow_invalid() {
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
        .args(["list", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn list_compact_invalid_registry_file_exits_three() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    fs::write(&reg, "not json {{{").unwrap();

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(3));
}

/// Feature 018: fixture contracts for list/show JSON and compact (mirrors status-report JSON contracts).
#[test]
fn list_json_contract_has_stable_array_shape_and_order() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let expected = json!([
        {
            "id": "001-a",
            "title": "First",
            "status": "active",
            "created": "2026-03-22",
            "summary": "sum",
            "specPath": "specs/001-a/spec.md",
            "sectionHeadings": ["H"]
        },
        {
            "id": "002-b",
            "title": "Second",
            "status": "draft",
            "created": "2026-03-22",
            "summary": "sum",
            "specPath": "specs/002-b/spec.md",
            "sectionHeadings": ["H"]
        }
    ]);

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let got: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("list --json emits valid JSON");
    assert_eq!(got, expected, "list --json contract must remain stable");
}

#[test]
fn list_compact_contract_matches_expected_serialization() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let expected = json!([
        {
            "id": "001-a",
            "title": "First",
            "status": "active",
            "created": "2026-03-22",
            "summary": "sum",
            "specPath": "specs/001-a/spec.md",
            "sectionHeadings": ["H"]
        },
        {
            "id": "002-b",
            "title": "Second",
            "status": "draft",
            "created": "2026-03-22",
            "summary": "sum",
            "specPath": "specs/002-b/spec.md",
            "sectionHeadings": ["H"]
        }
    ]);
    let expected_line = serde_json::to_string(&expected).unwrap();

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let got: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("list --compact emits valid JSON");
    assert_eq!(got, expected);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout.trim_end(),
        expected_line,
        "compact list line must match serde_json::to_string of expected value"
    );
}

#[test]
fn show_json_contract_has_stable_object_shape() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let expected = json!({
        "id": "001-a",
        "title": "First",
        "status": "active",
        "created": "2026-03-22",
        "summary": "sum",
        "specPath": "specs/001-a/spec.md",
        "sectionHeadings": ["H"]
    });

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let got: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("show --json emits valid JSON");
    assert_eq!(got, expected, "show --json contract must remain stable");
}

#[test]
fn show_compact_contract_matches_expected_serialization() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let expected = json!({
        "id": "001-a",
        "title": "First",
        "status": "active",
        "created": "2026-03-22",
        "summary": "sum",
        "specPath": "specs/001-a/spec.md",
        "sectionHeadings": ["H"]
    });
    let expected_line = serde_json::to_string(&expected).unwrap();

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let got: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("show --compact emits valid JSON");
    assert_eq!(got, expected);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim_end(), expected_line);
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
fn list_json_emits_valid_array_sorted_by_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let arr: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("list --json emits valid JSON");
    let items = arr.as_array().expect("list --json must be a JSON array");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], "001-a");
    assert_eq!(items[1]["id"], "002-b");
    assert_eq!(items[0]["title"], "First");
    assert_eq!(items[1]["title"], "Second");
}

#[test]
fn list_json_status_filter_matches_text_semantics() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--json", "--status", "draft"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let arr: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("list --json emits valid JSON");
    let items = arr.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "002-b");
}

#[test]
fn list_json_id_prefix_filter_matches_text_semantics() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_ok());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--json", "--id-prefix", "001"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let arr: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("list --json emits valid JSON");
    let items = arr.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "001-a");
}

#[test]
fn list_without_json_still_emits_text_table() {
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
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("id "));
    assert!(stdout.contains("001-a"));
}

#[test]
fn list_json_validation_failed_exits_one_without_allow_invalid() {
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
        .args(["list", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn list_json_missing_features_array_exits_three() {
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
        .args(["list", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(3));
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

#[test]
fn status_report_json_contract_has_stable_row_shape_and_order() {
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
    for row in arr {
        let obj = row.as_object().expect("row must be JSON object");
        assert_eq!(obj.len(), 3, "row shape must remain stable");
        assert!(obj.contains_key("status"));
        assert!(obj.contains_key("count"));
        assert!(obj.contains_key("ids"));
    }

    let expected = json!([
        { "status": "draft", "count": 1, "ids": ["004-d"] },
        { "status": "active", "count": 1, "ids": ["001-a"] },
        { "status": "superseded", "count": 1, "ids": ["003-c"] },
        { "status": "retired", "count": 1, "ids": ["002-b"] }
    ]);
    assert_eq!(rows, expected, "default JSON contract must remain stable");
}

#[test]
fn status_report_json_nonzero_only_contract_is_stable() {
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
            { "id": "009-b", "title": "B", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/009-b/spec.md", "sectionHeadings": ["H"] },
            { "id": "009-a", "title": "A", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/009-a/spec.md", "sectionHeadings": ["H"] },
            { "id": "010-z", "title": "Z", "status": "retired", "created": "2026-03-22", "summary": "s", "specPath": "specs/010-z/spec.md", "sectionHeadings": ["H"] }
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
    let expected = json!([
        { "status": "active", "count": 2, "ids": ["009-a", "009-b"] },
        { "status": "retired", "count": 1, "ids": ["010-z"] }
    ]);
    assert_eq!(rows, expected, "nonzero JSON contract must remain stable");
}

#[test]
fn status_report_status_filter_text_mode_keeps_selected_row() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_statuses());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--status", "active"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("active     1"));
    assert!(!stdout.contains("draft"));
    assert!(!stdout.contains("superseded"));
    assert!(!stdout.contains("retired"));
}

#[test]
fn status_report_status_filter_json_mode_keeps_selected_row() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    let mut v = fixture_registry_statuses();
    v["features"] = json!([
        { "id": "010-z", "title": "Z", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/010-z/spec.md", "sectionHeadings": ["H"] },
        { "id": "001-a", "title": "A", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/001-a/spec.md", "sectionHeadings": ["H"] },
        { "id": "099-x", "title": "X", "status": "draft", "created": "2026-03-22", "summary": "s", "specPath": "specs/099-x/spec.md", "sectionHeadings": ["H"] }
    ]);
    write_registry(&reg, &v);

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--json", "--status", "active"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));

    let rows: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("status-report --json emits valid JSON");
    let expected = json!([
        { "status": "active", "count": 2, "ids": ["001-a", "010-z"] }
    ]);
    assert_eq!(rows, expected);
}

#[test]
fn status_report_status_filter_rejects_unknown_value() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_statuses());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--status", "unknown"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn status_report_compact_equal_to_pretty_when_parsed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_statuses());

    let exe = registry_consumer_exe();
    let pretty = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--json"])
        .output()
        .expect("spawn");
    let compact = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(pretty.status.code(), Some(0));
    assert_eq!(compact.status.code(), Some(0));
    let vp: serde_json::Value = serde_json::from_slice(&pretty.stdout).unwrap();
    let vc: serde_json::Value = serde_json::from_slice(&compact.stdout).unwrap();
    assert_eq!(vp, vc);
}

#[test]
fn status_report_compact_output_is_single_line() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_statuses());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.lines().count(), 1);
    assert!(!stdout.trim_end().contains('\n'));
}

#[test]
fn status_report_json_and_compact_are_mutually_exclusive() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    write_registry(&reg, &fixture_registry_statuses());

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--json", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn status_report_compact_status_filter_matches_pretty_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    let mut v = fixture_registry_statuses();
    v["features"] = json!([
        { "id": "010-z", "title": "Z", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/010-z/spec.md", "sectionHeadings": ["H"] },
        { "id": "001-a", "title": "A", "status": "active", "created": "2026-03-22", "summary": "s", "specPath": "specs/001-a/spec.md", "sectionHeadings": ["H"] },
        { "id": "099-x", "title": "X", "status": "draft", "created": "2026-03-22", "summary": "s", "specPath": "specs/099-x/spec.md", "sectionHeadings": ["H"] }
    ]);
    write_registry(&reg, &v);

    let exe = registry_consumer_exe();
    let pretty = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--json", "--status", "active"])
        .output()
        .expect("spawn");
    let compact = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--compact", "--status", "active"])
        .output()
        .expect("spawn");
    assert_eq!(pretty.status.code(), Some(0));
    assert_eq!(compact.status.code(), Some(0));
    let vp: serde_json::Value = serde_json::from_slice(&pretty.stdout).unwrap();
    let vc: serde_json::Value = serde_json::from_slice(&compact.stdout).unwrap();
    assert_eq!(vp, vc);
}

#[test]
fn status_report_compact_nonzero_only_matches_pretty_json() {
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
    let pretty = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--json", "--nonzero-only"])
        .output()
        .expect("spawn");
    let compact = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--compact", "--nonzero-only"])
        .output()
        .expect("spawn");
    assert_eq!(pretty.status.code(), Some(0));
    assert_eq!(compact.status.code(), Some(0));
    let vp: serde_json::Value = serde_json::from_slice(&pretty.stdout).unwrap();
    let vc: serde_json::Value = serde_json::from_slice(&compact.stdout).unwrap();
    assert_eq!(vp, vc);
}

#[test]
fn status_report_compact_validation_failed_exits_one_without_allow_invalid() {
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
        .args(["status-report", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn status_report_compact_invalid_registry_file_exits_three() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reg = dir.path().join("registry.json");
    fs::write(&reg, "not json {{{").unwrap();

    let exe = registry_consumer_exe();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(3));
}

// --- Feature 019: README examples contract (fixture transcripts + README markers) ---

fn readme_fixture_registry_list_show() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/readme_examples/registry_list_show.json")
}

fn readme_fixture_registry_status_report() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/readme_examples/registry_status_report.json")
}

fn readme_contract_section<'a>(readme: &'a str, id: &str) -> Option<&'a str> {
    let start = format!("<!-- readme-contract:{id} -->");
    let end = format!("<!-- /readme-contract:{id} -->");
    let i = readme.find(&start)?;
    let tail = &readme[i + start.len()..];
    let j = tail.find(&end)?;
    Some(&tail[..j])
}

fn first_markdown_fence_inner(section: &str) -> &str {
    let s = section.trim_start();
    let s = s.strip_prefix("```").expect("expected opening ``` fence");
    let body = match s.find('\n') {
        None => s,
        Some(nl) => {
            let first = s[..nl].trim();
            if !first.is_empty()
                && first.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                &s[nl + 1..]
            } else {
                s
            }
        }
    };
    let end = body.rfind("```").expect("expected closing ``` fence");
    &body[..end]
}

fn assert_bytes_eq_stdout(actual: &[u8], expected: &str, label: &str) {
    let expected_b = expected.as_bytes();
    assert_eq!(
        actual, expected_b,
        "{label}: stdout must match committed transcript"
    );
}

#[test]
fn readme_example_cli_list_text_matches_transcript() {
    let exe = registry_consumer_exe();
    let reg = readme_fixture_registry_list_show();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("list")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    assert_bytes_eq_stdout(
        &out.stdout,
        include_str!("fixtures/readme_examples/expected/list_text.txt"),
        "list text",
    );
}

#[test]
fn readme_example_cli_list_json_matches_transcript() {
    let exe = registry_consumer_exe();
    let reg = readme_fixture_registry_list_show();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    assert_bytes_eq_stdout(
        &out.stdout,
        include_str!("fixtures/readme_examples/expected/list_json.txt"),
        "list --json",
    );
}

#[test]
fn readme_example_cli_list_compact_matches_transcript() {
    let exe = registry_consumer_exe();
    let reg = readme_fixture_registry_list_show();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["list", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    assert_bytes_eq_stdout(
        &out.stdout,
        include_str!("fixtures/readme_examples/expected/list_compact.txt"),
        "list --compact",
    );
}

#[test]
fn readme_example_cli_show_text_matches_transcript() {
    let exe = registry_consumer_exe();
    let reg = readme_fixture_registry_list_show();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    assert_bytes_eq_stdout(
        &out.stdout,
        include_str!("fixtures/readme_examples/expected/show_text.txt"),
        "show (default)",
    );
}

#[test]
fn readme_example_cli_show_json_matches_transcript() {
    let exe = registry_consumer_exe();
    let reg = readme_fixture_registry_list_show();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    assert_bytes_eq_stdout(
        &out.stdout,
        include_str!("fixtures/readme_examples/expected/show_json.txt"),
        "show --json",
    );
}

#[test]
fn readme_example_cli_show_compact_matches_transcript() {
    let exe = registry_consumer_exe();
    let reg = readme_fixture_registry_list_show();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "001-a", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    assert_bytes_eq_stdout(
        &out.stdout,
        include_str!("fixtures/readme_examples/expected/show_compact.txt"),
        "show --compact",
    );
}

#[test]
fn readme_example_cli_status_report_text_matches_transcript() {
    let exe = registry_consumer_exe();
    let reg = readme_fixture_registry_status_report();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("status-report")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    assert_bytes_eq_stdout(
        &out.stdout,
        include_str!("fixtures/readme_examples/expected/status_report_text.txt"),
        "status-report text",
    );
}

#[test]
fn readme_example_cli_status_report_json_matches_transcript() {
    let exe = registry_consumer_exe();
    let reg = readme_fixture_registry_status_report();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--json"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    assert_bytes_eq_stdout(
        &out.stdout,
        include_str!("fixtures/readme_examples/expected/status_report_json.txt"),
        "status-report --json",
    );
}

#[test]
fn readme_example_cli_status_report_compact_matches_transcript() {
    let exe = registry_consumer_exe();
    let reg = readme_fixture_registry_status_report();
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["status-report", "--compact"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(0));
    assert_bytes_eq_stdout(
        &out.stdout,
        include_str!("fixtures/readme_examples/expected/status_report_compact.txt"),
        "status-report --compact",
    );
}

#[test]
fn readme_markers_embed_exact_transcripts() {
    let readme_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let readme = fs::read_to_string(&readme_path).expect("read README");

    let checks: &[(&str, &str)] = &[
        (
            "list-text",
            include_str!("fixtures/readme_examples/expected/list_text.txt"),
        ),
        (
            "list-json",
            include_str!("fixtures/readme_examples/expected/list_json.txt"),
        ),
        (
            "list-compact",
            include_str!("fixtures/readme_examples/expected/list_compact.txt"),
        ),
        (
            "show-text",
            include_str!("fixtures/readme_examples/expected/show_text.txt"),
        ),
        (
            "show-json",
            include_str!("fixtures/readme_examples/expected/show_json.txt"),
        ),
        (
            "show-compact",
            include_str!("fixtures/readme_examples/expected/show_compact.txt"),
        ),
        (
            "status-report-text",
            include_str!("fixtures/readme_examples/expected/status_report_text.txt"),
        ),
        (
            "status-report-json",
            include_str!("fixtures/readme_examples/expected/status_report_json.txt"),
        ),
        (
            "status-report-compact",
            include_str!("fixtures/readme_examples/expected/status_report_compact.txt"),
        ),
    ];

    for (id, expected) in checks {
        let section = readme_contract_section(&readme, id).unwrap_or_else(|| {
            panic!("README missing readme-contract marker pair for {id}");
        });
        let got = first_markdown_fence_inner(section);
        assert_eq!(
            got, *expected,
            "README fenced body for {id} must match tests/fixtures/readme_examples/expected/{id}.txt (use that file as source of truth)"
        );
    }
}

fn error_contract_fixture(name: &str) -> PathBuf {
    PathBuf::from(format!("tests/fixtures/error_contract/{name}"))
}

fn assert_bytes_eq_stderr(actual: &[u8], expected: &str, label: &str) {
    let expected_b = expected.as_bytes();
    assert_eq!(
        actual, expected_b,
        "{label}: stderr must match committed transcript"
    );
}

/// Feature 020: fixture-based error contracts for key runtime failure paths.
#[test]
fn error_contract_list_missing_file_stderr_and_exit() {
    let exe = registry_consumer_exe();
    let reg = error_contract_fixture("does_not_exist.json");
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("list")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(out.stdout, b"");
    assert_bytes_eq_stderr(
        &out.stderr,
        include_str!("fixtures/error_contract/expected/list_missing_file.stderr.txt"),
        "list missing file",
    );
}

#[test]
fn error_contract_list_invalid_json_stderr_and_exit() {
    let exe = registry_consumer_exe();
    let reg = error_contract_fixture("registry_invalid_json.json");
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("list")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(out.stdout, b"");
    assert_bytes_eq_stderr(
        &out.stderr,
        include_str!("fixtures/error_contract/expected/list_invalid_json.stderr.txt"),
        "list invalid json",
    );
}

#[test]
fn error_contract_list_validation_failed_stderr_and_exit() {
    let exe = registry_consumer_exe();
    let reg = error_contract_fixture("registry_validation_failed.json");
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("list")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_bytes_eq_stderr(
        &out.stderr,
        include_str!("fixtures/error_contract/expected/list_validation_failed.stderr.txt"),
        "list validation failed",
    );
}

#[test]
fn error_contract_show_not_found_stderr_and_exit() {
    let exe = registry_consumer_exe();
    let reg = error_contract_fixture("registry_ok_single.json");
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .args(["show", "999-nope"])
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"");
    assert_bytes_eq_stderr(
        &out.stderr,
        include_str!("fixtures/error_contract/expected/show_not_found.stderr.txt"),
        "show not found",
    );
}

#[test]
fn error_contract_status_report_missing_features_stderr_and_exit() {
    let exe = registry_consumer_exe();
    let reg = error_contract_fixture("registry_missing_features.json");
    let out = Command::new(&exe)
        .args(["--registry-path"])
        .arg(&reg)
        .arg("status-report")
        .output()
        .expect("spawn");
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(out.stdout, b"");
    assert_bytes_eq_stderr(
        &out.stderr,
        include_str!("fixtures/error_contract/expected/status_report_missing_features.stderr.txt"),
        "status-report missing features",
    );
}
