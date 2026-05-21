//! Spec 154 (logical-unit ownership grammar) — positive parser cases.
//!
//! Exercises the eight accepted shapes documented in spec 154 §5
//! and the backwards-compatibility window §8 in synthetic spec
//! fixtures. Negative cases (V-021..V-024 firing on malformed or
//! unresolvable declarations) live in
//! `spec154_unit_grammar_negative.rs`.

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
created: "2026-05-21"
summary: "Spec 154 unit-grammar fixture."
{frontmatter_extra}---
{body}
"#
    );
    fs::write(path, raw).unwrap();
}

fn write_synthetic_workspace(root: &Path, member_dirs: &[(&str, &str)]) {
    let members_toml = member_dirs
        .iter()
        .map(|(dir, _name)| format!("    {dir:?},"))
        .collect::<Vec<_>>()
        .join("\n");
    let manifest = format!(
        "[workspace]\nresolver = \"2\"\nmembers = [\n{members_toml}\n]\n"
    );
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    for (dir, name) in member_dirs {
        let crate_root = root.join(dir);
        fs::create_dir_all(crate_root.join("src")).unwrap();
        let member_manifest = format!(
            "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
        );
        fs::write(crate_root.join("Cargo.toml"), member_manifest).unwrap();
        fs::write(crate_root.join("src/lib.rs"), "// fixture\n").unwrap();
    }
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

/// §3.1 — `crate:` unit with valid workspace-member id parses cleanly
/// and surfaces no V-021.
#[test]
fn crate_unit_with_valid_id_passes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write_synthetic_workspace(root, &[("crates/foo", "foo-crate")]);
    fs::create_dir_all(root.join("specs/700-crate-unit")).unwrap();
    write_spec(
        &root.join("specs/700-crate-unit"),
        "700-crate-unit",
        "establishes:\n  - unit: { kind: crate, id: foo-crate }\n",
        "# Crate unit\n",
    );
    let reg = compile(root);
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
    assert!(violations_with_code(&reg, "V-021").is_empty());
}

/// §3.5 — `directory:` unit pointing at an extant directory passes.
#[test]
fn directory_unit_existing_passes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("infra/clusters/dev")).unwrap();
    fs::create_dir_all(root.join("specs/701-dir-unit")).unwrap();
    write_spec(
        &root.join("specs/701-dir-unit"),
        "701-dir-unit",
        "establishes:\n  - unit: { kind: directory, path: infra/clusters/dev }\n",
        "# Directory unit\n",
    );
    let reg = compile(root);
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
    assert!(violations_with_code(&reg, "V-022").is_empty());
}

/// §3.6 — `file:` unit pointing at an extant file passes.
#[test]
fn file_unit_existing_passes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::write(root.join("deny.toml"), "# fixture\n").unwrap();
    fs::create_dir_all(root.join("specs/702-file-unit")).unwrap();
    write_spec(
        &root.join("specs/702-file-unit"),
        "702-file-unit",
        "establishes:\n  - unit: { kind: file, path: deny.toml }\n",
        "# File unit\n",
    );
    let reg = compile(root);
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
    assert!(violations_with_code(&reg, "V-023").is_empty());
}

/// §3.2 / §3.3 — `symbol:` and `module:` units parse cleanly without
/// emitting V-021..V-023; their existence checks are deferred to
/// Segment 3 (the codebase-indexer resolver).
#[test]
fn symbol_and_module_units_defer_resolution() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/703-symbol-module")).unwrap();
    write_spec(
        &root.join("specs/703-symbol-module"),
        "703-symbol-module",
        concat!(
            "establishes:\n",
            "  - unit: { kind: symbol, id: foo_crate::bar::baz }\n",
            "  - unit: { kind: module, id: foo_crate::bar }\n",
        ),
        "# Symbol/module units\n",
    );
    let reg = compile(root);
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
    assert!(violations_with_code(&reg, "V-021").is_empty());
    assert!(violations_with_code(&reg, "V-022").is_empty());
    assert!(violations_with_code(&reg, "V-023").is_empty());
}

/// §3.4 — `section:` unit on a Makefile-shaped file (anchor resolution
/// itself lands in Segment 3, but the parse-shape is accepted today).
#[test]
fn section_unit_parses() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::write(root.join("Makefile"), "deploy:\n\techo deploy\n").unwrap();
    fs::create_dir_all(root.join("specs/704-section-unit")).unwrap();
    write_spec(
        &root.join("specs/704-section-unit"),
        "704-section-unit",
        "co_authority:\n  - with_specs: [\"700-other\"]\n    unit: { kind: section, file: Makefile, anchor: deploy }\n",
        "# Section unit\n",
    );
    let reg = compile(root);
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
}

