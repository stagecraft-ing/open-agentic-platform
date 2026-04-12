// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: AGENT_AUTOMATION
// Spec: spec/agent/automation.md

use crate::schemas::PlanTask;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolTier {
    Tier1, // Autonomous — safe to auto-execute (read-only, diagnostic)
    Tier2, // Gated — requires human approval (writes, bounded mutations)
    Tier3, // Manual — dangerous or unclassified (execution, arbitrary commands)
}

/// Unified tool metadata: tier classification plus coarse permission requirements.
/// Single source of truth for both agent crate and axiomregent router (Feature 036).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolMetadata {
    pub tier: ToolTier,
    pub requires_file_read: bool,
    pub requires_file_write: bool,
    pub requires_network: bool,
}

impl ToolTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolTier::Tier1 => "tier1",
            ToolTier::Tier2 => "tier2",
            ToolTier::Tier3 => "tier3",
        }
    }
}

impl std::str::FromStr for ToolTier {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tier1" => Ok(ToolTier::Tier1),
            "tier2" => Ok(ToolTier::Tier2),
            "tier3" => Ok(ToolTier::Tier3),
            _ => Err(()),
        }
    }
}

/// Returns unified tier and permission metadata for a tool.
/// Single source of truth consumed by both the agent crate and axiomregent router (Feature 036).
pub fn get_tool_metadata(tool_name: &str) -> ToolMetadata {
    match tool_name {
        // Tier 1 — read-only / diagnostic
        "gov.preflight" | "gov.drift" | "features.impact" => ToolMetadata {
            tier: ToolTier::Tier1,
            requires_file_read: true,
            requires_file_write: false,
            requires_network: false,
        },
        "checkpoint.list"
        | "checkpoint.info"
        | "checkpoint.diff"
        | "checkpoint.verify"
        | "checkpoint.timeline"
        | "checkpoint.status" => ToolMetadata {
            tier: ToolTier::Tier1,
            requires_file_read: true,
            requires_file_write: false,
            requires_network: false,
        },
        // snapshot.* legacy aliases (registered by checkpoint provider)
        "snapshot.info" | "snapshot.list" | "snapshot.diff" => ToolMetadata {
            tier: ToolTier::Tier1,
            requires_file_read: true,
            requires_file_write: false,
            requires_network: false,
        },
        "xray.scan" => ToolMetadata {
            tier: ToolTier::Tier1,
            requires_file_read: true,
            requires_file_write: false,
            requires_network: false,
        },
        "run.status" | "run.logs" => ToolMetadata {
            tier: ToolTier::Tier1,
            requires_file_read: false,
            requires_file_write: false,
            requires_network: true,
        },
        "agent.verify" => ToolMetadata {
            tier: ToolTier::Tier1,
            requires_file_read: true,
            requires_file_write: false,
            requires_network: false,
        },

        // Tier 2 — bounded mutations
        // repo.* are the canonical names; workspace.* are backward-compat aliases
        "repo.apply_patch"
        | "repo.write_file"
        | "repo.delete"
        | "workspace.apply_patch"
        | "workspace.write_file"
        | "workspace.delete" => ToolMetadata {
            tier: ToolTier::Tier2,
            requires_file_read: false,
            requires_file_write: true,
            requires_network: false,
        },
        "write_file" => ToolMetadata {
            tier: ToolTier::Tier2,
            requires_file_read: false,
            requires_file_write: true,
            requires_network: false,
        },
        "checkpoint.create"
        | "checkpoint.restore"
        | "checkpoint.fork"
        | "checkpoint.gc" => ToolMetadata {
            tier: ToolTier::Tier2,
            requires_file_read: false,
            requires_file_write: true,
            requires_network: false,
        },
        "snapshot.create" => ToolMetadata {
            tier: ToolTier::Tier2,
            requires_file_read: false,
            requires_file_write: true,
            requires_network: false,
        },
        "agent.propose" => ToolMetadata {
            tier: ToolTier::Tier2,
            requires_file_read: false,
            requires_file_write: true,
            requires_network: false,
        },

        // Tier 3 — dangerous / explicit
        "run.execute" => ToolMetadata {
            tier: ToolTier::Tier3,
            requires_file_read: false,
            requires_file_write: false,
            requires_network: true,
        },
        "agent.execute" => ToolMetadata {
            tier: ToolTier::Tier3,
            requires_file_read: false,
            requires_file_write: true,
            requires_network: true,
        },

        // Skill commands (spec 071) — Tier2 gated, require file read
        name if name.starts_with("skill.") => ToolMetadata {
            tier: ToolTier::Tier2,
            requires_file_read: true,
            requires_file_write: false,
            requires_network: false,
        },

        // Tier 3 catch-all — unclassified / unknown tools
        _ => ToolMetadata {
            tier: ToolTier::Tier3,
            requires_file_read: false,
            requires_file_write: false,
            requires_network: false,
        },
    }
}

