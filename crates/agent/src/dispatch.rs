// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: 043-agent-organizer — dispatch protocol (specs/043-agent-organizer/spec.md § Dispatch protocol)

use crate::complexity::score_complexity;
use crate::plan::{
    AgentRole, ComplexityBand, ComplexityBlock, ComplexityBreakdown, ExecutionPlan, ModelTier,
    PlanContext, PlanMode, TeamAgent, TeamBlock, WorkflowBlock, WorkflowPhase,
};
use regex::Regex;
use std::sync::LazyLock;

/// Outcome of mandatory trigger evaluation (before score-based branch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MandatoryOutcome {
    /// FR-006 / NEVER delegate list — direct handling regardless of score.
    Direct(&'static str),
    /// FR-005 / ALWAYS delegate list — delegation regardless of score.
    Delegated(&'static str),
    /// No mandatory rule; use complexity score (FR-003 / FR-004).
    None,
}

/// Spec § Dispatch protocol — ordered substring checks (case-insensitive).
/// More specific phrases (e.g. `build project`) appear before generic delegate `build`.
static DIRECT_SUBSTRINGS: &[(&str, &'static str)] = &[
    // NEVER — single-command execution (diagram "build" is delegate; narrow first)
    ("build project", "never_single_command"),
    ("run the tests", "never_single_command"),
    ("run tests", "never_single_command"),
    ("run test", "never_single_command"),
    ("cargo test", "never_single_command"),
    ("npm test", "never_single_command"),
    ("pnpm test", "never_single_command"),
    ("yarn test", "never_single_command"),
    ("pytest", "never_single_command"),
    ("mvn test", "never_single_command"),
    ("dotnet test", "never_single_command"),
    // NEVER — simple lookups
    ("show me", "never_simple_lookup"),
    ("show the", "never_simple_lookup"),
    ("what's the status", "never_simple_lookup"),
    ("status of", "never_simple_lookup"),
    ("where is", "never_simple_lookup"),
    ("where's", "never_simple_lookup"),
    ("find file", "never_simple_lookup"),
    ("open file", "never_simple_lookup"),
    ("locate ", "never_simple_lookup"),
    // NEVER — configuration tweaks
    ("enable feature", "never_config_tweak"),
    ("disable feature", "never_config_tweak"),
    ("toggle ", "never_config_tweak"),
    ("turn on ", "never_config_tweak"),
    ("turn off ", "never_config_tweak"),
    // NEVER — conversational + diagram direct
    ("what is", "diagram_what_is"),
    ("what's", "diagram_whats"),
    ("who is", "never_conversational"),
    ("how does", "never_conversational"),
    ("why is", "never_conversational"),
    ("why does", "never_conversational"),
    ("how do i", "diagram_how_do_i"),
    ("single file edit", "diagram_single_file_edit"),
    ("run command", "diagram_run_command"),
    ("config change", "diagram_config_change"),
];

static EXPLAIN_DIRECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bexplain\b").expect("explain word boundary"));

/// ALWAYS delegate — multi-domain / cross-cutting (prose list).
static DELEGATE_SUBSTRINGS: &[(&str, &'static str)] = &[
    ("multi-file", "always_multi_file"),
    ("multiple files", "always_multi_file"),
    ("across files", "always_multi_file"),
    ("cross-module", "always_cross_module"),
    ("cross module", "always_cross_module"),
    ("across modules", "always_cross_module"),
    ("architecture design", "always_architecture"),
    ("architecture review", "always_architecture"),
    ("full test suite", "always_full_test_suite"),
    ("multiple components", "always_multi_component"),
    ("multi-component", "always_multi_component"),
    ("security audit", "always_security_audit"),
    ("performance analysis", "always_performance_analysis"),
    // Diagram mandatory delegate
    ("implement feature", "diagram_implement_feature"),
    ("debug across", "diagram_debug_across"),
    ("create test suite", "diagram_create_test_suite"),
    ("generate docs", "diagram_generate_docs"),
    ("review pr", "diagram_review_pr"),
    ("analyze architecture", "diagram_analyze_architecture"),
];

/// Single-token or short delegate triggers (substring — catches "refactoring", etc.).
static DELEGATE_SUBSTRINGS_SHORT: &[(&str, &'static str)] = &[
    ("refactor", "diagram_refactor"),
    ("migrate", "diagram_migrate"),
];

/// `\bbuild\b` — avoids matching "building" as a delegate-only hit when inappropriate.
static DELEGATE_BUILD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bbuild\b").expect("delegate build"));

/// Evaluate mandatory triggers per spec: direct branch first, then delegate, then score (contract notes).
pub fn evaluate_mandatory_triggers(prompt: &str) -> MandatoryOutcome {
    let lower = prompt.to_lowercase();

    for (needle, label) in DIRECT_SUBSTRINGS {
        if lower.contains(needle) {
            return MandatoryOutcome::Direct(label);
        }
    }
    if EXPLAIN_DIRECT.is_match(prompt) {
        return MandatoryOutcome::Direct("diagram_explain");
    }

    if lower.contains("frontend") && lower.contains("backend") {
        return MandatoryOutcome::Delegated("always_frontend_backend");
    }

    for (needle, label) in DELEGATE_SUBSTRINGS {
        if lower.contains(needle) {
            return MandatoryOutcome::Delegated(label);
        }
    }
    for (needle, label) in DELEGATE_SUBSTRINGS_SHORT {
        if lower.contains(needle) {
            return MandatoryOutcome::Delegated(label);
        }
    }
    if DELEGATE_BUILD.is_match(prompt) {
        return MandatoryOutcome::Delegated("diagram_build");
    }

    MandatoryOutcome::None
}

fn stub_team_and_workflow(band: ComplexityBand, request: &str) -> (TeamBlock, WorkflowBlock) {
    let n = match band {
        ComplexityBand::Simple | ComplexityBand::Moderate => 2,
        ComplexityBand::Complex => 3,
        ComplexityBand::HighlyComplex => 4,
    };

    let roles = [
        (AgentRole::Lead, ModelTier::Sonnet),
        (AgentRole::Support, ModelTier::Sonnet),
        (AgentRole::Reviewer, ModelTier::Sonnet),
        (AgentRole::Support, ModelTier::Opus),
    ];

    let mut agents = Vec::with_capacity(n);
    for i in 0..n {
        let id = format!("stub-agent-{}", i + 1);
        let (role, model) = roles[i % roles.len()];
        agents.push(TeamAgent {
            agent_id: id,
            role,
            justification: format!(
                "Phase 2 deterministic placeholder (band {band:?}) — registry integration in Phase 3"
            ),
            model,
        });
    }

    let task_base = if request.len() > 120 {
        format!("{}…", &request[..120])
    } else {
        request.to_string()
    };

    let mut phases = Vec::new();
    if n >= 2 {
        phases.push(WorkflowPhase {
            id: "phase-1".to_string(),
            name: "Analysis".to_string(),
            agents: vec![agents[0].agent_id.clone()],
            task: format!("Analyze requirements: {task_base}"),
            depends_on: vec![],
            output: "analysis-notes.md".to_string(),
            success_gate: "Scope and risks documented".to_string(),
            model: ModelTier::Sonnet,
        });
        phases.push(WorkflowPhase {
            id: "phase-2".to_string(),
            name: "Implementation".to_string(),
            agents: agents.iter().take(2).map(|a| a.agent_id.clone()).collect(),
            task: "Implement per analysis".to_string(),
            depends_on: vec!["phase-1".to_string()],
            output: "implementation".to_string(),
            success_gate: "Build succeeds and tests pass".to_string(),
            model: ModelTier::Opus,
        });
    }
    if n >= 3 {
        phases.push(WorkflowPhase {
            id: "phase-3".to_string(),
            name: "Verification".to_string(),
            agents: vec![agents[n - 1].agent_id.clone()],
            task: "Review and verify".to_string(),
            depends_on: vec!["phase-2".to_string()],
            output: "review-report.md".to_string(),
            success_gate: "No critical findings".to_string(),
            model: ModelTier::Sonnet,
        });
    }

    (TeamBlock { agents }, WorkflowBlock { phases })
}

fn merge_breakdown(breakdown: ComplexityBreakdown, label: Option<&str>) -> ComplexityBlock {
    let mut block: ComplexityBlock = breakdown.into();
    block.mandatory_trigger = label.map(String::from);
    block
}

/// Build a full [`ExecutionPlan`] from a user prompt: mandatory triggers, then score (FR-003–FR-006).
pub fn build_execution_plan(prompt: &str, ctx: &PlanContext) -> ExecutionPlan {
    let request_id = ctx
        .request_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let breakdown = score_complexity(prompt);
    let mandatory = evaluate_mandatory_triggers(prompt);

    match mandatory {
        MandatoryOutcome::Direct(label) => ExecutionPlan {
            request_id,
            mode: PlanMode::Direct,
            complexity: merge_breakdown(breakdown, Some(label)),
            team: None,
            workflow: None,
            warnings: None,
        },
        MandatoryOutcome::Delegated(label) => {
            let (team, workflow) = stub_team_and_workflow(breakdown.band, prompt);
            ExecutionPlan {
                request_id,
                mode: PlanMode::Delegated,
                complexity: merge_breakdown(breakdown, Some(label)),
                team: Some(team),
                workflow: Some(workflow),
                warnings: None,
            }
        }
        MandatoryOutcome::None => {
            if breakdown.score <= 25 {
                ExecutionPlan {
                    request_id,
                    mode: PlanMode::Direct,
                    complexity: merge_breakdown(breakdown, None),
                    team: None,
                    workflow: None,
                    warnings: None,
                }
            } else {
                let (team, workflow) = stub_team_and_workflow(breakdown.band, prompt);
                ExecutionPlan {
                    request_id,
                    mode: PlanMode::Delegated,
                    complexity: merge_breakdown(breakdown, None),
                    team: Some(team),
                    workflow: Some(workflow),
                    warnings: Some(vec!["phase2_stub_team_workflow".to_string()]),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::PlanMode;

    #[test]
    fn sc001_simple_typo_request_is_direct_low_score() {
        let p = "fix the typo in README.md";
        let plan = build_execution_plan(p, &PlanContext::default());
        assert_eq!(plan.mode, PlanMode::Direct);
        assert!(plan.complexity.score <= 25);
        assert!(plan.team.is_none());
    }

    #[test]
    fn sc002_complex_oauth_request_is_delegated_with_team_and_workflow() {
        let p = "implement user authentication with OAuth2 across the API and frontend, add tests, and update the docs";
        let plan = build_execution_plan(p, &PlanContext::default());
        assert_eq!(plan.mode, PlanMode::Delegated);
        let team = plan.team.as_ref().expect("team");
        assert!((2..=4).contains(&team.agents.len()));
        let wf = plan.workflow.as_ref().expect("workflow");
        assert!(!wf.phases.is_empty());
    }

    #[test]
    fn sc003_mandatory_delegate_short_prompt_still_delegated() {
        let p = "implement feature";
        let plan = build_execution_plan(p, &PlanContext::default());
        assert_eq!(plan.mode, PlanMode::Delegated);
        assert!(
            plan.complexity.score <= 25,
            "score should be low but mode still delegated"
        );
        assert_eq!(
            plan.complexity.mandatory_trigger.as_deref(),
            Some("diagram_implement_feature")
        );
    }

    #[test]
    fn sc004_mandatory_direct_overrides_high_score() {
        // Inflate score beyond the simple band without matching a delegate trigger first.
        let p = format!(
            "what is {}",
            format!("{}{}", "x".repeat(1900), " create ".repeat(16))
        );
        let plan = build_execution_plan(&p, &PlanContext::default());
        assert_eq!(plan.mode, PlanMode::Direct);
        assert!(plan.complexity.score > 25, "expected inflated score");
        assert_eq!(
            plan.complexity.mandatory_trigger.as_deref(),
            Some("diagram_what_is")
        );
    }

    #[test]
    fn direct_wins_before_delegate_when_both_substrings_present() {
        let p = "what is implement feature";
        let plan = build_execution_plan(p, &PlanContext::default());
        assert_eq!(plan.mode, PlanMode::Direct);
    }

    #[test]
    fn build_project_is_direct_not_delegate_build() {
        let p = "please build project locally";
        let plan = build_execution_plan(p, &PlanContext::default());
        assert_eq!(plan.mode, PlanMode::Direct);
    }

    #[test]
    fn empty_string_is_score_only_direct() {
        let plan = build_execution_plan("", &PlanContext::default());
        assert_eq!(plan.mode, PlanMode::Direct);
        assert_eq!(plan.complexity.score, 0);
    }
}
