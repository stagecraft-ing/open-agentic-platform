//! V-005 / V-006 and codeAliases emission.

use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn v005_duplicate_alias_across_features() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let d1 = root.join("specs/001-a");
    let d2 = root.join("specs/002-b");
    fs::create_dir_all(&d1).unwrap();
    fs::create_dir_all(&d2).unwrap();
    fs::write(
        d1.join("spec.md"),
        r#"---
id: "001-a"
title: "A"
status: draft
created: "2026-03-22"
summary: "x"
code_aliases:
  - SHARED_ALIAS
---
# A
"#,
    )
    .unwrap();
    fs::write(
        d2.join("spec.md"),
        r#"---
id: "002-b"
title: "B"
status: draft
created: "2026-03-22"
summary: "y"
code_aliases:
  - SHARED_ALIAS
---
# B
"#,
    )
    .unwrap();

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).expect("registry JSON");
    let viol = v["validation"]["violations"].as_array().unwrap();
    assert!(
        viol.iter()
            .any(|x| x["code"] == "V-005" && x["severity"] == "error"),
        "expected V-005 errors, got {viol:?}"
    );
    assert_eq!(v["validation"]["passed"], false);
}

#[test]
fn v006_malformed_alias_emits_warning_and_omits_invalid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let d = root.join("specs/001-a");
    fs::create_dir_all(&d).unwrap();
    fs::write(
        d.join("spec.md"),
        r#"---
id: "001-a"
title: "A"
status: draft
created: "2026-03-22"
summary: "x"
code_aliases:
  - BAD_alias
  - GOOD_ALIAS
---
# A
"#,
    )
    .unwrap();

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).unwrap();
    let viol = v["validation"]["violations"].as_array().unwrap();
    assert!(
        viol.iter()
            .any(|x| x["code"] == "V-006" && x["severity"] == "warning"),
        "expected V-006 warning, got {viol:?}"
    );
    let feats = v["features"].as_array().unwrap();
    let rec = feats.iter().find(|f| f["id"] == "001-a").unwrap();
    assert_eq!(rec["codeAliases"], json_str_list(&["GOOD_ALIAS"]));
}

fn json_str_list(items: &[&str]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|s| Value::String((*s).to_string()))
            .collect(),
    )
}

#[test]
fn code_aliases_sorted_deterministically() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let d = root.join("specs/001-a");
    fs::create_dir_all(&d).unwrap();
    fs::write(
        d.join("spec.md"),
        r#"---
id: "001-a"
title: "A"
status: draft
created: "2026-03-22"
summary: "x"
code_aliases:
  - ZEBRA
  - ALPHA
---
# A
"#,
    )
    .unwrap();

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).unwrap();
    let feats = v["features"].as_array().unwrap();
    let rec = feats.iter().find(|f| f["id"] == "001-a").unwrap();
    assert_eq!(rec["codeAliases"], json_str_list(&["ALPHA", "ZEBRA"]));
}

#[test]
fn repo_spec_version_is_1_3_0() {
    let out = open_agentic_spec_compiler::compile(&repo_root()).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).unwrap();
    assert_eq!(v["specVersion"], "1.3.0");
}
