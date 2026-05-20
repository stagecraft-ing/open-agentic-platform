//! Self-validation of output against Feature 000 JSON Schemas.
//!
//! Schemas are baked into the binary via `include_str!` so the producer
//! carries its own contract — no filesystem coupling at the consumer side.
//! Diverges from codebase-indexer's runtime-file-read pattern by design:
//! spec-compiler's fixture-test landscape uses the `specs/000-*` directory
//! namespace with semantic meaning to the compiler (short-form id resolution,
//! dir-name ↔ id consistency), so a runtime-read pattern would collide
//! with that namespace. Schemas now live under `standards/schemas/spec-spine/`.

const REGISTRY_SCHEMA: &str =
    include_str!("../../../../standards/schemas/spec-spine/registry.schema.json");

const BUILD_META_SCHEMA: &str =
    include_str!("../../../../standards/schemas/spec-spine/build-meta.schema.json");

/// Validate registry JSON against the embedded
/// `registry.schema.json` (Feature 000).
pub fn validate_registry_against_schema(registry_json: &[u8]) -> Result<(), String> {
    validate_against(registry_json, REGISTRY_SCHEMA)
}

/// Validate build-meta JSON against the embedded
/// `build-meta.schema.json` (Feature 000).
pub fn validate_build_meta_against_schema(build_meta_json: &[u8]) -> Result<(), String> {
    validate_against(build_meta_json, BUILD_META_SCHEMA)
}

fn validate_against(instance_json: &[u8], schema_raw: &str) -> Result<(), String> {
    let mut schema_value: serde_json::Value =
        serde_json::from_str(schema_raw).map_err(|e| format!("failed to parse schema: {e}"))?;

    // Avoid resolving the meta-schema URL; draft is inferred from
    // schema content.
    if let Some(o) = schema_value.as_object_mut() {
        o.remove("$schema");
    }

    let instance: serde_json::Value = serde_json::from_slice(instance_json)
        .map_err(|e| format!("failed to parse instance JSON: {e}"))?;

    let validator = jsonschema::validator_for(&schema_value)
        .map_err(|e| format!("invalid JSON Schema: {e}"))?;

    if let Err(error) = validator.validate(&instance) {
        return Err(format!("schema validation failed: {error}"));
    }

    Ok(())
}
