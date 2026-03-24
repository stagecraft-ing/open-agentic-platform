// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: MCP_ROUTER
// Spec: spec/core/router.md

#![recursion_limit = "256"] // Increased for large json! macros in router

pub mod agent_tools;
pub mod config;
pub mod feature_tools;
pub mod internal_client;
pub mod io;
pub mod router;
// pub mod skills;
pub mod snapshot;
pub mod util;
pub mod workspace;
pub use featuregraph;
pub use xray;
pub mod run_tools;
// pub mod xray_tools;
