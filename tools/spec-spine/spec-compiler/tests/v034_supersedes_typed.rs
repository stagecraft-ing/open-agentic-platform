//! V-034 (spec 216 Phase 2a): `supersedes` is typed (full | partial).
//!
//! OAP previously routed `supersedes` through `optional_string_list`, which
//! silently dropped every object entry, discarding the structured partial
//! form (`{spec, scope: partial, unit?, note?, rationale?}`) that the
//! standalone spec-spine library honours. Phase 2a replaces that with a typed
//! parse: full-scope entries normalise to bare ids, partial entries emit the
//! structured form, and a malformed envelope is a hard V-034 error.

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
created: "2026-06-15"
summary: "V-034 fixture spec."
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

/// AC-007: a partial entry with a `unit:` compiles and the registry records
/// the structured partial form verbatim (previously dropped to null).
#[test]
fn v034_partial_unit_compiles_and_records() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/990-partial-unit")).unwrap();
    // Create the referenced directory so the unit's existence check is clean.
    fs::create_dir_all(root.join("platform")).unwrap();
    write_spec(
        &root.join("specs/990-partial-unit"),
        "990-partial-unit",
        "supersedes:\n  - spec: \"001-spec-compiler-mvp\"\n    scope: partial\n    unit: { kind: directory, path: platform }\n",
    );

    let reg = compile(root);
    assert!(
        collect_violations(&reg, "V-034").is_empty(),
        "well-formed partial supersedes must not trip V-034"
    );
    let s = &feature(&reg, "990-partial-unit")["supersedes"];
    let arr = s.as_array().expect("supersedes recorded as array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["spec"].as_str(), Some("001-spec-compiler-mvp"));
    assert_eq!(arr[0]["scope"].as_str(), Some("partial"));
    assert_eq!(arr[0]["unit"]["kind"].as_str(), Some("directory"));
    assert_eq!(arr[0]["unit"]["path"].as_str(), Some("platform"));
}

/// AC-009 (partial-note): a partial entry scoped by a prose `note:` with no
/// `unit:` is valid and records structured (partial does NOT require unit).
#[test]
fn v034_partial_note_only_is_clean() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/991-partial-note")).unwrap();
    write_spec(
        &root.join("specs/991-partial-note"),
        "991-partial-note",
        "supersedes:\n  - spec: \"001-spec-compiler-mvp\"\n    scope: partial\n    note: \"retires the legacy read-time translation layer only\"\n",
    );

    let reg = compile(root);
    assert!(
        collect_violations(&reg, "V-034").is_empty(),
        "partial-by-note (no unit) must not trip V-034"
    );
    let s = &feature(&reg, "991-partial-note")["supersedes"];
    let arr = s.as_array().expect("supersedes recorded");
    assert_eq!(arr[0]["scope"].as_str(), Some("partial"));
    assert!(arr[0]["note"].as_str().unwrap().contains("read-time"));
    assert!(arr[0].get("unit").is_none(), "note-only entry has no unit");
}

/// AC-008: every full-scope form (bare string, `{spec}`, `{spec, scope: full}`)
/// compiles and emits the byte-identical bare-string `["001-..."]`.
#[test]
fn v034_full_forms_normalise_to_bare_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let cases = [
        ("992-full-bare", "supersedes:\n  - \"001-spec-compiler-mvp\"\n"),
        ("993-full-spec-only", "supersedes:\n  - spec: \"001-spec-compiler-mvp\"\n"),
        (
            "994-full-explicit",
            "supersedes:\n  - spec: \"001-spec-compiler-mvp\"\n    scope: full\n",
        ),
    ];
    for (id, fm) in cases {
        fs::create_dir_all(root.join(format!("specs/{id}"))).unwrap();
        write_spec(&root.join(format!("specs/{id}")), id, fm);
    }

    let reg = compile(root);
    assert!(
        collect_violations(&reg, "V-034").is_empty(),
        "full-scope forms must not trip V-034"
    );
    let expected = Value::Array(vec![Value::String("001-spec-compiler-mvp".to_string())]);
    for (id, _) in cases {
        assert_eq!(
            feature(&reg, id)["supersedes"],
            expected,
            "{id}: full scope must normalise to a bare-string list"
        );
    }
}

/// AC-009: each malformed envelope fails compile with V-034.
#[test]
fn v034_rejects_malformed_envelopes() {
    let cases: [(&str, &str); 4] = [
        // missing required `spec`
        ("995-missing-spec", "supersedes:\n  - scope: full\n"),
        // scope outside {full, partial}
        (
            "996-bad-scope",
            "supersedes:\n  - spec: \"001-x\"\n    scope: bogus\n",
        ),
        // unknown envelope key (`paths:` is the retired pre-154 form)
        (
            "997-unknown-key",
            "supersedes:\n  - spec: \"001-x\"\n    scope: partial\n    paths: [\"a/b.rs\"]\n",
        ),
        // entry is neither a string nor an object
        ("998-non-string", "supersedes:\n  - 42\n"),
    ];
    for (id, fm) in cases {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(format!("specs/{id}"))).unwrap();
        write_spec(&root.join(format!("specs/{id}")), id, fm);

        let reg = compile(root);
        let v034 = collect_violations(&reg, "V-034");
        assert!(
            !v034.is_empty(),
            "{id}: malformed supersedes must trip V-034; got none"
        );
        assert_eq!(v034[0]["severity"].as_str(), Some("error"));
        // Malformed field dropped so the registry stays schema-conformant.
        assert!(
            feature(&reg, id).get("supersedes").is_none(),
            "{id}: malformed supersedes must not be recorded"
        );
    }
}