/// Returns the tier classification for a tool.
/// Thin wrapper around `get_tool_metadata` — use that function when permission flags are also needed.
pub fn get_tool_tier(tool_name: &str) -> ToolTier {
    get_tool_metadata(tool_name).tier
}

/// Returns the set of tool names that have explicit tier assignments (not the Tier3 catch-all).
/// Used by coverage tests to verify all router tools are classified.
pub fn explicitly_classified_tools() -> &'static [&'static str] {
    &[
        // Tier 1
        "gov.preflight",
        "gov.drift",
        "features.impact",
        // checkpoint read-only tools
        "checkpoint.list",
        "checkpoint.info",
        "checkpoint.diff",
        "checkpoint.verify",
        "checkpoint.timeline",
        "checkpoint.status",
        // snapshot.* legacy aliases (registered by checkpoint provider)
        "snapshot.info",
        "snapshot.list",
        "snapshot.diff",
        "xray.scan",
        "run.status",
        "run.logs",
        "agent.verify",
        // Tier 2 — canonical repo.* names
        "repo.apply_patch",
        "repo.write_file",
        "repo.delete",
        // workspace.* backward-compat aliases
        "workspace.apply_patch",
        "workspace.write_file",
        "workspace.delete",
        "write_file",
        "checkpoint.create",
        "checkpoint.restore",
        "checkpoint.fork",
        "checkpoint.gc",
        "snapshot.create",
        "agent.propose",
        // Tier 3 (explicit)
        "run.execute",
        "agent.execute",
    ]
}

/// Returns the full metadata (tier + permission flags) for all explicitly classified tools.
/// Provides a single source of truth for coverage tests and tooling that needs both tier and
/// permission data without calling `get_tool_metadata` per tool.
pub fn explicitly_classified_metadata() -> &'static [(&'static str, ToolMetadata)] {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Vec<(&'static str, ToolMetadata)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        explicitly_classified_tools()
            .iter()
            .map(|&name| (name, get_tool_metadata(name)))
            .collect()
    })
}

pub fn calculate_plan_tier(tasks: &[PlanTask]) -> ToolTier {
    let mut max_tier = ToolTier::Tier1;

    for task in tasks {
        for call in &task.tool_calls {
            let tool_tier = get_tool_tier(&call.tool_name);
            if tool_tier > max_tier {
                max_tier = tool_tier;
            }
        }
    }

    max_tier
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::ToolCall;
    use serde_json::json;

    fn make_task(tools: Vec<&str>) -> PlanTask {
        PlanTask {
            id: "t1".to_string(),
            step_type: "test".to_string(),
            description: "desc".to_string(),
            tool_calls: tools
                .into_iter()
                .map(|t| ToolCall {
                    tool_name: t.to_string(),
                    arguments: json!({}),
                })
                .collect(),
        }
    }

    #[test]
    fn test_tier_calculation() {
        // Pure Tier 1
        let t1 = make_task(vec!["gov.preflight", "features.impact"]);
        assert_eq!(calculate_plan_tier(&[t1]), ToolTier::Tier1);

        // Tier 2 introduced
        let t2 = make_task(vec!["write_file"]);
        assert_eq!(
            calculate_plan_tier(&[make_task(vec!["gov.drift"]), t2]),
            ToolTier::Tier2
        );

        // Unknown tool -> Tier 3
        let t3 = make_task(vec!["rm_rf_root"]);
        assert_eq!(calculate_plan_tier(&[t3]), ToolTier::Tier3);

        // Newly classified Tier 1 tools (Feature 036)
        let t4 = make_task(vec!["checkpoint.info", "xray.scan", "run.logs"]);
        assert_eq!(calculate_plan_tier(&[t4]), ToolTier::Tier1);

        // agent.propose is Tier 2
        let t5 = make_task(vec!["agent.propose"]);
        assert_eq!(calculate_plan_tier(&[t5]), ToolTier::Tier2);

        // agent.execute is Tier 3
        let t6 = make_task(vec!["agent.execute"]);
        assert_eq!(calculate_plan_tier(&[t6]), ToolTier::Tier3);
    }
}
