//! FR-006 / FR-007: deterministic policy evaluation for a tool call against a compiled bundle.
//! FR-008 / SC-007 / SC-008: coherence scheduler (see [`coherence`]).
//! FR-009 / FR-010 / NF-004 / SC-009: proof-chain records (see [`proof_chain`]).
//!
//! ## Permission Runtime (spec 068)
//!
//! The permission runtime adds 5-tier settings layering, glob-based rule matching,
//! denial tracking with escalation, live settings reload, and audit logging.
//! See [`permission`], [`settings`], [`merge`], [`denial`], [`watcher`], [`audit`].

pub mod coherence;
pub mod proof_chain;

// --- spec 068: Permission Runtime and Settings Layering ---
pub mod audit;
pub mod denial;
pub mod merge;
pub mod permission;
pub mod settings;
#[cfg(feature = "native")]
pub mod watcher;

pub use coherence::{CoherenceScheduler, CoherenceSchedulerConfig, PrivilegeLevel};
pub use proof_chain::{
    NF004_MAX_BYTES_EXCLUDING_CONTEXT, ProofChainError, ProofChainWriter, ProofPrivilege,
    ProofRecord, ProofRecordDecision, compute_record_hash, nf004_payload_bytes, verify_proof_chain,
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// Policy rule as emitted by the policy compiler (`specs/047` fenced `policy` blocks).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyRule {
    pub id: String,
    pub description: String,
    pub mode: String,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<String>,
    #[serde(rename = "sourcePath")]
    pub source_path: String,
    /// When `true` on a `destructive_operation` constitution rule, permits destructive patterns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_destructive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_diff_lines: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_diff_bytes: Option<u64>,
}

/// Active constitution + shards (subset of `policy-bundle.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyBundle {
    pub constitution: Vec<PolicyRule>,
    pub shards: BTreeMap<String, Vec<PolicyRule>>,
}

