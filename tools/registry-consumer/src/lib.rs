//! Read-only access to compiler-emitted `registry.json` (Feature 000 / 002).
//!
//! Cut D W-03 introduces a typed-reader surface (`Registry`, `Feature`,
//! `ImplementsField`, `RegistryError`, [`load`]) alongside the original
//! `serde_json::Value`-based helpers. The typed surface is the
//! load-bearing API going forward; the Value-based helpers stay public
//! during the W-03→W-05/W-12 transition so the in-tree binary's
//! byte-identical output paths (compliance-report, etc.) keep working.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────
// Backwards-compatible Value-based surface (kept additive for W-03;
// callers migrate over W-04..W-06b.)
// ─────────────────────────────────────────────────────────────────────

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

/// Read and parse `registry.json` from disk (Value-based).
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

/// Apply [`FeatureFilter`] to a feature list (Value-based). All set filters must match (AND).
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

/// Find one feature by exact `id`, or `None` (Value-based).
pub fn find_feature_by_id(v: &Value, feature_id: &str) -> Option<Value> {
    let arr = v.pointer("/features")?.as_array()?;
    arr.iter()
        .find(|f| f.get("id").and_then(|x| x.as_str()) == Some(feature_id))
        .cloned()
}

/// Build a deterministic status report from `features[]` (Value-based).
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

/// Build a deterministic implementation-status report from `features[]` (Value-based).
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

// ─────────────────────────────────────────────────────────────────────
// Typed-reader API (W-03).
//
// Schema-version dispatch lives in `schema_v1`; the top-level [`load`]
// peeks at `specVersion`, accepts the 1.x family today, and rejects
// otherwise with `RegistryError::UnknownSchemaVersion`. W-06c will add
// a `schema_v2` arm when registry.json bumps to 2.0.0.
// ─────────────────────────────────────────────────────────────────────

/// Top-level registry.json object as a typed value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    #[serde(rename = "specVersion")]
    pub spec_version: String,
    #[serde(default)]
    pub features: Vec<Feature>,
    #[serde(default)]
    pub validation: Validation,
    #[serde(default)]
    pub build: Option<Value>,
    /// OAP-specific top-level extension (spec 074 factory ingestion).
    /// Cut D W-06a/c moves this out to the OAP enricher.
    #[serde(rename = "factoryProjects", default)]
    pub factory_projects: Option<Value>,
    /// Verbatim Value of the entire registry.json. Used by callers that
    /// need to re-emit registry contents byte-identically (e.g. CLI
    /// `show` and JSON-array paths).
    #[serde(skip)]
    pub raw: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Validation {
    #[serde(default)]
    pub passed: bool,
    #[serde(default)]
    pub violations: Vec<Value>,
}

/// A single feature record. Mirrors the spec-compiler's normalized
/// frontmatter keys; the [`Feature::raw`] field carries the unmodified
/// JSON value so consumers can re-emit byte-identical output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub shape: Option<String>,
    #[serde(default)]
    pub category: Vec<String>,
    #[serde(default)]
    pub implementation: Option<String>,
    #[serde(default)]
    pub implements: Option<ImplementsField>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(rename = "featureBranch", default)]
    pub feature_branch: Option<String>,
    #[serde(rename = "specPath", default)]
    pub spec_path: Option<String>,
    #[serde(rename = "codeAliases", default)]
    pub code_aliases: Vec<String>,
    #[serde(rename = "dependsOn", default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub owner: Option<String>,
    /// OAP-specific spec 102 surface. Cut D W-06a/b/c migrates this
    /// field out of the generic registry. Until then, the typed
    /// reader exposes it as an opaque Value.
    #[serde(default)]
    pub compliance: Option<Value>,
    /// OAP-specific spec 074 surface — same migration path as
    /// `compliance`.
    #[serde(default)]
    pub adapter: Option<Value>,
    /// Verbatim Value as parsed from `features[]`. Used by callers that
    /// re-emit feature records byte-identically.
    #[serde(skip)]
    pub raw: Value,
    /// Any other normalized fields the compiler emits today (extra
    /// frontmatter, governance lifecycle, kind-specific shapes, etc.).
    /// Kept as an opaque BTreeMap so the typed reader survives
    /// spec-compiler additions without breaking.
    #[serde(flatten, default)]
    pub extra: BTreeMap<String, Value>,
}

