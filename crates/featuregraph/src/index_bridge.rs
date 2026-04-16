// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/102-governed-excellence/spec.md — FR-021, FR-022

//! Bridge from the codebase-indexer's `index.json` to featuregraph's `FeatureGraph`.
//!
//! FR-021: The codebase-indexer is the single authoritative source of structural
//! spec-to-code traceability. This module reads its output to populate FeatureGraph
//! rather than performing independent source-file scanning.
//!
//! FR-022: The `// Feature:` header convention becomes optional enrichment;
//! this index-based path is the primary traceability source.

use crate::graph::FeatureNode;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// A single traceability mapping from the codebase-index.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexTraceMapping {
    spec_id: String,
    spec_status: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    implementing_paths: Vec<ImplementingPath>,
}

#[derive(Debug, Clone, Deserialize)]
struct ImplementingPath {
    path: String,
    #[allow(dead_code)]
    source: String,
}

/// The traceability section of the codebase-index.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexTraceability {
    #[serde(default)]
    mappings: Vec<IndexTraceMapping>,
    #[serde(default)]
    orphaned_specs: Vec<String>,
    #[serde(default)]
    untraced_code: Vec<String>,
}

/// Top-level codebase-index structure (only the parts we need).
#[derive(Debug, Clone, Deserialize)]
struct CodebaseIndex {
    traceability: IndexTraceability,
}

/// Load traceability mappings from the codebase-index and produce FeatureNode entries.
///
/// Returns a map of spec_id → FeatureNode with impl_files populated from the index.
/// The caller can merge these with any scanner-derived enrichment.
pub fn load_from_index(index_path: &Path) -> Result<HashMap<String, FeatureNode>, String> {
    let content = std::fs::read_to_string(index_path)
        .map_err(|e| format!("cannot read {}: {e}", index_path.display()))?;

    let index: CodebaseIndex =
        serde_json::from_str(&content).map_err(|e| format!("invalid index JSON: {e}"))?;

    let mut nodes = HashMap::new();

    for mapping in &index.traceability.mappings {
        let impl_files: Vec<String> = mapping
            .implementing_paths
            .iter()
            .map(|p| p.path.clone())
            .collect();

        let node = FeatureNode {
            feature_id: mapping.spec_id.clone(),
            title: String::new(), // populated from registry if needed
            spec_path: format!("specs/{}/spec.md", mapping.spec_id),
            status: mapping.spec_status.clone(),
            implementation: String::new(),
            governance: String::new(),
            owner: String::new(),
            group: String::new(),
            depends_on: mapping.depends_on.clone(),
            impl_files,
            test_files: Vec::new(),
            violations: Vec::new(),
        };

        nodes.insert(mapping.spec_id.clone(), node);
    }

    Ok(nodes)
}

/// Get orphaned specs and untraced code paths from the index.
pub fn load_diagnostics(index_path: &Path) -> Result<(Vec<String>, Vec<String>), String> {
    let content = std::fs::read_to_string(index_path)
        .map_err(|e| format!("cannot read {}: {e}", index_path.display()))?;

    let index: CodebaseIndex =
        serde_json::from_str(&content).map_err(|e| format!("invalid index JSON: {e}"))?;

    Ok((
        index.traceability.orphaned_specs,
        index.traceability.untraced_code,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_from_real_index() {
        // Use the actual codebase-index if available.
        let index_path = Path::new("build/codebase-index/index.json");
        if !index_path.exists() {
            // Skip if not in repo root.
            return;
        }

        let nodes = load_from_index(index_path).unwrap();
        assert!(
            !nodes.is_empty(),
            "expected at least one traceability mapping"
        );

        // Check that a known spec has implementing paths.
        if let Some(node) = nodes.get("102-governed-excellence") {
            assert!(!node.impl_files.is_empty());
            assert!(
                node.impl_files
                    .contains(&"crates/factory-engine".to_string())
            );
        }
    }

    #[test]
    fn load_from_synthetic_index() {
        let dir = tempfile::tempdir().unwrap();
        let index_path = dir.path().join("index.json");

        let index_json = serde_json::json!({
            "build": {},
            "diagnostics": {},
            "factory": [],
            "infrastructure": {},
            "inventory": [],
            "schemaVersion": "1.0.0",
            "traceability": {
                "mappings": [
                    {
                        "specId": "042-multi-provider",
                        "specStatus": "active",
                        "dependsOn": ["033"],
                        "implementingPaths": [
                            { "path": "crates/provider-registry", "source": "spec-implements" }
                        ]
                    }
                ],
                "orphanedSpecs": [],
                "untracedCode": ["packages/types"]
            }
        });

        fs::write(
            &index_path,
            serde_json::to_string_pretty(&index_json).unwrap(),
        )
        .unwrap();

        let nodes = load_from_index(&index_path).unwrap();
        assert_eq!(nodes.len(), 1);

        let node = &nodes["042-multi-provider"];
        assert_eq!(node.status, "active");
        assert_eq!(node.depends_on, vec!["033"]);
        assert_eq!(node.impl_files, vec!["crates/provider-registry"]);

        let (orphaned, untraced) = load_diagnostics(&index_path).unwrap();
        assert!(orphaned.is_empty());
        assert_eq!(untraced, vec!["packages/types"]);
    }
}
