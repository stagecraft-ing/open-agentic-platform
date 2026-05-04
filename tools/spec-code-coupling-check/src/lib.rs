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

/// One uncovered path in the diff with the full sorted list of legitimate
/// owners, partitioned by source class.
///
/// Spec 130 introduced the primary-owner heuristic (any one owner's edit
/// clears the path). Spec 133 broadens the set of legitimate owners to
/// include amenders (`amends:`) and amendment-record targets
/// (`amendmentRecord`) when the path is itself a `specs/X/spec.md` path.
/// All three classes compose: any one owner from any class clears the
/// path, and the renderer labels each owner by source so reviewers can
/// audit which class of coupling applies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnerSet {
    /// Specs that claim this path via their `implements:` list. The
    /// historical spec 127 / spec 130 source class.
    pub implements: BTreeSet<String>,
    /// Specs that amend the spec whose `spec.md` is this path
    /// (`amends:` forward link, spec 119 protocol). Empty when the
    /// path is not a `specs/X/spec.md` path.
    pub amends: BTreeSet<String>,
    /// The amendment-record target named on the amended spec's
    /// frontmatter (`amendment_record:` reverse link). Empty when the
    /// path is not a `specs/X/spec.md` path or the amended spec carries
    /// no record.
    pub amendment_record: BTreeSet<String>,
}

impl OwnerSet {
    /// True when no owner of any class has been recorded.
    pub fn is_empty(&self) -> bool {
        self.implements.is_empty()
            && self.amends.is_empty()
            && self.amendment_record.is_empty()
    }

    /// Sorted union of every owner across all source classes. Useful for
    /// the "any one in diff clears the path" check and for tests/clients
    /// that don't care about source provenance.
    pub fn all_unique_sorted(&self) -> Vec<String> {
        let mut out: BTreeSet<String> = BTreeSet::new();
        out.extend(self.implements.iter().cloned());
        out.extend(self.amends.iter().cloned());
        out.extend(self.amendment_record.iter().cloned());
        out.into_iter().collect()
    }

    /// True when at least one owner from any class is named by a
    /// `specs/<id>/spec.md` entry in the diff.
    pub fn any_owner_in_diff(&self, diff_paths: &BTreeSet<String>) -> bool {
        let in_diff = |id: &String| diff_paths.contains(&format!("specs/{id}/spec.md"));
        self.implements.iter().any(in_diff)
            || self.amends.iter().any(in_diff)
            || self.amendment_record.iter().any(in_diff)
    }
}

/// One uncovered path in the diff with the full sorted set of legitimate
/// owners, source-tagged so the renderer can surface each class
/// separately (spec 133 FR-004) while still composing under spec 130's
/// primary-owner heuristic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub path: String,
    pub owners: OwnerSet,
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

/// Compute coupling violations under spec 130's primary-owner heuristic
/// extended by spec 133 to recognise amend-protocol couplings.
///
/// `diff_paths` is the set of files changed in the PR (any side, normalised
/// to forward slashes). `pr_body` may carry a `Spec-Drift-Waiver:` line.
///
/// **Resolver** (spec 130 + spec 133): a path's *legitimate owners* is the
/// union of three source classes:
/// - `implements:` — every spec whose `implements:` list claims the path
///   (exact or prefix match). Spec 127 / 130 source.
/// - `amends:` — when the path is `specs/X/spec.md`, every spec whose
///   `amends:` list contains `X`. Spec 133 FR-001.
/// - `amendmentRecord` — when the path is `specs/X/spec.md` and the
///   amended spec X's mapping carries an `amendmentRecord` target,
///   that target. Spec 133 FR-002.
///
/// **Heuristic** (spec 130, unchanged): the path is cleared if **any one**
/// owner across any class has its `spec.md` in the diff. The rendered
/// output groups owners by source class so reviewers can audit which
/// coupling applies.
pub fn check_coupling(
    index: &CodebaseIndex,
    diff_paths: &BTreeSet<String>,
    pr_body: &str,
) -> Outcome {
    let waiver_reason = parse_waiver(pr_body);
    let claim_index = build_claim_index(index);

    // Path-centric aggregation: for each non-bypass diff path, collect
    // every legitimate owner across the three source classes.
    let mut path_owners: BTreeMap<String, OwnerSet> = BTreeMap::new();
    for path in diff_paths {
        if is_bypass(path) {
            continue;
        }
        let owners = legitimate_owners(path, &claim_index, index);
        if owners.is_empty() {
            continue;
        }
        path_owners.insert(path.clone(), owners);
    }

    // Primary-owner heuristic (spec 130): a path is covered when ANY one
    // of its legitimate owners has its spec.md in the diff. Spec 133
    // expands the set of owners; the heuristic itself is unchanged.
    let mut violations: Vec<Violation> = Vec::new();
    for (path, owners) in path_owners {
        if owners.any_owner_in_diff(diff_paths) {
            continue;
        }
        violations.push(Violation { path, owners });
    }
    violations.sort_by(|a, b| a.path.cmp(&b.path));

    Outcome {
        violations,
        waiver_reason,
    }
}

