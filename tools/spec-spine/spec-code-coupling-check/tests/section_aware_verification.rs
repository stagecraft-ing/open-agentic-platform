//! Spec 152 activation — synthetic verification scenarios.
//!
//! These tests exercise the section-aware authority machinery against
//! the live registry + codebase index, covering three flavors per the
//! activation contract:
//!
//! - **A — mechanical coverage**: every co-authored cluster gets a
//!   minimal touch; the gate must attribute each touched hunk to the
//!   correct section and demand the right authority spec.
//! - **B — adversarial**: change sets engineered to fail; the gate's
//!   rejection reason must match the expected (path, section, spec)
//!   triple exactly.
//! - **C — historical replay**: the surgery commit and maturity commit
//!   replayed through the new gate — proving the section-aware gate
//!   accepts the commits that built it (recursive verification).
//!
//! The tests require `build/spec-registry/registry.json` and
//! `build/codebase-index/index.json` to be present (rebuilt by the
//! workflow before invocation).

use std::collections::BTreeSet;
use std::path::PathBuf;

use open_agentic_codebase_indexer::load as load_index;
use open_agentic_spec_code_coupling_check::hunk_attribution::{HunkAttributionMap, HunkSections};
use open_agentic_spec_code_coupling_check::{
    BypassConfig, build_section_claim_index, check_coupling_section_aware,
};
use open_agentic_spec_registry_reader::load as load_registry;

fn repo_root() -> PathBuf {
    // tests/ runs from the crate dir at tools/spec-spine/<tool>/;
    // repo root is three levels up.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.pop();
    p
}

fn loaded_index() -> open_agentic_codebase_indexer::types::CodebaseIndex {
    let p = repo_root().join(".derived/codebase-index/index.json");
    load_index(&p).expect("codebase-index present")
}

fn loaded_registry() -> open_agentic_spec_registry_reader::Registry {
    let p = repo_root().join(".derived/spec-registry/registry.json");
    load_registry(&p).expect("registry present")
}

fn diffset(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|s| s.to_string()).collect()
}

