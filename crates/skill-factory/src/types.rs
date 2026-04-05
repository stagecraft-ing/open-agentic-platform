// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Core types for the Skill and Command Factory (spec 071).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Skill execution type (FR-004/005/006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillType {
    /// Render body as system prompt for sub-agent (FR-004).
    Prompt,
    /// Spawn independent sub-agent via dispatch (FR-005).
    Agent,
    /// Background execution, returns task ID (FR-006).
    Headless,
}

impl Default for SkillType {
    fn default() -> Self {
        Self::Prompt
    }
}

/// Which tools a skill is allowed to use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AllowedTools {
    /// All tools permitted.
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

/// Handler type for hook declarations inside skill frontmatter (FR-008).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookHandlerType {
    Bash,
    Agent,
    Prompt,
}

/// A single hook declaration inside a skill's frontmatter (FR-008).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillHookDeclaration {
    pub name: String,
    #[serde(rename = "type")]
    pub handler_type: HookHandlerType,
    /// Optional condition expression (e.g. "tool == 'Bash' && ...").
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Command or template to execute.
    pub run: String,
}

/// Parsed YAML frontmatter from a skill file (FR-001).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type", default)]
    pub skill_type: SkillType,
    #[serde(default)]
    pub allowed_tools: AllowedTools,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub hooks: HashMap<String, Vec<SkillHookDeclaration>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
}

/// A fully parsed skill — frontmatter plus the markdown body (FR-001).
#[derive(Debug, Clone)]
pub struct ParsedSkill {
    pub frontmatter: SkillFrontmatter,
    /// The markdown body (prompt template). Contains `$ARGS` placeholder.
    pub body: String,
    /// Path to the source `.md` file.
    pub source_path: PathBuf,
}

impl ParsedSkill {
    /// Render the prompt body with `$ARGS` replaced.
    pub fn render_prompt(&self, args: &str) -> String {
        self.body.replace("$ARGS", args)
    }
}

/// Outcome of loading a single skill file (FR-009).
#[derive(Debug)]
pub enum SkillLoadResult {
    /// Parsed and ready to register.
    Ok(ParsedSkill),
    /// Loaded with warnings (e.g. deprecated field).
    Warning {
        skill: ParsedSkill,
        message: String,
    },
    /// Failed to parse — non-fatal, other skills still load.
    Error {
        path: PathBuf,
        message: String,
    },
}

impl SkillLoadResult {
    pub fn skill(&self) -> Option<&ParsedSkill> {
        match self {
            Self::Ok(s) | Self::Warning { skill: s, .. } => Some(s),
            Self::Error { .. } => None,
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }
}

/// Aggregated result from `SkillFactory::load_from_dir` (FR-002).
#[derive(Debug)]
pub struct SkillFactoryLoadResult {
    /// Successfully loaded skills.
    pub skills: Vec<ParsedSkill>,
    /// Hook declarations collected from all loaded skills.
    pub hooks: Vec<CollectedHook>,
    /// Per-file load results (for diagnostics).
    pub results: Vec<SkillLoadResult>,
}

/// A hook declaration extracted from a skill, tagged with event type and skill name.
#[derive(Debug, Clone)]
pub struct CollectedHook {
    /// Event type key (e.g. "PreToolUse", "PostToolUse").
    pub event: String,
    /// The hook declaration from frontmatter.
    pub declaration: SkillHookDeclaration,
    /// Name of the skill that declared this hook.
    pub skill_name: String,
}
