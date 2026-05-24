//! Spec 179 (domain frontmatter field) — V-030 cases.
//!
//! Synthetic-fixture tests covering the closed-enum validator on the
//! new `domain:` frontmatter field:
//!
//! - Invalid value fires V-030 at error severity and the offending value
//!   is suppressed from the emitted feature record (mirrors V-007's
//!   risk-level handling, so the artifact stays schema-conformant).
//! - Each valid enum value (`opc | platform | substrate | tooling`)
//!   parses cleanly, fires no V-030, and round-trips through the
//!   emitted record.
//! - A spec that omits `domain:` parses cleanly at the compiler layer;
//!   absence is V-031's concern (emitted by spec-lint), not the
//!   compiler's, so spec-compiler stays silent on omission.

use serde_json::Value;
use std::fs;
use std::path::Path;

fn write_spec(dir: &Path, id: &str, extra: &str, body: &str) {
    let path = dir.join("spec.md");
    let raw = format!(
        r#"---
id: "{id}"
title: "Fixture for {id}"
status: draft
created: "2026-05-24"
summary: "Spec 179 domain-field fixture."
{extra}---
{body}
"#
    );
    fs::write(path, raw).unwrap();
}

fn compile(root: &Path) -> Value {
    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    serde_json::from_slice(&out.registry_json).expect("registry JSON")
}

fn violations_with_code<'a>(reg: &'a Value, code: &str) -> Vec<&'a Value> {
    reg["validation"]["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .filter(|v| v["code"].as_str() == Some(code))
        .collect()
}

fn find_feature<'a>(reg: &'a Value, id: &str) -> &'a Value {
    reg["features"]
        .as_array()
        .expect("features array")
        .iter()
        .find(|f| f["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("feature {id:?} not found in registry"))
}

/// V-030 — `domain:` carries a value outside the closed enum.
#[test]
fn v030_fires_on_invalid_domain_value() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/830-bad-domain")).unwrap();
    write_spec(
        &root.join("specs/830-bad-domain"),
        "830-bad-domain",
        "domain: cockpit\n",
        "# Bad domain value\n",
    );
    let reg = compile(root);
    let hits = violations_with_code(&reg, "V-030");
    assert_eq!(hits.len(), 1, "V-030 should fire exactly once");
    assert_eq!(hits[0]["severity"].as_str(), Some("error"));
    let feat = find_feature(&reg, "830-bad-domain");
    assert!(
        feat.get("domain").is_none(),
        "invalid domain value must be suppressed from the emitted feature record"
    );
}

/// Positive — each of the four valid enum values parses, emits, and
/// fires no V-030.
#[test]
fn valid_domain_values_round_trip_cleanly() {
    for value in ["opc", "platform", "substrate", "tooling"] {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let id = format!("831-domain-{value}");
        fs::create_dir_all(root.join(format!("specs/{id}"))).unwrap();
        write_spec(
            &root.join(format!("specs/{id}")),
            &id,
            &format!("domain: {value}\n"),
            "# Valid domain\n",
        );
        let reg = compile(root);
        assert!(
            violations_with_code(&reg, "V-030").is_empty(),
            "valid domain {value:?} should not fire V-030"
        );
        let feat = find_feature(&reg, &id);
        assert_eq!(feat["domain"].as_str(), Some(value));
    }
}

/// Omission — spec-compiler is silent when `domain:` is absent.
/// V-031 (the absent-field warning) is owned by spec-lint, not the
/// compiler.
#[test]
fn missing_domain_is_silent_at_compile_time() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/832-no-domain")).unwrap();
    write_spec(
        &root.join("specs/832-no-domain"),
        "832-no-domain",
        "",
        "# Domain omitted\n",
    );
    let reg = compile(root);
    assert!(violations_with_code(&reg, "V-030").is_empty());
    assert!(violations_with_code(&reg, "V-031").is_empty());
    let feat = find_feature(&reg, "832-no-domain");
    assert!(feat.get("domain").is_none());
}
