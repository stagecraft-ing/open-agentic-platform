// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: FEATUREGRAPH_REGISTRY
// Spec: spec/core/featuregraph.md

use featuregraph::scanner::Scanner;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[test]
fn test_golden_graph() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let repo_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();

    // Ensure we are in the right repo
    assert!(
        repo_root.join("spec/features.yaml").exists(),
        "Repo root not found at {:?}",
        repo_root
    );

    let scanner = Scanner::new(repo_root);
    let graph = scanner.scan().expect("Failed to scan repo");

    let json_output = serde_json::to_string_pretty(&graph).expect("Failed to serialize graph");

    let golden_path = Path::new("tests/golden/features_graph.json");

    if std::env::var("UPDATE_GOLDEN").is_ok() {
        fs::write(golden_path, json_output).expect("Failed to write golden file");
    } else {
        if !golden_path.exists() {
            // First run, write it? Or fail?
            // Failing is better, but for bootstrapping let's write or warn.
            // User plan said "commit as golden file".
            panic!("Golden file not found. Run with UPDATE_GOLDEN=1 to create it.");
        }

        let golden_content = fs::read_to_string(golden_path).expect("Failed to read golden file");

        // Normalize line endings
        let expected: Value = serde_json::from_str(&golden_content).unwrap();
        let actual: Value = serde_json::from_str(&json_output).unwrap();

        assert_eq!(
            expected, actual,
            "Feature graph does not match golden file. Run UPDATE_GOLDEN=1 to update."
        );
    }
}
