// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/075-elucid-workflow-engine/spec.md

//! Post-step verification hook execution (075 FR-005, FR-006).

use crate::manifest::VerifyCommand;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Result of running a verification command sequence.
#[derive(Clone, Debug)]
pub enum VerifyOutcome {
    /// All commands passed.
    Passed,
    /// A command failed. Includes the command index, command string, and stderr.
    Failed {
        command_index: usize,
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },
}

/// Run all verification commands in sequence (075 FR-005).
///
/// Commands run in the given `project_root` directory with `working_dir` resolved
/// relative to it. Returns on first failure.
pub async fn run_verify_commands(
    commands: &[VerifyCommand],
    project_root: &Path,
) -> VerifyOutcome {
    for (i, vc) in commands.iter().enumerate() {
        let work_dir = project_root.join(&vc.working_dir);

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(vc.timeout_ms),
            Command::new("sh")
                .arg("-c")
                .arg(&vc.command)
                .current_dir(&work_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) if output.status.success() => {
                // Command passed, continue to next.
            }
            Ok(Ok(output)) => {
                return VerifyOutcome::Failed {
                    command_index: i,
                    command: vc.command.clone(),
                    exit_code: output.status.code(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                };
            }
            Ok(Err(io_err)) => {
                return VerifyOutcome::Failed {
                    command_index: i,
                    command: vc.command.clone(),
                    exit_code: None,
                    stderr: format!("failed to execute: {io_err}"),
                };
            }
            Err(_elapsed) => {
                return VerifyOutcome::Failed {
                    command_index: i,
                    command: vc.command.clone(),
                    exit_code: None,
                    stderr: format!("timed out after {}ms", vc.timeout_ms),
                };
            }
        }
    }
    VerifyOutcome::Passed
}

/// Build a retry instruction by prepending verification failure context (075 FR-006).
pub fn build_retry_instruction(original_instruction: &str, stderr: &str) -> String {
    format!(
        "PREVIOUS ATTEMPT FAILED. Fix the following errors:\n\n\
         --- Verification Output ---\n\
         {stderr}\n\
         ---\n\n\
         Original instruction: {original_instruction}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn passing_commands() {
        let cmds = vec![
            VerifyCommand {
                command: "true".into(),
                working_dir: ".".into(),
                timeout_ms: 5000,
            },
            VerifyCommand {
                command: "echo ok".into(),
                working_dir: ".".into(),
                timeout_ms: 5000,
            },
        ];
        let result = run_verify_commands(&cmds, Path::new("/tmp")).await;
        assert!(matches!(result, VerifyOutcome::Passed));
    }

    #[tokio::test]
    async fn failing_command_returns_stderr() {
        let cmds = vec![
            VerifyCommand {
                command: "true".into(),
                working_dir: ".".into(),
                timeout_ms: 5000,
            },
            VerifyCommand {
                command: "sh -c 'echo boom >&2; exit 1'".into(),
                working_dir: ".".into(),
                timeout_ms: 5000,
            },
        ];
        let result = run_verify_commands(&cmds, Path::new("/tmp")).await;
        match result {
            VerifyOutcome::Failed {
                command_index,
                stderr,
                ..
            } => {
                assert_eq!(command_index, 1);
                assert!(stderr.contains("boom"));
            }
            _ => panic!("expected failure"),
        }
    }

    #[test]
    fn retry_instruction_format() {
        let retry = build_retry_instruction("write the code", "type error on line 5");
        assert!(retry.contains("PREVIOUS ATTEMPT FAILED"));
        assert!(retry.contains("type error on line 5"));
        assert!(retry.contains("write the code"));
    }
}
