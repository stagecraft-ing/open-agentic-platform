//! Machine-check emitted JSON against Feature 000 JSON Schemas (T014).

use jsonschema::validator_for;
use serde_json::Value;

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn load_schema(relative_to_repo: &str) -> Value {
    let path = repo_root().join(relative_to_repo);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("read {}: {e}", path.display());
    });
    let mut v: Value = serde_json::from_str(&raw).expect("schema JSON");
    // Avoid resolving the meta-schema URL; draft is inferred from the JSON Schema content.
    if let Some(o) = v.as_object_mut() {
        o.remove("$schema");
    }
    v
}

#[test]
fn compile_output_matches_feature_000_registry_schema() {
    let root = repo_root();
    let out = open_agentic_spec_compiler::compile(&root).expect("compile");
    let schema = load_schema("specs/000-bootstrap-spec-system/contracts/registry.schema.json");
    let validator = validator_for(&schema).expect("compile registry.schema.json");
    let instance: Value = serde_json::from_slice(&out.registry_json).expect("registry JSON");
    if let Err(e) = validator.validate(&instance) {
        panic!("registry.json does not validate: {e}");
    }
}

#[test]
fn compile_output_matches_feature_000_build_meta_schema() {
    let root = repo_root();
    let out = open_agentic_spec_compiler::compile(&root).expect("compile");
    let schema = load_schema("specs/000-bootstrap-spec-system/contracts/build-meta.schema.json");
    let validator = validator_for(&schema).expect("compile build-meta.schema.json");
    let instance: Value = serde_json::from_slice(&out.build_meta_json).expect("build-meta JSON");
    if let Err(e) = validator.validate(&instance) {
        panic!("build-meta.json does not validate: {e}");
    }
}
