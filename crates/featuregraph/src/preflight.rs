// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: GOVERNANCE_ENGINE
// Spec: spec/core/governance.md

use crate::graph::{FeatureGraph, Violation};
use crate::scanner::HeaderParser;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightIntent {
    Edit,
    Create,
    Delete,
    Refactor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightMode {
    Worktree,
    Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightRequest {
    pub intent: PreflightIntent,
    pub mode: PreflightMode,
    pub changed_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeTier {
    #[serde(rename = "tier1")]
    Tier1, // Autonomous
    #[serde(rename = "tier2")]
    Tier2, // Gated
    #[serde(rename = "tier3")]
    Tier3, // Forbidden
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightResponse {
    pub allowed: bool,
    pub safety_tier: ChangeTier,
    pub violations: Vec<Violation>,
    pub graph_fingerprint: String,
}

pub struct PreflightChecker {
    root: PathBuf,
    parser: HeaderParser,
}

impl PreflightChecker {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            parser: HeaderParser::new(),
        }
    }

    pub fn check(
        &self,
        graph: &FeatureGraph,
        req: &PreflightRequest,
    ) -> Result<PreflightResponse, anyhow::Error> {
        let mut violations = Vec::new();
        let known_features: HashSet<&String> =
            graph.features.iter().map(|f| &f.feature_id).collect();

        // 1. Check Policy Violations
        self.check_policy_violations(req, &mut violations);

        // 2. Check Feature Graph Consistency

        for rel_path in &req.changed_paths {
            let abs_path = self.root.join(rel_path);

            if !abs_path.exists() {
                continue;
            }

            if !is_eligible_file(rel_path) {
                continue;
            }

            match self.parser.parse_file(&abs_path) {
                Ok(header) => {
                    if let Some(fid) = &header.feature_id {
                        if !known_features.contains(fid) {
                            violations.push(Violation {
                                code: "DANGLING_FEATURE_ID".to_string(),
                                severity: "error".to_string(),
                                path: rel_path.clone(),
                                feature_id: Some(fid.clone()),
                                message: format!(
                                    "Feature '{}' is not defined in the feature manifest (registry.json or spec/features.yaml)",
                                    fid
                                ),
                                suggested_fix: Some(
                                    "Add feature to the compiled registry (spec-compiler) or spec/features.yaml"
                                        .to_string(),
                                ),
                            });
                        } else {
                            // Check SPEC_PATH_MISMATCH
                            if let Some(node) = graph.features.iter().find(|f| &f.feature_id == fid)
                                && let Some(declared) = &header.spec_path
                                && declared != &node.spec_path
                            {
                                violations.push(Violation {
                                    code: "SPEC_PATH_MISMATCH".to_string(),
                                    severity: "warning".to_string(),
                                    path: rel_path.clone(),
                                    feature_id: Some(fid.clone()),
                                    message: format!(
                                        "File declares spec {} but registry says {}",
                                        declared, node.spec_path
                                    ),
                                    suggested_fix: Some(format!(
                                        "Update header to Spec: {}",
                                        node.spec_path
                                    )),
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    violations.push(Violation {
                        code: "INVALID_HEADER_FORMAT".to_string(),
                        severity: "error".to_string(),
                        path: rel_path.clone(),
                        feature_id: None,
                        message: e.to_string(),
                        suggested_fix: Some("Fix header format".to_string()),
                    });
                }
            }
        }

        violations.sort_by(|a, b| a.code.cmp(&b.code).then(a.path.cmp(&b.path)));

        let safety_tier = self.calculate_safety_tier(req, &violations);

        // Tier 3 is never allowed
        let allowed = violations.is_empty() && safety_tier != ChangeTier::Tier3;

        Ok(PreflightResponse {
            allowed,
            safety_tier,
            violations,
            graph_fingerprint: graph.graph_fingerprint.clone(),
        })
    }

    fn check_policy_violations(&self, req: &PreflightRequest, violations: &mut Vec<Violation>) {
        for path in &req.changed_paths {
            // Policy: Do not edit generated files
            if path.contains("generated/") || path.ends_with(".gen.rs") {
                violations.push(Violation {
                    code: "EDIT_GENERATED_FILE".to_string(),
                    severity: "error".to_string(),
                    path: path.clone(),
                    feature_id: None,
                    message: "Manual edits to generated files are forbidden".to_string(),
                    suggested_fix: Some("Modify the source generator instead".to_string()),
                });
            }
        }
    }

    fn calculate_safety_tier(
        &self,
        req: &PreflightRequest,
        violations: &[Violation],
    ) -> ChangeTier {
        // If there are any errors or Forbidden violations, it's Tier 3
        if violations.iter().any(|v| v.severity == "error") {
            return ChangeTier::Tier3;
        }

        // Tier 1: Documentation only changes
        let all_docs = req.changed_paths.iter().all(|p| {
            p.ends_with(".md") || p.ends_with(".txt") || p.ends_with(".png") || p.ends_with(".jpg")
        });

        if all_docs {
            return ChangeTier::Tier1;
        }

        // Tier 2: Code changes (Default)
        ChangeTier::Tier2
    }
}

fn is_eligible_file(path: &str) -> bool {
    let allowed_exts = [
        ".go", ".rs", ".ts", ".tsx", ".js", ".jsx", ".c", ".cc", ".cpp", ".h", ".hpp", ".java",
        ".kt", ".py", ".sh", ".bash", ".zsh",
    ];
    for ext in allowed_exts {
        if path.ends_with(ext) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{FeatureGraph, FeatureNode};
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_preflight_dangling() {
        // Setup graph
        let mut graph = FeatureGraph::new();
        graph.features.push(FeatureNode {
            feature_id: "KNOWN".to_string(),
            title: "Known Feature".to_string(),
            spec_path: "spec/known.md".to_string(),
            status: "done".to_string(),
            governance: "approved".to_string(),
            owner: "test-team".to_string(),
            group: "test".to_string(),
            depends_on: vec![],
            impl_files: vec![],
            test_files: vec![],
            violations: vec![],
        });

        // We need to trick is_eligible_file check or rename tempfile
        // Tempfile usually has random name. We can't easily control extension with NamedTempFile without builder.
        // Let's create a dir and write a file with extension.
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "// Feature: UNKNOWN").unwrap();

        let checker = PreflightChecker::new(temp_dir.path());
        let req = PreflightRequest {
            intent: PreflightIntent::Edit,
            mode: PreflightMode::Worktree,
            changed_paths: vec!["test.rs".to_string()],
            snapshot_id: None,
        };

        let res = checker.check(&graph, &req).unwrap();
        assert!(!res.allowed);
        assert_eq!(res.violations[0].code, "DANGLING_FEATURE_ID");
    }
}
