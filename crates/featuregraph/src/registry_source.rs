// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: FEATUREGRAPH_REGISTRY

//! Load feature manifest entries from **`build/spec-registry/registry.json`**
//! (spec-compiler output) for the featuregraph scanner.

use crate::graph::FeatureNode;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Root shape emitted by `tools/spec-compiler` (minimal fields for scanning).
#[derive(Debug, Deserialize)]
pub struct CompiledRegistry {
    pub features: Vec<RegistryFeatureRecord>,
}

/// Item under `implements:` list form: `{path, primary?}` per spec 147.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ImplementsItem {
    pub path: String,
    #[serde(default)]
    pub primary: Option<bool>,
}

/// Spec 147 — `implements:` field shape. Scalar form is valid only for
/// `kind: capability` and carries a target registry spec id (NOT a file
/// path); list form is valid for any kind and carries code-path claims.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ImplementsField {
    Scalar(String),
    Items(Vec<ImplementsItem>),
}

impl ImplementsField {
    /// Return the file-path claims under list form. Scalar form (capability
    /// → registry spec-id reference) contributes nothing to file-path
    /// traceability and returns an empty vec.
    pub fn impl_files(&self) -> Vec<String> {
        match self {
            ImplementsField::Scalar(_) => Vec::new(),
            ImplementsField::Items(items) => items.iter().map(|i| i.path.clone()).collect(),
        }
    }
}

/// One feature row in the compiled registry (`features[]`).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RegistryFeatureRecord {
    pub id: String,
    pub title: String,
    #[serde(rename = "specPath")]
    pub spec_path: String,
    pub status: String,
    #[serde(default)]
    pub implementation: Option<String>,
    #[serde(rename = "codeAliases", default)]
    pub code_aliases: Vec<String>,
    #[serde(rename = "dependsOn", default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub risk: Option<String>,
    /// Spec 147 — promoted from `extraFrontmatter` to a top-level field.
    #[serde(default)]
    pub implements: Option<ImplementsField>,
}

/// Parse `registry.json` and return feature records (sorted by id for determinism).
pub fn load_registry_records(path: &Path) -> anyhow::Result<Vec<RegistryFeatureRecord>> {
    let bytes = std::fs::read(path)?;
    let reg: CompiledRegistry = serde_json::from_slice(&bytes)?;
    let mut features = reg.features;
    features.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(features)
}

