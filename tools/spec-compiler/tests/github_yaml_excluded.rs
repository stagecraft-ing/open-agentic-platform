//! R6: `.github/` is excluded from V-004 standalone-YAML scan (CI workflows are tooling).

use serde_json::Value;
use std::fs;
use std::path::Path;

fn minimal_spec(path: &Path) {
    fs::write(
        path,
        r#"---
id: "099-github-exclude"
title: "Fixture"
status: draft
created: "2026-03-22"
summary: "V-004 .github exclusion fixture."
---
# Fixture
"#,
    )
    .unwrap();
}

#[test]
fn github_workflow_yaml_does_not_trigger_v004() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    fs::create_dir_all(root.join("specs/099-github-exclude")).unwrap();
    minimal_spec(&root.join("specs/099-github-exclude/spec.md"));

    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::write(
        root.join(".github/workflows/ci.yml"),
        "name: ci\non: push\njobs: {}\n",
    )
    .unwrap();

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).expect("registry JSON");
    let violations = v["validation"]["violations"].as_array().expect("violations array");
    let v004_under_github: Vec<_> = violations
        .iter()
        .filter(|x| x["code"].as_str() == Some("V-004"))
        .filter(|x| {
            x["path"]
                .as_str()
                .map(|p| p.contains(".github"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        v004_under_github.is_empty(),
        "V-004 must not flag .github/ (research R6); got {violations:?}"
    );
    assert!(
        v["validation"]["passed"].as_bool() == Some(true),
        "expected validation.passed true when only .github has yml"
    );
}
