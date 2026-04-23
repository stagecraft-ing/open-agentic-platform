// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Legacy `goa-software-factory` manifest completion assessment.
//!
//! The legacy shape has five numbered stage keys:
//!
//! - `stage1_businessRequirements`
//! - `stage2_serviceRequirements`
//! - `stage3_databaseDesign`
//! - `stage4_apiControllers`
//! - `stage5_clientInterface`
//!
//! A stage counts as terminal-complete iff `status` is a recognised
//! completion token (`PASSED`, `PASS`, `COMPLETE`, `COMPLETED`) **and**
//! `completedAt` is a non-empty string. Non-stage sibling keys (e.g.
//! `audit`, `clientDocumentation`) are ignored for completeness; they are
//! supplementary artefacts, not gates.

use serde::{Deserialize, Serialize};

pub const REQUIRED_STAGES: [&str; 5] = [
    "stage1_businessRequirements",
    "stage2_serviceRequirements",
    "stage3_databaseDesign",
    "stage4_apiControllers",
    "stage5_clientInterface",
];

const TERMINAL_STATUS_TOKENS: [&str; 4] = ["PASSED", "PASS", "COMPLETE", "COMPLETED"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LegacyStageStatus {
    Passed,
    Other(String),
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyManifest {
    #[serde(default)]
    pub stages: serde_json::Map<String, serde_json::Value>,
}

/// Returns `(complete, incomplete_stage_names)`. `complete` is true only
/// when all five required stages are terminally complete with a
/// `completedAt` timestamp.
pub fn assess_completion(manifest: &serde_json::Value) -> (bool, Vec<String>) {
    let stages = match manifest.get("stages").and_then(|v| v.as_object()) {
        Some(o) => o,
        None => return (false, REQUIRED_STAGES.iter().map(|s| s.to_string()).collect()),
    };

    let mut incomplete = Vec::new();
    for required in REQUIRED_STAGES {
        let stage = stages.get(required);
        if !is_stage_complete(stage) {
            incomplete.push(required.to_string());
        }
    }
    (incomplete.is_empty(), incomplete)
}

fn is_stage_complete(stage: Option<&serde_json::Value>) -> bool {
    let Some(stage) = stage else { return false };
    let status = stage
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    if !TERMINAL_STATUS_TOKENS.contains(&status.as_str()) {
        return false;
    }
    stage
        .get("completedAt")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_when_all_five_stages_pass() {
        let manifest = serde_json::json!({
            "stages": {
                "stage1_businessRequirements": { "status": "PASSED", "completedAt": "2026-04-21T13:32:00Z" },
                "stage2_serviceRequirements": { "status": "PASSED", "completedAt": "2026-04-21T14:15:00Z" },
                "stage3_databaseDesign":      { "status": "passed", "completedAt": "2026-04-21T15:05:00Z" },
                "stage4_apiControllers":      { "status": "COMPLETED", "completedAt": "2026-04-22T00:00:00Z" },
                "stage5_clientInterface":     { "status": "PASS", "completedAt": "2026-04-21T22:15:00Z" }
            }
        });
        let (complete, incomplete) = assess_completion(&manifest);
        assert!(complete);
        assert!(incomplete.is_empty());
    }

    #[test]
    fn missing_stage_is_incomplete() {
        let manifest = serde_json::json!({
            "stages": {
                "stage1_businessRequirements": { "status": "PASSED", "completedAt": "2026-04-21T13:32:00Z" }
            }
        });
        let (complete, incomplete) = assess_completion(&manifest);
        assert!(!complete);
        assert_eq!(incomplete.len(), 4);
    }

    #[test]
    fn status_without_completed_at_is_incomplete() {
        let manifest = serde_json::json!({
            "stages": {
                "stage1_businessRequirements": { "status": "PASSED" },
                "stage2_serviceRequirements": { "status": "PASSED", "completedAt": "2026-04-21T14:15:00Z" },
                "stage3_databaseDesign":      { "status": "PASSED", "completedAt": "2026-04-21T15:05:00Z" },
                "stage4_apiControllers":      { "status": "PASSED", "completedAt": "2026-04-22T00:00:00Z" },
                "stage5_clientInterface":     { "status": "PASSED", "completedAt": "2026-04-21T22:15:00Z" }
            }
        });
        let (complete, incomplete) = assess_completion(&manifest);
        assert!(!complete);
        assert_eq!(incomplete, vec!["stage1_businessRequirements"]);
    }

    #[test]
    fn non_terminal_status_is_incomplete() {
        let manifest = serde_json::json!({
            "stages": {
                "stage1_businessRequirements": { "status": "IN_PROGRESS", "completedAt": "" },
                "stage2_serviceRequirements": { "status": "PASSED", "completedAt": "2026-04-21T14:15:00Z" },
                "stage3_databaseDesign":      { "status": "PASSED", "completedAt": "2026-04-21T15:05:00Z" },
                "stage4_apiControllers":      { "status": "PASSED", "completedAt": "2026-04-22T00:00:00Z" },
                "stage5_clientInterface":     { "status": "PASSED", "completedAt": "2026-04-21T22:15:00Z" }
            }
        });
        let (complete, _) = assess_completion(&manifest);
        assert!(!complete);
    }
}
