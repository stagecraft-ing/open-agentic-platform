//! Spec 156 (references-edge provenance grammar) — V-025..V-029 cases.
//!
//! Synthetic-fixture tests covering each of the five spec 156
//! validation rules:
//!
//! - V-025 — `unit:`/`provenance:` mutually-exclusive arms on
//!   `references:` entries.
//! - V-026 — closed enum for `provenance.kind:`
//!   (`knowledge | code-fingerprint`).
//! - V-027 — kind ↔ URI-scheme alignment, including the required
//!   `/project/<uuid>/knowledge/<uuid>` segment shape on knowledge URIs.
//! - V-028 — URI body well-formedness (canonical UUIDs for
//!   `knowledge`; 64-hex SHA-256 for `code-fingerprint`).
//! - V-029 — advisory: provenance entry without `role:` recommends
//!   `role: derivation`. Warning severity; does NOT flip
//!   validation.passed.
//!
//! Positive cases (clean parse + canonical emission) appear inline as
//! the last two tests in this file.

use serde_json::Value;
use std::fs;
use std::path::Path;

const PROJ: &str = "8c4f1234-1234-4abc-9def-1234567890ab";
const KNOWLEDGE: &str = "2a91abcd-1111-4222-a333-444555666777";
const DIGEST: &str = "5e3b00112233445566778899aabbccddeeff00112233445566778899aabbccdd";

