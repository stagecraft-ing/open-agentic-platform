// SPDX-License-Identifier: AGPL-3.0-or-later
// Spec 091: Validate that the `risk` frontmatter field is checked against
// VALID_RISK_LEVELS and produces a V-007 violation on invalid values.

use serde_json::Value;
use std::fs;

/// Create a minimal spec repo in a temp directory with one spec.
fn make_temp_spec_repo(frontmatter: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec_dir = dir.path().join("specs/999-risk-test");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("spec.md"),
        format!(
            "---\n{frontmatter}\n---\n# Risk Test\n\n## Section\n\nBody text.\n"
        ),
    )
    .unwrap();
    dir
}

#[test]
fn sc091_3_invalid_risk_produces_v007() {
    let dir = make_temp_spec_repo(
        r#"id: "999-risk-test"
title: "Risk test"
status: draft
created: "2026-04-12"
summary: "Test invalid risk value."
risk: banana"#,
    );

    let out = open_agentic_spec_compiler::compile(dir.path()).expect("compile");
    assert!(
        !out.validation_passed,
        "compile should fail with invalid risk"
    );

    let registry: Value = serde_json::from_slice(&out.registry_json).expect("parse JSON");
    let violations = registry["validation"]["violations"]
        .as_array()
        .expect("violations array");

    let v007: Vec<&Value> = violations
        .iter()
        .filter(|v| v["code"].as_str() == Some("V-007"))
        .collect();
    assert!(
        !v007.is_empty(),
        "expected V-007 violation for invalid risk, got: {violations:?}"
    );
    assert!(
        v007[0]["message"]
            .as_str()
            .unwrap()
            .contains("banana"),
        "V-007 message should mention the invalid value"
    );
}

#[test]
fn sc091_3_valid_risk_passes() {
    let dir = make_temp_spec_repo(
        r#"id: "999-risk-test"
title: "Risk test"
status: draft
created: "2026-04-12"
summary: "Test valid risk value."
risk: high"#,
    );

    let out = open_agentic_spec_compiler::compile(dir.path()).expect("compile");

    let registry: Value = serde_json::from_slice(&out.registry_json).expect("parse JSON");
    let violations = registry["validation"]["violations"]
        .as_array()
        .expect("violations array");

    let v007: Vec<&Value> = violations
        .iter()
        .filter(|v| v["code"].as_str() == Some("V-007"))
        .collect();
    assert!(
        v007.is_empty(),
        "valid risk should not produce V-007, got: {v007:?}"
    );
}