/// Spec 133: parse `specs/<id>/spec.md` into `<id>`. Returns `None` for
/// paths that do not match the canonical spec.md location (e.g. crate
/// paths, sub-files like `specs/<id>/plan.md`, or doc paths).
pub fn spec_id_for_spec_md_path(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("specs/")?;
    let (id, tail) = rest.split_once('/')?;
    if tail == "spec.md" { Some(id) } else { None }
}

/// Spec 133: compute the full set of legitimate owners for `path` across
/// `implements:` (spec 127), `amends:` (FR-001), and `amendmentRecord:`
/// (FR-002). The two latter sources only fire when `path` itself names a
/// spec's `spec.md`. Each source class is collected separately so the
/// renderer (FR-004) can label owners by provenance.
pub fn legitimate_owners(
    path: &str,
    claim_index: &BTreeMap<String, BTreeSet<String>>,
    index: &CodebaseIndex,
) -> OwnerSet {
    let mut owners = OwnerSet::default();

    // Path 1 — implements: claimants (spec 127, spec 130).
    for (claim, claimants) in claim_index {
        if claim_matches(claim, path) {
            owners.implements.extend(claimants.iter().cloned());
        }
    }

    // Paths 2 & 3 fire only for `specs/<id>/spec.md`.
    let Some(amended_id) = spec_id_for_spec_md_path(path) else {
        return owners;
    };

    // Spec 133 strict-expansion (FR-005): the new amend pathways MUST NOT
    // newly enrol a path that today has no `implements:` claimant — that
    // would convert a silent-today path into a firing path whenever some
    // amender is not in the diff (e.g. editing your own spec.md when an
    // unrelated amender exists). The contract is "strictly expands the
    // set of accepted couplings; it never removes existing ones." Adding
    // owners only matters when the path is already firing today; for
    // paths today silent, amend resolution is suppressed.
    if owners.implements.is_empty() {
        return owners;
    }

    // Path 2 — amenders (FR-001). The indexer resolves short-form ids to
    // full ids at compile time, so a direct equality check suffices.
    for mapping in &index.traceability.mappings {
        if mapping.amends.iter().any(|id| id == amended_id) {
            owners.amends.insert(mapping.spec_id.clone());
        }
    }

    // Path 3 — amendment record on the amended spec (FR-002).
    if let Some(amended_mapping) = index
        .traceability
        .mappings
        .iter()
        .find(|m| m.spec_id == amended_id)
    {
        if let Some(record) = &amended_mapping.amendment_record {
            owners.amendment_record.insert(record.clone());
        }
    }

    owners
}

