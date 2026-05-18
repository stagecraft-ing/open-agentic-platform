//! V-004: consolidated product trees and root pnpm files must not fail the standalone-YAML scan.

use serde_json::Value;
use std::fs;
use std::path::Path;

fn minimal_spec(path: &Path) {
    fs::write(
        path,
        r#"---
id: "098-v004-consolidation"
title: "Fixture"
status: draft
created: "2026-03-22"
summary: "V-004 consolidation exclusion fixture."
---
# Fixture
"#,
    )
    .unwrap();
}

#[test]
fn root_pnpm_workspace_files_do_not_trigger_v004() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    fs::create_dir_all(root.join("specs/098-v004-consolidation")).unwrap();
    minimal_spec(&root.join("specs/098-v004-consolidation/spec.md"));

    fs::write(
        root.join("pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    )
    .unwrap();
    fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).expect("registry JSON");
    let violations = v["validation"]["violations"]
        .as_array()
        .expect("violations array");
    let v004_pnpm: Vec<_> = violations
        .iter()
        .filter(|x| x["code"].as_str() == Some("V-004"))
        .filter(|x| {
            x["path"]
                .as_str()
                .map(|p| p.contains("pnpm-"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        v004_pnpm.is_empty(),
        "V-004 must not flag root pnpm workspace files; got {violations:?}"
    );
    assert!(
        v["validation"]["passed"].as_bool() == Some(true),
        "expected validation.passed true"
    );
}

#[test]
fn root_sops_yaml_does_not_trigger_v004() {
    // `.sops.yaml` is the SOPS CLI's own config file format — tool-format
    // YAML consumed by an external binary, not authored OAP truth. Spec 151
    // §Clarification 9 + plan.md §"Constitution check" record the rationale.
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    fs::create_dir_all(root.join("specs/098-v004-consolidation")).unwrap();
    minimal_spec(&root.join("specs/098-v004-consolidation/spec.md"));

    fs::write(
        root.join(".sops.yaml"),
        "creation_rules:\n  - path_regex: secrets/.*\\.yaml$\n    age: age1example\n",
    )
    .unwrap();

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).expect("registry JSON");
    let violations = v["validation"]["violations"]
        .as_array()
        .expect("violations array");
    let v004_sops: Vec<_> = violations
        .iter()
        .filter(|x| x["code"].as_str() == Some("V-004"))
        .filter(|x| {
            x["path"]
                .as_str()
                .map(|p| p.ends_with(".sops.yaml"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        v004_sops.is_empty(),
        "V-004 must not flag root .sops.yaml (SOPS tool config); got {violations:?}"
    );
    assert!(
        v["validation"]["passed"].as_bool() == Some(true),
        "expected validation.passed true"
    );
}
