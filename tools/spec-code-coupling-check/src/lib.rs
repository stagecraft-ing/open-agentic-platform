//! Spec/code coupling gate (spec 127).
//!
//! Reads `build/codebase-index/index.json` via typed deserialization
//! (the consumer-binary exception in spec 103) and checks that every
//! diff path claimed by some spec's `implements:` list is accompanied
//! by an edit to that spec's `spec.md`. Bypass list, waivers, and the
//! self-test are documented in spec 127 §4.

use open_agentic_codebase_indexer::types::{CodebaseIndex, SCHEMA_VERSION};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Path prefixes that never owe a spec edit. Limited to genuinely
/// orthogonal scaffolding: process docs, GitHub Actions metadata
/// (workflows are governed by spec 118's `# Spec:` header convention,
/// orthogonal to this diff-based check), and repo-metadata files.
///
/// Notably **NOT** bypassed:
/// - `Makefile` — claimed by specs 104, 105, 116, 127; changes route
///   through whichever owners are affected.
/// - `AGENTS.md` — claimed by spec 103; changes must amend that spec.
/// - `crates/`, `tools/`, `apps/`, `packages/`, `platform/services/`.
pub const BYPASS_PREFIXES: &[&str] = &[
    ".github/",
    "docs/",
    "README.md",
    "CLAUDE.md",
    "DEVELOPERS.md",
    "LICENSE",
    "CHANGELOG.md",
    "CODEOWNERS",
    ".gitignore",
    ".gitattributes",
];

/// Case-sensitive PR-body waiver keyword. Spec 127 FR-005.
pub const WAIVER_KEYWORD: &str = "Spec-Drift-Waiver:";

/// Per-spec violation: a spec owns at least one offending path that the
/// diff changed without also changing `specs/<id>/spec.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub spec_id: String,
    pub paths: Vec<String>,
}

#[derive(Debug)]
pub struct Outcome {
    pub violations: Vec<Violation>,
    pub waiver_reason: Option<String>,
}

impl Outcome {
    /// Exit code semantics: 0 if no violations OR if any waiver is present.
    pub fn exit_code(&self) -> i32 {
        if self.violations.is_empty() || self.waiver_reason.is_some() {
            0
        } else {
            1
        }
    }
}

/// True if `path` is exempt from coupling enforcement.
pub fn is_bypass(path: &str) -> bool {
    BYPASS_PREFIXES.iter().any(|prefix| {
        if prefix.ends_with('/') {
            path.starts_with(prefix)
        } else {
            // Exact match on root files; defensively allow trailing-slash variants.
            path == *prefix || path == format!("{prefix}/")
        }
    })
}

/// Slash-anchored prefix match: `claim` is a path declared in `implements:`,
/// `path` is a path from the diff. Treats a directory claim as owning every
/// file under it; treats a file claim as exact match.
pub fn claim_matches(claim: &str, path: &str) -> bool {
    if claim == path {
        return true;
    }
    let claim_dir = if claim.ends_with('/') {
        claim.to_string()
    } else {
        format!("{claim}/")
    };
    path.starts_with(&claim_dir)
}

/// Extract the first `Spec-Drift-Waiver:` line's reason text from `body`.
/// Multi-waiver bodies use the first occurrence; whitespace is trimmed.
pub fn parse_waiver(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(WAIVER_KEYWORD) {
            let reason = rest.trim();
            if !reason.is_empty() {
                return Some(reason.to_string());
            }
        }
    }
    None
}