/// Render an outcome for human-readable CI logs. Empty for clean runs;
/// otherwise a path-centric block listing every legitimate owner per
/// uncovered path, partitioned by source class (spec 130 + spec 133).
///
/// Format (spec 133 FR-004): when an owner is named via `amends:` or
/// `amendmentRecord`, the renderer prints aligned `implements:`,
/// `amends:`, `amendment_record:` rows beneath the path so reviewers can
/// audit which coupling class applies. For implements-only violations
/// the renderer keeps the spec 130 compact form ("claimed by N specs").
pub fn render(outcome: &Outcome) -> String {
    if let Some(reason) = &outcome.waiver_reason {
        let count = outcome.violations.len();
        if count == 0 {
            return String::new();
        }
        let mut s = format!(
            "::warning::spec-code-coupling-check: {count} path(s) waived by PR body — reason: {reason}\n",
        );
        for v in &outcome.violations {
            s.push_str(&format!("  {} (waived)\n", v.path));
            for c in v.owners.all_unique_sorted() {
                s.push_str(&format!("    {c}\n"));
            }
        }
        return s;
    }
    if outcome.violations.is_empty() {
        return String::new();
    }
    let mut s = format!(
        "spec-code-coupling-check: {} path(s) lack a claimant edit.\n\n",
        outcome.violations.len(),
    );
    for v in &outcome.violations {
        s.push_str(&render_violation_block(v));
    }
    s.push('\n');
    s.push_str("To resolve, amend ANY ONE claimant's spec.md (per spec 130\n");
    s.push_str("primary-owner heuristic; spec 133 also accepts amender\n");
    s.push_str("or amendment-record edits), or add 'Spec-Drift-Waiver: <reason>'\n");
    s.push_str("to the PR body.\n");
    s
}

/// Render a single violation block. Spec 133 FR-004 mandates per-class
/// labels when the amend or amendment_record source classes carry
/// owners. For implements-only violations the renderer falls back to
/// spec 130's compact form so existing CI output stays stable.
fn render_violation_block(v: &Violation) -> String {
    let has_amend_link = !v.owners.amends.is_empty()
        || !v.owners.amendment_record.is_empty();
    if !has_amend_link {
        return render_implements_only_block(&v.path, &v.owners.implements);
    }

    let mut s = format!("  {}\n", v.path);
    push_owner_class(&mut s, "implements", &v.owners.implements);
    push_owner_class(&mut s, "amends", &v.owners.amends);
    push_owner_class(&mut s, "amendment_record", &v.owners.amendment_record);
    s
}

fn render_implements_only_block(path: &str, claimants: &BTreeSet<String>) -> String {
    if claimants.len() == 1 {
        let only = claimants.iter().next().expect("len==1");
        return format!("  {path} (claimed by {only})\n");
    }
    let mut s = format!("  {path} (claimed by {} specs)\n", claimants.len());
    for c in claimants {
        s.push_str(&format!("    {c}\n"));
    }
    s
}