/// Inputs for a single evaluation (NF-003: all data passed in — no host calls).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallContext {
    pub tool_name: String,
    /// Concatenated or JSON-serialized arguments for scanning.
    pub arguments_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposed_file_content: Option<String>,
    /// Set for write/edit operations when diff metrics are known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_lines: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_bytes: Option<u64>,
    /// Shard scope tags to merge (e.g. `domain:payments`). Constitution is always included.
    #[serde(default)]
    pub active_shard_scopes: Vec<String>,

    // --- spec 093: Spec-driven preflight fields ---
    /// Feature IDs affected by the tool call's target files (populated via featuregraph lookup).
    #[serde(default)]
    pub feature_ids: Vec<String>,
    /// Highest risk level among affected features: low / medium / high / critical.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_spec_risk: Option<String>,
    /// Deduplicated statuses of affected features: draft / active / deprecated.
    #[serde(default)]
    pub spec_statuses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicyOutcome {
    Allow,
    Deny,
    Degrade,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyDecision {
    pub outcome: PolicyOutcome,
    /// Stable, machine-readable reason code + detail.
    pub reason: String,
    /// Rule ids consulted for this outcome (may be empty for unconditional allow).
    #[serde(default)]
    pub rule_ids: Vec<String>,
}

/// Core entrypoint (FR-006).
pub fn evaluate(ctx: &ToolCallContext, bundle: &PolicyBundle) -> PolicyDecision {
    if let Some(d) = gate_secrets_scanner(ctx, bundle) {
        return d;
    }
    if let Some(d) = gate_destructive_operation(ctx, bundle) {
        return d;
    }
    if let Some(d) = gate_tool_allowlist(ctx, bundle) {
        return d;
    }
    // Spec 093: spec-derived gates (after security gates, before diff size)
    if let Some(d) = gate_spec_status(ctx, bundle) {
        return d;
    }
    if let Some(d) = gate_spec_risk(ctx, bundle) {
        return d;
    }
    if let Some(d) = gate_diff_size_limiter(ctx, bundle) {
        return d;
    }
    PolicyDecision {
        outcome: PolicyOutcome::Allow,
        reason: "policy:allow:no_gate_triggered".into(),
        rule_ids: vec![],
    }
}

fn gate_secrets_scanner(ctx: &ToolCallContext, bundle: &PolicyBundle) -> Option<PolicyDecision> {
    let haystack = format!(
        "{}\n{}",
        ctx.arguments_summary,
        ctx.proposed_file_content.as_deref().unwrap_or("")
    );
    if !secrets_match(&haystack) {
        return None;
    }
    let rule_id = bundle
        .constitution
        .iter()
        .find(|r| r.mode == "enforce" && r.gate.as_deref() == Some("secrets_scanner"))
        .map(|r| r.id.clone())
        .unwrap_or_else(|| "KERNEL:BUILTIN-SECRETS".into());
    Some(PolicyDecision {
        outcome: PolicyOutcome::Deny,
        reason: "policy:deny:secrets_scanner:pattern_match".into(),
        rule_ids: vec![rule_id],
    })
}

fn gate_destructive_operation(
    ctx: &ToolCallContext,
    bundle: &PolicyBundle,
) -> Option<PolicyDecision> {
    if !destructive_match(&ctx.tool_name, &ctx.arguments_summary) {
        return None;
    }
    let permitted = bundle.constitution.iter().any(|r| {
        r.gate.as_deref() == Some("destructive_operation") && r.allow_destructive == Some(true)
    });
    if permitted {
        return None;
    }
    let block_rule = bundle
        .constitution
        .iter()
        .find(|r| r.gate.as_deref() == Some("destructive_operation") && r.mode == "enforce");
    let rule_id = block_rule
        .map(|r| r.id.clone())
        .unwrap_or_else(|| "KERNEL:BUILTIN-DESTRUCTIVE".into());
    Some(PolicyDecision {
        outcome: PolicyOutcome::Deny,
        reason: "policy:deny:destructive_operation:matched_pattern".into(),
        rule_ids: vec![rule_id],
    })
}

fn gate_tool_allowlist(ctx: &ToolCallContext, bundle: &PolicyBundle) -> Option<PolicyDecision> {
    let mut allowed: Vec<String> = Vec::new();
    let mut originating_rule_ids: BTreeSet<String> = BTreeSet::new();
    for r in &bundle.constitution {
        if r.gate.as_deref() == Some("tool_allowlist") {
            originating_rule_ids.insert(r.id.clone());
            if let Some(ref tools) = r.allowed_tools {
                allowed.extend(tools.iter().cloned());
            }
        }
    }
    for scope in &ctx.active_shard_scopes {
        if let Some(rules) = bundle.shards.get(scope) {
            for r in rules {
                if r.gate.as_deref() == Some("tool_allowlist") {
                    originating_rule_ids.insert(r.id.clone());
                    if let Some(ref tools) = r.allowed_tools {
                        allowed.extend(tools.iter().cloned());
                    }
                }
            }
        }
    }
    if allowed.is_empty() {
        return None;
    }
    let set: std::collections::BTreeSet<_> = allowed.into_iter().collect();
    if set.contains(&ctx.tool_name) {
        None
    } else {
        Some(PolicyDecision {
            outcome: PolicyOutcome::Deny,
            reason: "policy:deny:tool_allowlist:not_listed".into(),
            rule_ids: originating_rule_ids.into_iter().collect(),
        })
    }
}

/// Spec 093, Slice 3: if affected features include `draft` specs, degrade to read-only;
/// if any are `deprecated`, deny outright. Skipped when `spec_statuses` is empty.
fn gate_spec_status(ctx: &ToolCallContext, _bundle: &PolicyBundle) -> Option<PolicyDecision> {
    if ctx.spec_statuses.is_empty() {
        return None;
    }
    if ctx.spec_statuses.iter().any(|s| s == "deprecated") {
        return Some(PolicyDecision {
            outcome: PolicyOutcome::Deny,
            reason: "policy:deny:spec_status:deprecated".into(),
            rule_ids: vec!["KERNEL:SPEC-STATUS".into()],
        });
    }
    if ctx.spec_statuses.iter().any(|s| s == "draft") {
        return Some(PolicyDecision {
            outcome: PolicyOutcome::Degrade,
            reason: "policy:degrade:spec_status:draft_read_only".into(),
            rule_ids: vec!["KERNEL:SPEC-STATUS".into()],
        });
    }
    None
}

/// Spec 093, Slice 4: gate based on spec risk level. `critical` → degrade (manual confirm),
/// `high` → degrade (gated). `medium`/`low`/absent → no restriction.
fn gate_spec_risk(ctx: &ToolCallContext, _bundle: &PolicyBundle) -> Option<PolicyDecision> {
    match ctx.max_spec_risk.as_deref() {
        Some("critical") => Some(PolicyDecision {
            outcome: PolicyOutcome::Degrade,
            reason: "policy:degrade:spec_risk:critical_requires_confirmation".into(),
            rule_ids: vec!["KERNEL:SPEC-RISK".into()],
        }),
        Some("high") => Some(PolicyDecision {
            outcome: PolicyOutcome::Degrade,
            reason: "policy:degrade:spec_risk:high_gated".into(),
            rule_ids: vec!["KERNEL:SPEC-RISK".into()],
        }),
        _ => None,
    }
}

fn gate_diff_size_limiter(ctx: &ToolCallContext, bundle: &PolicyBundle) -> Option<PolicyDecision> {
    let applicable = applicable_rules(ctx, bundle);
    let refs: Vec<&PolicyRule> = applicable.to_vec();
    let (max_lines, max_bytes) = effective_diff_limits(&refs);

    if let (Some(dl), Some(limit)) = (ctx.diff_lines, max_lines) {
        if dl > limit {
            let rule_id = refs
                .iter()
                .find(|r| {
                    r.gate.as_deref() == Some("diff_size_limiter")
                        && r.max_diff_lines == Some(limit)
                })
                .map(|r| r.id.clone())
                .unwrap_or_else(|| "KERNEL:BUILTIN-DIFF".into());
            return Some(PolicyDecision {
                outcome: PolicyOutcome::Deny,
                reason: "policy:deny:diff_size_limiter:threshold_exceeded".into(),
                rule_ids: vec![rule_id],
            });
        }
    }

    if let (Some(db), Some(limit)) = (ctx.diff_bytes, max_bytes) {
        if db > limit {
            let rule_id = refs
                .iter()
                .find(|r| {
                    r.gate.as_deref() == Some("diff_size_limiter")
                        && r.max_diff_bytes == Some(limit)
                })
                .map(|r| r.id.clone())
                .unwrap_or_else(|| "KERNEL:BUILTIN-DIFF".into());
            return Some(PolicyDecision {
                outcome: PolicyOutcome::Deny,
                reason: "policy:deny:diff_size_limiter:threshold_exceeded".into(),
                rule_ids: vec![rule_id],
            });
        }
    }

    None
}

fn applicable_rules<'a>(ctx: &'a ToolCallContext, bundle: &'a PolicyBundle) -> Vec<&'a PolicyRule> {
    let mut v: Vec<&PolicyRule> = bundle.constitution.iter().collect();
    for scope in &ctx.active_shard_scopes {
        if let Some(rules) = bundle.shards.get(scope) {
            v.extend(rules.iter());
        }
    }
    v
}

fn effective_diff_limits(rules: &[&PolicyRule]) -> (Option<u32>, Option<u64>) {
    let mut min_lines: Option<u32> = None;
    let mut min_bytes: Option<u64> = None;
    for r in rules {
        if r.gate.as_deref() != Some("diff_size_limiter") {
            continue;
        }
        if let Some(ml) = r.max_diff_lines {
            min_lines = Some(match min_lines {
                None => ml,
                Some(x) => x.min(ml),
            });
        }
        if let Some(mb) = r.max_diff_bytes {
            min_bytes = Some(match min_bytes {
                None => mb,
                Some(x) => x.min(mb),
            });
        }
    }
    (min_lines, min_bytes)
}

fn secrets_match(haystack: &str) -> bool {
    for re in secret_regexes().iter() {
        if re.is_match(haystack) {
            return true;
        }
    }
    false
}

fn secret_regexes() -> &'static [Regex] {
    static RE: OnceLock<Vec<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        vec![
            Regex::new(r"(?i)sk-[a-zA-Z0-9]{20,}").expect("regex"),
            Regex::new(r"(?i)-----BEGIN [A-Z ]*PRIVATE KEY-----").expect("regex"),
            Regex::new(r#"(?i)(api[_-]?key|secret|token)\s*[:=]\s*['\"]?[a-zA-Z0-9_\-]{24,}"#)
                .expect("regex"),
        ]
    })
}