/// Load and version-check the codebase index. Returns the same typed
/// `CodebaseIndex` the indexer produces — no ad-hoc parsing.
pub fn load_index(path: &Path) -> Result<CodebaseIndex, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    let index: CodebaseIndex = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parse {}: {e}", path.display()))?;

    // Major-version compat: index `1.x` is consumed by the gate; minor bumps
    // are additive (per spec 118). Hard error on a major-version mismatch so
    // schema drift surfaces at run time as well as build time.
    let actual_major = index
        .schema_version
        .split('.')
        .next()
        .unwrap_or("");
    let expected_major = SCHEMA_VERSION
        .split('.')
        .next()
        .unwrap_or("");
    if actual_major != expected_major {
        return Err(format!(
            "schema version mismatch: index reports {} but gate built against {}; \
             run `cargo build --release --manifest-path tools/codebase-indexer/Cargo.toml` \
             then `./tools/codebase-indexer/target/release/codebase-indexer compile`",
            index.schema_version, SCHEMA_VERSION
        ));
    }
    Ok(index)
}

/// Build the lookup table: claimed path → set of spec IDs that claim it.
/// Multiple specs can claim the same path; all owners must observe the rule.
pub fn build_claim_index(
    index: &CodebaseIndex,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut by_claim: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for mapping in &index.traceability.mappings {
        for ip in &mapping.implementing_paths {
            by_claim
                .entry(ip.path.clone())
                .or_default()
                .insert(mapping.spec_id.clone());
        }
    }
    by_claim
}

/// Compute coupling violations.
///
/// `diff_paths` is the set of files changed in the PR (any side, normalized
/// to forward slashes). `pr_body` may carry a `Spec-Drift-Waiver:` line.
pub fn check_coupling(
    index: &CodebaseIndex,
    diff_paths: &BTreeSet<String>,
    pr_body: &str,
) -> Outcome {
    let waiver_reason = parse_waiver(pr_body);
    let claim_index = build_claim_index(index);

    // For each diff path that is not bypass-listed, find the set of specs
    // that claim it. Aggregate into spec_id -> set of offending paths.
    let mut owed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in diff_paths {
        if is_bypass(path) {
            continue;
        }
        for (claim, owners) in &claim_index {
            if claim_matches(claim, path) {
                for spec_id in owners {
                    owed.entry(spec_id.clone())
                        .or_default()
                        .insert(path.clone());
                }
            }
        }
    }

    // A spec edit in the diff clears that spec's owed paths.
    let mut violations: Vec<Violation> = Vec::new();
    for (spec_id, paths) in owed {
        let spec_md = format!("specs/{spec_id}/spec.md");
        if diff_paths.contains(&spec_md) {
            continue;
        }
        let mut paths_sorted: Vec<String> = paths.into_iter().collect();
        paths_sorted.sort();
        violations.push(Violation {
            spec_id,
            paths: paths_sorted,
        });
    }
    violations.sort_by(|a, b| a.spec_id.cmp(&b.spec_id));

    Outcome {
        violations,
        waiver_reason,
    }
}

