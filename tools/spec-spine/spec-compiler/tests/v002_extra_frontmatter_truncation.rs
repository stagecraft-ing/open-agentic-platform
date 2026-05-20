//! Regression: V-002 (b) extraFrontmatter over-size produces a violation
//! AND truncates the emitted shape to the alphabetically-first 8 entries.
//!
//! Per spec-compiler V-002 (b) semantics + Epic 2 I12 operator decision #7,
//! the producer side is permissive on overflow (8-entry truncation) while
//! the violation remains the source-of-truth signal. This mirrors the
//! V-007 risk-level normalization pattern (emit a violation AND keep the
//! registry shape conformant to the schema's maxProperties = 8 constraint).

use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn write_spec(root: &std::path::Path, id: &str, extra_keys: &[&str]) {
    let dir = root.join("specs").join(format!("099-{id}"));
    fs::create_dir_all(&dir).unwrap();

    let mut frontmatter = String::from(
        "---\n\
         id: \"099-fixture\"\n\
         title: \"Truncation fixture\"\n\
         status: draft\n\
         created: \"2026-05-20\"\n\
         summary: \"Spec with 9 extraFrontmatter keys to trigger V-002 (b).\"\n",
    );
    for key in extra_keys {
        frontmatter.push_str(&format!("{key}: \"placeholder\"\n"));
    }
    frontmatter.push_str("---\n\n# Fixture\n\n## Section\n");

    fs::write(dir.join("spec.md"), frontmatter).unwrap();
}

#[test]
fn v002_over_size_triggers_violation_and_truncates_to_first_eight() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    let extras = &[
        "z_last", "y_ninth", "a_first", "b_second", "c_third", "d_fourth",
        "e_fifth", "f_sixth", "g_seventh",
    ];
    write_spec(root, "fixture", extras);

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).expect("registry JSON");

    let violations = v["validation"]["violations"]
        .as_array()
        .expect("violations array");
    let oversize: Vec<&Value> = violations
        .iter()
        .filter(|x| {
            x["code"].as_str() == Some("V-002")
                && x["message"]
                    .as_str()
                    .map(|m| m.contains("exceeds maxProperties"))
                    .unwrap_or(false)
        })
        .collect();
    assert_eq!(
        oversize.len(),
        1,
        "expected exactly one V-002 (b) over-size violation; got {oversize:?}"
    );

    let features = v["features"].as_array().expect("features");
    let feat = features
        .iter()
        .find(|f| f["id"] == "099-fixture")
        .expect("fixture feature emitted");
    let extra = feat["extraFrontmatter"]
        .as_object()
        .expect("extraFrontmatter is an object");
    assert_eq!(
        extra.len(),
        8,
        "V-002 (b) MUST truncate emitted extraFrontmatter to 8 entries"
    );

    let kept_keys: Vec<&str> = extra.keys().map(|s| s.as_str()).collect();
    let alpha_first_eight = [
        "a_first", "b_second", "c_third", "d_fourth", "e_fifth", "f_sixth",
        "g_seventh", "y_ninth",
    ];
    for k in &alpha_first_eight {
        assert!(
            kept_keys.contains(k),
            "expected alphabetically-first key {k:?} in kept set: {kept_keys:?}"
        );
    }
    assert!(
        !kept_keys.contains(&"z_last"),
        "z_last MUST be dropped — it's the 9th alphabetic entry"
    );
}

#[test]
fn v002_under_threshold_no_truncation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    write_spec(
        root,
        "fixture",
        &["a", "b", "c"], // 3 keys, well under 8
    );

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).expect("registry JSON");

    let violations = v["validation"]["violations"]
        .as_array()
        .expect("violations array");
    let oversize: Vec<&Value> = violations
        .iter()
        .filter(|x| {
            x["code"].as_str() == Some("V-002")
                && x["message"]
                    .as_str()
                    .map(|m| m.contains("exceeds maxProperties"))
                    .unwrap_or(false)
        })
        .collect();
    assert!(
        oversize.is_empty(),
        "under-threshold extraFrontmatter MUST NOT trigger V-002 (b); got {oversize:?}"
    );

    let features = v["features"].as_array().expect("features");
    let feat = features
        .iter()
        .find(|f| f["id"] == "099-fixture")
        .expect("fixture feature emitted");
    let extra = feat["extraFrontmatter"].as_object().expect("extraFrontmatter object");
    assert_eq!(extra.len(), 3, "all 3 keys preserved under threshold");
}

// Suppress unused-import warning when the test runner doesn't pick up
// PathBuf above.
#[allow(dead_code)]
fn _force_pathbuf_import() -> PathBuf {
    PathBuf::new()
}
