// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/075-elucid-workflow-engine/spec.md — FR-007

//! Bridges Elucid agent definitions to the OAP `AgentRegistry` trait.

use async_trait::async_trait;
use elucid_contracts::AgentPrompt;
use orchestrator::AgentRegistry;
use std::collections::HashSet;

/// Bridges Elucid process and adapter agents into the orchestrator's `AgentRegistry`.
///
/// Agent ID mapping:
/// - Process agents: `elucid-{role}` (e.g., `elucid-business-analyst`)
/// - Scaffold agents: `elucid-{role}-{adapter}` (e.g., `elucid-api-scaffolder-next-prisma`)
pub struct ElucidAgentBridge {
    agent_ids: HashSet<String>,
    prompts: Vec<AgentPrompt>,
}

impl ElucidAgentBridge {
    /// Create a bridge from loaded process and adapter agent prompts.
    pub fn new(process_agents: Vec<AgentPrompt>, adapter_agents: Vec<AgentPrompt>) -> Self {
        let mut agent_ids = HashSet::new();
        let mut prompts = Vec::new();

        for agent in process_agents {
            agent_ids.insert(agent.id.clone());
            prompts.push(agent);
        }

        for agent in adapter_agents {
            agent_ids.insert(agent.id.clone());
            prompts.push(agent);
        }

        Self { agent_ids, prompts }
    }

    /// Get the prompt text for a given agent ID.
    pub fn get_prompt(&self, agent_id: &str) -> Option<&str> {
        self.prompts
            .iter()
            .find(|a| a.id == agent_id)
            .map(|a| a.prompt_text.as_str())
    }

    /// List all registered agent IDs.
    pub fn agent_ids(&self) -> impl Iterator<Item = &str> {
        self.agent_ids.iter().map(String::as_str)
    }

    /// Number of registered agents.
    pub fn len(&self) -> usize {
        self.agent_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agent_ids.is_empty()
    }
}

#[async_trait]
impl AgentRegistry for ElucidAgentBridge {
    async fn has_agent(&self, agent_id: &str) -> bool {
        self.agent_ids.contains(agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(id: &str, role: &str) -> AgentPrompt {
        AgentPrompt {
            id: id.into(),
            role: role.into(),
            tier: 1,
            prompt_text: format!("You are the {role} agent."),
            model_hint: None,
            source_path: std::path::PathBuf::from(format!("agents/{id}.md")),
        }
    }

    #[tokio::test]
    async fn bridge_registers_all_agents() {
        let process = vec![
            make_agent("elucid-business-analyst", "business-analyst"),
            make_agent("elucid-data-architect", "data-architect"),
        ];
        let adapter = vec![
            make_agent("elucid-api-scaffolder-next-prisma", "api-scaffolder"),
        ];

        let bridge = ElucidAgentBridge::new(process, adapter);
        assert_eq!(bridge.len(), 3);
        assert!(bridge.has_agent("elucid-business-analyst").await);
        assert!(bridge.has_agent("elucid-api-scaffolder-next-prisma").await);
        assert!(!bridge.has_agent("unknown-agent").await);
    }

    #[test]
    fn get_prompt_returns_text() {
        let agents = vec![make_agent("elucid-test", "tester")];
        let bridge = ElucidAgentBridge::new(agents, vec![]);
        assert!(bridge.get_prompt("elucid-test").unwrap().contains("tester"));
        assert!(bridge.get_prompt("missing").is_none());
    }
}