/// Spec 133 FR-004: per-class owner list, aligned for readability.
/// The class label width matches the longest label so adjacent rows
/// line up (`implements:        `, `amends:            `,
/// `amendment_record:  `). An empty class is omitted entirely so the
/// reviewer's eye lands on the present sources.
fn push_owner_class(buf: &mut String, label: &str, members: &BTreeSet<String>) {
    if members.is_empty() {
        return;
    }
    // 16 chars covers "amendment_record" exactly; trailing colon plus 2
    // spaces gives the aligned column.
    const LABEL_WIDTH: usize = 16;
    let padded = format!("{label}:{:width$}", "", width = LABEL_WIDTH - label.len() + 1);
    let mut iter = members.iter();
    if let Some(first) = iter.next() {
        buf.push_str(&format!("    {padded}{first}\n"));
    }
    for rest in iter {
        // Subsequent entries align under the first id column.
        let blanks = " ".repeat(LABEL_WIDTH + 2);
        buf.push_str(&format!("    {blanks}{rest}\n"));
    }
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
            amends: Vec::new(),
            amendment_record: None,
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

    /// AC-1: a coupling violation produces a path-centric block.
    #[test]
    fn ac1_violation_when_path_changed_without_spec_edit() {
        let idx = index_claiming("044-multi-agent-orchestration", &["crates/orchestrator"]);
        let diff = diffset(&["crates/orchestrator/src/lib.rs"]);
        let outcome = check_coupling(&idx, &diff, "");
        assert_eq!(outcome.violations.len(), 1);
        assert_eq!(outcome.violations[0].path, "crates/orchestrator/src/lib.rs");
        assert_eq!(
            outcome.violations[0].owners.all_unique_sorted(),
            vec!["044-multi-agent-orchestration"]
        );
        assert!(outcome.violations[0].owners.amends.is_empty());
        assert!(outcome.violations[0].owners.amendment_record.is_empty());
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
    fn multi_spec_diff_separate_paths_each_owed() {
        let mut idx = index_claiming("044-multi-agent-orchestration", &["crates/orchestrator"]);
        idx.traceability.mappings.push(TraceMapping {
            spec_id: "067-tool-definition-registry".to_string(),
            spec_status: Some("approved".to_string()),
            depends_on: Vec::new(),
            amends: Vec::new(),
            amendment_record: None,
            implementing_paths: vec![ImplementingPath {
                path: "crates/tool-registry".to_string(),
                name: None,
                source: Some(TraceSource::SpecImplements),
            }],
        });
        // Two paths, each with a single distinct claimant. Both fire.
        let diff = diffset(&[
            "crates/orchestrator/src/lib.rs",
            "crates/tool-registry/src/lib.rs",
        ]);
        let outcome = check_coupling(&idx, &diff, "");
        assert_eq!(outcome.violations.len(), 2);
        // Sorted by path.
        assert_eq!(outcome.violations[0].path, "crates/orchestrator/src/lib.rs");
        assert_eq!(outcome.violations[1].path, "crates/tool-registry/src/lib.rs");
        assert_eq!(outcome.violations[0].owners.implements.len(), 1);
        assert_eq!(outcome.violations[1].owners.implements.len(), 1);
    }

    /// Spec 130 SC-1: a path claimed by N≥2 specs is cleared when ANY one
    /// claimant's spec.md is in the diff. The remaining claimants do not
    /// need to edit.
    #[test]
    fn primary_owner_heuristic_clears_when_any_claimant_edits() {
        // Two specs both claim crates/orchestrator (busy crate scenario).
        let mut idx = index_claiming("044-multi-agent-orchestration", &["crates/orchestrator"]);
        idx.traceability.mappings.push(TraceMapping {
            spec_id: "079-scheduling".to_string(),
            spec_status: Some("approved".to_string()),
            depends_on: Vec::new(),
            amends: Vec::new(),
            amendment_record: None,
            implementing_paths: vec![ImplementingPath {
                path: "crates/orchestrator".to_string(),
                name: None,
                source: Some(TraceSource::SpecImplements),
            }],
        });
        // Diff edits an orchestrator file but only ONE of the two claimant
        // spec.md files. Heuristic accepts.
        let diff = diffset(&[
            "crates/orchestrator/src/lib.rs",
            "specs/079-scheduling/spec.md",
        ]);
        let outcome = check_coupling(&idx, &diff, "");
        assert!(
            outcome.violations.is_empty(),
            "heuristic should clear the path; got violations: {:?}",
            outcome.violations
        );
        assert_eq!(outcome.exit_code(), 0);
    }

    /// When zero claimants edit, the violation lists ALL claimants for
    /// reviewer transparency (spec 130 FR-002).
    #[test]
    fn multi_claim_violation_names_all_claimants() {
        let mut idx = index_claiming("044-multi-agent-orchestration", &["crates/orchestrator"]);
        for spec_id in ["052-state-persistence", "079-scheduling"] {
            idx.traceability.mappings.push(TraceMapping {
                spec_id: spec_id.to_string(),
                spec_status: Some("approved".to_string()),
                depends_on: Vec::new(),
                amends: Vec::new(),
                amendment_record: None,
                implementing_paths: vec![ImplementingPath {
                    path: "crates/orchestrator".to_string(),
                    name: None,
                    source: Some(TraceSource::SpecImplements),
                }],
            });
        }
        let diff = diffset(&["crates/orchestrator/src/lib.rs"]);
        let outcome = check_coupling(&idx, &diff, "");
        assert_eq!(outcome.violations.len(), 1);
        let v = &outcome.violations[0];
        assert_eq!(v.path, "crates/orchestrator/src/lib.rs");
        assert_eq!(v.owners.implements.len(), 3);
        // All three claimants surfaced, sorted.
        assert_eq!(
            v.owners.all_unique_sorted(),
            vec![
                "044-multi-agent-orchestration",
                "052-state-persistence",
                "079-scheduling",
            ]
        );
        let rendered = render(&outcome);
        // Header signals multi-claim (implements-only path keeps the
        // spec 130 compact form).
        assert!(rendered.contains("claimed by 3 specs"));
        for c in v.owners.implements.iter() {
            assert!(rendered.contains(c.as_str()));
        }
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
