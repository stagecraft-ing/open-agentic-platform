//! V-014 — list-form `implements:` is rejected after spec 152 activation.
//!
//! The scalar capability→registry pointer (`implements: "<spec-id>"` for
//! `kind: capability`) is the only accepted shape. List form is retired.

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
created: "2026-05-20"
summary: "V-014 fixture spec."
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

#[test]
fn v014_errors_on_list_form_implements() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/990-legacy-list-form")).unwrap();
    write_spec(
        &root.join("specs/990-legacy-list-form"),
        "990-legacy-list-form",
        "implements:\n  - path: crates/foo/src/lib.rs\n",
    );

    let reg = compile(root);
    let v014 = collect_violations(&reg, "V-014");
    assert_eq!(
        v014.len(),
        1,
        "expected one V-014 violation; got: {v014:?}"
    );
    assert_eq!(v014[0]["severity"].as_str(), Some("error"));
    let msg = v014[0]["message"].as_str().unwrap();
    assert!(
        msg.contains("list form was retired"),
        "expected 'list form was retired' in message; got: {msg}"
    );
}

#[test]
fn v014_accepts_scalar_form_for_capability_kind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/991-capability-impl")).unwrap();
    write_spec(
        &root.join("specs/991-capability-impl"),
        "991-capability-impl",
        "kind: capability\nimplements: \"148-auth-driver-registry\"\nprovides:\n  driver: saml\n",
    );

    let reg = compile(root);
    let v014 = collect_violations(&reg, "V-014");
    assert!(
        v014.is_empty(),
        "scalar form for kind=capability must pass; got V-014: {v014:?}"
    );
}

#[test]
fn v014_warns_on_scalar_form_for_non_capability_kind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/992-wrong-kind")).unwrap();
    write_spec(
        &root.join("specs/992-wrong-kind"),
        "992-wrong-kind",
        "kind: governance\nimplements: \"148-auth-driver-registry\"\n",
    );

    let reg = compile(root);
    let v014 = collect_violations(&reg, "V-014");
    assert_eq!(v014.len(), 1);
    assert_eq!(v014[0]["severity"].as_str(), Some("warning"));
    let msg = v014[0]["message"].as_str().unwrap();
    assert!(
        msg.contains("scalar form is reserved for kind=capability"),
        "expected scalar-kind warning; got: {msg}"
    );
}
