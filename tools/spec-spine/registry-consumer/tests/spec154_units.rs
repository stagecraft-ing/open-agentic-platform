//! Spec 154 — typed-reader surface for `LogicalUnit` / `UnitEdge`.
//!
//! Verifies that the new `Feature::units()` API exposes both
//! explicitly-declared units (the new authoring channel) and
//! legacy-derived units (synthesised from bare-string paths and
//! legacy `paths:` lists), with the `legacy` flag correctly set so
//! downstream consumers — notably the coupling gate in Tier 2
//! Segment 4 — can prefer typed authority where it has been
//! declared.

use open_agentic_spec_registry_reader::{LogicalUnit, load};
use std::fs;
use std::path::Path;

fn run_spec_compiler(repo_root: &Path) {
    let out = open_agentic_spec_compiler::compile_and_write(repo_root).expect("compile");
    assert!(out.validation_passed, "synthetic corpus must validate");
}

fn write_synthetic_workspace(root: &Path, members: &[(&str, &str)]) {
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
        let member_manifest = format!(
            "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n"
        );
        fs::write(crate_root.join("Cargo.toml"), member_manifest).unwrap();
        fs::write(crate_root.join("src/lib.rs"), "// fixture\n").unwrap();
    }
}

fn write_spec(dir: &Path, id: &str, frontmatter_extra: &str) {
    let path = dir.join("spec.md");
    let raw = format!(
        r#"---
id: "{id}"
title: "Fixture for {id}"
status: draft
created: "2026-05-21"
summary: "Spec 154 typed-reader fixture."
{frontmatter_extra}---
# Body
"#
    );
    fs::write(path, raw).unwrap();
}

#[test]
fn units_extracts_explicit_with_legacy_references() {
    // Spec 154 Segment 6: bare-string entries are rejected on the
    // owning fields (establishes/extends/refines/...) but remain
    // accepted on `references:` per 4' (references is non-owning by
    // design — see spec 154 §4). The `legacy` flag persists for the
    // remaining bare-string `references:` channel so downstream
    // consumers can distinguish surface syntax even after the
    // owning-field excision.
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write_synthetic_workspace(root, &[("crates/foo", "foo-crate")]);
    fs::create_dir_all(root.join("specs/900-mixed-units")).unwrap();
    write_spec(
        &root.join("specs/900-mixed-units"),
        "900-mixed-units",
        concat!(
            "establishes:\n",
            "  - unit: { kind: crate, id: foo-crate }\n",
            "references:\n",
            "  - \"some/legacy/ref\"\n",
            "  - role: evidence\n",
            "    unit: { kind: symbol, id: foo::bar }\n",
        ),
    );

    run_spec_compiler(root);
    let registry = load(&root.join(".derived/spec-registry/registry.json")).expect("load");
    let feature = registry
        .find_by_id("900-mixed-units")
        .expect("feature in registry");
    let edges = feature.units();

    // Three edges in declaration order: explicit crate, legacy
    // reference file (bare-string in references is still valid), explicit
    // symbol reference.
    assert_eq!(edges.len(), 3, "got {:?}", edges);

    assert_eq!(edges[0].relationship, "establishes");
    assert_eq!(
        edges[0].unit,
        LogicalUnit::Crate {
            id: "foo-crate".into()
        }
    );
    assert!(!edges[0].legacy, "explicit unit declarations are not legacy");

    assert_eq!(edges[1].relationship, "references");
    assert_eq!(
        edges[1].unit,
        LogicalUnit::File {
            path: "some/legacy/ref".into()
        }
    );
    assert!(edges[1].legacy);
    assert!(edges[1].role.is_none());

    assert_eq!(edges[2].relationship, "references");
    assert_eq!(
        edges[2].unit,
        LogicalUnit::Symbol {
            id: "foo::bar".into()
        }
    );
    assert!(!edges[2].legacy);
    assert_eq!(edges[2].role.as_deref(), Some("evidence"));
}

#[test]
fn legacy_paths_lists_now_rejected() {
    // Spec 154 Segment 6: the `extends.paths` / `refines.paths` /
    // `co_authority.paths` legacy sub-list shapes are excised. Items
    // must be authored as typed `unit:` mappings.
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join("specs/901-other")).unwrap();
    write_spec(
        &root.join("specs/901-other"),
        "901-other",
        "establishes:\n  - unit: { kind: file, path: some/path.rs }\n",
    );
    fs::create_dir_all(root.join("specs/902-legacy-paths")).unwrap();
    write_spec(
        &root.join("specs/902-legacy-paths"),
        "902-legacy-paths",
        concat!(
            "extends:\n",
            "  - spec: \"901-other\"\n",
            "    paths:\n",
            "      - \"a/b.rs\"\n",
            "      - \"a/c.rs\"\n",
        ),
    );

    // Compile but allow non-passing validation — we expect rejection.
    let out = open_agentic_spec_compiler::compile_and_write(root).expect("compile");
    assert!(
        !out.validation_passed,
        "compiler must reject legacy `extends.paths:` shape post-excision"
    );
    let registry = load(&root.join(".derived/spec-registry/registry.json")).expect("load");
    assert!(!registry.validation.passed);
}