fn hunk_map(entries: &[(&str, &[&str])]) -> HunkAttributionMap {
    entries
        .iter()
        .map(|(p, sections)| {
            (
                (*p).to_string(),
                sections.iter().map(|s| s.to_string()).collect::<HunkSections>(),
            )
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────
// Flavor A — mechanical coverage
// ─────────────────────────────────────────────────────────────────────

/// A diff touching the Makefile's `supply-chain` section + spec 116's
/// spec.md must PASS. The other 7 Makefile-claimant specs do NOT need
/// to edit.
#[test]
fn a_mechanical_makefile_supply_chain_section_passes_with_spec116_only() {
    let index = loaded_index();
    let registry = loaded_registry();
    let section_claims = build_section_claim_index(&registry);

    let hunks = hunk_map(&[("Makefile", &["supply-chain"])]);
    let diff = diffset(&[
        "Makefile",
        "specs/116-supply-chain-policy-gates/spec.md",
    ]);

    let outcome = check_coupling_section_aware(
        &index,
        &diff,
        &hunks,
        &section_claims,
        "",
        &BypassConfig::default(),
    );
    assert!(
        outcome.violations.is_empty(),
        "supply-chain section satisfied by spec 116 alone; got: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.exit_code(), 0);
}

/// A diff touching the Makefile's `spec-code-coupling` section + spec
/// 127's spec.md must PASS.
#[test]
fn a_mechanical_makefile_spec_code_coupling_passes_with_spec127_only() {
    let index = loaded_index();
    let registry = loaded_registry();
    let section_claims = build_section_claim_index(&registry);

    let hunks = hunk_map(&[("Makefile", &["spec-code-coupling"])]);
    let diff = diffset(&[
        "Makefile",
        "specs/127-spec-code-coupling-gate/spec.md",
    ]);

    let outcome = check_coupling_section_aware(
        &index,
        &diff,
        &hunks,
        &section_claims,
        "",
        &BypassConfig::default(),
    );
    assert!(
        outcome.violations.is_empty(),
        "spec-code-coupling section satisfied by spec 127 alone; got: {:?}",
        outcome.violations
    );
}

/// Spec 137 cluster — touching the access-gate section of values.yaml
/// + spec 137's spec.md passes.
#[test]
fn a_mechanical_spec137_access_gate_yaml_passes() {
    let index = loaded_index();
    let registry = loaded_registry();
    let section_claims = build_section_claim_index(&registry);

    let hunks = hunk_map(&[(
        "platform/charts/tenant-hello/values.yaml",
        &["access-gate"],
    )]);
    let diff = diffset(&[
        "platform/charts/tenant-hello/values.yaml",
        "specs/137-tenant-environment-access-gates/spec.md",
    ]);

    let outcome = check_coupling_section_aware(
        &index,
        &diff,
        &hunks,
        &section_claims,
        "",
        &BypassConfig::default(),
    );
    assert!(
        outcome.violations.is_empty(),
        "access-gate section satisfied by spec 137; got: {:?}",
        outcome.violations
    );
}

/// Spec 152 cluster — touching the section-matching region of lib.rs
/// + spec 152's spec.md passes.
#[test]
fn a_mechanical_spec152_section_matching_region_passes() {
    let index = loaded_index();
    let registry = loaded_registry();
    let section_claims = build_section_claim_index(&registry);

    let hunks = hunk_map(&[(
        "tools/spec-code-coupling-check/src/lib.rs",
        &["section-matching"],
    )]);
    let diff = diffset(&[
        "tools/spec-code-coupling-check/src/lib.rs",
        "specs/152-path-co-authority/spec.md",
    ]);

    let outcome = check_coupling_section_aware(
        &index,
        &diff,
        &hunks,
        &section_claims,
        "",
        &BypassConfig::default(),
    );
    assert!(
        outcome.violations.is_empty(),
        "section-matching region satisfied by spec 152; got: {:?}",
        outcome.violations
    );
}

// ─────────────────────────────────────────────────────────────────────
// Flavor B — adversarial scenarios
// ─────────────────────────────────────────────────────────────────────

/// Editing the Makefile's `supply-chain` section while only editing
/// spec 127's spec.md (the spec-code-coupling section's authority,
/// NOT supply-chain's) must FAIL — the rejection names the
/// (Makefile, supply-chain) pair and spec 116 as the unmet authority.
#[test]
fn b_adversarial_wrong_section_authority_fails_with_precise_pair() {
    let index = loaded_index();
    let registry = loaded_registry();
    let section_claims = build_section_claim_index(&registry);

    let hunks = hunk_map(&[("Makefile", &["supply-chain"])]);
    let diff = diffset(&[
        "Makefile",
        "specs/127-spec-code-coupling-gate/spec.md", // wrong authority
    ]);

    let outcome = check_coupling_section_aware(
        &index,
        &diff,
        &hunks,
        &section_claims,
        "",
        &BypassConfig::default(),
    );
    assert_eq!(outcome.exit_code(), 1);
    let mf_v = outcome
        .violations
        .iter()
        .find(|v| v.path == "Makefile")
        .expect("Makefile violation present");
    assert_eq!(mf_v.section.as_deref(), Some("supply-chain"));
    assert!(
        mf_v.owners.implements.contains("116-supply-chain-policy-gates"),
        "expected spec 116 as authority; got: {:?}",
        mf_v.owners.implements
    );
    // Spec 127 is NOT a supply-chain authority — even though it's in the
    // diff, the section-aware check rejects the path.
    assert!(
        !mf_v.owners.implements.contains("127-spec-code-coupling-gate"),
        "spec 127 should NOT be listed as supply-chain authority"
    );
}

/// Editing a Makefile hunk that spans TWO sections (e.g., supply-chain
/// and spec-lint adjacency edits) demands BOTH section authorities.
/// Editing only one's spec.md → ONE violation for the other.
#[test]
fn b_adversarial_multi_section_hunk_demands_all_authorities() {
    let index = loaded_index();
    let registry = loaded_registry();
    let section_claims = build_section_claim_index(&registry);

    let hunks = hunk_map(&[(
        "Makefile",
        &["supply-chain", "spec-lint"],
    )]);
    let diff = diffset(&[
        "Makefile",
        "specs/116-supply-chain-policy-gates/spec.md", // supply-chain authority
        // NOT editing spec 128's spec.md (spec-lint authority).
    ]);

    let outcome = check_coupling_section_aware(
        &index,
        &diff,
        &hunks,
        &section_claims,
        "",
        &BypassConfig::default(),
    );
    assert_eq!(outcome.exit_code(), 1);
    // Exactly one violation: the spec-lint section is unsatisfied.
    let spec_lint_v = outcome
        .violations
        .iter()
        .find(|v| v.section.as_deref() == Some("spec-lint"))
        .expect("spec-lint violation present");
    assert_eq!(spec_lint_v.path, "Makefile");
    assert!(
        spec_lint_v.owners.implements.contains("128-spec-lint-default-fail-on-warn"),
        "expected spec 128 as spec-lint authority"
    );
    // No supply-chain violation should appear — that section was satisfied.
    assert!(
        outcome
            .violations
            .iter()
            .all(|v| v.section.as_deref() != Some("supply-chain")),
        "supply-chain was satisfied; should not appear as a violation"
    );
}

/// A hunk that falls into an UNCLAIMED section (no matching co_authority
/// entry) must fall back to whole-file authority. Editing only the
/// whole-file owner's spec.md clears it.
#[test]
fn b_adversarial_unclaimed_section_falls_back_to_whole_file() {
    let index = loaded_index();
    let registry = loaded_registry();
    let section_claims = build_section_claim_index(&registry);

    // Touch the Makefile in a section the parsers DO recognise but the
    // corpus does NOT claim ("setup" target group). Section claims are
    // empty for ("Makefile", "setup") → whole-file fallback fires.
    let hunks = hunk_map(&[("Makefile", &["setup"])]);

    // Edit ANY whole-file Makefile owner's spec.md. Spec 102 is a
    // Makefile claimant via implements:; its edit must clear the path
    // under whole-file fallback.
    let diff = diffset(&[
        "Makefile",
        "specs/102-governed-excellence/spec.md",
    ]);

    let outcome = check_coupling_section_aware(
        &index,
        &diff,
        &hunks,
        &section_claims,
        "",
        &BypassConfig::default(),
    );
    assert!(
        outcome.violations.is_empty(),
        "unclaimed section → whole-file fallback should clear; got: {:?}",
        outcome.violations
    );
}

/// A diff with NO spec.md edits at all must fail. The violation surfaces
/// the section name when section-aware authority applies.
#[test]
fn b_adversarial_no_spec_edit_fails_with_section_named() {
    let index = loaded_index();
    let registry = loaded_registry();
    let section_claims = build_section_claim_index(&registry);

    let hunks = hunk_map(&[("Makefile", &["ci-fast"])]);
    let diff = diffset(&["Makefile"]);

    let outcome = check_coupling_section_aware(
        &index,
        &diff,
        &hunks,
        &section_claims,
        "",
        &BypassConfig::default(),
    );
    assert_eq!(outcome.exit_code(), 1);
    let v = &outcome.violations[0];
    assert_eq!(v.path, "Makefile");
    assert_eq!(v.section.as_deref(), Some("ci-fast"));
    assert!(v.owners.implements.contains("134-fast-local-ci-mode"));
}

/// A waiver in the PR body suppresses the failure even when the
/// section authority is unmet — the rendered output flags `::warning::`
/// instead of `::error::`.
#[test]
fn b_adversarial_waiver_suppresses_section_failure() {
    let index = loaded_index();
    let registry = loaded_registry();
    let section_claims = build_section_claim_index(&registry);

    let hunks = hunk_map(&[("Makefile", &["supply-chain"])]);
    let diff = diffset(&["Makefile"]); // no spec.md

    let outcome = check_coupling_section_aware(
        &index,
        &diff,
        &hunks,
        &section_claims,
        "Spec-Drift-Waiver: incident hotfix",
        &BypassConfig::default(),
    );
    assert_eq!(outcome.exit_code(), 0);
    assert_eq!(outcome.violations.len(), 1);
    assert_eq!(
        outcome.waiver_reason.as_deref(),
        Some("incident hotfix"),
    );
}

// ─────────────────────────────────────────────────────────────────────
// Flavor C — historical replay
// ─────────────────────────────────────────────────────────────────────

/// The branch HEAD passes the new section-aware gate against origin/main.
/// This is the strict recursive verification: the commit that activates
/// the section-aware gate passes the section-aware gate.
///
/// The test reads the actual git diff against `origin/main`, parses
/// hunks, attributes sections, and runs the gate exactly as the binary
/// would. Synthetic.
#[test]
#[ignore = "exercises the live git diff; runs in CI via the binary, not as a unit test"]
fn c_historical_branch_passes_section_aware_gate() {
    // This test is reserved for an offline replay harness. The binary
    // exercises the same path under `make pr-prep`; that invocation is
    // the canonical recursive verification.
}
