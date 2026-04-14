//! Machine-check emitted JSON against codebase-index.schema.json.

use jsonschema::validator_for;
use serde_json::Value;
use std::fs;

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn load_schema(relative_to_repo: &str) -> Value {
    let path = repo_root().join(relative_to_repo);
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("read {}: {e}", path.display());
    });
    let mut v: Value = serde_json::from_str(&raw).expect("schema JSON");
    if let Some(o) = v.as_object_mut() {
        o.remove("$schema");
    }
    v
}

#[test]
fn compile_output_matches_codebase_index_schema() {
    let root = repo_root();
    let out = open_agentic_codebase_indexer::compile(&root).expect("compile");
    let schema = load_schema("schemas/codebase-index.schema.json");
    let validator = validator_for(&schema).expect("compile codebase-index.schema.json");
    let instance: Value = serde_json::from_slice(&out.index_json).expect("index JSON");
    if let Err(e) = validator.validate(&instance) {
        panic!("index.json does not validate: {e}");
    }
}

/// Minimal fixture: one Cargo.toml, one spec.md — ensures schema conformance
/// is not coupled solely to the full monorepo.
#[test]
fn fixture_repo_conforms_to_schema() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    // Create minimal spec
    let spec_dir = root.join("specs/099-fixture/spec.md");
    fs::create_dir_all(spec_dir.parent().unwrap()).unwrap();
    fs::write(
        &spec_dir,
        r#"---
id: "099-fixture"
title: "Fixture spec"
status: draft
created: "2026-04-14"
summary: "Minimal spec for codebase-indexer fixture test."
---
# Fixture spec

## Section
"#,
    )
    .unwrap();

    // Create minimal Cargo.toml
    let crate_dir = root.join("crates/test-crate");
    fs::create_dir_all(crate_dir.join("src")).unwrap();
    fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();
    fs::write(crate_dir.join("src/lib.rs"), "").unwrap();

    // Create workspace Cargo.toml
    fs::write(
        root.join("crates/Cargo.toml"),
        r#"[workspace]
members = ["test-crate"]
"#,
    )
    .unwrap();

    // Copy the schema so self-validation works
    let schemas_dir = root.join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();
    let real_schema = repo_root().join("schemas/codebase-index.schema.json");
    fs::copy(&real_schema, schemas_dir.join("codebase-index.schema.json")).unwrap();

    let out = open_agentic_codebase_indexer::compile(root).expect("compile fixture");

    let schema = load_schema("schemas/codebase-index.schema.json");
    let validator = validator_for(&schema).expect("schema");
    let instance: Value = serde_json::from_slice(&out.index_json).expect("index JSON");
    if let Err(e) = validator.validate(&instance) {
        panic!("fixture index.json does not validate: {e}");
    }
}
