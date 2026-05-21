//! Spec 154 (logical-unit ownership grammar) — negative cases.
//!
//! Synthetic-fixture tests for the four type-check violations the
//! compiler emits against explicitly-kinded units that fail
//! resolver-free validation (V-021 unknown crate, V-022 missing
//! directory, V-023 missing file, V-024 malformed unit shape).
//! Companion positive cases live in `spec154_unit_grammar.rs`.

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
summary: "Spec 154 unit-grammar negative fixture."
{frontmatter_extra}---
{body}
"#
    );
    fs::write(path, raw).unwrap();
}

fn write_workspace(root: &Path, members: &[(&str, &str)]) {
    let toml_members = members
        .iter()
        .map(|(dir, _)| format!("    {dir:?},"))
        .collect::<Vec<_>>()
        .join("\n");
    let manifest = format!(
        "[workspace]\nresolver = \"2\"\nmembers = [\n{toml_members}\n]\n"
    );
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    for (dir, name) in members {
        let crate_root = root.join(dir);
        fs::create_dir_all(crate_root.join("src")).unwrap();
        let manifest = format!(
            "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
        );
        fs::write(crate_root.join("Cargo.toml"), manifest).unwrap();
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

/// V-021 — `crate:` unit references an id not present in `[workspace]
/// members` manifest names.
#[test]
fn v021_fires_on_unknown_crate_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write_workspace(root, &[("crates/foo", "foo-crate")]);
    fs::create_dir_all(root.join("specs/800-bad-crate")).unwrap();
    write_spec(
        &root.join("specs/800-bad-crate"),
        "800-bad-crate",
        "establishes:\n  - unit: { kind: crate, id: nonexistent-crate }\n",
        "# Bad crate\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-021");
    assert_eq!(vs.len(), 1, "expected one V-021: {:?}", reg["validation"]["violations"]);
    assert_eq!(vs[0]["severity"].as_str(), Some("error"));
    assert!(vs[0]["message"]
        .as_str()
        .unwrap()
        .contains("nonexistent-crate"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-022 — `directory:` unit references a path that does not exist
/// in the worktree.
#[test]
fn v022_fires_on_missing_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/801-bad-dir")).unwrap();
    write_spec(
        &root.join("specs/801-bad-dir"),
        "801-bad-dir",
        "establishes:\n  - unit: { kind: directory, path: nope/does/not/exist }\n",
        "# Bad directory\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-022");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"]
        .as_str()
        .unwrap()
        .contains("nope/does/not/exist"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-023 — explicitly-kinded `file:` unit references a path that does
/// not exist as a file.
#[test]
fn v023_fires_on_missing_file_with_explicit_kind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/802-bad-file")).unwrap();
    write_spec(
        &root.join("specs/802-bad-file"),
        "802-bad-file",
        "establishes:\n  - unit: { kind: file, path: not-here.toml }\n",
        "# Bad file\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-023");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"].as_str().unwrap().contains("not-here.toml"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-024 — unit mapping with an unknown `kind:` value.
#[test]
fn v024_fires_on_unknown_kind() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/803-bad-kind")).unwrap();
    write_spec(
        &root.join("specs/803-bad-kind"),
        "803-bad-kind",
        "establishes:\n  - unit: { kind: invented-kind, id: foo }\n",
        "# Unknown kind\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-024");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"]
        .as_str()
        .unwrap()
        .contains("invented-kind"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-024 — unit mapping with a known `kind:` but missing a required
/// field (here: `crate:` without `id:`).
#[test]
fn v024_fires_on_missing_required_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write_workspace(root, &[("crates/foo", "foo-crate")]);
    fs::create_dir_all(root.join("specs/804-missing-field")).unwrap();
    write_spec(
        &root.join("specs/804-missing-field"),
        "804-missing-field",
        "establishes:\n  - unit: { kind: crate }\n",
        "# Missing id\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-024");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"].as_str().unwrap().contains("`id:`"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// Bare-string `establishes:` entries do NOT fire V-023 even when
/// they reference non-existent paths — the legacy authoring channel
/// is unvalidated during the compat window. (Companion to the
/// positive `bare_string_establishes_does_not_fire_v023` test.)
#[test]
fn bare_strings_remain_unvalidated() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/805-bare-string-unvalidated")).unwrap();
    write_spec(
        &root.join("specs/805-bare-string-unvalidated"),
        "805-bare-string-unvalidated",
        "establishes:\n  - \"some/legacy/missing.rs\"\n",
        "# Bare-string legacy\n",
    );
    let reg = compile(root);
    assert!(violations_with_code(&reg, "V-023").is_empty());
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(true));
}
