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
///
/// **`pub(crate)` by design.** External callers MUST use
/// [`serialize_json_canonical`] so the lex-key-order contract holds at the
/// emission boundary. This non-canonical helper is kept crate-internal so
/// [`serialize_json_canonical`] can reuse it after canonicalizing.
pub(crate) fn serialize_json_compact_or_pretty<T: Serialize>(
    value: &T,
    compact: bool,
) -> Result<String, serde_json::Error> {
    if compact {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
}

/// Canonicalize a `serde_json::Value` to lexicographically-sorted object keys,
/// recursively.
///
/// `serde_json`'s `preserve_order` feature is currently load-neutral for
/// `crates/xray`'s observable output (xray's `to_canonical_json` does its
/// own explicit lex sort in `crates/xray/src/canonical.rs:21-51`, so the
/// emitted bytes are alphabetical with or without `preserve_order`), but
/// the feature cannot be locally disabled under resolver 2 because xray's
/// `crates/xray/Cargo.toml:15` requests it and feature unification is
/// monotonic.
///
/// Without explicit canonicalization at registry-consumer's emission
/// boundary, the workspace-wide activation of `preserve_order` would
/// flip CLI key order from lex (BTreeMap-style) to struct-declaration
/// order (IndexMap-style). The principle this helper encodes: ordering
/// requirements are explicit at the serialization boundary, never
/// implicit via shared-dep feature flags. A future follow-up may remove
/// `preserve_order` from xray (since xray's observable output doesn't
/// need it) and retire this helper — but the explicit-ordering principle
/// remains the better hardened contract regardless.
pub fn canonicalize_value(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut out = serde_json::Map::new();
            for (k, v) in entries {
                out.insert(k, canonicalize_value(v));
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(canonicalize_value).collect()),
        other => other,
    }
}