/// `implements:` is polymorphic in registry.json: a scalar string (a
/// spec-id reference like `"148-auth-driver-registry"`), or a list of
/// items where each item is either a bare path string or an object
/// like `{"path": "tools/spec-compiler"}` carrying optional metadata
/// (spec 147 §`implements:`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImplementsField {
    Scalar(String),
    List(Vec<Value>),
}

impl ImplementsField {
    /// Best-effort extraction of implementation **path** references.
    /// Walks the list form, pulling bare string entries directly and
    /// `path:` fields from object entries. The scalar form carries a
    /// target spec-id (NOT a file path; see spec 147 §`implements:`),
    /// so it contributes an empty Vec. Callers needing the spec-id
    /// should use [`Self::as_scalar`].
    pub fn paths(&self) -> Vec<&str> {
        match self {
            ImplementsField::Scalar(_) => Vec::new(),
            ImplementsField::List(items) => items
                .iter()
                .filter_map(|v| v.as_str().or_else(|| v.get("path").and_then(|x| x.as_str())))
                .collect(),
        }
    }

    /// Return the spec-id reference when this field carries the
    /// scalar form, or `None` for list form.
    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            ImplementsField::Scalar(s) => Some(s.as_str()),
            ImplementsField::List(_) => None,
        }
    }
}

/// Error variants returned by the typed-reader API.
#[derive(Debug)]
pub enum RegistryError {
    Io(std::io::Error),
    Json(serde_json::Error),
    UnknownSchemaVersion(String),
    MissingFeaturesArray,
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Io(e) => write!(f, "{e}"),
            RegistryError::Json(e) => write!(f, "{e}"),
            RegistryError::UnknownSchemaVersion(v) => {
                write!(f, "unsupported registry specVersion: {v}")
            }
            RegistryError::MissingFeaturesArray => write!(f, "missing features array"),
        }
    }
}

impl std::error::Error for RegistryError {}

impl From<std::io::Error> for RegistryError {
    fn from(e: std::io::Error) -> Self {
        RegistryError::Io(e)
    }
}

impl From<serde_json::Error> for RegistryError {
    fn from(e: serde_json::Error) -> Self {
        RegistryError::Json(e)
    }
}

/// Read a `registry.json` from disk into a typed [`Registry`].
///
/// Peeks at `specVersion` to dispatch. Recognizes the 1.x family (pre-
/// Cut D and contract-test fixtures) and the 2.x family (post-W-06c,
/// after spec-compiler trims `factoryProjects` and per-feature
/// `compliance`). The structural Registry/Feature types are
/// shape-compatible across both: 2.x just leaves the OAP overlay
/// fields empty (handled by `#[serde(default)]`).
pub fn load(path: &Path) -> Result<Registry, RegistryError> {
    let raw = std::fs::read_to_string(path)?;
    let v: Value = serde_json::from_str(&raw)?;
    let version = v
        .get("specVersion")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if version.starts_with("1.") || version.starts_with("2.") {
        return schema_v1_v2::parse(v, version);
    }
    Err(RegistryError::UnknownSchemaVersion(version))
}

mod schema_v1_v2 {
    //! Combined 1.x/2.x dispatch arm. registry.json ships as 1.5.0
    //! pre-W-06c, 2.0.0 post-W-06c; the structural shape of the
    //! Registry/Feature types is unchanged (the schema 2.0 bump
    //! removes `factoryProjects` + per-feature `compliance:` which
    //! are already `Option`/`#[serde(default)]` in the typed reader,
    //! so the same deserializer covers both).
    use super::*;

    pub(super) fn parse(v: Value, version: String) -> Result<Registry, RegistryError> {
        let features_v = v
            .get("features")
            .and_then(|x| x.as_array())
            .ok_or(RegistryError::MissingFeaturesArray)?
            .clone();
        let mut features: Vec<Feature> = Vec::with_capacity(features_v.len());
        for fv in features_v {
            let mut feature: Feature = serde_json::from_value(fv.clone())?;
            feature.raw = fv;
            features.push(feature);
        }
        let validation: Validation = v
            .get("validation")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        Ok(Registry {
            spec_version: version,
            features,
            validation,
            build: v.get("build").cloned(),
            factory_projects: v.get("factoryProjects").cloned(),
            raw: v,
        })
    }
}