fn destructive_match(tool_name: &str, args: &str) -> bool {
    let combined = format!("{tool_name}\n{args}");
    let lower = combined.to_lowercase();
    DESTRUCTIVE_SUBSTRINGS.iter().any(|s| lower.contains(s))
}

static DESTRUCTIVE_SUBSTRINGS: &[&str] = &[
    "rm -rf",
    "rm -fr",
    "git reset --hard",
    "delete_file",
    "shred ",
    "mkfs.",
];

/// Serialize decision to canonical JSON bytes for determinism checks (sorted keys).
pub fn decision_to_canonical_json(decision: &PolicyDecision) -> String {
    let v = serde_json::to_value(decision).expect("decision json");
    canonical_json_sorted(v)
}

pub(crate) fn canonical_json_sorted(v: serde_json::Value) -> String {
    sort_json_value(v)
}

fn sort_json_value(v: serde_json::Value) -> String {
    use serde_json::Value;
    match v {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            for k in keys {
                let inner = map.get(&k).expect("key from own iterator").clone();
                let s = sort_json_value(inner);
                out.insert(
                    k,
                    serde_json::from_str(&s).expect("re-parsing own JSON output"),
                );
            }
            serde_json::to_string(&Value::Object(out)).expect("stringify")
        }
        Value::Array(arr) => {
            let sorted: Vec<serde_json::Value> = arr
                .into_iter()
                .map(|x| {
                    serde_json::from_str(&sort_json_value(x)).expect("re-parsing own JSON output")
                })
                .collect();
            serde_json::to_string(&Value::Array(sorted)).expect("stringify")
        }
        other => serde_json::to_string(&other).expect("stringify"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle_from_constitution(rules: Vec<PolicyRule>) -> PolicyBundle {
        PolicyBundle {
            constitution: rules,
            shards: BTreeMap::new(),
        }
    }

    #[test]
    fn sc003_destructive_operation_denies_with_rule_id() {
        let bundle = bundle_from_constitution(vec![PolicyRule {
            id: "D-1".into(),
            description: "block destructive".into(),
            mode: "enforce".into(),
            scope: "global".into(),
            gate: Some("destructive_operation".into()),
            source_path: "CLAUDE.md".into(),
            allow_destructive: None,
            allowed_tools: None,
            max_diff_lines: None,
            max_diff_bytes: None,
        }]);
        let ctx = ToolCallContext {
            tool_name: "bash".into(),
            arguments_summary: "rm -rf /tmp/x".into(),
            proposed_file_content: None,
            diff_lines: None,
            diff_bytes: None,
            active_shard_scopes: vec![],
            feature_ids: vec![],
            max_spec_risk: None,
            spec_statuses: vec![],
        };
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Deny);
        assert_eq!(d.rule_ids, vec!["D-1"]);
        assert!(d.reason.contains("destructive"));
    }

    #[test]
    fn sc004_secrets_scanner_denies_even_with_other_rules() {
        let bundle = bundle_from_constitution(vec![
            PolicyRule {
                id: "S-1".into(),
                description: "secrets".into(),
                mode: "enforce".into(),
                scope: "global".into(),
                gate: Some("secrets_scanner".into()),
                source_path: "CLAUDE.md".into(),
                allow_destructive: None,
                allowed_tools: None,
                max_diff_lines: None,
                max_diff_bytes: None,
            },
            PolicyRule {
                id: "D-1".into(),
                description: "destructive".into(),
                mode: "enforce".into(),
                scope: "global".into(),
                gate: Some("destructive_operation".into()),
                source_path: "CLAUDE.md".into(),
                allow_destructive: None,
                allowed_tools: None,
                max_diff_lines: None,
                max_diff_bytes: None,
            },
        ]);
        let ctx = ToolCallContext {
            tool_name: "write".into(),
            arguments_summary: r#"sk-123456789012345678901234567890"#.into(),
            proposed_file_content: None,
            diff_lines: None,
            diff_bytes: None,
            active_shard_scopes: vec![],
            feature_ids: vec![],
            max_spec_risk: None,
            spec_statuses: vec![],
        };
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Deny);
        assert_eq!(d.rule_ids, vec!["S-1"]);
        assert!(d.reason.contains("secrets"));
    }

    #[test]
    fn sc005_tool_allowlist_denies_unknown_tool() {
        let bundle = bundle_from_constitution(vec![PolicyRule {
            id: "T-1".into(),
            description: "tools".into(),
            mode: "enforce".into(),
            scope: "global".into(),
            gate: Some("tool_allowlist".into()),
            source_path: "CLAUDE.md".into(),
            allow_destructive: None,
            allowed_tools: Some(vec!["read_file".into(), "grep".into()]),
            max_diff_lines: None,
            max_diff_bytes: None,
        }]);
        let ctx = ToolCallContext {
            tool_name: "shell".into(),
            arguments_summary: "".into(),
            proposed_file_content: None,
            diff_lines: None,
            diff_bytes: None,
            active_shard_scopes: vec![],
            feature_ids: vec![],
            max_spec_risk: None,
            spec_statuses: vec![],
        };
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Deny);
        assert_eq!(d.rule_ids, vec!["T-1"]);
        assert!(d.reason.contains("tool_allowlist"));
    }

    #[test]
    fn p3_003_allowlist_merges_originating_rule_ids_from_shards() {
        let mut shards = BTreeMap::new();
        shards.insert(
            "domain:a".into(),
            vec![PolicyRule {
                id: "T-SHARD".into(),
                description: "shard tools".into(),
                mode: "enforce".into(),
                scope: "domain:a".into(),
                gate: Some("tool_allowlist".into()),
                source_path: ".claude/policies/a.md".into(),
                allow_destructive: None,
                allowed_tools: Some(vec!["read_file".into()]),
                max_diff_lines: None,
                max_diff_bytes: None,
            }],
        );
        let bundle = PolicyBundle {
            constitution: vec![PolicyRule {
                id: "T-CON".into(),
                description: "const tools".into(),
                mode: "enforce".into(),
                scope: "global".into(),
                gate: Some("tool_allowlist".into()),
                source_path: "CLAUDE.md".into(),
                allow_destructive: None,
                allowed_tools: Some(vec!["grep".into()]),
                max_diff_lines: None,
                max_diff_bytes: None,
            }],
            shards,
        };
        let ctx = ToolCallContext {
            tool_name: "bash".into(),
            arguments_summary: "".into(),
            proposed_file_content: None,
            diff_lines: None,
            diff_bytes: None,
            active_shard_scopes: vec!["domain:a".into()],
            feature_ids: vec![],
            max_spec_risk: None,
            spec_statuses: vec![],
        };
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Deny);
        assert_eq!(d.rule_ids, vec!["T-CON", "T-SHARD"]);
    }

    #[test]
    fn sc006_diff_size_limiter_denies_large_diff() {
        let bundle = bundle_from_constitution(vec![PolicyRule {
            id: "L-1".into(),
            description: "diff cap".into(),
            mode: "enforce".into(),
            scope: "global".into(),
            gate: Some("diff_size_limiter".into()),
            source_path: "CLAUDE.md".into(),
            allow_destructive: None,
            allowed_tools: None,
            max_diff_lines: Some(10),
            max_diff_bytes: None,
        }]);
        let ctx = ToolCallContext {
            tool_name: "apply_patch".into(),
            arguments_summary: "".into(),
            proposed_file_content: None,
            diff_lines: Some(100),
            diff_bytes: None,
            active_shard_scopes: vec![],
            feature_ids: vec![],
            max_spec_risk: None,
            spec_statuses: vec![],
        };
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Deny);
        assert_eq!(d.rule_ids, vec!["L-1"]);
        assert!(d.reason.contains("diff_size"));
    }

    #[test]
    fn determinism_identical_inputs_identical_canonical_json() {
        let bundle = bundle_from_constitution(vec![PolicyRule {
            id: "T-1".into(),
            description: "tools".into(),
            mode: "enforce".into(),
            scope: "global".into(),
            gate: Some("tool_allowlist".into()),
            source_path: "CLAUDE.md".into(),
            allow_destructive: None,
            allowed_tools: Some(vec!["a".into()]),
            max_diff_lines: None,
            max_diff_bytes: None,
        }]);
        let ctx = ToolCallContext {
            tool_name: "b".into(),
            arguments_summary: "".into(),
            proposed_file_content: None,
            diff_lines: None,
            diff_bytes: None,
            active_shard_scopes: vec![],
            feature_ids: vec![],
            max_spec_risk: None,
            spec_statuses: vec![],
        };
        let a = decision_to_canonical_json(&evaluate(&ctx, &bundle));
        let b = decision_to_canonical_json(&evaluate(&ctx, &bundle));
        assert_eq!(a, b);
    }

    // --- Spec 093: spec-status gate tests ---

    fn empty_bundle() -> PolicyBundle {
        bundle_from_constitution(vec![])
    }

    fn base_ctx() -> ToolCallContext {
        ToolCallContext {
            tool_name: "write_file".into(),
            arguments_summary: "".into(),
            proposed_file_content: None,
            diff_lines: None,
            diff_bytes: None,
            active_shard_scopes: vec![],
            feature_ids: vec![],
            max_spec_risk: None,
            spec_statuses: vec![],
        }
    }

    #[test]
    fn sc093_2_draft_status_degrades() {
        let bundle = empty_bundle();
        let mut ctx = base_ctx();
        ctx.spec_statuses = vec!["draft".into()];
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Degrade);
        assert!(d.reason.contains("draft_read_only"));
        assert_eq!(d.rule_ids, vec!["KERNEL:SPEC-STATUS"]);
    }

    #[test]
    fn sc093_deprecated_status_denies() {
        let bundle = empty_bundle();
        let mut ctx = base_ctx();
        ctx.spec_statuses = vec!["deprecated".into()];
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Deny);
        assert!(d.reason.contains("deprecated"));
    }

    #[test]
    fn sc093_active_status_allows() {
        let bundle = empty_bundle();
        let mut ctx = base_ctx();
        ctx.spec_statuses = vec!["active".into()];
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Allow);
    }

    #[test]
    fn sc093_empty_statuses_allows() {
        let bundle = empty_bundle();
        let ctx = base_ctx();
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Allow);
    }

    // --- Spec 093: spec-risk gate tests ---

    #[test]
    fn sc093_3_critical_risk_degrades() {
        let bundle = empty_bundle();
        let mut ctx = base_ctx();
        ctx.max_spec_risk = Some("critical".into());
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Degrade);
        assert!(d.reason.contains("critical_requires_confirmation"));
        assert_eq!(d.rule_ids, vec!["KERNEL:SPEC-RISK"]);
    }

    #[test]
    fn sc093_high_risk_degrades() {
        let bundle = empty_bundle();
        let mut ctx = base_ctx();
        ctx.max_spec_risk = Some("high".into());
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Degrade);
        assert!(d.reason.contains("high_gated"));
    }

    #[test]
    fn sc093_medium_risk_allows() {
        let bundle = empty_bundle();
        let mut ctx = base_ctx();
        ctx.max_spec_risk = Some("medium".into());
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Allow);
    }

    #[test]
    fn sc093_low_risk_allows() {
        let bundle = empty_bundle();
        let mut ctx = base_ctx();
        ctx.max_spec_risk = Some("low".into());
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Allow);
    }

    #[test]
    fn sc093_no_risk_allows() {
        let bundle = empty_bundle();
        let ctx = base_ctx();
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Allow);
    }

    #[test]
    fn sc093_status_gate_runs_before_risk_gate() {
        let bundle = empty_bundle();
        let mut ctx = base_ctx();
        ctx.spec_statuses = vec!["draft".into()];
        ctx.max_spec_risk = Some("critical".into());
        // Status gate (Degrade:draft) fires before risk gate
        let d = evaluate(&ctx, &bundle);
        assert_eq!(d.outcome, PolicyOutcome::Degrade);
        assert!(d.reason.contains("draft_read_only"));
    }
}
