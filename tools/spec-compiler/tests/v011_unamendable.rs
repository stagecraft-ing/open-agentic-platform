//! V-011 — `amends_sections` must not overlap an amended spec's `unamendable` (spec 132).
//!
//! Tooling-only check: spec 000's frontmatter is not yet edited to declare
//! any unamendable anchors; the proposed amendment is staged in
//! /tmp/spec_000_proposed_amendment.diff for human review. These tests use
//! synthetic fixture specs to exercise the compiler logic.

use serde_json::Value;
use std::fs;
use std::path::Path;

fn write_spec(dir: &Path, id: &str, frontmatter_extra: &str, body: &str) {
    let path = dir.join("spec.md");
    let raw = format!(
        r#"---
id: "{id}"
title: "Fixture for {id}"
status: draft
created: "2026-05-02"
summary: "V-011 fixture spec."
{frontmatter_extra}---
{body}
"#
    );
    fs::write(path, raw).unwrap();
}

fn compile(root: &Path) -> Value {
    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    serde_json::from_slice(&out.registry_json).expect("registry JSON")
}

fn collect_v011(reg: &Value) -> Vec<&Value> {
    reg["validation"]["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .filter(|v| v["code"].as_str() == Some("V-011"))
        .collect()
}

/// V-011 fires when an amending spec's `amends_sections` overlaps the
/// amended spec's `unamendable` list.
#[test]
fn v011_fires_on_overlap() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/000-frozen-fixture")).unwrap();
    fs::create_dir_all(root.join("specs/990-bad-amender")).unwrap();

    // Spec 000 with a frozen anchor.
    write_spec(
        &root.join("specs/000-frozen-fixture"),
        "000-frozen-fixture",
        "unamendable:\n  - \"V-001\"\n  - \"markdown-truth-boundary\"\n",
        "# Constitution fixture\n",
    );
    // Spec 990 declares it amends V-001.
    write_spec(
        &root.join("specs/990-bad-amender"),
        "990-bad-amender",
        "amends:\n  - \"000-frozen-fixture\"\namends_sections:\n  - \"V-001\"\n",
        "# Bad amender\n",
    );

    let reg = compile(root);
    let violations = collect_v011(&reg);
    assert_eq!(
        violations.len(),
        1,
        "expected 1 V-011 violation; got: {:?}",
        reg["validation"]["violations"]
    );
    let v = violations[0];
    assert_eq!(v["severity"].as_str(), Some("error"));
    assert!(
        v["message"].as_str().unwrap().contains("V-001"),
        "message must name the section: {v:?}"
    );
    assert!(v["message"].as_str().unwrap().contains("990-bad-amender"));
    assert!(v["message"].as_str().unwrap().contains("000-frozen-fixture"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-011 does NOT fire when the amending spec's `amends_sections` are
/// outside the unamendable list.
#[test]
fn v011_silent_when_amender_targets_non_frozen_anchor() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/000-frozen-fixture")).unwrap();
    fs::create_dir_all(root.join("specs/991-ok-amender")).unwrap();

    write_spec(
        &root.join("specs/000-frozen-fixture"),
        "000-frozen-fixture",
        "unamendable:\n  - \"V-001\"\n",
        "# Fixture\n",
    );
    write_spec(
        &root.join("specs/991-ok-amender"),
        "991-ok-amender",
        "amends:\n  - \"000-frozen-fixture\"\namends_sections:\n  - \"narrative-section\"\n",
        "# Amender targeting non-frozen anchor\n",
    );

    let reg = compile(root);
    assert!(
        collect_v011(&reg).is_empty(),
        "expected no V-011; got {:?}",
        reg["validation"]["violations"]
    );
}

/// V-011 short-form id resolution: `amends: ["000"]` resolves to
/// `000-frozen-fixture` via the leading-3-digit prefix index.
#[test]
fn v011_resolves_short_form_amends_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/000-frozen-fixture")).unwrap();
    fs::create_dir_all(root.join("specs/992-short-form-amender")).unwrap();

    write_spec(
        &root.join("specs/000-frozen-fixture"),
        "000-frozen-fixture",
        "unamendable:\n  - \"V-001\"\n",
        "# Fixture\n",
    );
    write_spec(
        &root.join("specs/992-short-form-amender"),
        "992-short-form-amender",
        "amends:\n  - \"000\"\namends_sections:\n  - \"V-001\"\n",
        "# Short-form amender\n",
    );

    let reg = compile(root);
    assert_eq!(
        collect_v011(&reg).len(),
        1,
        "short-form amends must resolve and trigger V-011"
    );
}

/// An empty unamendable list cannot trigger V-011 even when amends_sections
/// is non-empty.
#[test]
fn v011_silent_when_amended_spec_has_empty_unamendable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/000-fixture")).unwrap();
    fs::create_dir_all(root.join("specs/993-amender")).unwrap();

    // No `unamendable:` field at all.
    write_spec(
        &root.join("specs/000-fixture"),
        "000-fixture",
        "",
        "# Plain fixture\n",
    );
    write_spec(
        &root.join("specs/993-amender"),
        "993-amender",
        "amends:\n  - \"000-fixture\"\namends_sections:\n  - \"any-anchor\"\n",
        "# Amender\n",
    );

    let reg = compile(root);
    assert!(collect_v011(&reg).is_empty());
}

/// V-011 surfaces every overlapping anchor, not just the first.
#[test]
fn v011_reports_each_overlap() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/000-frozen-multi")).unwrap();
    fs::create_dir_all(root.join("specs/994-greedy-amender")).unwrap();

    write_spec(
        &root.join("specs/000-frozen-multi"),
        "000-frozen-multi",
        "unamendable:\n  - \"V-001\"\n  - \"V-002\"\n  - \"V-003\"\n",
        "# Multi-frozen fixture\n",
    );
    write_spec(
        &root.join("specs/994-greedy-amender"),
        "994-greedy-amender",
        "amends:\n  - \"000-frozen-multi\"\namends_sections:\n  - \"V-001\"\n  - \"V-002\"\n",
        "# Tries to amend two frozen anchors\n",
    );

    let reg = compile(root);
    let violations = collect_v011(&reg);
    assert_eq!(violations.len(), 2);
    let messages: Vec<&str> = violations
        .iter()
        .map(|v| v["message"].as_str().unwrap())
        .collect();
    assert!(messages.iter().any(|m| m.contains("V-001")));
    assert!(messages.iter().any(|m| m.contains("V-002")));
}