impl Registry {
    /// Lookup a feature by exact id.
    pub fn find_by_id(&self, id: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == id)
    }

    /// Return features sorted lexicographically by `id`. Borrows; no clone.
    pub fn features_sorted(&self) -> Vec<&Feature> {
        let mut out: Vec<&Feature> = self.features.iter().collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// Apply [`FeatureFilter`] to typed features and return the matching subset.
    pub fn filter<'a>(&'a self, filter: FeatureFilter<'_>) -> Vec<&'a Feature> {
        self.features_sorted()
            .into_iter()
            .filter(|f| feature_matches_filter(f, &filter))
            .collect()
    }

    /// Lifecycle-status report (typed). One row per known status, in
    /// the [`KNOWN_STATUSES`] order, with sorted feature ids.
    pub fn status_report(&self) -> Vec<StatusRow> {
        let mut out: Vec<StatusRow> = KNOWN_STATUSES
            .iter()
            .map(|s| (s.to_string(), 0usize, Vec::<String>::new()))
            .collect();
        for f in self.features_sorted() {
            let Some(status) = f.status.as_deref() else {
                continue;
            };
            if let Some((_, count, ids)) = out.iter_mut().find(|(s, _, _)| s == status) {
                *count += 1;
                ids.push(f.id.clone());
            }
        }
        for (_, _, ids) in &mut out {
            ids.sort();
        }
        out
    }

    /// Implementation-lifecycle report (typed). Same shape as
    /// [`Self::status_report`].
    pub fn implementation_report(&self) -> Vec<StatusRow> {
        let mut out: Vec<StatusRow> = KNOWN_IMPLEMENTATIONS
            .iter()
            .map(|s| (s.to_string(), 0usize, Vec::<String>::new()))
            .collect();
        for f in self.features_sorted() {
            let Some(imp) = f.implementation.as_deref() else {
                continue;
            };
            if let Some((_, count, ids)) = out.iter_mut().find(|(s, _, _)| s == imp) {
                *count += 1;
                ids.push(f.id.clone());
            }
        }
        for (_, _, ids) in &mut out {
            ids.sort();
        }
        out
    }

    /// `validation.passed` gate (mirrors [`authoritative_or_allow_invalid`]).
    pub fn authoritative_or_allow_invalid(&self, allow_invalid: bool) -> Result<(), &'static str> {
        if allow_invalid || self.validation.passed {
            return Ok(());
        }
        Err(
            "registry is not authoritative (validation.passed is false); use --allow-invalid for diagnostics",
        )
    }
}

fn feature_matches_filter(f: &Feature, filter: &FeatureFilter<'_>) -> bool {
    if let Some(s) = filter.status {
        if f.status.as_deref() != Some(s) {
            return false;
        }
    }
    if let Some(prefix) = filter.id_prefix {
        if !f.id.starts_with(prefix) {
            return false;
        }
    }
    if let Some(imp) = filter.implementation {
        if f.implementation.as_deref() != Some(imp) {
            return false;
        }
    }
    if let Some(k) = filter.kind {
        if f.kind.as_deref() != Some(k) {
            return false;
        }
    }
    if let Some(sh) = filter.shape {
        if f.shape.as_deref() != Some(sh) {
            return false;
        }
    }
    if let Some(cat) = filter.category {
        if !f.category.iter().any(|c| c == cat) {
            return false;
        }
    }
    true
}

