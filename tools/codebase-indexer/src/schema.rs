//! Self-validation of output against JSON Schema (FR-09).

use std::fs;
use std::path::Path;

/// Validate index JSON against `schemas/codebase-index.schema.json`.
pub fn validate_against_schema(
    index_json: &[u8],
    repo_root: &Path,
) -> Result<(), String> {
    let schema_path = repo_root.join("schemas/codebase-index.schema.json");
    let schema_raw = fs::read_to_string(&schema_path)
        .map_err(|e| format!("failed to read schema: {e}"))?;

    let schema_value: serde_json::Value = serde_json::from_str(&schema_raw)
        .map_err(|e| format!("failed to parse schema: {e}"))?;

    let instance: serde_json::Value = serde_json::from_slice(index_json)
        .map_err(|e| format!("failed to parse index JSON: {e}"))?;

    let validator = jsonschema::validator_for(&schema_value)
        .map_err(|e| format!("invalid JSON Schema: {e}"))?;

    let result = validator.validate(&instance);
    if let Err(error) = result {
        return Err(format!("schema validation failed: {error}"));
    }

    Ok(())
}
