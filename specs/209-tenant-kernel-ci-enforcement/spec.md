---
id: "209-tenant-kernel-ci-enforcement"
title: "Tenant Kernel CI Enforcement Activation (ASI04 continuous validation)"
feature_branch: "feat/209-tenant-kernel-ci-enforcement"
status: draft
implementation: pending
kind: capability
domain: platform
created: "2026-06-11"
authors: ["open-agentic-platform"]
language: en
summary: >
  Convert the produced application from ASI-aware to ASI-enforcing. The
  born-with kernel (spec 167) seeds every produced project with the spec
  spine, validators, and a tenant CI workflow — but the machinery is
  carried, not wired: kernel emission does not auto-fire from pipeline
  transitions, tenant runs do not auto-emit governance certificates
  (spec 168's tenant-emit leg), and the seeded CI does not actually run
  the inherited gates (registry compile, spec-lint, coupling gate,
  provenance validator, certificate verify). A produced app that makes
  an unprovenance claim or drifts spec-from-code is not structurally
  prevented from merging it — the gates only fire on OAP's side. This
  spec is the activation: the seeded CI becomes enforcing
  (fail-the-PR, not advisory), emission and certificate auto-fire close
  the documented deferrals of specs 167/168, and the tenant CI verifies
  vended-tool integrity against .kernel-version before trusting any
  vended binary.
code_aliases: ["TENANT_KERNEL_CI_ENFORCEMENT"]
compliance:
  - framework: "owasp-asi-2026"
    controls: ["ASI04"]
depends_on:
  - "167-born-with-spec-spine-kernel"
  - "168-per-project-governance-certificate"
  - "112-factory-project-lifecycle"
extends:
  # tenant-ci.yml.tmpl was retired by spec 167's PR-2 npm-kernel impl
  # (the vendored-binary CI template); repointed to the surviving tenant gate
  # template. NOTE (plan G3): 209's enforcement premise now targets the npm
  # tenant CI — the prebuilt template's `spec-spine.yml` (`npx --no-install
  # spec-spine couple`), which lives in template-encore, not OAP. This is a
  # placeholder repoint; the full premise rewrite (advisory→blocking on the
  # npm CI) is owed when 209 is implemented (still draft).
  - spec: "167-born-with-spec-spine-kernel"
    nature: additive
    unit: { kind: file, path: crates/factory-engine/templates/kernel/tenant.makefile.tmpl }
  - spec: "168-per-project-governance-certificate"
    nature: additive
    unit: { kind: file, path: crates/factory-engine/src/governance_certificate.rs }
  # Same precedent as specs 196, 194, 193, 187, 183: a new spec adds a row
  # to the featuregraph golden.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
refines:
  - aspect: "emission-auto-fire"
    unit: { kind: file, path: crates/factory-engine/src/kernel_emission/emit.rs }
  - aspect: "vended-binary-integrity"
    unit: { kind: file, path: crates/factory-engine/src/kernel_emission/version.rs }
references:
  - role: context
    unit: { kind: file, path: crates/factory-engine/src/stages/quality_gates.rs }
  - role: context
    unit: { kind: file, path: docs/owasp-agentic-top-10-2026.md }
---

# Feature Specification: Tenant Kernel CI Enforcement Activation

**Feature Branch**: `209-tenant-kernel-ci-enforcement`
**Created**: 2026-06-11
**Status**: Draft (follow-on filed by the ASI gap-closure pass)
**Input**: The ASI 2026 gap analysis (2026-06-10) concluded: "The factory
produces applications that are born with a governance kernel but lack
the live runtime enforcement machinery to prevent violations of that
governance. The produced app is ASI-aware but not ASI-enforcing."

## Purpose

Specs 167 and 168 deliberately landed mechanism-first and documented
their activation deferrals: emission is callable and tested but no
pipeline transition fires it; the certificate emitter ships in the
kernel but tenant runs do not invoke it; the seeded tenant CI exists as
a template whose gates are scaffolding. Each deferral was honest — and
each is now the difference between a governed deliverable and a
deliverable that merely contains governance documents.

The activation matters for the same reason OAP's own gates matter: a
covenant without a gate decays. A tenant team that inherits a spec
spine but can merge drift will merge drift; six months later the
kernel's registry describes an application that no longer exists, and
the "single audit chain you can hand to a regulator" claim quietly
stops being true one commit past handoff.

This spec is scoped to *activation* of existing machinery. It adds no
new gate semantics — every gate it wires is one OAP already runs on
itself.

## Functional requirements (sketch — refine before implementation)

- **FR-001 — Enforcing tenant CI.** The seeded workflow runs, blocking:
  registry compile, spec-lint, the spec/code coupling gate against the
  tenant's own spine, the provenance validator over factory-written
  claims, and `verify-certificate` over the project's certificate. A
  red gate fails the tenant PR. Per-CI-platform templates beyond GitHub
  Actions remain the spec 167 deferral they already are.
- **FR-002 — Kernel emission auto-fire.** The project-creation pipeline
  transition invokes kernel emission; a project cannot complete creation
  with a missing or partial kernel (fail-closed, attributable). Closes
  the spec 167 "production pipeline auto-fire" deferral.
- **FR-003 — Tenant-emit certificate auto-fire.** A tenant-side factory
  run emits its governance certificate at termination (success or halt)
  under `.factory/runs/<run-id>/` — the spec 168 FR-002 tenant-emit leg
  — using the vended emitter, with the kernel's verifier able to verify
  offline.
- **FR-004 — Vended-tool integrity.** Before trusting any vended binary,
  the tenant CI verifies its content hash against `.kernel-version`
  (spec 167 FR-005's pinned-toolchain record). A hash mismatch fails the
  build naming the binary — the produced app's gates must not be
  satisfiable by swapping the gatekeeper (ASI04 m6, continuous
  validation; the spec 102 do-not-trust-the-producer posture applied to
  tooling).
- **FR-005 — Degraded-mode visibility.** A gate that cannot run (missing
  binary, no network for advisory data) fails visibly with reason —
  skip-as-pass is forbidden across all seeded gates (the spec 200 FR-004
  posture, uniformly applied).

## Acceptance criteria (sketch)

- **AC-1.** A freshly produced project's first CI run executes all
  seeded gates green, end-to-end from `git push` with no manual setup.
- **AC-2.** A seeded drift fixture (code edit without spine edit) fails
  the tenant coupling gate; a seeded unprovenance claim fails the
  validator; both fail the PR, not a log line.
- **AC-3.** A tenant factory run leaves a certificate that the vended
  verifier accepts; halting the run mid-stage still leaves a certificate
  (termination contract honored).
- **AC-4.** Tampering with a vended binary fails tenant CI with a hash
  diagnostic naming the binary.
- **AC-5.** Deleting a gate binary yields a visible failure with reason,
  not a skipped-green run.

## Out of scope

- New gate semantics (every wired gate exists; specs 127/130/133 own
  coupling semantics, 121 owns provenance, 168 owns the certificate).
- The SBOM/dependency parity gate content (spec 203 defines it; this
  spec provides the enforcing CI home it lands in).
- Tenant retrofit beyond born-with projects (spec 165's promotion path;
  retrofit activation follows the same contract once that path emits
  kernels).
- OAP-side CI (specs 104/135/177 own it).

## Sequencing

Implementable now — it activates landed machinery (specs 167/168 are
approved with documented deferrals; this spec is those deferrals'
delivery vehicle). Coordinates with spec 203 (whose parity gate lands in
the CI surface this spec makes enforcing) and precedes spec 210's
falsifiability check for the same reason.