// Silence "unused" lint when only the Value-based path is exercised
// internally during the W-03→W-05 transition.
#[allow(dead_code)]
fn _force_json_dep() -> Value {
    json!({})
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

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

#[cfg(test)]
mod typed_tests {
    use super::*;

    fn write_fixture(path: &Path, json_str: &str) {
        std::fs::write(path, json_str).unwrap();
    }

    fn fixture_15() -> &'static str {
        r#"{
            "specVersion": "1.5.0",
            "build": {"compilerId":"test","compilerVersion":"0.1.0","inputRoot":".","contentHash":"deadbeef"},
            "features": [
                {"id":"001-a","title":"First","status":"approved","kind":"platform","category":["security"],"implementation":"complete","specPath":"specs/001-a/spec.md","implements":"src/a.rs"},
                {"id":"002-b","title":"Second","status":"draft","kind":"capability","shape":"driver","category":["auth","identity"],"implementation":"pending","specPath":"specs/002-b/spec.md","implements":["src/b1.rs","src/b2.rs"]}
            ],
            "validation": {"passed": true, "violations": []}
        }"#
    }

    fn fixture_unknown_version() -> &'static str {
        r#"{"specVersion":"3.0.0","features":[],"validation":{"passed":true,"violations":[]}}"#
    }

    #[test]
    fn load_accepts_1x_specversion() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_fixture(&p, fixture_15());
        let r = load(&p).expect("load typed registry");
        assert_eq!(r.spec_version, "1.5.0");
        assert_eq!(r.features.len(), 2);
        assert_eq!(r.features[0].id, "001-a");
    }

    #[test]
    fn load_rejects_unknown_specversion() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_fixture(&p, fixture_unknown_version());
        let err = load(&p).expect_err("load must reject 3.0.0");
        assert!(matches!(err, RegistryError::UnknownSchemaVersion(_)));
    }

    #[test]
    fn typed_find_by_id_returns_feature() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_fixture(&p, fixture_15());
        let r = load(&p).unwrap();
        let f = r.find_by_id("002-b").expect("found");
        assert_eq!(f.id, "002-b");
        assert_eq!(f.title.as_deref(), Some("Second"));
        assert!(matches!(f.implements, Some(ImplementsField::List(_))));
    }

    #[test]
    fn typed_status_report_buckets_known_statuses() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_fixture(&p, fixture_15());
        let r = load(&p).unwrap();
        let report = r.status_report();
        assert_eq!(report.len(), KNOWN_STATUSES.len());
        let approved = report.iter().find(|(s, _, _)| s == "approved").unwrap();
        assert_eq!(approved.1, 1);
        assert_eq!(approved.2, vec!["001-a"]);
        let draft = report.iter().find(|(s, _, _)| s == "draft").unwrap();
        assert_eq!(draft.1, 1);
        assert_eq!(draft.2, vec!["002-b"]);
    }

    #[test]
    fn typed_filter_kind_and_category() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_fixture(&p, fixture_15());
        let r = load(&p).unwrap();
        let out = r.filter(FeatureFilter {
            kind: Some("capability"),
            category: Some("auth"),
            ..Default::default()
        });
        assert_eq!(out.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), vec!["002-b"]);
    }

    #[test]
    fn implements_field_scalar_and_list() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_fixture(&p, fixture_15());
        let r = load(&p).unwrap();
        let a = r.find_by_id("001-a").unwrap();
        // Scalar form is a spec-id reference (spec 147), not a file path.
        match &a.implements {
            Some(ImplementsField::Scalar(s)) => assert_eq!(s, "src/a.rs"),
            other => panic!("expected Scalar, got {other:?}"),
        }
        assert_eq!(a.implements.as_ref().unwrap().as_scalar(), Some("src/a.rs"));
        assert!(a.implements.as_ref().unwrap().paths().is_empty());
        let b = r.find_by_id("002-b").unwrap();
        match &b.implements {
            Some(ImplementsField::List(v)) => {
                let strs: Vec<&str> = v.iter().filter_map(|x| x.as_str()).collect();
                assert_eq!(strs, vec!["src/b1.rs", "src/b2.rs"]);
            }
            other => panic!("expected List, got {other:?}"),
        }
        assert_eq!(b.implements.as_ref().unwrap().paths(), vec!["src/b1.rs", "src/b2.rs"]);
        assert!(b.implements.as_ref().unwrap().as_scalar().is_none());
    }

    #[test]
    fn implements_field_list_of_path_objects() {
        // Spec 003 in the production registry uses `[{"path": "..."}, ...]`
        // shape. The typed reader must accept it.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_fixture(
            &p,
            r#"{
                "specVersion":"1.5.0",
                "features":[{"id":"x","implements":[{"path":"tools/foo"},{"path":"tools/bar"}]}],
                "validation":{"passed":true,"violations":[]}
            }"#,
        );
        let r = load(&p).unwrap();
        let x = r.find_by_id("x").unwrap();
        assert_eq!(x.implements.as_ref().unwrap().paths(), vec!["tools/foo", "tools/bar"]);
    }

    #[test]
    fn feature_raw_preserves_original_value() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_fixture(&p, fixture_15());
        let r = load(&p).unwrap();
        let a = r.find_by_id("001-a").unwrap();
        // The raw Value carries the verbatim JSON parse — every key
        // present in the source registry must be reachable.
        assert_eq!(a.raw.get("id").and_then(|v| v.as_str()), Some("001-a"));
        assert_eq!(
            a.raw.get("specPath").and_then(|v| v.as_str()),
            Some("specs/001-a/spec.md")
        );
    }

    #[test]
    fn authoritative_gate_typed() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_fixture(&p, fixture_15());
        let r = load(&p).unwrap();
        assert!(r.authoritative_or_allow_invalid(false).is_ok());
    }
}