/// Render an outcome for human-readable CI logs. Empty for clean runs;
/// otherwise a multi-line block per the spec 127 §8 worked example.
pub fn render(outcome: &Outcome) -> String {
    if let Some(reason) = &outcome.waiver_reason {
        let count = outcome.violations.len();
        if count == 0 {
            return String::new();
        }
        let mut s = format!(
            "::warning::spec-code-coupling-check: {count} violation(s) waived by PR body — reason: {reason}\n",
        );
        for v in &outcome.violations {
            s.push_str(&format!("  {} (waived)\n", v.spec_id));
            for p in &v.paths {
                s.push_str(&format!("    {p}\n"));
            }
        }
        return s;
    }
    if outcome.violations.is_empty() {
        return String::new();
    }
    let mut s = format!(
        "spec-code-coupling-check: {} spec(s) owe a spec.md edit.\n\n",
        outcome.violations.len(),
    );
    for v in &outcome.violations {
        s.push_str(&format!("  {}\n", v.spec_id));
        for p in &v.paths {
            s.push_str(&format!("    {p}\n"));
        }
    }
    s.push('\n');
    s.push_str("To resolve, either:\n");
    s.push_str("  - amend the named spec.md in this PR, or\n");
    s.push_str("  - add a 'Spec-Drift-Waiver: <reason>' line to the PR body.\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use open_agentic_codebase_indexer::types::{
        BuildInfo, Diagnostics, ImplementingPath, Infrastructure, TraceMapping, TraceSource,
        Traceability,
    };

    fn empty_index() -> CodebaseIndex {
        CodebaseIndex {
            schema_version: SCHEMA_VERSION.to_string(),
            build: BuildInfo {
                indexer_id: "codebase-indexer".to_string(),
                indexer_version: "test".to_string(),
                repo_root: ".".to_string(),
                content_hash: "test".to_string(),
            },
            inventory: Vec::new(),
            traceability: Traceability {
                mappings: Vec::new(),
                orphaned_specs: Vec::new(),
                untraced_code: Vec::new(),
            },
            factory: Vec::new(),
            infrastructure: Infrastructure {
                tools: Vec::new(),
                agents: Vec::new(),
                commands: Vec::new(),
                rules: Vec::new(),
                schemas: Vec::new(),
            },
            workflow_traceability: Vec::new(),
            diagnostics: Diagnostics {
                warnings: Vec::new(),
                errors: Vec::new(),
            },
        }
    }

    fn index_claiming(spec_id: &str, paths: &[&str]) -> CodebaseIndex {
        let mut idx = empty_index();
        idx.traceability.mappings.push(TraceMapping {
            spec_id: spec_id.to_string(),
            spec_status: Some("approved".to_string()),
            depends_on: Vec::new(),
            implementing_paths: paths
                .iter()
                .map(|p| ImplementingPath {
                    path: (*p).to_string(),
                    name: None,
                    source: Some(TraceSource::SpecImplements),
                })
                .collect(),
        });
        idx
    }

    fn diffset(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn bypass_recognises_workflow_and_docs_and_root_md() {
        assert!(is_bypass(".github/workflows/foo.yml"));
        assert!(is_bypass("docs/ARCHITECTURE.md"));
        assert!(is_bypass("README.md"));
        assert!(is_bypass("CLAUDE.md"));
        assert!(is_bypass("LICENSE"));
        assert!(!is_bypass("crates/orchestrator/src/lib.rs"));
        assert!(!is_bypass("specs/044-multi-agent-orchestration/spec.md"));
        // Critical: Makefile and AGENTS.md are claimed by specs and MUST
        // route through the gate.
        assert!(!is_bypass("Makefile"));
        assert!(!is_bypass("AGENTS.md"));
    }

    #[test]
    fn claim_matches_exact_and_prefix() {
        assert!(claim_matches("crates/orchestrator", "crates/orchestrator/src/lib.rs"));
        assert!(claim_matches("crates/orchestrator/", "crates/orchestrator/src/lib.rs"));
        assert!(claim_matches("Makefile", "Makefile"));
        // Exact-string equality matches even when the path has no extension.
        assert!(claim_matches("crates/orchestrator/src", "crates/orchestrator/src"));
        // Must not match a sibling with a shared prefix string.
        assert!(!claim_matches("crates/orches", "crates/orchestrator/src/lib.rs"));
        // A claim that is a non-prefix substring must not match.
        assert!(!claim_matches("orchestrator", "crates/orchestrator/src/lib.rs"));
    }

    #[test]
    fn waiver_extracts_reason_and_ignores_blank() {
        assert_eq!(
            parse_waiver("hello\nSpec-Drift-Waiver: hotfix OPS-123\nworld"),
            Some("hotfix OPS-123".to_string())
        );
        // Leading whitespace allowed, but the keyword must be intact.
        assert_eq!(
            parse_waiver("    Spec-Drift-Waiver:    rebuild after incident   "),
            Some("rebuild after incident".to_string())
        );
        // Blank reason: not a waiver.
        assert_eq!(parse_waiver("Spec-Drift-Waiver:   "), None);
        assert_eq!(parse_waiver(""), None);
        assert_eq!(parse_waiver("no waiver here"), None);
    }

    /// AC-1: a coupling violation produces a Violation block.
    #[test]
    fn ac1_violation_when_path_changed_without_spec_edit() {
        let idx = index_claiming("044-multi-agent-orchestration", &["crates/orchestrator"]);
        let diff = diffset(&["crates/orchestrator/src/lib.rs"]);
        let outcome = check_coupling(&idx, &diff, "");
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].spec_id, "044-multi-agent-orchestration");
        assert_eq!(outcome.violations[0].paths, vec!["crates/orchestrator/src/lib.rs"]);
        assert_eq!(outcome.exit_code(), 1);
    }

    /// AC-2: same diff with the spec.md added clears the violation.
    #[test]
    fn ac2_no_violation_when_spec_edited() {
        let idx = index_claiming("044-multi-agent-orchestration", &["crates/orchestrator"]);
        let diff = diffset(&[
            "crates/orchestrator/src/lib.rs",
            "specs/044-multi-agent-orchestration/spec.md",
        ]);
        let outcome = check_coupling(&idx, &diff, "");
        assert!(outcome.violations.is_empty());
        assert_eq!(outcome.exit_code(), 0);
    }

    /// AC-3: a docs-only diff is silent.
    #[test]
    fn ac3_bypass_paths_never_violate() {
        let idx = index_claiming("044-multi-agent-orchestration", &["crates/orchestrator"]);
        let diff = diffset(&["docs/ARCHITECTURE.md", ".github/workflows/foo.yml"]);
        let outcome = check_coupling(&idx, &diff, "");
        assert!(outcome.violations.is_empty());
        assert_eq!(outcome.exit_code(), 0);
    }

    /// AC-4: waiver suppresses the failure but the violation is preserved
    /// in the rendered output for review-time visibility.
    #[test]
    fn ac4_waiver_suppresses_exit_but_keeps_violations() {
        let idx = index_claiming("044-multi-agent-orchestration", &["crates/orchestrator"]);
        let diff = diffset(&["crates/orchestrator/src/lib.rs"]);
        let body = "rolling forward an incident\n\nSpec-Drift-Waiver: hotfix for OPS-123\n";
        let outcome = check_coupling(&idx, &diff, body);
        assert_eq!(outcome.exit_code(), 0);
        assert_eq!(outcome.violations.len(), 1);
        assert!(outcome.waiver_reason.as_deref() == Some("hotfix for OPS-123"));
        let rendered = render(&outcome);
        assert!(rendered.contains("::warning::"));
        assert!(rendered.contains("hotfix for OPS-123"));
    }

    #[test]
    fn multi_spec_diff_aggregates_per_owner() {
        let mut idx = index_claiming("044-multi-agent-orchestration", &["crates/orchestrator"]);
        idx.traceability.mappings.push(TraceMapping {
            spec_id: "067-tool-definition-registry".to_string(),
            spec_status: Some("approved".to_string()),
            depends_on: Vec::new(),
            implementing_paths: vec![ImplementingPath {
                path: "crates/tool-registry".to_string(),
                name: None,
                source: Some(TraceSource::SpecImplements),
            }],
        });
        let diff = diffset(&[
            "crates/orchestrator/src/lib.rs",
            "crates/tool-registry/src/lib.rs",
        ]);
        let outcome = check_coupling(&idx, &diff, "");
        assert_eq!(outcome.violations.len(), 2);
        // Sorted by spec id.
        assert_eq!(outcome.violations[0].spec_id, "044-multi-agent-orchestration");
        assert_eq!(outcome.violations[1].spec_id, "067-tool-definition-registry");
    }

    #[test]
    fn render_clean_run_is_empty() {
        let outcome = Outcome {
            violations: Vec::new(),
            waiver_reason: None,
        };
        assert!(render(&outcome).is_empty());
    }
}
