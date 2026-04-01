// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Opaque handle to the policy kernel for permission evaluation.
/// Wraps a boxed trait object so callers can supply any policy backend.
pub struct PolicyKernelHandle(pub Box<dyn PolicyEvaluator>);

/// Minimal interface the tool registry needs from the policy kernel.
/// Implementations bridge to `open_agentic_policy_kernel::evaluate`.
pub trait PolicyEvaluator: Send + Sync {
    fn evaluate(&self, tool_name: &str, arguments_summary: &str) -> PermissionResult;
}

/// Handle to an MCP client for delegating tool calls.
pub trait McpClient: Send + Sync {
    fn call_tool(&self, name: &str, input: Value) -> anyhow::Result<ToolResult>;
}

/// Shared context passed to tool permission checks and execution.
pub struct ToolContext {
    pub policy: Option<PolicyKernelHandle>,
    pub workflow_id: Option<String>,
    /// Opaque application state — tools downcast via `Any` if needed.
    pub state: Option<Box<dyn std::any::Any + Send + Sync>>,
}

impl ToolContext {
    pub fn empty() -> Self {
        Self {
            policy: None,
            workflow_id: None,
            state: None,
        }
    }
}

/// Result of a permission check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionResult {
    /// Tool may execute.
    Allow,
    /// Tool is blocked with a reason.
    Deny(String),
    /// Human confirmation required; includes the prompt.
    Ask(String),
}

/// Output of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Value,
    pub is_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl ToolResult {
    pub fn success(content: Value) -> Self {
        Self { content, is_error: false, metadata: None }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: Value::String(message.into()),
            is_error: true,
            metadata: None,
        }
    }
}

/// The core tool contract (FR-001). Object-safe (NF-002).
///
/// Every tool — built-in, MCP-bridged, or Tauri command — implements this trait
/// and registers with [`crate::ToolRegistry`].
pub trait ToolDef: Send + Sync {
    /// Unique tool name (e.g. `"file_read"`, `"mcp__server__tool"`).
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// JSON Schema describing the tool's input parameters.
    fn input_schema(&self) -> Value;

    /// Permission gate — consults the policy kernel before execution (FR-004).
    /// Defaults to `Ask` when no policy kernel is available.
    fn can_use(&self, ctx: &ToolContext) -> anyhow::Result<PermissionResult> {
        match &ctx.policy {
            Some(handle) => Ok(handle.0.evaluate(self.name(), "")),
            None => Ok(PermissionResult::Ask(
                format!("No policy kernel — confirm use of tool '{}'", self.name()),
            )),
        }
    }

    /// Execute the tool with the given (already schema-validated) input.
    fn execute(&self, input: Value, ctx: &mut ToolContext) -> anyhow::Result<ToolResult>;
}
