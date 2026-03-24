// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: MCP_SNAPSHOT_WORKSPACE
// Spec: spec/core/snapshot-workspace.md

pub mod lease;
pub mod store;
pub mod tools;

// We will implement the actual tools in the submodules or here?
// For cleanliness, we can keep the tool impls in submodules or a tools.rs file.
// Let's expose them here for now.
