---
id: "202-run-blast-radius-governor"
title: "Run Blast-Radius Governor (ASI08 cascading-failure caps)"
feature_branch: "feat/202-run-blast-radius-governor"
status: draft
implementation: pending
kind: governance
domain: platform
created: "2026-06-11"
authors: ["open-agentic-platform"]
language: en
summary: >
  Close the one ASI 2026 control whose residual is stated but unowned. Spec
  198's all-ten table marks ASI08 (Cascading Failures) "partial — residual
  stated" and names blast-radius caps as follow-on; this spec is that
  follow-on. Existing levers (halt-on-failure, stop-hook 166, HITL gate
  predicates 198 FR-008, introspection 172) are reactive gates — they fire
  on an observed failure or at a declared checkpoint. What is missing is the
  quantitative ceiling between gates: per-run and per-stage budgets (tool
  invocations, file mutations, spawned work, wall-clock, cost), metered by
  the engine and circuit-breaking fail-closed when exceeded, plus fan-out
  and feedback-loop detection (ASI08 m6/m7, ASI02 m5 adaptive tool
  budgeting). A run that exceeds its admitted budget pauses at the next
  instruction boundary and requires a human resume; consumption actuals are
  bound into the governance certificate so the receipt shows how close the
  run came to its ceiling.
code_aliases: ["RUN_BLAST_RADIUS_GOVERNOR"]
compliance:
  - framework: "owasp-asi-2026"
    controls: ["ASI02", "ASI08"]
depends_on:
  - "198-factory-governance-envelope"
  - "166-opc-stop-hook-gate-chain"
  - "172-opc-live-agent-session-introspection"
  - "075-factory-workflow-engine"
extends:
  # Budget declarations are an additive, ASI08-tagged section of the
  # envelope schema spec 198 establishes.
  - spec: "198-factory-governance-envelope"
    nature: additive
    unit: { kind: file, path: standards/schemas/factory/governance-envelope.schema.yaml }
  # Same precedent as specs 196, 194, 193, 187, 183: a new spec adds a row
  # to the featuregraph golden.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
refines:
  - aspect: "run-budget-metering"
    unit: { kind: file, path: crates/orchestrator/src/lib.rs }
references:
  - role: enforcer
    unit: { kind: crate, id: factory-engine }
  - role: context
    unit: { kind: file, path: platform/services/stagecraft/api/factory/runsScheduler.ts }
  - role: context
    unit: { kind: file, path: docs/owasp-agentic-top-10-2026.md }
---

# Feature Specification: Run Blast-Radius Governor

**Feature Branch**: `202-run-blast-radius-governor`
**Created**: 2026-06-11
**Status**: Draft (follow-on filed by the ASI gap-closure pass; named in
spec 198's all-ten table, ASI08 row)
**Input**: Spec 198 §"All-ten ASI coverage" records ASI08 as the only
control that is *partial with an unowned residual*: "blast-radius caps are
follow-on". ASI06's residual got spec 200 and ASI09's got spec 201 at
filing time; ASI08's did not. This spec gives it an owner.

## Purpose

ASI08 is about propagation and amplification, not the initial defect. OAP
already interposes gates — per-stage verification fails closed (075), the
stop-hook chain blocks session close (166), HITL predicates are declared in
the admitted envelope (198 FR-008), and an operator can watch and
force-disconnect a session (172). All of these are *event-shaped*: they
fire at a boundary or on an observed failure. Between boundaries, a
misbehaving run is unbounded — a feedback loop, a retry storm, or a
fan-out of repeated near-identical intents can do arbitrary work before
any gate is consulted (the ASI08 detection hooks: rapid fan-out,
oscillating retries, downstream queue storms, repeated identical intents).

The governor adds the *quantity* dimension: declared ceilings, engine-side
metering, and a circuit breaker between planner and executor (ASI08 m7),
composing with — never replacing — the existing gates.

## Functional requirements (sketch — refine before implementation)

- **FR-001 — Declared budgets in the envelope.** The governance envelope
  gains an additive, ASI08-tagged `budgets:` section: per-run and per-stage
  ceilings for tool invocations, file mutations, spawned agents/sub-tasks,
  wall-clock, and token/cost. Budgets are admitted like every other
  envelope field (two-sided validation, spec 198 FR-001/FR-003). An absent
  budget does NOT mean unlimited: platform defaults apply, fail-closed and
  visible in the admission record. Org policy may tighten, never loosen,
  the platform defaults (the spec 047 five-tier merge direction).
- **FR-002 — Engine-side metering and circuit break.** The OPC engine and
  orchestrator meter every countable action against the admitted budget.
  Exceeding any ceiling pauses the run at the next instruction boundary —
  the same pause semantics as a failed grant renewal (spec 198 FR-005) —
  surfaces the breach via introspection (172) with the metered actuals,
  and requires an explicit human resume with a raised ceiling or an
  abort. The engine never silently resumes or auto-raises a budget.
- **FR-003 — Fan-out and feedback-loop detection.** Beyond static
  ceilings, the governor detects the ASI08 propagation signatures:
  repeated near-identical intents within a window, oscillating
  retry/compensation loops between stages, and queue-storm thresholds on
  the platform-side run scheduler. Detection throttles first (rate cap),
  breaks second (pause) — detection thresholds are budget fields, not
  hardcodes.
- **FR-004 — Approval-velocity (governance-drift) counter.** Bulk
  rubber-stamping is ASI08's governance-drift vulnerability. The governor
  counts approvals per actor per window and surfaces anomalous approval
  velocity on the approval surfaces (composing with spec 201's evidence
  rows); it records, it does not block — blocking on human behaviour is an
  org-policy decision, not a platform hardcode.
- **FR-005 — Certificate binding of consumption actuals.** The governance
  certificate records, per budget axis, ceiling and actual at termination
  (success or halt). `verify-certificate` checks the actuals are present
  and internally consistent with the stage records; a run that
  circuit-broke is visibly marked, mirroring how halts already surface.

## Acceptance criteria (sketch)

- **AC-1.** A run whose stage exceeds its admitted tool-invocation budget
  pauses fail-closed at the next instruction boundary with an attributable
  error naming the axis, ceiling, and actual; resume requires a human actor
  id.
- **AC-2.** A seeded feedback-loop fixture (two stages retrying each other)
  trips the oscillation detector before exhausting wall-clock.
- **AC-3.** A factory admitted with no `budgets:` section runs under
  platform defaults; the admission record shows the defaulted values.
- **AC-4.** The emitted certificate carries ceiling/actual pairs per axis;
  tampering with an actual fails `verify-certificate`.
- **AC-5.** No code path raises a budget without a new admission or an
  audited human override.

## Out of scope

- The kill switch (org-wide halt is spec 208; the governor bounds a single
  run, it does not stop the fleet).
- Org risk-budget *values* — what the right ceilings are is filed org
  policy (the spec 198 FR-013 posture: depth of policy is org-filed, not
  OAP-hardcoded).
- Model-quality or semantic judgment of whether work is "useful" — the
  governor counts, it does not evaluate.
- HITL gate declaration and presentation (specs 198 FR-008 and 201).

## Sequencing

Gated on spec 198 reaching `implementation: complete` (the budgets section
rides the envelope schema and the pause semantics ride grant renewal).
Detection-only parts of FR-003/FR-004 may land earlier behind a
non-blocking flag.
