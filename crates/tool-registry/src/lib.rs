// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Unified Tool Definition Registry (spec 067).
//!
//! Every tool in the platform — Rust crate tools, OPC Tauri commands, and
//! MCP-bridged tools — registers through a single schema-driven interface.

mod types;
mod registry;
mod mcp;
mod event;

pub use types::{ToolDef, ToolContext, ToolResult, PermissionResult};
pub use registry::ToolRegistry;
pub use mcp::McpToolDef;
pub use event::{ToolEvent, ToolEventKind};

#[cfg(test)]
mod tests;
