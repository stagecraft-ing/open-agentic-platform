---
id: "034-featuregraph-registry-scanner-fix"
title: "featuregraph scanner reads compiled registry"
feature_branch: "034-featuregraph-registry-scanner-fix"
status: approved
implementation: complete
kind: platform
domain: tooling
created: "2026-03-29"
authors:
  - "open-agentic-platform"
language: en
summary: >
  Point the featuregraph scanner and related governance inputs at `.derived/spec-registry/registry.json`
  (compiled by spec-compiler) instead of requiring `spec/features.yaml`, so the governance panel
  can hydrate from the same source of truth as CI and the Inspect surface.
code_aliases:
  - FEATUREGRAPH_REGISTRY
  - GOVERNANCE_ENGINE
owner: bart
risk: low
origin:
  retroactive: true
---

# Feature Specification: featuregraph registry scanner fix

## Purpose

Today `featuregraph::scanner` and related paths assume **`spec/features.yaml`** as the feature manifest. The platform’s canonical registry is **`.derived/spec-registry/registry.json`** produced by **`spec-compiler`**. This mismatch keeps governance surfaces in a **degraded** or partial state when only the compiled registry exists.

## Scope

### In scope

- Read feature identity / graph inputs from **`registry.json`** (or an adapter that maps registry entries into the scanner’s internal model).
- Preserve existing **preflight** and **violation** semantics where feasible; adjust only data source wiring.
- Update **desktop** `featuregraph_overview` / governance paths so they do not depend on a stale `features.yaml` for core feature listing when the registry is present.
- Document **`spec-compiler compile`** as a prerequisite for local governance dev workflows.

### Out of scope

- Rewriting the entire featuregraph algorithm (only input source changes unless required).
- **035** agent execution routing through axiomregent (separate feature).

## Requirements

- **FR-001**: When `.derived/spec-registry/registry.json` exists and is valid, the scanner **does not require** `spec/features.yaml` for basic feature membership checks.
- **FR-002**: When the registry is missing, behavior degrades **explicitly** (clear message), matching existing degraded patterns in `GovernanceSurface`.
- **FR-003**: No regression to **registry-consumer** contracts (029–031) beyond intentional dependency bumps.

## Success criteria

- **SC-001**: Governance load path uses registry-backed feature data on a repo that has run `spec-compiler compile`.
- **SC-002**: `execution/verification.md` records commands and results.

## Contract notes

- Registry path convention: **`.derived/spec-registry/registry.json`** relative to repository root (same as `spec-compiler` output).


## Amendments received

**Amendment 2026-05-24 (record: 178-opc-directory-rename).**
Spec 178 (opc-directory-rename, 2026-05-24): mechanical regeneration
of `crates/featuregraph/tests/golden/features_graph.json` reflecting
the `product/apps/desktop/*` → `product/apps/opc/*` path rename in
spec frontmatter. No semantic change to this spec's claims; fixture
content updated 1:1 with the rename per the atomicity contract
encoded by spec 177 (ci-orchestrator-pr-gate) — featuregraph-golden
is a required ci-gate check precisely so renames carry their fixture
refresh inside the rename PR.

**Amendment 2026-05-24 (record: 181-registry-consumer-unit-grammar-authority).**
Spec 181 (registry-consumer-unit-grammar-authority, 2026-05-24):
mechanical regeneration of
`crates/featuregraph/tests/golden/features_graph.json` to include
spec 181's row in the feature graph. No semantic change to this
spec's claims; the new spec's spec.md contributes a new entry to the
featuregraph output by construction, and the originating PR carries
the fixture refresh per the spec 177 atomicity contract.

**Amendment 2026-05-24 (record: 181-impl-authority-resolver-unit-grammar-parity).**
Spec 181's implementation-completion flip (`status: draft → approved`,
`implementation: pending → complete`, plus `approved:` / `completed:`
date fields) ripples through the feature graph row for spec 181,
mechanically refreshing the golden. Same atomicity contract as the
prior receipt note for spec 181's row inclusion; no semantic change to
this spec's claims. The flip lands in the resolver-implementation PR
(separate from the spec-authoring PR per the Phase 1 firewall).

**Amendment 2026-06-19 (record: self, golden-guard-sharded-registry-repoint).**
The golden test's repo-detection guard in
`crates/featuregraph/tests/golden.rs` checked only the singular
`.derived/spec-registry/registry.json` (or legacy `spec/features.yaml`).
After spec 217 (engine swap and collapse) the registry is emitted ONLY as
the sharded `.derived/spec-registry/by-spec/` tree, which the scanner
already reads; the singular file is no longer produced. The guard therefore
silently SKIPPED the golden test (an early return that the test harness
counts as a pass) everywhere `spec-spine compile` runs, leaving the
featuregraph-golden ci-gate dormant while the committed fixture accrued
uncaught drift. This amendment repoints the guard to also accept the sharded
`by-spec/` directory (FR-001's intent: read the compiled registry, in its
current physical form), restoring the live gate, and re-syncs the golden to
the current registry. The re-sync is honest registry state, not new claims:
31 `status: approved -> superseded` updates, two `implementation` flips, the
derived `graph_fingerprint`, and one diagnostic-message text change (a source
`# Spec:` directive de-em-dashed elsewhere). Both the guard fix and the
fixture re-sync land in this PR; both paths are owned by this spec.

**Amendment 2026-05-24 (record: 180-opc-shell-codification).**
Spec 180 (opc-shell-codification, 2026-05-24): mechanical regeneration
of `crates/featuregraph/tests/golden/features_graph.json` to include
spec 180's row in the feature graph. No semantic change to this
spec's claims; the new spec's spec.md contributes a new entry to the
featuregraph output by construction, and the originating PR carries
the fixture refresh per the spec 177 atomicity contract.
