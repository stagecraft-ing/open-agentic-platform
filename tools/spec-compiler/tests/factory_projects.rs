//! Integration tests for Factory Build Spec discovery and indexing (074 FR-007).

use serde_json::Value;
use std::fs;

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// When a `.factory/build-spec.yaml` exists, the compiler emits a `factoryProjects` array.
#[test]
fn factory_project_indexed_in_registry() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    // Minimal spec so compilation succeeds.
    let spec_dir = root.join("specs/099-test");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("spec.md"),
        r#"---
id: "099-test"
title: "Test"
status: draft
created: "2026-04-14"
summary: "Fixture for factory project test."
---
# Test
"#,
    )
    .unwrap();

    // Factory build spec in a project subdirectory.
    let factory_dir = root.join("projects/my-app/.factory");
    fs::create_dir_all(&factory_dir).unwrap();
    fs::write(
        factory_dir.join("build-spec.yaml"),
        "project:\n  name: my-app\n  variant: dual\n  org: acme\n",
    )
    .unwrap();

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).unwrap();

    let factory_projects = v["factoryProjects"]
        .as_array()
        .expect("factoryProjects should be an array");
    assert_eq!(factory_projects.len(), 1);

    let proj = &factory_projects[0];
    assert_eq!(proj["projectName"], "my-app");
    assert_eq!(proj["variant"], "dual");
    assert_eq!(proj["org"], "acme");
    assert_eq!(
        proj["buildSpecPath"],
        "projects/my-app/.factory/build-spec.yaml"
    );
    // Content hash is a 64-char hex string.
    let hash = proj["contentHash"].as_str().unwrap();
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

/// When no `.factory/build-spec.yaml` exists, `factoryProjects` is absent (opt-in).
#[test]
fn factory_projects_absent_when_no_build_specs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    let spec_dir = root.join("specs/099-test");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("spec.md"),
        r#"---
id: "099-test"
title: "Test"
status: draft
created: "2026-04-14"
summary: "No factory project here."
---
# Test
"#,
    )
    .unwrap();

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).unwrap();

    assert!(
        v.get("factoryProjects").is_none(),
        "factoryProjects should be absent when no build specs exist"
    );
}

/// A malformed build spec emits a V-010 warning but does not fail compilation.
#[test]
fn malformed_build_spec_emits_v010_warning() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    let spec_dir = root.join("specs/099-test");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("spec.md"),
        r#"---
id: "099-test"
title: "Test"
status: draft
created: "2026-04-14"
summary: "Fixture."
---
# Test
"#,
    )
    .unwrap();

    // Build spec missing project.name.
    let factory_dir = root.join(".factory");
    fs::create_dir_all(&factory_dir).unwrap();
    fs::write(
        factory_dir.join("build-spec.yaml"),
        "project:\n  description: missing name\n",
    )
    .unwrap();

    let out = open_agentic_spec_compiler::compile(root).expect("compile");

    // Compilation should succeed (V-010 is a warning, not an error).
    assert!(
        out.validation_passed,
        "V-010 is a warning; validation should still pass"
    );

    let v: Value = serde_json::from_slice(&out.registry_json).unwrap();

    // factoryProjects should be absent (the only build spec was malformed).
    assert!(v.get("factoryProjects").is_none());

    // V-010 violation should be present.
    let violations = v["validation"]["violations"].as_array().unwrap();
    let v010 = violations.iter().find(|v| v["code"] == "V-010");
    assert!(v010.is_some(), "V-010 violation should be emitted");
    assert_eq!(v010.unwrap()["severity"], "warning");
}

/// Pipeline state sibling enriches the factory project record with adapter and status.
#[test]
fn pipeline_state_enriches_factory_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    let spec_dir = root.join("specs/099-test");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("spec.md"),
        r#"---
id: "099-test"
title: "Test"
status: draft
created: "2026-04-14"
summary: "Fixture."
---
# Test
"#,
    )
    .unwrap();

    let factory_dir = root.join("app/.factory");
    fs::create_dir_all(&factory_dir).unwrap();
    fs::write(
        factory_dir.join("build-spec.yaml"),
        "project:\n  name: enriched-app\n  variant: single-internal\n",
    )
    .unwrap();
    fs::write(
        factory_dir.join("pipeline-state.json"),
        r#"{"adapter": "encore-react", "status": "completed"}"#,
    )
    .unwrap();

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).unwrap();

    let projects = v["factoryProjects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);

    let proj = &projects[0];
    assert_eq!(proj["projectName"], "enriched-app");
    assert_eq!(proj["adapter"], "encore-react");
    assert_eq!(proj["pipelineStatus"], "completed");
}

/// The real repo compiles with the updated schema (factoryProjects is optional).
#[test]
fn real_repo_schema_conformance_with_factory_projects() {
    let root = repo_root();
    let out = open_agentic_spec_compiler::compile(&root).expect("compile");

    let schema_path = root.join("specs/000-bootstrap-spec-system/contracts/registry.schema.json");
    let schema_raw = fs::read_to_string(&schema_path).expect("read schema");
    let mut schema: Value = serde_json::from_str(&schema_raw).expect("parse schema");
    if let Some(o) = schema.as_object_mut() {
        o.remove("$schema");
    }

    let validator = jsonschema::validator_for(&schema).expect("compile schema");
    let instance: Value = serde_json::from_slice(&out.registry_json).expect("parse registry");
    if let Err(e) = validator.validate(&instance) {
        panic!("registry.json does not validate against updated schema: {e}");
    }
}
