// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: AGENT_AUTOMATION
// Spec: spec/agent/automation.md

use crate::schemas::PlanTask;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Tier1, // Safe to auto-execute
    Tier2, // Requires human approval
    Tier3, // Human only
}

impl Tier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Tier::Tier1 => "tier1",
            Tier::Tier2 => "tier2",
            Tier::Tier3 => "tier3",
        }
    }
}

impl std::str::FromStr for Tier {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tier1" => Ok(Tier::Tier1),
            "tier2" => Ok(Tier::Tier2),
            "tier3" => Ok(Tier::Tier3),
            _ => Err(()),
        }
    }
}

pub fn get_tool_tier(tool_name: &str) -> Tier {
    match tool_name {
        // Tier 1: Read-only / Safe checks
        "gov.preflight" | "gov.drift" | "features.impact" | "snapshot.info" => Tier::Tier1,

        // Tier 2: Modifications that can be impactful
        "workspace.apply_patch"
        | "workspace.write_file"
        | "workspace.delete"
        | "write_file"
        | "snapshot.create" => Tier::Tier2,

        // Tier 3: Anything else is assumed dangerous/unknown until classified
        _ => Tier::Tier3,
    }
}

pub fn calculate_plan_tier(tasks: &[PlanTask]) -> Tier {
    let mut max_tier = Tier::Tier1;

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
        assert_eq!(calculate_plan_tier(&[t1]), Tier::Tier1);

        // Tier 2 introduced
        let t2 = make_task(vec!["write_file"]);
        assert_eq!(
            calculate_plan_tier(&[make_task(vec!["gov.drift"]), t2]),
            Tier::Tier2
        );

        // Unknown tool -> Tier 3
        let t3 = make_task(vec!["rm_rf_root"]);
        assert_eq!(calculate_plan_tier(&[t3]), Tier::Tier3);
    }
}
