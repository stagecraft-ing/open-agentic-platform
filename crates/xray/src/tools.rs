// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: XRAY_ANALYSIS
// Spec: spec/xray/analysis.md

use crate::scan_target;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct ScanResult {
    pub digest: String,
    pub files_count: usize,
    pub index: Value,
}

pub struct XrayTools;

impl XrayTools {
    pub fn new() -> Self {
        Self
    }

    /// Run a scan on the repository or a subdirectory
    pub fn xray_scan(&self, repo_root: &Path, path: Option<String>) -> Result<Value> {
        let target_path = if let Some(p) = path {
            repo_root.join(p)
        } else {
            repo_root.to_path_buf()
        };

        // Security check: ensure target is within repo_root
        if !target_path.starts_with(repo_root) {
            return Err(anyhow::anyhow!(
                "Target path must be within repository root"
            ));
        }

        // Run the scan
        let index = scan_target(&target_path, None).context("Failed to scan target")?;

        // Convert to JSON
        let index_json = serde_json::to_value(&index)?;

        Ok(index_json)
    }
}

impl Default for XrayTools {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_xray_scan_basic() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create some files
        fs::write(root.join("test.txt"), "hello world").unwrap();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();

        let tools = XrayTools::new();
        let result = tools.xray_scan(root, None).expect("Scan failed");

        // Verify result structure
        assert!(result.is_object());
        let root_node = result.as_object().unwrap();

        assert!(root_node.contains_key("digest"));
        assert!(root_node.contains_key("files"));

        let files = root_node["files"].as_array().expect("files array");
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_xray_scan_subdir() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub/foo.txt"), "foo").unwrap();

        let tools = XrayTools::new();
        let result = tools
            .xray_scan(root, Some("sub".to_string()))
            .expect("Scan failed");

        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0]["path"], "foo.txt"); // Scanner returns relative to scan root
                                                 // xray::scan_target uses to_string_lossy of the path relative to scan target or something.
                                                 // Let's check xray implementation if needed, but for now assuming it works.
    }
}