/// Convenience wrapper: convert `value` to canonical JSON (object keys
/// sorted lexicographically, recursively), then serialize compact or pretty.
/// **This is the public emission boundary** — every external caller that
/// emits JSON to stdout/stderr/disk MUST route through this helper so the
/// lex-key-order contract is upheld regardless of `serde_json`'s
/// `preserve_order` feature state.
pub fn serialize_json_canonical<T: Serialize>(
    value: &T,
    compact: bool,
) -> Result<String, serde_json::Error> {
    let v = serde_json::to_value(value)?;
    let canon = canonicalize_value(v);
    serialize_json_compact_or_pretty(&canon, compact)
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

// ─────────────────────────────────────────────────────────────────────
// Relationship-graph helpers (Side Quest II Concern 5)
// ─────────────────────────────────────────────────────────────────────

/// One outgoing relationship edge from a spec.
#[derive(Debug, Clone, Serialize)]
pub struct OutgoingEdge {
    /// Relationship kind: "establishes", "extends", "refines", "supersedes",
    /// "amends", "co_authority", or "constrains".
    pub kind: String,
    /// Target spec id (for typed references) or None (for path-only entries).
    pub spec: Option<String>,
    /// Paths claimed by this edge, when present.
    pub paths: Vec<String>,
    /// Extra metadata captured verbatim (scope, nature, aspect, etc.).
    pub meta: Value,
}

/// One incoming relationship edge pointing at a spec from another spec.
#[derive(Debug, Clone, Serialize)]
pub struct IncomingEdge {
    /// Spec id of the source (the spec that holds the reference).
    pub from_spec: String,
    /// Relationship kind (same vocabulary as [`OutgoingEdge::kind`]).
    pub kind: String,
}

/// Full relationship view for one spec.
#[derive(Debug, Clone, Serialize)]
pub struct RelationshipView {
    pub spec_id: String,
    pub outgoing: Vec<OutgoingEdge>,
    pub incoming: Vec<IncomingEdge>,
}

/// One entry in a supersession chain.
#[derive(Debug, Clone, Serialize)]
pub struct ChainEntry {
    pub spec_id: String,
    /// "full" or "partial" when this entry was derived from a supersedes
    /// relationship; empty string when it is the root query spec.
    pub scope: String,
}

/// A structural problem found during graph validation.
#[derive(Debug, Clone, Serialize)]
pub struct GraphProblem {
    pub kind: String,
    pub message: String,
    pub specs: Vec<String>,
}

// ── field extraction helpers ──────────────────────────────────────────

/// Extract spec ids referenced in `extends[].spec`.
fn extends_spec_refs(raw: &Value) -> Vec<String> {
    raw.get("extends")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get("spec").and_then(|s| s.as_str()))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Extract spec ids referenced in `supersedes[].spec` (or bare string form).
fn supersedes_spec_refs(raw: &Value) -> Vec<(String, String)> {
    // Returns (spec_id, scope)
    raw.get("supersedes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    if let Some(s) = item.as_str() {
                        // Legacy bare string — treat as full scope.
                        return Some((s.to_string(), "full".to_string()));
                    }
                    let spec = item.get("spec").and_then(|s| s.as_str())?;
                    let scope = item
                        .get("scope")
                        .and_then(|s| s.as_str())
                        .unwrap_or("full")
                        .to_string();
                    Some((spec.to_string(), scope))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Extract spec ids referenced in `refines[].refines_specs[]`.
fn refines_spec_refs(raw: &Value) -> Vec<String> {
    raw.get("refines")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .flat_map(|item| {
                    item.get("refines_specs")
                        .and_then(|v| v.as_array())
                        .map(|ids| {
                            ids.iter()
                                .filter_map(|s| s.as_str())
                                .map(String::from)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Extract spec ids referenced in `co_authority[].with_specs[]`.
fn co_authority_spec_refs(raw: &Value) -> Vec<String> {
    raw.get("co_authority")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .flat_map(|item| {
                    item.get("with_specs")
                        .and_then(|v| v.as_array())
                        .map(|ids| {
                            ids.iter()
                                .filter_map(|s| s.as_str())
                                .map(String::from)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Extract spec ids referenced in `constrains[].target_specs[]`.
fn constrains_spec_refs(raw: &Value) -> Vec<(String, String)> {
    // Returns (target_spec_id, kind)
    raw.get("constrains")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .flat_map(|item| {
                    let kind = item
                        .get("kind")
                        .and_then(|k| k.as_str())
                        .unwrap_or("")
                        .to_string();
                    item.get("target_specs")
                        .and_then(|v| v.as_array())
                        .map(|ids| {
                            ids.iter()
                                .filter_map(|s| s.as_str())
                                .map(|s| (s.to_string(), kind.clone()))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Extract spec ids referenced in `amends[]` (bare string or object with `spec` key).
fn amends_spec_refs(raw: &Value) -> Vec<String> {
    raw.get("amends")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    if let Some(s) = item.as_str() {
                        return Some(s.to_string());
                    }
                    item.get("spec").and_then(|s| s.as_str()).map(String::from)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Build all outgoing edges for a feature's raw JSON.
fn outgoing_edges(raw: &Value) -> Vec<OutgoingEdge> {
    let mut edges: Vec<OutgoingEdge> = Vec::new();

    // establishes: array of strings (paths)
    if let Some(arr) = raw.get("establishes").and_then(|v| v.as_array()) {
        for path in arr.iter().filter_map(|v| v.as_str()) {
            edges.push(OutgoingEdge {
                kind: "establishes".to_string(),
                spec: None,
                paths: vec![path.to_string()],
                meta: Value::Null,
            });
        }
    }

    // extends: [{spec, paths, nature, rationale?}]
    if let Some(arr) = raw.get("extends").and_then(|v| v.as_array()) {
        for item in arr {
            let spec = item.get("spec").and_then(|s| s.as_str()).map(String::from);
            let paths: Vec<String> = item
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            edges.push(OutgoingEdge {
                kind: "extends".to_string(),
                spec,
                paths,
                meta: item.clone(),
            });
        }
    }

    // refines: [{paths, aspect, refines_specs?}]
    if let Some(arr) = raw.get("refines").and_then(|v| v.as_array()) {
        for item in arr {
            let paths: Vec<String> = item
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            edges.push(OutgoingEdge {
                kind: "refines".to_string(),
                spec: None,
                paths,
                meta: item.clone(),
            });
        }
    }

    // supersedes: [{spec, scope, paths?, rationale?}] OR [str]
    if let Some(arr) = raw.get("supersedes").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                edges.push(OutgoingEdge {
                    kind: "supersedes".to_string(),
                    spec: Some(s.to_string()),
                    paths: Vec::new(),
                    meta: json!({"scope": "full"}),
                });
            } else {
                let spec = item.get("spec").and_then(|s| s.as_str()).map(String::from);
                let paths: Vec<String> = item
                    .get("paths")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                edges.push(OutgoingEdge {
                    kind: "supersedes".to_string(),
                    spec,
                    paths,
                    meta: item.clone(),
                });
            }
        }
    }

    // amends: [str] OR [{spec, change_type}]
    if let Some(arr) = raw.get("amends").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                edges.push(OutgoingEdge {
                    kind: "amends".to_string(),
                    spec: Some(s.to_string()),
                    paths: Vec::new(),
                    meta: Value::Null,
                });
            } else {
                let spec = item.get("spec").and_then(|s| s.as_str()).map(String::from);
                edges.push(OutgoingEdge {
                    kind: "amends".to_string(),
                    spec,
                    paths: Vec::new(),
                    meta: item.clone(),
                });
            }
        }
    }

    // co_authority: [{paths, section, with_specs}]
    if let Some(arr) = raw.get("co_authority").and_then(|v| v.as_array()) {
        for item in arr {
            let paths: Vec<String> = item
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            let with_specs: Vec<String> = item
                .get("with_specs")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            for spec in &with_specs {
                edges.push(OutgoingEdge {
                    kind: "co_authority".to_string(),
                    spec: Some(spec.clone()),
                    paths: paths.clone(),
                    meta: item.clone(),
                });
            }
            if with_specs.is_empty() {
                edges.push(OutgoingEdge {
                    kind: "co_authority".to_string(),
                    spec: None,
                    paths,
                    meta: item.clone(),
                });
            }
        }
    }

    // constrains: [{kind, paths?, target_specs?}]
    if let Some(arr) = raw.get("constrains").and_then(|v| v.as_array()) {
        for item in arr {
            let constraint_kind = item
                .get("kind")
                .and_then(|k| k.as_str())
                .unwrap_or("")
                .to_string();
            let paths: Vec<String> = item
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            let target_specs: Vec<String> = item
                .get("target_specs")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            if target_specs.is_empty() {
                edges.push(OutgoingEdge {
                    kind: format!("constrains({})", constraint_kind),
                    spec: None,
                    paths,
                    meta: item.clone(),
                });
            } else {
                for tspec in &target_specs {
                    edges.push(OutgoingEdge {
                        kind: format!("constrains({})", constraint_kind),
                        spec: Some(tspec.clone()),
                        paths: paths.clone(),
                        meta: item.clone(),
                    });
                }
            }
        }
    }

    edges
}

impl Registry {
    /// Build the full relationship view for a given spec id.
    ///
    /// Returns `None` if the spec id is not found in the registry.
    pub fn graph_relationships(&self, spec_id: &str) -> Option<RelationshipView> {
        let feature = self.find_by_id(spec_id)?;
        let outgoing = outgoing_edges(&feature.raw);

        let mut incoming: Vec<IncomingEdge> = Vec::new();
        for f in &self.features {
            if f.id == spec_id {
                continue;
            }
            let raw = &f.raw;

            // extends[].spec → incoming "extends"
            if extends_spec_refs(raw).contains(&spec_id.to_string()) {
                incoming.push(IncomingEdge {
                    from_spec: f.id.clone(),
                    kind: "extends".to_string(),
                });
            }
            // supersedes[].spec → incoming "supersedes"
            for (sid, _) in supersedes_spec_refs(raw) {
                if sid == spec_id {
                    incoming.push(IncomingEdge {
                        from_spec: f.id.clone(),
                        kind: "supersedes".to_string(),
                    });
                }
            }
            // refines[].refines_specs[] → incoming "refines"
            if refines_spec_refs(raw).contains(&spec_id.to_string()) {
                incoming.push(IncomingEdge {
                    from_spec: f.id.clone(),
                    kind: "refines".to_string(),
                });
            }
            // co_authority[].with_specs[] → incoming "co_authority"
            if co_authority_spec_refs(raw).contains(&spec_id.to_string()) {
                incoming.push(IncomingEdge {
                    from_spec: f.id.clone(),
                    kind: "co_authority".to_string(),
                });
            }
            // constrains[].target_specs[] → incoming "constrains"
            for (tid, _) in constrains_spec_refs(raw) {
                if tid == spec_id {
                    incoming.push(IncomingEdge {
                        from_spec: f.id.clone(),
                        kind: "constrains".to_string(),
                    });
                }
            }
            // amends[] → incoming "amends"
            if amends_spec_refs(raw).contains(&spec_id.to_string()) {
                incoming.push(IncomingEdge {
                    from_spec: f.id.clone(),
                    kind: "amends".to_string(),
                });
            }
        }

        Some(RelationshipView {
            spec_id: spec_id.to_string(),
            outgoing,
            incoming,
        })
    }

    /// Walk the supersession chain for a given spec id, returning entries
    /// ordered oldest → newest.
    ///
    /// Returns `None` if the spec id is not found in the registry.
    pub fn supersession_chain(&self, spec_id: &str) -> Option<Vec<ChainEntry>> {
        // Verify the spec exists.
        self.find_by_id(spec_id)?;

        let mut chain: Vec<ChainEntry> = Vec::new();

        // Walk backwards: what does this spec supersede?
        fn walk_back(
            registry: &Registry,
            current: &str,
            chain: &mut Vec<ChainEntry>,
            visited: &mut std::collections::HashSet<String>,
        ) {
            if !visited.insert(current.to_string()) {
                return; // cycle guard
            }
            if let Some(f) = registry.find_by_id(current) {
                for (sid, scope) in supersedes_spec_refs(&f.raw) {
                    walk_back(registry, &sid, chain, visited);
                    chain.push(ChainEntry {
                        spec_id: sid,
                        scope,
                    });
                }
            }
        }

        let mut visited = std::collections::HashSet::new();
        walk_back(self, spec_id, &mut chain, &mut visited);

        // The queried spec sits in the middle.
        chain.push(ChainEntry {
            spec_id: spec_id.to_string(),
            scope: String::new(),
        });

        // Walk forwards: what supersedes this spec?
        fn walk_forward(
            registry: &Registry,
            target: &str,
            chain: &mut Vec<ChainEntry>,
            visited: &mut std::collections::HashSet<String>,
        ) {
            for f in &registry.features {
                for (sid, scope) in supersedes_spec_refs(&f.raw) {
                    if sid == target && !visited.contains(&f.id) {
                        visited.insert(f.id.clone());
                        chain.push(ChainEntry {
                            spec_id: f.id.clone(),
                            scope,
                        });
                        walk_forward(registry, &f.id, chain, visited);
                    }
                }
            }
        }

        walk_forward(self, spec_id, &mut chain, &mut visited);

        Some(chain)
    }

    /// Find all specs whose `constrains[].target_specs` includes `target_spec_id`.
    ///
    /// Returns a Vec of `(constraining_spec_id, constraint_kind)` pairs.
    pub fn constraints_on(&self, target_spec_id: &str) -> Vec<(String, String)> {
        let mut result: Vec<(String, String)> = Vec::new();
        for f in &self.features {
            for (tid, kind) in constrains_spec_refs(&f.raw) {
                if tid == target_spec_id {
                    result.push((f.id.clone(), kind));
                }
            }
        }
        result.sort();
        result
    }

    /// Find the authority set for a code path.
    ///
    /// Walks all specs looking for those that claim this path via:
    /// - `establishes[]`
    /// - `extends[].paths`
    /// - `refines[].paths`
    /// - `co_authority[].paths`
    ///
    /// Superseded (status = "superseded") specs are excluded.
    /// If `section` is Some, additionally filters `co_authority` entries
    /// by matching `co_authority[].section`.
    pub fn authority_for_path(
        &self,
        path: &str,
        section: Option<&str>,
    ) -> Vec<AuthorityEntry> {
        let mut result: Vec<AuthorityEntry> = Vec::new();
        for f in &self.features {
            // Exclude fully superseded specs.
            if f.status.as_deref() == Some("superseded") {
                continue;
            }
            let raw = &f.raw;
            let mut relationship = String::new();

            // Check establishes[].
            let established = raw
                .get("establishes")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().any(|v| v.as_str() == Some(path)))
                .unwrap_or(false);
            if established {
                relationship = "establishes".to_string();
            }

            // Check extends[].paths.
            if relationship.is_empty() {
                let extended = raw
                    .get("extends")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().any(|item| {
                            item.get("paths")
                                .and_then(|v| v.as_array())
                                .map(|ps| ps.iter().any(|p| p.as_str() == Some(path)))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);
                if extended {
                    relationship = "extends".to_string();
                }
            }

            // Check refines[].paths.
            if relationship.is_empty() {
                let refined = raw
                    .get("refines")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().any(|item| {
                            item.get("paths")
                                .and_then(|v| v.as_array())
                                .map(|ps| ps.iter().any(|p| p.as_str() == Some(path)))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);
                if refined {
                    relationship = "refines".to_string();
                }
            }

            // Check co_authority[].paths (optionally filtered by section).
            if relationship.is_empty() {
                let co_auth = raw
                    .get("co_authority")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().any(|item| {
                            let path_match = item
                                .get("paths")
                                .and_then(|v| v.as_array())
                                .map(|ps| ps.iter().any(|p| p.as_str() == Some(path)))
                                .unwrap_or(false);
                            let section_match = match section {
                                None => true,
                                Some(s) => item
                                    .get("section")
                                    .and_then(|v| v.as_str())
                                    == Some(s),
                            };
                            path_match && section_match
                        })
                    })
                    .unwrap_or(false);
                if co_auth {
                    relationship = "co_authority".to_string();
                }
            }

            if !relationship.is_empty() {
                result.push(AuthorityEntry {
                    spec_id: f.id.clone(),
                    relationship,
                });
            }
        }
        result.sort_by(|a, b| a.spec_id.cmp(&b.spec_id));
        result
    }

    /// Validate the relationship graph, returning a list of structural problems.
    ///
    /// Checks:
    /// - Cycles in `extends` chains
    /// - Cycles in `supersedes` chains
    /// - Dangling references (target spec does not exist in registry)
    ///
    /// Exit semantics: callers should exit with code 1 when the returned Vec is non-empty.
    pub fn validate_graph(&self) -> Vec<GraphProblem> {
        let mut problems: Vec<GraphProblem> = Vec::new();
        let all_ids: std::collections::HashSet<&str> =
            self.features.iter().map(|f| f.id.as_str()).collect();

        // Spec 132 V-011 — short-form ids (`"000"`, `"104"`) resolve
        // against the full id set by 3-digit prefix match. The compiler
        // does this for `amends:` and `amendment_record:`; the validator
        // does the same so legitimate corpus references aren't flagged
        // as dangling.
        let is_known = |sid: &str| -> bool {
            if all_ids.contains(sid) {
                return true;
            }
            if sid.len() == 3 && sid.bytes().all(|b| b.is_ascii_digit()) {
                let prefix = format!("{sid}-");
                return all_ids.iter().any(|id| id.starts_with(&prefix));
            }
            false
        };

        // ── Dangling reference checks ─────────────────────────────────
        for f in &self.features {
            let raw = &f.raw;

            // extends[].spec
            for sid in extends_spec_refs(raw) {
                if !is_known(&sid) {
                    problems.push(GraphProblem {
                        kind: "dangling_reference".to_string(),
                        message: format!(
                            "{} extends unknown spec {}",
                            f.id, sid
                        ),
                        specs: vec![f.id.clone(), sid],
                    });
                }
            }

            // supersedes[].spec
            for (sid, _) in supersedes_spec_refs(raw) {
                if !is_known(&sid) {
                    problems.push(GraphProblem {
                        kind: "dangling_reference".to_string(),
                        message: format!(
                            "{} supersedes unknown spec {}",
                            f.id, sid
                        ),
                        specs: vec![f.id.clone(), sid],
                    });
                }
            }

            // refines[].refines_specs[]
            for sid in refines_spec_refs(raw) {
                if !is_known(&sid) {
                    problems.push(GraphProblem {
                        kind: "dangling_reference".to_string(),
                        message: format!(
                            "{} refines unknown spec {}",
                            f.id, sid
                        ),
                        specs: vec![f.id.clone(), sid],
                    });
                }
            }

            // co_authority[].with_specs[]
            for sid in co_authority_spec_refs(raw) {
                if !is_known(&sid) {
                    problems.push(GraphProblem {
                        kind: "dangling_reference".to_string(),
                        message: format!(
                            "{} co_authority references unknown spec {}",
                            f.id, sid
                        ),
                        specs: vec![f.id.clone(), sid],
                    });
                }
            }

            // constrains[].target_specs[]
            for (sid, _) in constrains_spec_refs(raw) {
                if !is_known(&sid) {
                    problems.push(GraphProblem {
                        kind: "dangling_reference".to_string(),
                        message: format!(
                            "{} constrains unknown spec {}",
                            f.id, sid
                        ),
                        specs: vec![f.id.clone(), sid],
                    });
                }
            }

            // amends[]
            for sid in amends_spec_refs(raw) {
                if !is_known(&sid) {
                    problems.push(GraphProblem {
                        kind: "dangling_reference".to_string(),
                        message: format!(
                            "{} amends unknown spec {}",
                            f.id, sid
                        ),
                        specs: vec![f.id.clone(), sid],
                    });
                }
            }
        }

        // ── Cycle detection: extends ──────────────────────────────────
        // Build adjacency map: spec_id → [extended spec ids]
        let extends_map: std::collections::HashMap<&str, Vec<String>> = self
            .features
            .iter()
            .map(|f| (f.id.as_str(), extends_spec_refs(&f.raw)))
            .collect();

        let mut extends_visited: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut reported_extends_cycles: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for f in &self.features {
            if extends_visited.contains(&f.id) {
                continue;
            }
            let mut path: Vec<String> = Vec::new();
            detect_cycle(
                &f.id,
                &extends_map,
                &mut path,
                &mut extends_visited,
                &mut reported_extends_cycles,
                &mut problems,
                "extends_cycle",
            );
        }

        // ── Cycle detection: supersedes ───────────────────────────────
        let supersedes_map: std::collections::HashMap<&str, Vec<String>> = self
            .features
            .iter()
            .map(|f| {
                let targets: Vec<String> =
                    supersedes_spec_refs(&f.raw).into_iter().map(|(s, _)| s).collect();
                (f.id.as_str(), targets)
            })
            .collect();

        let mut sup_visited: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut reported_sup_cycles: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for f in &self.features {
            if sup_visited.contains(&f.id) {
                continue;
            }
            let mut path: Vec<String> = Vec::new();
            detect_cycle(
                &f.id,
                &supersedes_map,
                &mut path,
                &mut sup_visited,
                &mut reported_sup_cycles,
                &mut problems,
                "supersedes_cycle",
            );
        }

        problems
    }
}

/// DFS cycle detection helper shared by extends and supersedes walks.
fn detect_cycle(
    node: &str,
    adj: &std::collections::HashMap<&str, Vec<String>>,
    path: &mut Vec<String>,
    visited: &mut std::collections::HashSet<String>,
    reported: &mut std::collections::HashSet<String>,
    problems: &mut Vec<GraphProblem>,
    kind: &str,
) {
    if let Some(pos) = path.iter().position(|x| x == node) {
        // Cycle found: path[pos..] forms the cycle.
        let cycle: Vec<String> = path[pos..].to_vec();
        let key = {
            let mut sorted = cycle.clone();
            sorted.sort();
            sorted.join(",")
        };
        if reported.insert(key) {
            let cycle_str = cycle.join(" → ");
            problems.push(GraphProblem {
                kind: kind.to_string(),
                message: format!("cycle detected: {} → {}", cycle_str, node),
                specs: cycle,
            });
        }
        return;
    }
    if visited.contains(node) {
        return;
    }
    path.push(node.to_string());
    if let Some(targets) = adj.get(node) {
        for t in targets {
            detect_cycle(t, adj, path, visited, reported, problems, kind);
        }
    }
    path.pop();
    visited.insert(node.to_string());
}

/// One entry in the authority set for a path.
#[derive(Debug, Clone, Serialize)]
pub struct AuthorityEntry {
    pub spec_id: String,
    /// How this spec claims authority: "establishes", "extends", "refines", or "co_authority".
    pub relationship: String,
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
// Graph-relationship tests live in the graph_tests module below.

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

// ─────────────────────────────────────────────────────────────────────
// Graph-relationship tests (Side Quest II Concern 5)
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod graph_tests {
    use super::*;
    use serde_json::json;

    // ── Fixture builders ─────────────────────────────────────────────

    fn make_registry(features: serde_json::Value) -> Registry {
        let raw = json!({
            "specVersion": "1.5.0",
            "features": features,
            "validation": {"passed": true, "violations": []}
        });
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        std::fs::write(&p, serde_json::to_vec(&raw).unwrap()).unwrap();
        load(&p).expect("fixture registry must load")
    }

    fn graph_fixture() -> Registry {
        make_registry(json!([
            {
                "id": "010-base",
                "status": "approved",
                "establishes": ["crates/foo", "crates/bar"]
            },
            {
                "id": "020-ext",
                "status": "approved",
                "extends": [{"spec": "010-base", "paths": ["crates/foo"], "nature": "additive"}]
            },
            {
                "id": "030-ref",
                "status": "approved",
                "refines": [{"paths": ["crates/bar"], "aspect": "performance", "refines_specs": ["010-base"]}]
            },
            {
                "id": "040-sup",
                "status": "approved",
                "supersedes": [{"spec": "010-base", "scope": "full"}]
            },
            {
                "id": "050-amend",
                "status": "approved",
                "amends": ["010-base"]
            },
            {
                "id": "060-coauth",
                "status": "approved",
                "co_authority": [{"paths": ["crates/foo"], "section": "security", "with_specs": ["010-base"]}]
            },
            {
                "id": "070-constrain",
                "status": "approved",
                "constrains": [{"kind": "must-implement", "target_specs": ["010-base"]}]
            }
        ]))
    }

    // ── graph_relationships ──────────────────────────────────────────

    #[test]
    fn graph_relationships_unknown_spec_returns_none() {
        let reg = graph_fixture();
        assert!(reg.graph_relationships("999-nope").is_none());
    }

    #[test]
    fn graph_relationships_outgoing_establishes() {
        let reg = graph_fixture();
        let view = reg.graph_relationships("010-base").unwrap();
        let establishes: Vec<&OutgoingEdge> = view
            .outgoing
            .iter()
            .filter(|e| e.kind == "establishes")
            .collect();
        assert_eq!(establishes.len(), 2, "010-base establishes two paths");
        assert!(establishes.iter().any(|e| e.paths == ["crates/foo"]));
        assert!(establishes.iter().any(|e| e.paths == ["crates/bar"]));
    }

    #[test]
    fn graph_relationships_outgoing_extends() {
        let reg = graph_fixture();
        let view = reg.graph_relationships("020-ext").unwrap();
        let edges: Vec<&OutgoingEdge> = view
            .outgoing
            .iter()
            .filter(|e| e.kind == "extends")
            .collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].spec.as_deref(), Some("010-base"));
        assert_eq!(edges[0].paths, ["crates/foo"]);
    }

    #[test]
    fn graph_relationships_incoming_extends() {
        let reg = graph_fixture();
        let view = reg.graph_relationships("010-base").unwrap();
        let incoming_ext: Vec<&IncomingEdge> = view
            .incoming
            .iter()
            .filter(|e| e.kind == "extends")
            .collect();
        assert_eq!(incoming_ext.len(), 1);
        assert_eq!(incoming_ext[0].from_spec, "020-ext");
    }

    #[test]
    fn graph_relationships_incoming_supersedes() {
        let reg = graph_fixture();
        let view = reg.graph_relationships("010-base").unwrap();
        let incoming_sup: Vec<&IncomingEdge> = view
            .incoming
            .iter()
            .filter(|e| e.kind == "supersedes")
            .collect();
        assert_eq!(incoming_sup.len(), 1);
        assert_eq!(incoming_sup[0].from_spec, "040-sup");
    }

    #[test]
    fn graph_relationships_incoming_refines() {
        let reg = graph_fixture();
        let view = reg.graph_relationships("010-base").unwrap();
        let incoming_ref: Vec<&IncomingEdge> = view
            .incoming
            .iter()
            .filter(|e| e.kind == "refines")
            .collect();
        assert_eq!(incoming_ref.len(), 1);
        assert_eq!(incoming_ref[0].from_spec, "030-ref");
    }

    #[test]
    fn graph_relationships_incoming_amends() {
        let reg = graph_fixture();
        let view = reg.graph_relationships("010-base").unwrap();
        let incoming_am: Vec<&IncomingEdge> = view
            .incoming
            .iter()
            .filter(|e| e.kind == "amends")
            .collect();
        assert_eq!(incoming_am.len(), 1);
        assert_eq!(incoming_am[0].from_spec, "050-amend");
    }

    #[test]
    fn graph_relationships_incoming_co_authority() {
        let reg = graph_fixture();
        let view = reg.graph_relationships("010-base").unwrap();
        let incoming_co: Vec<&IncomingEdge> = view
            .incoming
            .iter()
            .filter(|e| e.kind == "co_authority")
            .collect();
        assert_eq!(incoming_co.len(), 1);
        assert_eq!(incoming_co[0].from_spec, "060-coauth");
    }

    #[test]
    fn graph_relationships_incoming_constrains() {
        let reg = graph_fixture();
        let view = reg.graph_relationships("010-base").unwrap();
        let incoming_con: Vec<&IncomingEdge> = view
            .incoming
            .iter()
            .filter(|e| e.kind == "constrains")
            .collect();
        assert_eq!(incoming_con.len(), 1);
        assert_eq!(incoming_con[0].from_spec, "070-constrain");
    }

    #[test]
    fn graph_relationships_no_incoming_for_leaf() {
        let reg = graph_fixture();
        let view = reg.graph_relationships("040-sup").unwrap();
        assert!(
            view.incoming.is_empty(),
            "040-sup is not referenced by any other spec"
        );
    }

    // ── supersession_chain ───────────────────────────────────────────

    #[test]
    fn supersession_chain_unknown_spec_returns_none() {
        let reg = graph_fixture();
        assert!(reg.supersession_chain("999-nope").is_none());
    }

    #[test]
    fn supersession_chain_contains_root_and_superseder() {
        let reg = graph_fixture();
        let chain = reg.supersession_chain("010-base").unwrap();
        let ids: Vec<&str> = chain.iter().map(|e| e.spec_id.as_str()).collect();
        // 010-base is superseded by 040-sup
        assert!(ids.contains(&"010-base"), "root spec must appear in chain");
        assert!(ids.contains(&"040-sup"), "superseder must appear in chain");
        // 010-base must come before 040-sup (oldest first)
        let pos_base = ids.iter().position(|&s| s == "010-base").unwrap();
        let pos_sup = ids.iter().position(|&s| s == "040-sup").unwrap();
        assert!(pos_base < pos_sup, "oldest spec comes first");
    }

    #[test]
    fn supersession_chain_from_superseder_perspective() {
        let reg = graph_fixture();
        let chain = reg.supersession_chain("040-sup").unwrap();
        let ids: Vec<&str> = chain.iter().map(|e| e.spec_id.as_str()).collect();
        // Should show what 040-sup supersedes (010-base) before itself.
        assert!(ids.contains(&"010-base"));
        assert!(ids.contains(&"040-sup"));
        let pos_base = ids.iter().position(|&s| s == "010-base").unwrap();
        let pos_sup = ids.iter().position(|&s| s == "040-sup").unwrap();
        assert!(pos_base < pos_sup, "superseded spec appears before superseder");
    }

    #[test]
    fn supersession_chain_spec_with_no_supersedes_relation() {
        let reg = graph_fixture();
        // 050-amend has no supersedes; chain is just itself.
        let chain = reg.supersession_chain("050-amend").unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].spec_id, "050-amend");
    }

    #[test]
    fn supersession_chain_legacy_bare_string_form() {
        let reg = make_registry(json!([
            {"id": "aaa-old", "status": "superseded"},
            {"id": "bbb-new", "status": "approved", "supersedes": ["aaa-old"]}
        ]));
        let chain = reg.supersession_chain("aaa-old").unwrap();
        let ids: Vec<&str> = chain.iter().map(|e| e.spec_id.as_str()).collect();
        assert!(ids.contains(&"aaa-old"));
        assert!(ids.contains(&"bbb-new"));
    }

    // ── constraints_on ───────────────────────────────────────────────

    #[test]
    fn constraints_on_returns_constraining_specs() {
        let reg = graph_fixture();
        let result = reg.constraints_on("010-base");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "070-constrain");
        assert_eq!(result[0].1, "must-implement");
    }

    #[test]
    fn constraints_on_empty_when_no_constraints() {
        let reg = graph_fixture();
        let result = reg.constraints_on("040-sup");
        assert!(result.is_empty());
    }

    #[test]
    fn constraints_on_multiple_constraining_specs() {
        let reg = make_registry(json!([
            {"id": "aaa-target", "status": "approved"},
            {"id": "bbb-c1", "status": "approved", "constrains": [{"kind": "must-implement", "target_specs": ["aaa-target"]}]},
            {"id": "ccc-c2", "status": "approved", "constrains": [{"kind": "must-use", "target_specs": ["aaa-target"]}]}
        ]));
        let result = reg.constraints_on("aaa-target");
        assert_eq!(result.len(), 2);
        let kinds: Vec<&str> = result.iter().map(|(_, k)| k.as_str()).collect();
        assert!(kinds.contains(&"must-implement"));
        assert!(kinds.contains(&"must-use"));
    }

    // ── authority_for_path ───────────────────────────────────────────

    #[test]
    fn authority_for_path_establishes() {
        let reg = graph_fixture();
        let result = reg.authority_for_path("crates/foo", None);
        let ids: Vec<&str> = result.iter().map(|e| e.spec_id.as_str()).collect();
        assert!(ids.contains(&"010-base"), "010-base establishes crates/foo");
    }

    #[test]
    fn authority_for_path_extends() {
        let reg = graph_fixture();
        let result = reg.authority_for_path("crates/foo", None);
        let ids: Vec<&str> = result.iter().map(|e| e.spec_id.as_str()).collect();
        assert!(ids.contains(&"020-ext"), "020-ext extends crates/foo");
    }

    #[test]
    fn authority_for_path_refines() {
        let reg = graph_fixture();
        let result = reg.authority_for_path("crates/bar", None);
        let ids: Vec<&str> = result.iter().map(|e| e.spec_id.as_str()).collect();
        assert!(ids.contains(&"030-ref"), "030-ref refines crates/bar");
    }

    #[test]
    fn authority_for_path_co_authority() {
        let reg = graph_fixture();
        let result = reg.authority_for_path("crates/foo", None);
        let ids: Vec<&str> = result.iter().map(|e| e.spec_id.as_str()).collect();
        assert!(ids.contains(&"060-coauth"), "060-coauth co_authority crates/foo");
    }

    #[test]
    fn authority_for_path_section_filter() {
        let reg = graph_fixture();
        // With section "security" should find 060-coauth.
        let result = reg.authority_for_path("crates/foo", Some("security"));
        let ids: Vec<&str> = result.iter().map(|e| e.spec_id.as_str()).collect();
        assert!(ids.contains(&"060-coauth"));
        // With section "other" should NOT find 060-coauth.
        let result2 = reg.authority_for_path("crates/foo", Some("other-section"));
        let ids2: Vec<&str> = result2.iter().map(|e| e.spec_id.as_str()).collect();
        assert!(!ids2.contains(&"060-coauth"));
    }

    #[test]
    fn authority_for_path_excludes_superseded_specs() {
        let reg = make_registry(json!([
            {
                "id": "aaa-old",
                "status": "superseded",
                "establishes": ["crates/foo"]
            },
            {
                "id": "bbb-new",
                "status": "approved",
                "establishes": ["crates/foo"]
            }
        ]));
        let result = reg.authority_for_path("crates/foo", None);
        let ids: Vec<&str> = result.iter().map(|e| e.spec_id.as_str()).collect();
        assert!(!ids.contains(&"aaa-old"), "superseded spec must be excluded");
        assert!(ids.contains(&"bbb-new"));
    }

    #[test]
    fn authority_for_path_unknown_path_returns_empty() {
        let reg = graph_fixture();
        let result = reg.authority_for_path("no/such/path", None);
        assert!(result.is_empty());
    }

    // ── validate_graph ───────────────────────────────────────────────

    #[test]
    fn validate_graph_clean_registry_returns_no_problems() {
        let reg = graph_fixture();
        let problems = reg.validate_graph();
        assert!(
            problems.is_empty(),
            "clean fixture must produce no problems; got: {problems:?}"
        );
    }

    #[test]
    fn validate_graph_detects_dangling_extends_reference() {
        let reg = make_registry(json!([
            {"id": "aaa", "status": "approved", "extends": [{"spec": "999-nope", "paths": []}]}
        ]));
        let problems = reg.validate_graph();
        assert!(
            problems.iter().any(|p| p.kind == "dangling_reference"),
            "dangling extends must be reported"
        );
        assert!(problems.iter().any(|p| p.message.contains("999-nope")));
    }

    #[test]
    fn validate_graph_detects_dangling_supersedes_reference() {
        let reg = make_registry(json!([
            {"id": "aaa", "status": "approved", "supersedes": [{"spec": "999-nope", "scope": "full"}]}
        ]));
        let problems = reg.validate_graph();
        assert!(
            problems.iter().any(|p| p.kind == "dangling_reference"),
            "dangling supersedes must be reported"
        );
    }

    #[test]
    fn validate_graph_detects_dangling_amends_reference() {
        let reg = make_registry(json!([
            {"id": "aaa", "status": "approved", "amends": ["999-nope"]}
        ]));
        let problems = reg.validate_graph();
        assert!(
            problems.iter().any(|p| p.kind == "dangling_reference"),
            "dangling amends must be reported"
        );
    }

    #[test]
    fn validate_graph_detects_extends_cycle() {
        let reg = make_registry(json!([
            {"id": "aaa", "status": "approved", "extends": [{"spec": "bbb", "paths": []}]},
            {"id": "bbb", "status": "approved", "extends": [{"spec": "aaa", "paths": []}]}
        ]));
        let problems = reg.validate_graph();
        assert!(
            problems.iter().any(|p| p.kind == "extends_cycle"),
            "A extends B extends A must be reported as extends_cycle"
        );
    }

    #[test]
    fn validate_graph_detects_supersedes_cycle() {
        let reg = make_registry(json!([
            {"id": "aaa", "status": "approved", "supersedes": [{"spec": "bbb", "scope": "full"}]},
            {"id": "bbb", "status": "approved", "supersedes": [{"spec": "aaa", "scope": "full"}]}
        ]));
        let problems = reg.validate_graph();
        assert!(
            problems.iter().any(|p| p.kind == "supersedes_cycle"),
            "A supersedes B supersedes A must be reported as supersedes_cycle"
        );
    }

    #[test]
    fn validate_graph_no_self_reference_false_positive() {
        // A spec that references valid other specs should not produce problems.
        let reg = make_registry(json!([
            {"id": "aaa", "status": "approved"},
            {"id": "bbb", "status": "approved", "extends": [{"spec": "aaa", "paths": []}]},
            {"id": "ccc", "status": "approved", "supersedes": [{"spec": "aaa", "scope": "full"}]}
        ]));
        let problems = reg.validate_graph();
        assert!(
            problems.is_empty(),
            "valid graph must produce no problems; got: {problems:?}"
        );
    }

    // ── field extraction helpers ─────────────────────────────────────

    #[test]
    fn supersedes_bare_string_treated_as_full_scope() {
        let raw = json!({"supersedes": ["some-spec"]});
        let refs = supersedes_spec_refs(&raw);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0], ("some-spec".to_string(), "full".to_string()));
    }

    #[test]
    fn amends_bare_string_and_object_forms() {
        let raw = json!({"amends": ["spec-a", {"spec": "spec-b", "change_type": "additive"}]});
        let refs = amends_spec_refs(&raw);
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&"spec-a".to_string()));
        assert!(refs.contains(&"spec-b".to_string()));
    }

    #[test]
    fn refines_spec_refs_extracts_from_refines_specs_array() {
        let raw = json!({
            "refines": [
                {"paths": ["p1"], "aspect": "perf", "refines_specs": ["aaa", "bbb"]},
                {"paths": ["p2"], "aspect": "sec"}
            ]
        });
        let refs = refines_spec_refs(&raw);
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&"aaa".to_string()));
        assert!(refs.contains(&"bbb".to_string()));
    }
}
