//! Read-only access to compiler-emitted `registry.json` (Feature 000 / 002).

use serde::Serialize;
use serde_json::Value;
use std::path::Path;

/// Default path relative to the repository root (current working directory).
pub const DEFAULT_REGISTRY_REL_PATH: &str = "build/spec-registry/registry.json";
pub const KNOWN_STATUSES: [&str; 4] = ["draft", "approved", "superseded", "retired"];
pub const KNOWN_IMPLEMENTATIONS: [&str; 5] =
    ["pending", "in-progress", "complete", "n/a", "deferred"];

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

/// Filter criteria passed to [`filter_features`]. All fields are exact-match
/// (or prefix-match for [`Self::id_prefix`]); `category` membership-tests the
/// filter value against the feature's `category[]` list.
#[derive(Default, Clone, Copy)]
pub struct FeatureFilter<'a> {
    pub status: Option<&'a str>,
    pub id_prefix: Option<&'a str>,
    pub implementation: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub shape: Option<&'a str>,
    pub category: Option<&'a str>,
}

/// Apply [`FeatureFilter`] to a feature list. All set filters must match (AND).
pub fn filter_features(features: Vec<Value>, filter: FeatureFilter<'_>) -> Vec<Value> {
    features
        .into_iter()
        .filter(|f| {
            if let Some(s) = filter.status {
                match f.get("status").and_then(|x| x.as_str()) {
                    Some(st) if st == s => {}
                    _ => return false,
                }
            }
            if let Some(prefix) = filter.id_prefix {
                match f.get("id").and_then(|x| x.as_str()) {
                    Some(id) if id.starts_with(prefix) => {}
                    _ => return false,
                }
            }
            if let Some(imp) = filter.implementation {
                match f.get("implementation").and_then(|x| x.as_str()) {
                    Some(i) if i == imp => {}
                    _ => return false,
                }
            }
            if let Some(k) = filter.kind {
                match f.get("kind").and_then(|x| x.as_str()) {
                    Some(ki) if ki == k => {}
                    _ => return false,
                }
            }
            if let Some(sh) = filter.shape {
                match f.get("shape").and_then(|x| x.as_str()) {
                    Some(s) if s == sh => {}
                    _ => return false,
                }
            }
            if let Some(cat) = filter.category {
                let matched = f
                    .get("category")
                    .and_then(|x| x.as_array())
                    .map(|arr| arr.iter().any(|v| v.as_str() == Some(cat)))
                    .unwrap_or(false);
                if !matched {
                    return false;
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

/// Build a deterministic implementation-status report from `features[]`.
///
/// Returns one tuple per known implementation status in fixed order:
/// `(implementation, count, sorted_feature_ids)`.
pub fn implementation_report(v: &Value) -> Result<Vec<StatusRow>, &'static str> {
    let features = features_sorted(v)?;
    let mut out: Vec<StatusRow> = KNOWN_IMPLEMENTATIONS
        .iter()
        .map(|s| (s.to_string(), 0usize, Vec::<String>::new()))
        .collect();

    for f in &features {
        let imp = f
            .get("implementation")
            .and_then(|x| x.as_str())
            .unwrap_or("(unset)");
        let id = f
            .get("id")
            .and_then(|x| x.as_str())
            .ok_or("feature is missing id")?;
        if let Some((_, count, ids)) = out.iter_mut().find(|(s, _, _)| s == imp) {
            *count += 1;
            ids.push(id.to_string());
        }
        // Features with unset or unknown implementation values are silently skipped
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

#[cfg(test)]
mod filter_tests {
    use super::{FeatureFilter, filter_features};
    use serde_json::json;

    fn corpus() -> Vec<serde_json::Value> {
        vec![
            json!({"id":"148-r","kind":"registry","category":["auth","identity"]}),
            json!({"id":"149-c","kind":"capability","shape":"driver","category":["auth","security"]}),
            json!({"id":"150-p","kind":"profile","category":["identity","policy"]}),
            json!({"id":"100-x","kind":"platform"}),
        ]
    }

    fn ids(v: &[serde_json::Value]) -> Vec<&str> {
        v.iter().map(|f| f["id"].as_str().unwrap()).collect()
    }

    #[test]
    fn kind_filter_exact_match() {
        let out = filter_features(corpus(), FeatureFilter { kind: Some("registry"), ..Default::default() });
        assert_eq!(ids(&out), vec!["148-r"]);
    }

    #[test]
    fn shape_filter_exact_match() {
        let out = filter_features(corpus(), FeatureFilter { shape: Some("driver"), ..Default::default() });
        assert_eq!(ids(&out), vec!["149-c"]);
    }

    #[test]
    fn category_filter_matches_any_list_entry() {
        let out = filter_features(corpus(), FeatureFilter { category: Some("auth"), ..Default::default() });
        assert_eq!(ids(&out), vec!["148-r", "149-c"]);
    }

    #[test]
    fn category_filter_skips_features_without_category() {
        let out = filter_features(corpus(), FeatureFilter { category: Some("identity"), ..Default::default() });
        // 148 and 150 carry "identity"; 100-x has no category list, must be excluded.
        assert_eq!(ids(&out), vec!["148-r", "150-p"]);
    }

    #[test]
    fn filters_compose_with_and_semantics() {
        let out = filter_features(
            corpus(),
            FeatureFilter { kind: Some("capability"), category: Some("auth"), ..Default::default() },
        );
        assert_eq!(ids(&out), vec!["149-c"]);
        // Disjoint filters return empty.
        let out = filter_features(
            corpus(),
            FeatureFilter { kind: Some("capability"), category: Some("policy"), ..Default::default() },
        );
        assert!(out.is_empty());
    }
}
