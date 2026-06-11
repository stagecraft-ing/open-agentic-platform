---
id: "211-encore-test-ci-job"
title: "Encore-Test CI Job (ASI09 verification integrity — the encore-test CI gap, closed)"
feature_branch: "feat/211-encore-test-ci-job"
status: draft
implementation: pending
kind: governance
domain: tooling
created: "2026-06-11"
authors: ["open-agentic-platform"]
language: en
summary: >
  Close the encore-test CI gap: the bare-vitest exclude list in
  stagecraft's vite.config.ts assigns 22 DB-bound test files (specs 115,
  124, 137, 139–143, 198, 201) to the `encore test` lane, but no CI job
  runs that lane — CI runs bare vitest only, so DB-bound acceptance
  suites never execute before merge. The gap already shipped a real
  violation invisibly (spec 198's FR-013 correction note: audit INSERTs
  violating a live check constraint, covered by a test CI never ran).
  This spec adds an enforcing encore-test CI job dispatched from the
  spec-177 orchestrator, a Makefile mirror per the spec-104 parity
  contract, and a lane-coverage guard so a file excluded from bare
  vitest cannot silently skip both lanes. Its completion is the named
  trigger for spec 198's ASI09 row flipping to "Solid" and spec 201's
  implementation flipping to complete.
code_aliases: ["ENCORE_TEST_CI_JOB"]
compliance:
  - framework: "owasp-asi-2026"
    controls: ["ASI09"]
depends_on:
  - "104-makefile-ci-parity-contract"
  - "135-fast-ci-as-default"
  - "177-ci-orchestrator-pr-gate"
  - "198-factory-governance-envelope"
  - "201-anti-blind-approval-ui"
extends:
  # The spec-177 orchestrator gains a route dispatching the new job
  # (same shape as spec 191's ci-schema-parity route).
  - spec: "177-ci-orchestrator-pr-gate"
    nature: additive
    unit: { kind: file, path: .github/workflows/ci.yml }
  # The new workflow is classified enforcing in ci-parity-check's
  # ENFORCING_WORKFLOWS so the Makefile↔CI run-mirror covers it.
  - spec: "104-makefile-ci-parity-contract"
    nature: additive
    unit: { kind: file, path: tools/oap/ci-parity-check/src/lib.rs }
  # Same precedent as specs 196, 194, 193, 187, 183 and the 202–210
  # batch: a new spec adds a row to the featuregraph golden.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
references:
  # The lane assignment this spec enforces coverage of. The
  # `encore-test-gating` aspect of this file is owned by spec 201; this
  # spec consumes the exclude list as input, it does not reshape it.
  - role: context
    unit: { kind: file, path: platform/services/stagecraft/vite.config.ts }
  # The sibling workflow whose pinned encore CLI version and npm setup
  # the new job mirrors.
  - role: context
    unit: { kind: file, path: .github/workflows/ci-stagecraft.yml }
  - role: context
    unit: { kind: file, path: docs/owasp-agentic-top-10-2026.md }
---

# Feature Specification: Encore-Test CI Job

**Feature Branch**: `211-encore-test-ci-job`
**Created**: 2026-06-11
**Status**: Draft (files the last acknowledged-but-unfiled item from the
ASI 2026 gap-closure pass)
**Input**: Spec 198's FR-013 correction note and ASI09 table row, and
spec 201's phase-4 closeout amendment, all name "the encore-test CI gap,
discovered 2026-06-11" as a process follow-up tracked outside any spec.
This spec is that follow-up, filed.

## Purpose

Stagecraft's test suite is split into two lanes by
`platform/services/stagecraft/vite.config.ts`: pure suites run under
bare vitest (what `npm test` and CI execute), and DB-bound suites —
excluded under bare vitest — run only under `encore test`, which sets
`ENCORE_RUNTIME_LIB` and provisions per-test databases. The exclude
list currently assigns 22 files across ten specs (115, 124, 137,
139–143, 198, 201) to the encore lane. No CI job runs that lane.

The consequence is structural, not hypothetical: a spec's DB-bound
acceptance suites can be written, pass locally, and then silently stop
protecting anything, because nothing in the merge path ever executes
them. Spec 198's FR-013 correction note records the proven instance —
the phase 5 implementation emitted audit actions that violated the
migration-32 check constraint on every real-database INSERT, the
covering test (`overrideTrustClass.test.ts`) was encore-test-gated, and
the violation shipped invisibly. A verification loop with an
unexercised lane is the same defect OAP's own gates exist to prevent: a
covenant without a gate decays (spec 209's framing, applied to OAP
itself).

