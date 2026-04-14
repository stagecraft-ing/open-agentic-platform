// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: FEATUREGRAPH_REGISTRY

//! Load feature manifest entries from **`build/spec-registry/registry.json`**
//! (spec-compiler output) for the featuregraph scanner.

use serde::Deserialize;
use std::path::Path;

/// Root shape emitted by `tools/spec-compiler` (minimal fields for scanning).
#[derive(Debug, Deserialize)]
pub struct CompiledRegistry {
    pub features: Vec<RegistryFeatureRecord>,
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
}

/// Parse `registry.json` and return feature records (sorted by id for determinism).
pub fn load_registry_records(path: &Path) -> anyhow::Result<Vec<RegistryFeatureRecord>> {
    let bytes = std::fs::read(path)?;
    let reg: CompiledRegistry = serde_json::from_slice(&bytes)?;
    let mut features = reg.features;
    features.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(features)
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
}
