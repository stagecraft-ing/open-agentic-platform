// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// GovernedExecutor implementation that spawns the `claude` CLI for real agent dispatch.

use crate::{DispatchRequest, DispatchResult, GovernedExecutor};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

/// Callback trait for looking up agent domain prompts by ID.
///
/// This indirection avoids a circular dependency on factory-engine from the
/// orchestrator crate. Callers that have access to agent definitions can
/// implement this trait and inject it via `ClaudeCodeExecutor::with_prompt_lookup`.
pub trait AgentPromptLookup: Send + Sync {
    fn get_prompt(&self, agent_id: &str) -> Option<String>;
}

/// Executes workflow steps by spawning the `claude` CLI process.
///
/// Builds a system prompt from the registered agent domain prompt (if any)
/// combined with `DispatchRequest::system_prompt`, then invokes:
///
/// ```text
/// claude --print --output-format json --max-turns <N> \
///        --allowedTools <tools> --system-prompt <prompt> \
///        "<step_id>"
/// ```
///
/// Token usage is extracted from the JSON response and returned in
/// `DispatchResult::tokens_used`.
pub struct ClaudeCodeExecutor {
    project_path: PathBuf,
    prompt_lookup: Option<Arc<dyn AgentPromptLookup>>,
    max_turns: u32,
    allowed_tools: Vec<String>,
}

impl ClaudeCodeExecutor {
    /// Create a new executor anchored at `project_path`.
    ///
    /// Defaults: `max_turns = 25`, `allowed_tools = ["Read", "Write", "Bash", "Glob", "Grep"]`.
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            prompt_lookup: None,
            max_turns: 25,
            allowed_tools: vec![
                "Read".into(),
                "Write".into(),
                "Bash".into(),
                "Glob".into(),
                "Grep".into(),
            ],
        }
    }

    /// Attach a prompt lookup so the executor can prepend agent-specific
    /// domain prompts to each request.
    pub fn with_prompt_lookup(mut self, lookup: Arc<dyn AgentPromptLookup>) -> Self {
        self.prompt_lookup = Some(lookup);
        self
    }

    /// Override the default maximum number of agentic turns per step.
    pub fn with_max_turns(mut self, max_turns: u32) -> Self {
        self.max_turns = max_turns;
        self
    }

    /// Override the list of tools the `claude` process is allowed to use.
    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools;
        self
    }

    /// Build the combined system prompt for a request.
    fn build_system_prompt(&self, request: &DispatchRequest) -> String {
        let domain = self
            .prompt_lookup
            .as_ref()
            .and_then(|lk| lk.get_prompt(&request.agent_id));

        match domain {
            Some(d) if !d.is_empty() => format!("{}\n\n{}", d, request.system_prompt),
            _ => request.system_prompt.clone(),
        }
    }
}

#[async_trait]
impl GovernedExecutor for ClaudeCodeExecutor {
    async fn dispatch_step(&self, request: DispatchRequest) -> Result<DispatchResult, String> {
        let system_prompt = self.build_system_prompt(&request);
        let tools_arg = self.allowed_tools.join(",");
        let effective_max_turns = match request.effort {
            crate::EffortLevel::Deep => self.max_turns * 2,
            _ => self.max_turns,
        };
        let max_turns_str = effective_max_turns.to_string();

        // The user message is a brief task summary derived from the step ID.
        let user_message = format!("Execute step: {}", request.step_id);

        let output = tokio::process::Command::new("claude")
            .args([
                "--print",
                "--output-format",
                "json",
                "--max-turns",
                &max_turns_str,
                "--allowedTools",
                &tools_arg,
                "--system-prompt",
                &system_prompt,
                &user_message,
            ])
            .current_dir(&self.project_path)
            .output()
            .await
            .map_err(|e| format!("failed to spawn claude: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                let code = output
                    .status
                    .code()
                    .map_or_else(|| "unknown".to_string(), |c| c.to_string());
                return Err(format!(
                    "claude exited with status {code} (no stderr output)"
                ));
            }
            return Err(stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let tokens_used = parse_tokens(&stdout);

        Ok(DispatchResult { tokens_used })
    }
}

/// Extract token counts from the claude JSON output.
///
/// Expects the top-level `usage` object with `input_tokens` and
/// `output_tokens` fields. Returns `None` if the structure is absent or
/// cannot be parsed, so the executor remains resilient to format changes.
fn parse_tokens(json_str: &str) -> Option<u64> {
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let usage = v.get("usage")?;
    let input = usage.get("input_tokens")?.as_u64().unwrap_or(0);
    let output = usage.get("output_tokens")?.as_u64().unwrap_or(0);
    Some(input + output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tokens_extracts_sum() {
        let json = r#"{"result":"done","usage":{"input_tokens":100,"output_tokens":50}}"#;
        assert_eq!(parse_tokens(json), Some(150));
    }

    #[test]
    fn parse_tokens_returns_none_on_missing_usage() {
        let json = r#"{"result":"done"}"#;
        assert_eq!(parse_tokens(json), None);
    }

    #[test]
    fn parse_tokens_returns_none_on_invalid_json() {
        assert_eq!(parse_tokens("not json"), None);
    }

    #[test]
    fn build_system_prompt_combines_domain_and_request() {
        struct StaticLookup;
        impl AgentPromptLookup for StaticLookup {
            fn get_prompt(&self, _agent_id: &str) -> Option<String> {
                Some("Domain context.".into())
            }
        }

        let executor = ClaudeCodeExecutor::new("/tmp".into())
            .with_prompt_lookup(Arc::new(StaticLookup));

        let req = DispatchRequest {
            step_id: "s1".into(),
            agent_id: "agent-a".into(),
            effort: crate::EffortLevel::Investigate,
            system_prompt: "Task instructions.".into(),
            input_artifacts: vec![],
            output_artifacts: vec![],
        };

        let prompt = executor.build_system_prompt(&req);
        assert_eq!(prompt, "Domain context.\n\nTask instructions.");
    }

    #[test]
    fn build_system_prompt_falls_back_when_no_lookup() {
        let executor = ClaudeCodeExecutor::new("/tmp".into());

        let req = DispatchRequest {
            step_id: "s1".into(),
            agent_id: "agent-a".into(),
            effort: crate::EffortLevel::Investigate,
            system_prompt: "Task instructions.".into(),
            input_artifacts: vec![],
            output_artifacts: vec![],
        };

        let prompt = executor.build_system_prompt(&req);
        assert_eq!(prompt, "Task instructions.");
    }

    #[test]
    fn builder_setters_work() {
        let exec = ClaudeCodeExecutor::new("/workspace".into())
            .with_max_turns(10)
            .with_allowed_tools(vec!["Read".into()]);
        assert_eq!(exec.max_turns, 10);
        assert_eq!(exec.allowed_tools, vec!["Read"]);
    }

    #[test]
    fn effective_max_turns_doubles_for_deep() {
        let exec = ClaudeCodeExecutor::new("/tmp".into()).with_max_turns(25);
        let deep = match crate::EffortLevel::Deep {
            crate::EffortLevel::Deep => exec.max_turns * 2,
            _ => exec.max_turns,
        };
        assert_eq!(deep, 50);
        let quick = match crate::EffortLevel::Quick {
            crate::EffortLevel::Deep => exec.max_turns * 2,
            _ => exec.max_turns,
        };
        assert_eq!(quick, 25);
        let investigate = match crate::EffortLevel::Investigate {
            crate::EffortLevel::Deep => exec.max_turns * 2,
            _ => exec.max_turns,
        };
        assert_eq!(investigate, 25);
    }
}
