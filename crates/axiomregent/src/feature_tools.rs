// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: FEATUREGRAPH_REGISTRY
// Spec: spec/core/featuregraph.md

use anyhow::{Context, Result, anyhow};
use featuregraph::graph::{FeatureGraph, Violation};
use featuregraph::locate::{Selector, SelectorType, locate};
use featuregraph::preflight::{PreflightChecker, PreflightResponse};
use featuregraph::scanner::Scanner;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub use featuregraph::preflight::{PreflightIntent, PreflightMode, PreflightRequest};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum GraphMode {
    Worktree,
    Snapshot(String),
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct CacheKey {
    repo_root: PathBuf,
    mode: GraphMode,
}

#[derive(Serialize)]
pub struct FeatureOverview {
    pub feature_id: String,
    pub status: String,
    pub spec_path: String,
    pub impl_files_count: usize,
    pub test_files_count: usize,
}

pub struct FeatureTools {
    cache: Mutex<HashMap<CacheKey, Arc<FeatureGraph>>>,
}

impl Default for FeatureTools {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureTools {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn get_graph(&self, root: &Path, mode: GraphMode) -> Result<Arc<FeatureGraph>> {
        let key = CacheKey {
            repo_root: root.to_path_buf(),
            mode: mode.clone(),
        };

        // 1. Check Cache
        {
            let cache = self.cache.lock().unwrap();
            if let Some(graph) = cache.get(&key) {
                return Ok(graph.clone());
            }
        }

        // 2. Load Graph (Lock released during I/O)
        let graph = match &mode {
            GraphMode::Worktree => {
                let scanner = Scanner::new(root);
                scanner.scan().context("Failed to scan feature graph")?
            }
            GraphMode::Snapshot(_id) => {
                // Snapshot-mode feature graph scanning requires the featuregraph scanner to read
                // file contents from the blob store instead of the filesystem. Not yet implemented.
                // Use mode="worktree" for gov.preflight and gov.drift.
                return Err(anyhow!(
                    "Snapshot mode feature graph scanning is not yet implemented. \
                     Use mode=\"worktree\" for gov.preflight and gov.drift."
                ));
            }
        };
        let graph = Arc::new(graph);

        // 3. Store Cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(key, graph.clone());
        }

        Ok(graph)
    }

    pub fn invalidate(&self, root: &Path) {
        let mut cache = self.cache.lock().unwrap();
        // Invalidate Worktree entry for this root
        let key = CacheKey {
            repo_root: root.to_path_buf(),
            mode: GraphMode::Worktree,
        };
        cache.remove(&key);
    }

    pub fn locate(&self, root: &Path, selector_kind: &str, selector_value: &str) -> Result<Value> {
        // features.locate currently uses Worktree implicitly
        let graph = self.get_graph(root, GraphMode::Worktree)?;

        let kind = match selector_kind {
            "feature_id" => SelectorType::FeatureId,
            "spec_path" => SelectorType::SpecPath,
            "file_path" => SelectorType::FilePath,
            _ => return Err(anyhow!("Invalid selector type: {}", selector_kind)),
        };

        let selector = Selector {
            kind,
            value: selector_value.to_string(),
        };

        let matches = locate(&graph, &selector, &[]);
        Ok(serde_json::to_value(matches)?)
    }

    pub fn preflight(&self, root: &Path, req: PreflightRequest) -> Result<PreflightResponse> {
        let mode = match req.mode {
            PreflightMode::Worktree => GraphMode::Worktree,
            PreflightMode::Snapshot => {
                if let Some(id) = &req.snapshot_id {
                    GraphMode::Snapshot(id.clone())
                } else {
                    return Err(anyhow!("Snapshot ID required for snapshot mode"));
                }
            }
        };

        let graph = self.get_graph(root, mode)?;
        let checker = PreflightChecker::new(root);
        let response = checker.check(&graph, &req)?;
        Ok(response)
    }

    pub fn overview(
        &self,
        root: &Path,
        snapshot_id: Option<String>,
    ) -> Result<Vec<FeatureOverview>> {
        let mode = if let Some(id) = snapshot_id {
            GraphMode::Snapshot(id)
        } else {
            GraphMode::Worktree
        };
        let graph = self.get_graph(root, mode)?;

        let mut overview = Vec::new();
        for node in &graph.features {
            overview.push(FeatureOverview {
                feature_id: node.feature_id.clone(),
                status: node.status.clone(),
                spec_path: node.spec_path.clone(),
                impl_files_count: node.impl_files.len(),
                test_files_count: node.test_files.len(),
            });
        }
        // sort by feature_id
        overview.sort_by(|a, b| a.feature_id.cmp(&b.feature_id));
        Ok(overview)
    }

    pub fn impact(
        &self,
        root: &Path,
        changed_paths: Vec<String>,
        snapshot_id: Option<String>,
    ) -> Result<Vec<String>> {
        let mode = if let Some(id) = snapshot_id {
            GraphMode::Snapshot(id)
        } else {
            GraphMode::Worktree
        };
        let graph = self.get_graph(root, mode)?;

        let mut impacted_features = Vec::new();
        // Naive iteration for now (N * M) but safe given graph size.
        // Can build reverse index if needed.
        for node in &graph.features {
            let mut hit = false;
            for path in &changed_paths {
                // Determine if path belongs to this feature
                // 1. Check spec_path
                if &node.spec_path == path {
                    hit = true;
                }
                // 2. Check impl_files
                if node.impl_files.contains(path) {
                    hit = true;
                }
                // 3. Check test_files
                if node.test_files.contains(path) {
                    hit = true;
                }
                if hit {
                    break;
                }
            }
            if hit {
                impacted_features.push(node.feature_id.clone());
            }
        }
        impacted_features.sort();
        impacted_features.dedup();
        Ok(impacted_features)
    }

    pub fn drift(&self, root: &Path, snapshot_id: Option<String>) -> Result<Vec<Violation>> {
        let mode = if let Some(id) = snapshot_id {
            GraphMode::Snapshot(id)
        } else {
            GraphMode::Worktree
        };
        let graph = self.get_graph(root, mode)?;

        let mut all_violations = graph.violations.clone();
        for node in &graph.features {
            all_violations.extend(node.violations.clone());
        }

        all_violations.sort_by(|a, b| a.code.cmp(&b.code).then(a.path.cmp(&b.path)));
        Ok(all_violations)
    }
}
