// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/075-elucid-workflow-engine/spec.md — FR-010

//! Integration with Elucid's Python verification harness.
//!
//! The Python harness (`elucid/process/harness/`) is the reference implementation
//! for stage gate checks. Long-term, these migrate to Rust; short-term we invoke
//! the harness as a subprocess.

use crate::ElucidError;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Result of a gate check invocation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateCheckResult {
    pub passed: bool,
    pub stage_id: String,
    pub checks: Vec<CheckResult>,
    pub stdout: String,
    pub stderr: String,
}

/// Individual check result within a gate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckResult {
    pub id: String,
    pub passed: bool,
    pub message: Option<String>,
}

/// Run Elucid gate checks for a completed stage (FR-010).
///
/// Invokes the Python verification harness as a subprocess:
/// ```text
/// python -m elucid gate --stage <stage_id> --project <project_path>
/// ```
pub async fn run_elucid_gate_check(
    stage_id: &str,
    project_path: &Path,
    elucid_root: &Path,
) -> Result<GateCheckResult, ElucidError> {
    let harness_dir = elucid_root.join("process").join("harness");

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        Command::new("python3")
            .arg("-m")
            .arg("elucid")
            .arg("gate")
            .arg("--stage")
            .arg(stage_id)
            .arg("--project")
            .arg(project_path)
            .current_dir(&harness_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

            // Try to parse structured JSON output from the harness.
            let checks = parse_harness_output(&stdout);
            let passed = output.status.success() && checks.iter().all(|c| c.passed);

            Ok(GateCheckResult {
                passed,
                stage_id: stage_id.into(),
                checks,
                stdout,
                stderr,
            })
        }
        Ok(Err(io_err)) => Err(ElucidError::VerificationHarness {
            reason: format!("failed to execute Python harness: {io_err}"),
        }),
        Err(_elapsed) => Err(ElucidError::VerificationHarness {
            reason: format!("gate check for stage {stage_id} timed out after 120s"),
        }),
    }
}

/// Try to parse JSON check results from harness stdout.
/// Falls back to a single synthetic check if output isn't parseable.
fn parse_harness_output(stdout: &str) -> Vec<CheckResult> {
    // The harness outputs JSON lines, one per check.
    let mut results = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') {
            if let Ok(check) = serde_json::from_str::<CheckResult>(trimmed) {
                results.push(check);
            }
        }
    }
    if results.is_empty() && !stdout.trim().is_empty() {
        // Couldn't parse — create a synthetic result.
        results.push(CheckResult {
            id: "harness-output".into(),
            passed: true, // Assume passed if no structured output; exit code is authoritative.
            message: Some(stdout.trim().to_string()),
        });
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_check_results() {
        let stdout = r#"{"id":"check-1","passed":true,"message":"ok"}
{"id":"check-2","passed":false,"message":"missing artifact"}
"#;
        let checks = parse_harness_output(stdout);
        assert_eq!(checks.len(), 2);
        assert!(checks[0].passed);
        assert!(!checks[1].passed);
    }

    #[test]
    fn parse_non_json_output() {
        let stdout = "All checks passed\n";
        let checks = parse_harness_output(stdout);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].id, "harness-output");
    }

    #[test]
    fn parse_empty_output() {
        let checks = parse_harness_output("");
        assert!(checks.is_empty());
    }
}