/// Spec 147 AC-008 — build `FeatureNode` entries directly from
/// `registry.json`, reading `implements:` from the registry (no longer
/// dependent on `codebase-index/index.json` for the spec-to-code join).
///
/// Returns a map of `spec_id → FeatureNode` with `impl_files` populated
/// from each spec's `implements:` list-form items. Scalar `implements:`
/// (capability → registry spec-id) contributes nothing to `impl_files`
/// — those claims are spec-to-spec references, not code paths.
pub fn load_from_registry(path: &Path) -> Result<HashMap<String, FeatureNode>, String> {
    let records = load_registry_records(path).map_err(|e| format!("{e}"))?;
    let mut nodes = HashMap::with_capacity(records.len());
    for rec in records {
        let impl_files = rec
            .implements
            .as_ref()
            .map(|i| i.impl_files())
            .unwrap_or_default();
        let node = FeatureNode {
            feature_id: rec.id.clone(),
            title: rec.title,
            spec_path: rec.spec_path,
            status: rec.status,
            implementation: rec.implementation.unwrap_or_default(),
            governance: String::new(),
            owner: rec.owner.unwrap_or_default(),
            group: String::new(),
            depends_on: rec.depends_on,
            impl_files,
            test_files: Vec::new(),
            violations: Vec::new(),
        };
        nodes.insert(rec.id, node);
    }
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn parses_minimal_registry_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let json = r#"{
            "specVersion": "1.1.0",
            "features": [
                {
                    "id": "002-registry-consumer-mvp",
                    "title": "Registry consumer MVP",
                    "specPath": "specs/002-registry-consumer-mvp/spec.md",
                    "status": "draft",
                    "summary": "x",
                    "kind": "platform",
                    "created": "2026-03-22",
                    "authors": ["open-agentic-platform"]
                }
            ],
            "validation": { "passed": true, "violations": [] }
        }"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();

        let records = load_registry_records(&path).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "002-registry-consumer-mvp");
        assert_eq!(
            records[0].spec_path,
            "specs/002-registry-consumer-mvp/spec.md"
        );
        assert_eq!(records[0].status, "draft");
    }

    #[test]
    fn sorts_by_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let json = r#"{
            "features": [
                {"id":"b","title":"","specPath":"specs/b/spec.md","status":"draft"},
                {"id":"a","title":"","specPath":"specs/a/spec.md","status":"draft"}
            ],
            "validation": { "passed": true, "violations": [] }
        }"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();

        let records = load_registry_records(&path).unwrap();
        assert_eq!(records[0].id, "a");
        assert_eq!(records[1].id, "b");
    }

    #[test]
    fn parses_code_aliases_from_registry() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let json = r#"{
            "specVersion": "1.1.0",
            "features": [
                {
                    "id": "034-featuregraph-registry-scanner-fix",
                    "title": "t",
                    "specPath": "specs/034-featuregraph-registry-scanner-fix/spec.md",
                    "status": "active",
                    "summary": "x",
                    "created": "2026-03-29",
                    "sectionHeadings": [],
                    "codeAliases": ["FEATUREGRAPH_REGISTRY", "GOVERNANCE_ENGINE"]
                }
            ],
            "validation": { "passed": true, "violations": [] }
        }"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();

        let records = load_registry_records(&path).unwrap();
        assert_eq!(
            records[0].code_aliases,
            vec![
                "FEATUREGRAPH_REGISTRY".to_string(),
                "GOVERNANCE_ENGINE".to_string()
            ]
        );
    }

    #[test]
    fn sc091_1_parses_depends_on() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let json = r#"{
            "features": [
                {
                    "id": "091-test",
                    "title": "t",
                    "specPath": "specs/091/spec.md",
                    "status": "active",
                    "dependsOn": ["dep-a", "dep-b"]
                }
            ],
            "validation": { "passed": true, "violations": [] }
        }"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();

        let records = load_registry_records(&path).unwrap();
        assert_eq!(records[0].depends_on, vec!["dep-a", "dep-b"]);
    }

    #[test]
    fn sc091_1_parses_owner() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let json = r#"{
            "features": [
                {
                    "id": "091-owner",
                    "title": "t",
                    "specPath": "specs/091/spec.md",
                    "status": "active",
                    "owner": "platform-team"
                }
            ],
            "validation": { "passed": true, "violations": [] }
        }"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();

        let records = load_registry_records(&path).unwrap();
        assert_eq!(records[0].owner.as_deref(), Some("platform-team"));
    }

    #[test]
    fn sc091_1_parses_risk() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let json = r#"{
            "features": [
                {
                    "id": "091-risk",
                    "title": "t",
                    "specPath": "specs/091/spec.md",
                    "status": "active",
                    "risk": "high"
                }
            ],
            "validation": { "passed": true, "violations": [] }
        }"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();

        let records = load_registry_records(&path).unwrap();
        assert_eq!(records[0].risk.as_deref(), Some("high"));
    }

    #[test]
    fn sc091_2_missing_enriched_fields_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let json = r#"{
            "features": [
                {
                    "id": "091-minimal",
                    "title": "t",
                    "specPath": "specs/091/spec.md",
                    "status": "draft"
                }
            ],
            "validation": { "passed": true, "violations": [] }
        }"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();

        let records = load_registry_records(&path).unwrap();
        assert!(records[0].depends_on.is_empty());
        assert!(records[0].owner.is_none());
        assert!(records[0].risk.is_none());
    }

    #[test]
    fn parses_implements_list_form() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let json = r#"{
            "features": [
                {
                    "id": "127-spec-code-coupling-gate",
                    "title": "t",
                    "specPath": "specs/127/spec.md",
                    "status": "approved",
                    "implements": [
                        {"path": "tools/spec-code-coupling-check"},
                        {"path": "Makefile", "primary": true}
                    ]
                }
            ],
            "validation": { "passed": true, "violations": [] }
        }"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        let records = load_registry_records(&path).unwrap();
        let imp = records[0].implements.as_ref().expect("implements present");
        assert_eq!(
            imp.impl_files(),
            vec!["tools/spec-code-coupling-check".to_string(), "Makefile".to_string()]
        );
    }

    #[test]
    fn parses_implements_scalar_form_as_no_files() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let json = r#"{
            "features": [
                {
                    "id": "149-saml-auth-driver",
                    "title": "t",
                    "specPath": "specs/149/spec.md",
                    "status": "draft",
                    "implements": "148-auth-driver-registry"
                }
            ],
            "validation": { "passed": true, "violations": [] }
        }"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        let records = load_registry_records(&path).unwrap();
        let imp = records[0].implements.as_ref().expect("implements present");
        assert!(matches!(imp, ImplementsField::Scalar(_)));
        assert!(imp.impl_files().is_empty());
    }

    #[test]
    fn load_from_registry_builds_feature_nodes_with_impl_files() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let json = r#"{
            "features": [
                {
                    "id": "127-spec-code-coupling-gate",
                    "title": "Spec/Code Coupling Gate",
                    "specPath": "specs/127/spec.md",
                    "status": "approved",
                    "implementation": "complete",
                    "owner": "bart",
                    "implements": [
                        {"path": "tools/spec-code-coupling-check"},
                        {"path": "Makefile"}
                    ]
                },
                {
                    "id": "149-saml-auth-driver",
                    "title": "SAML auth driver",
                    "specPath": "specs/149/spec.md",
                    "status": "draft",
                    "implements": "148-auth-driver-registry"
                }
            ],
            "validation": { "passed": true, "violations": [] }
        }"#;
        std::fs::File::create(&path)
            .unwrap()
            .write_all(json.as_bytes())
            .unwrap();
        let nodes = load_from_registry(&path).unwrap();
        let n127 = nodes.get("127-spec-code-coupling-gate").expect("127 present");
        assert_eq!(
            n127.impl_files,
            vec!["tools/spec-code-coupling-check".to_string(), "Makefile".to_string()]
        );
        assert_eq!(n127.title, "Spec/Code Coupling Gate");
        assert_eq!(n127.owner, "bart");
        let n149 = nodes.get("149-saml-auth-driver").expect("149 present");
        assert!(n149.impl_files.is_empty(), "scalar implements contributes no file paths");
    }
}
