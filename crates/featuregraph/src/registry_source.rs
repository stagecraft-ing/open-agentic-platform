// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: 034-featuregraph-registry-scanner-fix

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
            "specVersion": "1.0.0",
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
        assert_eq!(records[0].spec_path, "specs/002-registry-consumer-mvp/spec.md");
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
}
