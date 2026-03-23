//! Read-only access to compiler-emitted `registry.json` (Feature 000 / 002).

use serde_json::Value;
use std::path::Path;

/// Default path relative to the repository root (current working directory).
pub const DEFAULT_REGISTRY_REL_PATH: &str = "build/spec-registry/registry.json";

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "{e}"),
            LoadError::Json(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        LoadError::Io(e)
    }
}

impl From<serde_json::Error> for LoadError {
    fn from(e: serde_json::Error) -> Self {
        LoadError::Json(e)
    }
}

/// Read and parse `registry.json` from disk.
pub fn load_registry(path: &Path) -> Result<Value, LoadError> {
    let raw = std::fs::read_to_string(path)?;
    let v: Value = serde_json::from_str(&raw)?;
    Ok(v)
}

/// Enforce Feature 002 safe-by-default: refuse when `validation.passed` is not true unless `allow_invalid`.
pub fn authoritative_or_allow_invalid(v: &Value, allow_invalid: bool) -> Result<(), &'static str> {
    if allow_invalid {
        return Ok(());
    }
    match v.pointer("/validation/passed").and_then(|x| x.as_bool()) {
        Some(true) => Ok(()),
        _ => Err(
            "registry is not authoritative (validation.passed is false); use --allow-invalid for diagnostics",
        ),
    }
}

/// Collect `features` as JSON values, sorted lexicographically by `id`.
pub fn features_sorted(v: &Value) -> Result<Vec<Value>, &'static str> {
    let arr = v
        .pointer("/features")
        .and_then(|x| x.as_array())
        .ok_or("missing features array")?;
    let mut out: Vec<Value> = arr.iter().cloned().collect();
    out.sort_by(|a, b| {
        let id_a = a.get("id").and_then(|x| x.as_str()).unwrap_or("");
        let id_b = b.get("id").and_then(|x| x.as_str()).unwrap_or("");
        id_a.cmp(id_b)
    });
    Ok(out)
}

/// Apply `--status` (exact) and `--id-prefix` (prefix on `id`) filters.
pub fn filter_features(
    features: Vec<Value>,
    status: Option<&str>,
    id_prefix: Option<&str>,
) -> Vec<Value> {
    features
        .into_iter()
        .filter(|f| {
            if let Some(s) = status {
                match f.get("status").and_then(|x| x.as_str()) {
                    Some(st) if st == s => {}
                    _ => return false,
                }
            }
            if let Some(prefix) = id_prefix {
                match f.get("id").and_then(|x| x.as_str()) {
                    Some(id) if id.starts_with(prefix) => {}
                    _ => return false,
                }
            }
            true
        })
        .collect()
}

/// Find one feature by exact `id`, or `None`.
pub fn find_feature_by_id(v: &Value, feature_id: &str) -> Option<Value> {
    let arr = v.pointer("/features")?.as_array()?;
    arr.iter()
        .find(|f| f.get("id").and_then(|x| x.as_str()) == Some(feature_id))
        .cloned()
}
