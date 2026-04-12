// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Core types for the Unified Agent and Skill Frontmatter Schema (spec 054).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// AgentType — superset of SkillType (FR-004)
// ---------------------------------------------------------------------------

/// Agent/skill execution type. Superset of `SkillType` from skill-factory.
///
/// - `prompt`, `agent`, `headless` correspond to the original `SkillType` variants.
/// - `process` and `scaffold` are factory-specific (Tier 1 and Tier 2 respectively).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    #[default]
    Prompt,
    Agent,
    Headless,
    Process,
    Scaffold,
}

// ---------------------------------------------------------------------------
// SafetyTier — maps to/from ToolTier (FR-006)
// ---------------------------------------------------------------------------

/// Safety tier classification for agents. Maps to `ToolTier` in `crates/agent/src/safety.rs`.
///
/// Custom deserialization accepts both string (`"tier1"`) and integer (`1`) formats
/// for backward compatibility with factory agents that use `tier: 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SafetyTier {
    Tier1,
    Tier2,
    Tier3,
}

impl SafetyTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            SafetyTier::Tier1 => "tier1",
            SafetyTier::Tier2 => "tier2",
            SafetyTier::Tier3 => "tier3",
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            SafetyTier::Tier1 => 1,
            SafetyTier::Tier2 => 2,
            SafetyTier::Tier3 => 3,
        }
    }
}

impl Serialize for SafetyTier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SafetyTier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SafetyTierVisitor;

        impl<'de> serde::de::Visitor<'de> for SafetyTierVisitor {
            type Value = SafetyTier;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    "a safety tier string (\"tier1\"/\"tier2\"/\"tier3\") or integer (1/2/3)",
                )
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<SafetyTier, E> {
                match v {
                    "tier1" => Ok(SafetyTier::Tier1),
                    "tier2" => Ok(SafetyTier::Tier2),
                    "tier3" => Ok(SafetyTier::Tier3),
                    _ => Err(E::custom(format!("unknown safety tier: {v}"))),
                }
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<SafetyTier, E> {
                match v {
                    1 => Ok(SafetyTier::Tier1),
                    2 => Ok(SafetyTier::Tier2),
                    3 => Ok(SafetyTier::Tier3),
                    _ => Err(E::custom(format!(
                        "safety tier integer must be 1, 2, or 3; got {v}"
                    ))),
                }
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<SafetyTier, E> {
                if v >= 0 {
                    self.visit_u64(v as u64)
                } else {
                    Err(E::custom(format!(
                        "safety tier integer must be 1, 2, or 3; got {v}"
                    )))
                }
            }
        }

        deserializer.deserialize_any(SafetyTierVisitor)
    }
}

// ---------------------------------------------------------------------------
// MutationCapability (FR-007)
// ---------------------------------------------------------------------------

/// Structured replacement for the desktop's boolean permission flags.
///
/// Derivable from `SafetyTier` when absent:
/// - `Tier1` → `ReadOnly`
/// - `Tier2` → `ReadWrite`
/// - `Tier3` → `Full`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationCapability {
    #[serde(rename = "read-only")]
    ReadOnly,
    #[serde(rename = "read-write")]
    ReadWrite,
    #[serde(rename = "full")]
    Full,
}

impl From<SafetyTier> for MutationCapability {
    fn from(tier: SafetyTier) -> Self {
        match tier {
            SafetyTier::Tier1 => MutationCapability::ReadOnly,
            SafetyTier::Tier2 => MutationCapability::ReadWrite,
            SafetyTier::Tier3 => MutationCapability::Full,
        }
    }
}

// ---------------------------------------------------------------------------
// GovernanceRequirement (FR-008)
// ---------------------------------------------------------------------------

/// Governance requirement for agent execution. Connects to spec 098's `governance_mode`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GovernanceRequirement {
    #[default]
    None,
    Advisory,
    Enforced,
}

// ---------------------------------------------------------------------------
// AllowedTools (FR-005) — moved from skill-factory
// ---------------------------------------------------------------------------

/// Which tools an agent or skill is allowed to use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AllowedTools {
    /// All tools permitted (wildcard `"*"`).
    All(AllToolsMarker),
    /// Specific tool names.
    List(Vec<String>),
}

/// Marker for the `"*"` wildcard in YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllToolsMarker(#[serde(deserialize_with = "deserialize_star")] String);

fn deserialize_star<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s == "*" {
        Ok(s)
    } else {
        Err(serde::de::Error::custom("expected \"*\""))
    }
}

impl Default for AllowedTools {
    fn default() -> Self {
        Self::All(AllToolsMarker("*".into()))
    }
}

impl AllowedTools {
    /// Convenience constructor for the wildcard.
    pub fn all() -> Self {
        Self::All(AllToolsMarker("*".into()))
    }

    /// Convenience constructor for a specific list.
    pub fn list(tools: Vec<String>) -> Self {
        Self::List(tools)
    }

