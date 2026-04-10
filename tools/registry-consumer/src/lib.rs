//! Read-only access to compiler-emitted `registry.json` (Feature 000 / 002).

use serde::Serialize;
use serde_json::Value;
use std::path::Path;

/// Default path relative to the repository root (current working directory).
pub const DEFAULT_REGISTRY_REL_PATH: &str = "build/spec-registry/registry.json";
pub const KNOWN_STATUSES: [&str; 4] = ["draft", "active", "superseded", "retired"];

/// `(status_name, count, sorted_feature_ids)` — one entry per known status.
pub type StatusRow = (String, usize, Vec<String>);

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
    let mut out: Vec<Value> = arr.to_vec();
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

/// Build a deterministic status report from `features[]`.
///
/// Returns one tuple per known status in fixed order:
/// `(status, count, sorted_feature_ids)`.
pub fn status_report(v: &Value) -> Result<Vec<StatusRow>, &'static str> {
    let features = features_sorted(v)?;
    let mut out: Vec<StatusRow> = KNOWN_STATUSES
        .iter()
        .map(|s| (s.to_string(), 0usize, Vec::<String>::new()))
        .collect();

    for f in &features {
        let status = f
            .get("status")
            .and_then(|x| x.as_str())
            .ok_or("feature is missing status")?;
        let id = f
            .get("id")
            .and_then(|x| x.as_str())
            .ok_or("feature is missing id")?;
        if let Some((_, count, ids)) = out.iter_mut().find(|(s, _, _)| s == status) {
            *count += 1;
            ids.push(id.to_string());
        }
    }

    for (_, _, ids) in &mut out {
        ids.sort();
    }
    Ok(out)
}

/// Serialize `value` as compact one-line JSON (`compact == true`) or pretty-printed JSON (`compact == false`).
pub fn serialize_json_compact_or_pretty<T: Serialize>(
    value: &T,
    compact: bool,
) -> Result<String, serde_json::Error> {
    if compact {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
}

#[cfg(test)]
mod serialize_tests {
    use super::serialize_json_compact_or_pretty;
    use serde_json::json;

    #[test]
    fn serialize_json_compact_or_pretty_matches_serde_json_helpers() {
        let v = json!({ "a": 1, "b": [2, 3] });
        assert_eq!(
            serialize_json_compact_or_pretty(&v, true).unwrap(),
            serde_json::to_string(&v).unwrap()
        );
        assert_eq!(
            serialize_json_compact_or_pretty(&v, false).unwrap(),
            serde_json::to_string_pretty(&v).unwrap()
        );
    }
}
