// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! ts-rs export coverage + JSONB round-trip checks for spec 111 Phase 2.
//!
//! ts-rs 12's `#[ts(export)]` attribute generates files as a side-effect of
//! `cargo test`. This file asserts the generated bindings land in the expected
//! location and that a canonical `UnifiedFrontmatter` serde-round-trips
//! through a JSON value (the JSONB wire form used by stagecraft's catalog).
//!
//! `make ci` runs these tests and then `git diff --exit-code` on the
//! generated directory — the drift gate for the "no schema drift" invariant
//! in spec 111 §2.1.

use std::path::{Path, PathBuf};

use agent_frontmatter::{
    AgentType, AllowedTools, GovernanceRequirement, HookDeclaration, HookHandlerType,
    MutationCapability, SafetyTier, UnifiedFrontmatter,
};

fn bindings_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("platform/services/stagecraft/api/agents/frontmatter")
}

/// The set of generated files that must be present after `cargo test`.
/// Keep this in sync with every `#[derive(TS)]` type in `src/types.rs`.
const EXPECTED_BINDINGS: &[&str] = &[
    "AgentType.ts",
    "GovernanceRequirement.ts",
    "HookDeclaration.ts",
    "HookHandlerType.ts",
    "MutationCapability.ts",
    "SafetyTier.ts",
    "UnifiedFrontmatter.ts",
];

/// ts-rs auto-exports every `#[ts(export)]`-marked type whenever `cargo test`
/// runs — see the docstring on `ts_rs::TS::export`. This test just asserts
/// the side-effect actually landed in the expected location. The `make ci`
/// drift gate (git diff --exit-code) does the subsequent freshness check.
#[test]
fn ts_rs_writes_every_expected_file() {
    let dir = bindings_dir();
    assert!(
        dir.is_dir(),
        "bindings directory missing: {}",
        dir.display()
    );

    for name in EXPECTED_BINDINGS {
        let path = dir.join(name);
        assert!(
            path.is_file(),
            "expected generated binding {} missing at {}",
            name,
            path.display()
        );
    }
}

#[test]
fn canonical_frontmatter_round_trips_through_jsonb() {
    let mut fm = UnifiedFrontmatter {
        name: "triage-agent".to_string(),
        description: Some("A triage agent that classifies incoming GitHub issues.".to_string()),
        agent_type: AgentType::Agent,
        model: Some("opus".to_string()),
        tags: vec!["triage".to_string(), "github".to_string()],
        display_name: Some("Triage".to_string()),
        trigger: Some("issue.opened".to_string()),
        allowed_tools: AllowedTools::list(vec!["read".to_string(), "edit".to_string()]),
        safety_tier: Some(SafetyTier::Tier2),
        mutation: Some(MutationCapability::ReadWrite),
        hooks: {
            let mut h = std::collections::HashMap::new();
            h.insert(
                "pre_tool".to_string(),
                vec![HookDeclaration {
                    name: "audit".to_string(),
                    handler_type: HookHandlerType::Bash,
                    condition: Some("tool == 'Bash'".to_string()),
                    run: "echo audited".to_string(),
                }],
            );
            h
        },
        governance: Some(GovernanceRequirement::Enforced),
        max_spec_risk: Some("medium".to_string()),
        version: Some("1.0.0".to_string()),
        author: Some("bart".to_string()),
        priority: Some(10),
        icon: Some("star".to_string()),
        stage: None,
        context_budget: None,
        standards_category: None,
        standards_tags: vec![],
        extra: std::collections::HashMap::new(),
    };
    fm.extra.insert(
        "x_future_key".to_string(),
        serde_json::json!({ "nested": [1, 2, 3] }),
    );

    let json: serde_json::Value = serde_json::to_value(&fm).expect("serialize");

    // Unknown fields travel at the top level via `#[serde(flatten)]`.
    assert_eq!(json["x_future_key"]["nested"][1], serde_json::json!(2));
    // The canonical tier form is the string; round-trip never emits `1`.
    assert_eq!(json["safety_tier"], serde_json::json!("tier2"));
    // Untagged AllowedTools::List → bare array on the wire.
    assert_eq!(
        json["allowed_tools"],
        serde_json::json!(["read", "edit"])
    );

    let back: UnifiedFrontmatter =
        serde_json::from_value(json.clone()).expect("deserialize");
    let re: serde_json::Value = serde_json::to_value(&back).expect("re-serialize");
    assert_eq!(json, re, "JSONB round-trip lost or rewrote a field");
}

#[test]
fn allowed_tools_wildcard_round_trips() {
    let fm = UnifiedFrontmatter {
        name: "wild".into(),
        description: None,
        agent_type: AgentType::Prompt,
        model: None,
        tags: vec![],
        display_name: None,
        trigger: None,
        allowed_tools: AllowedTools::all(),
        safety_tier: None,
        mutation: None,
        hooks: std::collections::HashMap::new(),
        governance: None,
        max_spec_risk: None,
        version: None,
        author: None,
        priority: None,
        icon: None,
        stage: None,
        context_budget: None,
        standards_category: None,
        standards_tags: vec![],
        extra: std::collections::HashMap::new(),
    };
    let json = serde_json::to_value(&fm).unwrap();
    // Wildcard emits the bare "*" string — matches the "*" | string[]
    // union in the TS mirror.
    assert_eq!(json["allowed_tools"], serde_json::json!("*"));
    let back: UnifiedFrontmatter = serde_json::from_value(json).unwrap();
    assert!(back.allowed_tools.is_all());
}
