//! Self-validation of output against JSON Schema (FR-09).

use std::fs;
use std::path::Path;

/// Validate index JSON against `standards/schemas/spec-spine/codebase-index.schema.json`.
pub fn validate_against_schema(index_json: &[u8], repo_root: &Path) -> Result<(), String> {
    validate_json_against(
        index_json,
        repo_root,
        "standards/schemas/spec-spine/codebase-index.schema.json",
        "index JSON",
    )
}

/// Validate the re-homed config-hash JSON against
/// `standards/schemas/spec-spine/config-hash.schema.json` (spec 188 Phase
/// 4). Symmetric with the index self-validation (FR-09): the one governed,
/// tracked slice is fail-loud at write time, the same contract
/// `check_config` reads at verify time.
pub fn validate_config_hash_against_schema(
    config_hash_json: &[u8],
    repo_root: &Path,
) -> Result<(), String> {
    validate_json_against(
        config_hash_json,
        repo_root,
        "standards/schemas/spec-spine/config-hash.schema.json",
        "config-hash JSON",
    )
}

/// Shared validator: parse the schema at `schema_rel` under `repo_root` and
/// validate `instance_json` against it. `instance_label` names the instance
/// in error messages.
fn validate_json_against(
    instance_json: &[u8],
    repo_root: &Path,
    schema_rel: &str,
    instance_label: &str,
) -> Result<(), String> {
    let schema_path = repo_root.join(schema_rel);
    let schema_raw =
        fs::read_to_string(&schema_path).map_err(|e| format!("failed to read schema: {e}"))?;

    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_raw).map_err(|e| format!("failed to parse schema: {e}"))?;

    let instance: serde_json::Value = serde_json::from_slice(instance_json)
        .map_err(|e| format!("failed to parse {instance_label}: {e}"))?;

    let validator = jsonschema::validator_for(&schema_value)
        .map_err(|e| format!("invalid JSON Schema: {e}"))?;

    let result = validator.validate(&instance);
    if let Err(error) = result {
        return Err(format!("schema validation failed: {error}"));
    }

    Ok(())
}