fn write_spec(dir: &Path, id: &str, frontmatter_extra: &str, body: &str) {
    let path = dir.join("spec.md");
    let raw = format!(
        r#"---
id: "{id}"
title: "Fixture for {id}"
status: draft
created: "2026-05-22"
summary: "Spec 156 provenance-grammar fixture."
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

/// V-025 — references entry carrying both `unit:` and `provenance:`
/// (mutually exclusive arms).
#[test]
fn v025_fires_on_both_unit_and_provenance() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/810-both-arms")).unwrap();
    write_spec(
        &root.join("specs/810-both-arms"),
        "810-both-arms",
        &format!(
            "references:\n  - role: derivation\n    unit: {{ kind: file, path: README.md }}\n    provenance:\n      kind: knowledge\n      ref: \"stagecraft://project/{PROJ}/knowledge/{KNOWLEDGE}\"\n",
        ),
        "# Both arms\n",
    );
    // Ensure README.md exists so unit-arm doesn't trigger V-023.
    fs::write(root.join("README.md"), "fixture\n").unwrap();
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-025");
    assert_eq!(vs.len(), 1, "expected one V-025: {:?}", reg["validation"]["violations"]);
    assert_eq!(vs[0]["severity"].as_str(), Some("error"));
    assert!(vs[0]["message"].as_str().unwrap().contains("mutually exclusive"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-025 — references entry carrying neither arm.
#[test]
fn v025_fires_on_neither_arm() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/811-neither-arm")).unwrap();
    write_spec(
        &root.join("specs/811-neither-arm"),
        "811-neither-arm",
        "references:\n  - role: evidence\n",
        "# Neither arm\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-025");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"].as_str().unwrap().contains("neither"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-026 — `provenance.kind:` is not in the closed enum.
#[test]
fn v026_fires_on_unknown_kind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/812-unknown-prov-kind")).unwrap();
    write_spec(
        &root.join("specs/812-unknown-prov-kind"),
        "812-unknown-prov-kind",
        "references:\n  - role: derivation\n    provenance:\n      kind: sbom\n      ref: \"sbom://something\"\n",
        "# Unknown kind\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-026");
    assert_eq!(vs.len(), 1, "expected one V-026: {:?}", reg["validation"]["violations"]);
    assert!(vs[0]["message"].as_str().unwrap().contains("sbom"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-026 — `provenance:` value missing the `kind:` field entirely.
#[test]
fn v026_fires_on_missing_kind_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/813-missing-kind")).unwrap();
    write_spec(
        &root.join("specs/813-missing-kind"),
        "813-missing-kind",
        "references:\n  - role: derivation\n    provenance:\n      ref: \"stagecraft://project/aa/knowledge/bb\"\n",
        "# Missing kind\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-026");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"].as_str().unwrap().contains("`kind:`"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-027 — kind/scheme mismatch: `knowledge` paired with the xray scheme.
#[test]
fn v027_fires_on_scheme_mismatch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/814-scheme-mismatch")).unwrap();
    write_spec(
        &root.join("specs/814-scheme-mismatch"),
        "814-scheme-mismatch",
        &format!(
            "references:\n  - role: derivation\n    provenance:\n      kind: knowledge\n      ref: \"xray-fingerprint://{DIGEST}\"\n",
        ),
        "# Scheme mismatch\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-027");
    assert_eq!(vs.len(), 1);
    let msg = vs[0]["message"].as_str().unwrap();
    assert!(msg.contains("xray-fingerprint"), "message: {msg}");
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-027 — knowledge URI without the required project/knowledge segment.
#[test]
fn v027_fires_on_missing_project_segment() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/815-missing-segment")).unwrap();
    write_spec(
        &root.join("specs/815-missing-segment"),
        "815-missing-segment",
        &format!(
            "references:\n  - role: derivation\n    provenance:\n      kind: knowledge\n      ref: \"stagecraft://item/{PROJ}\"\n",
        ),
        "# Missing segment\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-027");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"].as_str().unwrap().contains("/project/"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-028 — knowledge URI carrying values that are not canonical UUIDs.
#[test]
fn v028_fires_on_malformed_knowledge_uuid() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/816-bad-uuid")).unwrap();
    write_spec(
        &root.join("specs/816-bad-uuid"),
        "816-bad-uuid",
        "references:\n  - role: derivation\n    provenance:\n      kind: knowledge\n      ref: \"stagecraft://project/not-a-uuid/knowledge/also-not\"\n",
        "# Bad UUIDs\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-028");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"].as_str().unwrap().contains("not-a-uuid"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-028 — code-fingerprint URI carrying a digest that is not 64 hex chars.
#[test]
fn v028_fires_on_short_digest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/817-short-digest")).unwrap();
    write_spec(
        &root.join("specs/817-short-digest"),
        "817-short-digest",
        "references:\n  - role: derivation\n    provenance:\n      kind: code-fingerprint\n      ref: \"xray-fingerprint://abc123\"\n",
        "# Short digest\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-028");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"].as_str().unwrap().contains("abc123"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-028 — code-fingerprint URI with empty body.
#[test]
fn v028_fires_on_empty_ref_body() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/818-empty-body")).unwrap();
    write_spec(
        &root.join("specs/818-empty-body"),
        "818-empty-body",
        "references:\n  - role: derivation\n    provenance:\n      kind: code-fingerprint\n      ref: \"xray-fingerprint://\"\n",
        "# Empty body\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-028");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"].as_str().unwrap().contains("empty"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-029 — provenance entry without `role:` emits an advisory warning;
/// validation.passed remains true.
#[test]
fn v029_advisory_fires_on_missing_role_but_does_not_block() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/819-no-role")).unwrap();
    write_spec(
        &root.join("specs/819-no-role"),
        "819-no-role",
        &format!(
            "references:\n  - provenance:\n      kind: knowledge\n      ref: \"stagecraft://project/{PROJ}/knowledge/{KNOWLEDGE}\"\n",
        ),
        "# No role advisory\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-029");
    assert_eq!(vs.len(), 1, "expected one V-029: {:?}", reg["validation"]["violations"]);
    assert_eq!(vs[0]["severity"].as_str(), Some("warning"));
    assert!(vs[0]["message"].as_str().unwrap().contains("role: derivation"));
    // Advisory must not flip validation.passed.
    assert_eq!(
        reg["validation"]["passed"].as_bool(),
        Some(true),
        "V-029 is advisory; validation.passed should remain true"
    );
}

/// Positive case — a clean knowledge provenance entry parses and the
/// emitted registry shape carries `provenance.kind` + `provenance.ref`.
#[test]
fn knowledge_provenance_emits_canonical_shape() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/820-knowledge-ok")).unwrap();
    write_spec(
        &root.join("specs/820-knowledge-ok"),
        "820-knowledge-ok",
        &format!(
            "references:\n  - role: derivation\n    provenance:\n      kind: knowledge\n      ref: \"stagecraft://project/{PROJ}/knowledge/{KNOWLEDGE}\"\n",
        ),
        "# Knowledge OK\n",
    );
    let reg = compile(root);
    assert!(violations_with_code(&reg, "V-025").is_empty());
    assert!(violations_with_code(&reg, "V-026").is_empty());
    assert!(violations_with_code(&reg, "V-027").is_empty());
    assert!(violations_with_code(&reg, "V-028").is_empty());
    assert!(violations_with_code(&reg, "V-029").is_empty());
    let feat = find_feature(&reg, "820-knowledge-ok");
    let refs = feat["references"].as_array().expect("references emitted");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0]["role"].as_str(), Some("derivation"));
    assert_eq!(refs[0]["provenance"]["kind"].as_str(), Some("knowledge"));
    assert_eq!(
        refs[0]["provenance"]["ref"].as_str(),
        Some(format!("stagecraft://project/{PROJ}/knowledge/{KNOWLEDGE}").as_str())
    );
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
}

/// Positive case — a clean code-fingerprint provenance entry parses and
/// the emitted shape carries the canonical xray scheme URI.
#[test]
fn code_fingerprint_provenance_emits_canonical_shape() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/821-fingerprint-ok")).unwrap();
    write_spec(
        &root.join("specs/821-fingerprint-ok"),
        "821-fingerprint-ok",
        &format!(
            "references:\n  - role: derivation\n    provenance:\n      kind: code-fingerprint\n      ref: \"xray-fingerprint://{DIGEST}\"\n",
        ),
        "# Fingerprint OK\n",
    );
    let reg = compile(root);
    assert!(violations_with_code(&reg, "V-025").is_empty());
    assert!(violations_with_code(&reg, "V-026").is_empty());
    assert!(violations_with_code(&reg, "V-027").is_empty());
    assert!(violations_with_code(&reg, "V-028").is_empty());
    assert!(violations_with_code(&reg, "V-029").is_empty());
    let feat = find_feature(&reg, "821-fingerprint-ok");
    let refs = feat["references"].as_array().expect("references emitted");
    assert_eq!(refs.len(), 1);
    assert_eq!(
        refs[0]["provenance"]["kind"].as_str(),
        Some("code-fingerprint")
    );
    assert_eq!(
        refs[0]["provenance"]["ref"].as_str(),
        Some(format!("xray-fingerprint://{DIGEST}").as_str())
    );
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
}
