//! MCP resource definitions for gitctx.
//!
//! This module contains resource-related constants. The actual resource
//! implementation is in `server.rs` as part of the `ServerHandler` implementation.

/// Resource URI for the current repository context.
pub const CONTEXT_RESOURCE_URI: &str = "gitctx://context/current";
