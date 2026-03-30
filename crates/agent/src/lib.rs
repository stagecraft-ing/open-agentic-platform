// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: AGENT_AUTOMATION
// Spec: spec/agent/automation.md

pub mod agent;
pub mod canonical;
pub mod complexity;
pub mod dispatch;
pub mod executor;
pub mod id;
pub mod plan;
pub mod safety;
pub mod schemas;
pub mod validator;
pub mod verification;

pub use dispatch::{MandatoryOutcome, build_execution_plan, evaluate_mandatory_triggers};