/// §4 — `references:` is first-class. Bare strings normalise into
/// `{unit: {kind: file, path}}`; structured items preserve `role:`.
#[test]
fn references_first_class_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/705-references")).unwrap();
    write_spec(
        &root.join("specs/705-references"),
        "705-references",
        concat!(
            "references:\n",
            "  - \"some/legacy/path\"\n",
            "  - role: evidence\n",
            "    unit: { kind: symbol, id: foo_crate::bar }\n",
        ),
        "# References field\n",
    );
    let reg = compile(root);
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
    let feat = find_feature(&reg, "705-references");
    let refs = feat["references"].as_array().expect("references emitted");
    assert_eq!(refs.len(), 2);
    // Bare-string references stay as strings in the registry so the
    // typed reader can route them through the legacy code path.
    assert_eq!(refs[0].as_str(), Some("some/legacy/path"));
    // Structured items keep their full shape.
    assert_eq!(refs[1]["role"].as_str(), Some("evidence"));
    assert_eq!(refs[1]["unit"]["kind"].as_str(), Some("symbol"));
}

/// §8 — bare-string entries inside `establishes:` continue to parse
/// without firing V-023, even when they don't resolve to a file
/// (compat window — the legacy authoring channel doesn't validate
/// existence; explicit `{kind: file, ...}` does).
#[test]
fn bare_string_establishes_does_not_fire_v023() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/706-legacy-bare-string")).unwrap();
    write_spec(
        &root.join("specs/706-legacy-bare-string"),
        "706-legacy-bare-string",
        "establishes:\n  - \"path/that/does/not/exist.rs\"\n",
        "# Legacy bare-string\n",
    );
    let reg = compile(root);
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
    assert!(violations_with_code(&reg, "V-023").is_empty());
}

/// §5 — `constrains:` accepts both legacy `kind:` and new `flavor:`
/// discriminator without splitting the spec. Parser is tolerant of
/// both forms during the compat window.
#[test]
fn constrains_accepts_both_kind_and_flavor() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/707-legacy-kind")).unwrap();
    fs::create_dir_all(root.join("specs/708-new-flavor")).unwrap();
    write_spec(
        &root.join("specs/707-legacy-kind"),
        "707-legacy-kind",
        "constrains:\n  - kind: invariant-freeze\n    paths:\n      - some/file.json\n",
        "# Legacy kind discriminator\n",
    );
    write_spec(
        &root.join("specs/708-new-flavor"),
        "708-new-flavor",
        "constrains:\n  - flavor: invariant-freeze\n    paths:\n      - some/file.json\n",
        "# New flavor discriminator\n",
    );
    let reg = compile(root);
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
}

/// Mixed lists — legacy + new in one relationship field — parse cleanly.
#[test]
fn establishes_mixed_legacy_and_typed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write_synthetic_workspace(root, &[("crates/foo", "foo-crate")]);
    fs::create_dir_all(root.join("specs/709-mixed")).unwrap();
    write_spec(
        &root.join("specs/709-mixed"),
        "709-mixed",
        concat!(
            "establishes:\n",
            "  - \"legacy/path.rs\"\n",
            "  - unit: { kind: crate, id: foo-crate }\n",
            "  - unit: { kind: symbol, id: foo::bar }\n",
        ),
        "# Mixed list\n",
    );
    let reg = compile(root);
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
}

/// Legacy `co_authority.section:` synthesis path — parser accepts the
/// legacy form during the compat window.
#[test]
fn legacy_co_authority_section_synthesis() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::write(root.join("Makefile"), "deploy:\n\techo deploy\n").unwrap();
    fs::create_dir_all(root.join("specs/710-legacy-co-authority")).unwrap();
    write_spec(
        &root.join("specs/710-legacy-co-authority"),
        "710-legacy-co-authority",
        concat!(
            "co_authority:\n",
            "  - paths:\n",
            "      - Makefile\n",
            "    section: deploy\n",
            "    with_specs: [\"700-other\"]\n",
        ),
        "# Legacy co_authority\n",
    );
    let reg = compile(root);
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
}