    /// Returns true when all tools are allowed.
    pub fn is_all(&self) -> bool {
        matches!(self, Self::All(_))
    }
}

// ---------------------------------------------------------------------------
// HookDeclaration (FR-016) — moved from skill-factory (was SkillHookDeclaration)
// ---------------------------------------------------------------------------

/// Handler type for hook declarations inside frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookHandlerType {
    Bash,
    Agent,
    Prompt,
}

/// A single hook declaration inside an agent or skill's frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDeclaration {
    pub name: String,
    #[serde(rename = "type")]
    pub handler_type: HookHandlerType,
    /// Optional condition expression (e.g. `"tool == 'Bash' && ..."`).
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Command or template to execute.
    pub run: String,
}

// ---------------------------------------------------------------------------
// UnifiedFrontmatter (FR-001 through FR-016)
// ---------------------------------------------------------------------------

/// Unified YAML frontmatter schema for all agent and skill definition files.
///
/// Subsumes three formats:
/// - Claude Code agents (`.claude/agents/*.md`)
/// - Skills (`.claude/commands/*.md`)
/// - Factory agents (`factory/process/agents/*.md`, `factory/adapters/*/agents/*.md`)
///
/// Field aliases ensure backward compatibility (FR-012):
/// - `id` → `name`
/// - `role` → `display_name`
/// - `model_hint` → `model`
/// - `tools` → `allowed_tools`
/// - `tier` (u8) → `safety_tier`
///
/// Unknown fields are preserved via `serde(flatten)` for forward compatibility (FR-013).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFrontmatter {
    // -- Tier 1: Identity (always parsed) --
    /// Unique identifier (kebab-case enforced by linter). Alias: `id`.
    #[serde(alias = "id")]
    pub name: String,

    /// What the agent does. Minimum 50 characters recommended (linter check).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Execution type. Default: `prompt`.
    #[serde(rename = "type", default)]
    pub agent_type: AgentType,

    /// LLM model identifier. Alias: `model_hint`.
    #[serde(default, alias = "model_hint", skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Catalog tags (replaces `category`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Human-friendly display name. Alias: `role`.
    #[serde(default, alias = "role", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Trigger condition for automatic routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,

    // -- Tier 2: Capabilities (parsed on activation) --
    /// Tool allow-list. Alias: `tools`. Default: wildcard `"*"`.
    #[serde(default, alias = "tools")]
    pub allowed_tools: AllowedTools,

    /// Safety tier classification. Alias: `tier` (accepts u8: 1/2/3).
    #[serde(default, alias = "tier", skip_serializing_if = "Option::is_none")]
    pub safety_tier: Option<SafetyTier>,

    /// Mutation capability. Derived from `safety_tier` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mutation: Option<MutationCapability>,

    /// Hook declarations keyed by event name.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub hooks: HashMap<String, Vec<HookDeclaration>>,

    /// Governance requirement. Connects to spec 098.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance: Option<GovernanceRequirement>,

    /// Maximum spec risk level. Connects to spec 093.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_spec_risk: Option<String>,

    // -- Tier 3: Metadata (for tooling, never gates execution) --
    /// Semantic version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Attribution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Trigger ordering priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,

    /// Desktop display icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Factory pipeline stage number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<u8>,

    /// Factory token budget hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_budget: Option<String>,

    /// Standards category filter (spec 055).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standards_category: Option<String>,

    /// Standards tag filter (spec 055).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub standards_tags: Vec<String>,

    // -- Forward compatibility (FR-013) --
    /// Unknown fields preserved through parse-serialize round-trips.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl UnifiedFrontmatter {
    /// Apply derivation rules for absent fields (FR-015).
    ///
    /// - `type: process` implies `safety_tier: tier1`, `mutation: read-only`, `model: opus`
    /// - `type: scaffold` implies `safety_tier: tier2`, `mutation: read-write`, `model: sonnet`
    /// - `mutation` derived from `safety_tier` when absent
    pub fn apply_derivations(&mut self) {
        match self.agent_type {
            AgentType::Process => {
                if self.safety_tier.is_none() {
                    self.safety_tier = Some(SafetyTier::Tier1);
                }
                if self.mutation.is_none() {
                    self.mutation = Some(MutationCapability::ReadOnly);
                }
                if self.model.is_none() {
                    self.model = Some("opus".to_string());
                }
            }
            AgentType::Scaffold => {
                if self.safety_tier.is_none() {
                    self.safety_tier = Some(SafetyTier::Tier2);
                }
                if self.mutation.is_none() {
                    self.mutation = Some(MutationCapability::ReadWrite);
                }
                if self.model.is_none() {
                    self.model = Some("sonnet".to_string());
                }
            }
            _ => {}
        }

        // Derive mutation from safety_tier when absent.
        if self.mutation.is_none()
            && let Some(tier) = self.safety_tier
        {
            self.mutation = Some(MutationCapability::from(tier));
        }
    }
}
