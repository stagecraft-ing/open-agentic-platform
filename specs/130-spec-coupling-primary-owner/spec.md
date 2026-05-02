---
id: "130-spec-coupling-primary-owner"
slug: spec-coupling-primary-owner
title: "Spec/code coupling: primary-owner heuristic — relax to any-claimant edit"
status: approved
implementation: complete
amends: ["127"]
owner: bart
created: "2026-05-02"
approved: "2026-05-02"
kind: governance
risk: low
depends_on:
  - "127"  # spec-code-coupling-gate (the contract being relaxed)
  - "129"  # granular-package-oap-metadata (surfaced the over-fire scenario)
code_aliases: ["SPEC_COUPLING_PRIMARY_OWNER"]
implements:
  - path: tools/spec-code-coupling-check
summary: >
  Spec 127's gate fires once per (spec, path) pair: a path claimed by N
  specs requires all N spec.md files in the diff. In OAP's current
  corpus this cascades — `crates/orchestrator` is claimed by 12 specs;
  `crates/axiomregent` by 7. A simple file-touch demo for spec 129
  triggered violations on 20 specs. This amendment relaxes the rule:
  ANY one claimant's spec.md edit covers the path. The gate output
  surfaces all claimants on a violation block so reviewers can sanity-
  check that the right one was edited.
---

# 130 — Primary-owner heuristic for spec/code coupling

## 1. Problem Statement

Spec 127 introduced a CI gate that fails any PR whose diff touches a
path claimed in some spec's `implements:` list without amending that
spec's `spec.md`. The semantics are correct in principle. In practice,
OAP's `implements:` declarations are **broad-by-design**:

```
$ grep -rh "path: crates/" specs/*/spec.md | sort | uniq -c | sort -rn | head
12   - path: crates/orchestrator
 5   - path: crates/policy-kernel
 4   - path: crates/featuregraph
 4   - path: crates/axiomregent
```

Twelve specs claim `crates/orchestrator` because each governs a different
aspect (state-persistence, scheduling, agent-organizer, governance,
artifact-store, …). When a contributor changes one orchestrator file,
the gate fires once per claimant — twelve cosmetic spec amendments
covering one substantive change.

Spec 129 (granular `[package.metadata.oap]`) surfaced this load-bearing.
A demo that added `// Spec:` comment headers to three files in busy
crates would have demanded edits to 20 specs. The friction lands on
the wrong door.

## 2. Decision

Adopt the **primary-owner heuristic**: when a path is claimed by N≥2
specs, the gate accepts ANY ONE of their `spec.md` edits as covering
the path. Strict-but-not-cascading.

This does not establish "primary ownership" semantics in the index —
no claim is privileged over another. The gate simply infers that *some*
owner reviewed the change, which is the actual review-state question.
The remaining claimants may be tangentially related (governance lens,
deps-on relationship, scheduling overlay) and don't need to gate the
PR.

## 3. Scope

### In scope

- Refactor `tools/spec-code-coupling-check/src/lib.rs::check_coupling`
  from per-spec aggregation to per-path aggregation.
- Change the `Violation` shape: `{spec_id, paths}` → `{path, claimants}`.
- Update the renderer to surface every claimant on each violation block,
  with a "claimed by N specs" header for N≥2.
- Update the resolution hint: "amend ANY ONE claimant's spec.md".
- Amend spec 127 frontmatter: `amended: 2026-05-02`,
  `amendment_record: "130-spec-coupling-primary-owner"`, plus an
  in-body callout in §4 (FR-003).

### Out of scope

- `primary: true` per-claim flags (OQ-1; see §6).
- Re-narrowing existing `implements:` declarations to crate sub-paths
  (spec 129c — separate audit work).
- Changes to the bypass list, waiver mechanism, or `--paths-from`
  behaviour. All preserved.

## 4. Functional Requirements

- **FR-001 — any-claimant clears.** A diff path P with claimants
  `{S1, …, SN}` passes the gate if `specs/Si/spec.md ∈ diff` for at
  least one `i ∈ {1, …, N}`.
- **FR-002 — full claimant disclosure on violation.** When zero claimants
  edit, the violation block names the path and lists every claimant
  (sorted, one per line). For N=1 the block uses a compact one-line
  form `<path> (claimed by <id>)`; for N≥2 it uses an expanded form.
- **FR-003 — heuristic, not ownership.** This is a coverage relaxation.
  The index does not gain a `primary` flag. The gate does not pick a
  primary; it accepts any owner's edit as evidence of review.
- **FR-004 — extensibility.** A future spec MAY introduce explicit
  per-claim ownership flags (e.g. `primary: true` in `implements:`
  entries). When that lands, this heuristic remains as the fallback
  for paths without an explicit primary.
- **SC-001 — would-be Unit 4 demo passes.** A diff that adds `// Spec:`
  headers to `crates/axiomregent/src/{github,checkpoint}/mod.rs` and
  `crates/orchestrator/src/scheduler/mod.rs` plus the matching
  three claimant spec edits (one per file) passes the gate without
  the 17 cosmetic amendments the strict rule would demand.

## 5. Acceptance

- **AC-1.** Unit test
  `tests::primary_owner_heuristic_clears_when_any_claimant_edits`
  passes: a path claimed by 2 specs is cleared by editing one of them.
- **AC-2.** Unit test `tests::multi_claim_violation_names_all_claimants`
  passes: when no claimant edits, the violation block lists all of them
  and the renderer's header reads "claimed by N specs".
- **AC-3.** Existing AC tests from spec 127 continue to pass under the
  new shape: `ac1_violation_when_path_changed_without_spec_edit`,
  `ac2_no_violation_when_spec_edited`, `ac3_bypass_paths_never_violate`,
  `ac4_waiver_suppresses_exit_but_keeps_violations`.
- **AC-4.** `make ci` exits 0 on the Unit 4.5 commit, demonstrating the
  gate runs cleanly against the cumulative diff (Units 1–4.5).
- **AC-5.** `tools/ci-parity-check` continues to mirror
  `.github/workflows/ci-spec-code-coupling.yml` after no surface-API
  change to the binary.

## 6. Open Questions

- **OQ-1: should `primary: true` per-claim ownership replace the
  heuristic?** Defer to future spec; revisit if the heuristic produces
  ambiguous coverage decisions in practice. The heuristic is correct
  when claimants overlap by design (governance lens, deps-on); it is
  ambiguous when a claimant accidentally edits its `spec.md` for
  unrelated reasons in the same PR. If that pattern surfaces, an
  explicit `primary` flag is the next step.

## 7. Risks and Mitigations

- **Risk:** A claimant edits its `spec.md` for unrelated reasons in
  the same PR; the gate wrongly considers a coupled path "covered".
  **Mitigation:** Reviewers see the full claimant list in the
  violation block. The waiver mechanism remains for explicit decoupling.
  OQ-1 captures the per-claim primary flag as the eventual answer.

- **Risk:** Reviewers don't notice that they covered a path via the
  "wrong" claimant when several apply.
  **Mitigation:** When a path passes silently (no violation), the
  gate emits no per-path detail; this is a known trade-off. A future
  enhancement could add `--verbose` to print covered-path summaries
  with the claimant chosen.

- **Risk:** The amendment makes the gate weaker than the spec 127
  authoring intent.
  **Mitigation:** Spec 127 §1 ("Symmetry, not size") still holds at
  the path level — every claimed path still requires SOME owner to
  review. The relaxation is from N-of-N to 1-of-N, which matches how
  human review actually works on shared infrastructure.
