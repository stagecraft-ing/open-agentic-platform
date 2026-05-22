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

/// Segment 6 excision: bare-string `establishes:` entries are no
/// longer accepted at all. V-023 still doesn't fire (because no
/// `file:` unit was successfully parsed) — V-024 fires at parse time
/// instead. This test pins the post-excision contract: bare strings
/// emit V-024 (not V-023) and fail validation.
#[test]
fn bare_strings_fire_v024_post_excision() {
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
    assert!(
        violations_with_code(&reg, "V-023").is_empty(),
        "V-023 (file-existence) does not fire — bare strings never produce a parsed file: unit"
    );
    assert_eq!(
        violations_with_code(&reg, "V-024").len(),
        1,
        "V-024 (malformed unit) fires at parse time on bare-string establishes"
    );
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-024 (spec 155 §2.1) — `kind: symbol` id containing generic
/// type-parameter syntax is rejected at parse time.
#[test]
fn v024_fires_on_symbol_id_with_generics() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/806-symbol-generics")).unwrap();
    write_spec(
        &root.join("specs/806-symbol-generics"),
        "806-symbol-generics",
        "establishes:\n  - unit: { kind: symbol, id: \"Foo<T>\" }\n",
        "# Symbol with generic\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-024");
    assert_eq!(vs.len(), 1);
    let msg = vs[0]["message"].as_str().unwrap();
    assert!(msg.contains("Foo<T>"), "message should name the bad id: {msg}");
    assert!(
        msg.contains("`<`") || msg.contains("`>`"),
        "message should name the rule: {msg}"
    );
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// V-024 (spec 155 §2.1) — `kind: symbol` id containing lifetime
/// syntax is rejected at parse time (lifetimes are not part of
/// item-path identity any more than generic parameters are).
#[test]
fn v024_fires_on_symbol_id_with_lifetime() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/807-symbol-lifetime")).unwrap();
    write_spec(
        &root.join("specs/807-symbol-lifetime"),
        "807-symbol-lifetime",
        "establishes:\n  - unit: { kind: symbol, id: \"Foo<'a>\" }\n",
        "# Symbol with lifetime\n",
    );
    let reg = compile(root);
    let vs = violations_with_code(&reg, "V-024");
    assert_eq!(vs.len(), 1);
    assert!(vs[0]["message"].as_str().unwrap().contains("Foo<'a>"));
    assert_eq!(reg["validation"]["passed"].as_bool(), Some(false));
}

/// Negative control — a cleanly-pathed symbol id does NOT trigger
/// V-024. Asserts the predicate isn't accidentally rejecting valid
/// item paths.
#[test]
fn v024_does_not_fire_on_clean_symbol_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/808-symbol-clean")).unwrap();
    write_spec(
        &root.join("specs/808-symbol-clean"),
        "808-symbol-clean",
        "establishes:\n  - unit: { kind: symbol, id: canonical_json::canonicalize_value }\n",
        "# Clean symbol path\n",
    );
    let reg = compile(root);
    assert!(violations_with_code(&reg, "V-024").is_empty());
}

/// Negative control — a symbol id containing underscores and digits
/// (legal Rust identifier characters) does NOT trigger V-024. Guards
/// against an over-broad predicate that might reject identifier
/// content beyond `<` / `>`.
#[test]
fn v024_does_not_fire_on_symbol_with_underscore_and_digit() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/809-symbol-underscore-digit")).unwrap();
    write_spec(
        &root.join("specs/809-symbol-underscore-digit"),
        "809-symbol-underscore-digit",
        "establishes:\n  - unit: { kind: symbol, id: foo_2::bar_baz_3 }\n",
        "# Symbol with underscore and digit\n",
    );
    let reg = compile(root);
    assert!(violations_with_code(&reg, "V-024").is_empty());
}