The gap is also the named blocker for two documented lifecycle flips:
spec 198's ASI09 row reads "'Solid' awaits spec 201 AC-1–AC-5 verified
**in CI**", and spec 201 holds `implementation: in-progress` solely
because its DB-bound AC suites cannot run in CI. ASI09 (Human-Agent
Trust) is the driving control — a human approving on the basis of AC
suites that never run is exactly the blind trust the control names —
and the fix is continuous-validation discipline applied to OAP's own
verification loop.

## Functional requirements (sketch — refine before implementation)

- **FR-001 — Enforcing encore-test workflow.** A reusable workflow
  (working name `ci-stagecraft-encore.yml`) dispatched from the
  spec-177 orchestrator on the stagecraft route, running `encore test`
  with the same pinned Encore CLI version the existing
  `ci-stagecraft.yml` installs for codegen, so the vite-config encore
  lane (the full suite minus `check.test.ts`) executes against
  Encore-provisioned per-test databases. A red lane fails the PR
  through the spec-177 gate composition, and runs identically in
  `merge_group`.
- **FR-002 — Makefile mirror per the parity contract.** A
  `make ci-stagecraft-encore` target mirrors the workflow recipe; the
  workflow is classified in `ci-parity-check`'s `ENFORCING_WORKFLOWS`
  (spec 104) with aligned/divergent fixtures proving drift detection;
  the target joins `make ci-strict`. Whether it also joins the fast
  `make ci` lane is a measured spec-135 decision — only if warm runtime
  fits the ~5-minute budget; otherwise strict-only, with the
  measurement recorded.
- **FR-003 — Lane-coverage guard (skip-as-pass forbidden).** The two
  lanes must partition the suite, not leak: a guard asserts that every
  file in the bare-vitest exclude list actually executed in the encore
  lane (reporter output cross-checked against the exclude list, or an
  equivalent executed-file assertion). A DB-bound file skipped in both
  lanes is a CI failure naming the file — the spec 200 FR-004 /
  spec 209 FR-005 posture, applied to OAP's own test surface.
- **FR-004 — Runtime provisioning, fail-visible.** The job provisions
  the encore daemon and test databases on the hosted runner; a
  provisioning failure (daemon unreachable, image pull, migration
  error) fails with the cause surfaced, never skipped-green. No
  external network reliance beyond pinned tool installs (spec 158
  SHA-pinning applies to any new action refs).
- **FR-005 — The named trigger, discharged.** When the spec 201 AC
  suites (`approvalSummaryEndpoint.test.ts`) and the spec 198 FR-013
  suites (`overrideTrustClass.test.ts`, `grantDuplexHandlers.test.ts`)
  run green in this job, the flips those specs document fire: spec
  198's ASI09 row → "Solid"; spec 201 `implementation:` → `complete`.
  This spec's completion is the precondition both texts cite.

## Acceptance criteria (sketch)

- **AC-1.** A PR run executes `approvalSummaryEndpoint.test.ts` and
  `overrideTrustClass.test.ts` in CI and their results gate merge — the
  named trigger for the spec 198/201 flips is mechanically dischargeable.
- **AC-2.** Every file in the bare-vitest exclude list executes in the
  encore lane; removing a file from both lanes turns CI red naming the
  file (lane-coverage guard proven by fixture or seeded mutation).
- **AC-3.** A seeded DB-bound violation of the class that shipped
  invisibly (e.g. reverting the migration-46 constraint widening from
  spec 198's correction note) fails the PR.
- **AC-4.** `ci-parity-check` is green with the new workflow classified
  enforcing: the Makefile mirror exists, and the divergent fixture
  proves drift detection.
- **AC-5.** The job runs in `merge_group` with the same blocking
  semantics as on PRs (spec 177's gate composition, no PR-only carve-out).

## Out of scope

- **Lane assignment.** Which suites are DB-bound is owned by each
  suite's spec; the vite.config.ts `encore-test-gating` aspect is
  spec 201's. This spec enforces coverage of the assignment, it does
  not move files between lanes.
- **Restructuring `ci-stagecraft.yml`'s existing jobs.** The bare-vitest
  job stays as-is; this spec adds a sibling lane, not a rewrite.
- **Tenant-side CI** (spec 209 owns the produced-app analog).
- **Changing the `make ci` default composition** (spec 135 owns the
  fast-lane budget; FR-002 records the decision rule, not a new
  default).

## Sequencing

Implementable now — every dependency is landed machinery: the Encore
CLI is already version-pinned in CI for codegen, the exclude-list lane
assignment is live, and the spec-177 orchestrator already routes
stagecraft changes. Completion is the named trigger for spec 198's
ASI09 "Solid" flip and spec 201's `implementation: complete` (both
texts cite this spec). Per the gap-batch convention, relationship edges
above point only at verified existing paths; `establishes:` edges for
the new workflow file and its parity fixtures ride the implementation
PR that creates them (the spec 191 precedent).
