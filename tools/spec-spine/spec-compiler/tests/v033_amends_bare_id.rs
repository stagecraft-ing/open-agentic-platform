//! V-033 (spec 216 Phase 1): `amends` is a bare-id list only.
//!
//! OAP previously routed `amends` through `optional_string_list`, which
//! returns `None` on any non-string entry; `.unwrap_or_default()` then
//! collapsed the field to an empty list silently, so an object-form `amends`
//! recorded nothing and conferred no authority (the spec 125 defect PR #358
//! corrected). Phase 1 replaces that silent drop with a loud V-033 error,
//! mirroring the standalone spec-spine library's reject-on-object behaviour.

use serde_json::Value;
use std::fs;
use std::path::Path;

fn write_spec(dir: &Path, id: &str, frontmatter_extra: &str) {
    let path = dir.join("spec.md");
    let raw = format!(
        r#"---
id: "{id}"
title: "Fixture for {id}"
status: draft
created: "2026-06-14"
summary: "V-033 fixture spec."
{frontmatter_extra}---
"#
    );
    fs::write(path, raw).unwrap();
}

fn compile(root: &Path) -> Value {
    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    serde_json::from_slice(&out.registry_json).expect("registry JSON")
}

fn collect_violations<'a>(reg: &'a Value, code: &str) -> Vec<&'a Value> {
    reg["validation"]["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .filter(|v| v["code"].as_str() == Some(code))
        .collect()
}

fn feature<'a>(reg: &'a Value, id: &str) -> &'a Value {
    reg["features"]
        .as_array()
        .expect("features array")
        .iter()
        .find(|f| f["id"].as_str() == Some(id))
        .expect("feature present")
}

/// AC-001: an object-form `amends` entry fails compile with V-033 (error),
/// naming the bare-id + `refines`/`amends_sections` remediation. Previously
/// this compiled with `amends` silently empty.
#[test]
fn v033_errors_on_object_form_amends() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/990-object-form-amends")).unwrap();
    write_spec(
        &root.join("specs/990-object-form-amends"),
        "990-object-form-amends",
        "amends:\n  - spec: \"001-spec-compiler-mvp\"\n    unit: { kind: file, path: crates/foo/src/lib.rs }\n",
    );

    let reg = compile(root);
    let v033 = collect_violations(&reg, "V-033");
    assert_eq!(v033.len(), 1, "expected one V-033 violation; got: {v033:?}");
    assert_eq!(v033[0]["severity"].as_str(), Some("error"));
    let msg = v033[0]["message"].as_str().unwrap();
    assert!(
        msg.contains("spec ids only"),
        "expected 'spec ids only' in message; got: {msg}"
    );
    assert!(
        msg.contains("refines") && msg.contains("amends_sections"),
        "expected refines/amends_sections remediation; got: {msg}"
    );

    // The malformed field is dropped so the registry stays schema-conformant;
    // the V-033 violation is the source of truth for the rejection.
    let f = feature(&reg, "990-object-form-amends");
    assert!(
        f.get("amends").is_none(),
        "object-form amends must not be recorded; got: {:?}",
        f.get("amends")
    );
}

/// AC-002: a bare-id list compiles unchanged and records both ids.
#[test]
fn v033_accepts_bare_id_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/991-bare-id-amends")).unwrap();
    write_spec(
        &root.join("specs/991-bare-id-amends"),
        "991-bare-id-amends",
        "amends:\n  - \"001-spec-compiler-mvp\"\n  - \"002-registry-consumer-mvp\"\n",
    );

    let reg = compile(root);
    let v033 = collect_violations(&reg, "V-033");
    assert!(
        v033.is_empty(),
        "bare-id amends must not trip V-033; got: {v033:?}"
    );

    let f = feature(&reg, "991-bare-id-amends");
    let amends: Vec<&str> = f["amends"]
        .as_array()
        .expect("amends array recorded")
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        amends,
        vec!["001-spec-compiler-mvp", "002-registry-consumer-mvp"],
        "both bare ids must be recorded verbatim"
    );
}

/// Absent `amends` is clean: no V-033, no recorded field (distinguishes
/// "absent" from "present but malformed").
#[test]
fn v033_absent_amends_is_clean() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/992-no-amends")).unwrap();
    write_spec(&root.join("specs/992-no-amends"), "992-no-amends", "");

    let reg = compile(root);
    assert!(
        collect_violations(&reg, "V-033").is_empty(),
        "absent amends must not trip V-033"
    );
    let f = feature(&reg, "992-no-amends");
    assert!(
        f.get("amends").is_none(),
        "absent amends must not be recorded"
    );
}
