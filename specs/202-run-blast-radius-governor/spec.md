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
establishes:
  # Slice A (FR-001): new run_budget module
  - unit: { kind: file, path: crates/factory-contracts/src/run_budget.rs }
  # Slice A (FR-001): governance_envelope gains budgets: field
  - unit: { kind: file, path: crates/factory-contracts/src/governance_envelope.rs }
  # Slice B (FR-002): run-level meter + budget PreStepGate + gate chain
  - unit: { kind: file, path: crates/orchestrator/src/budget_gate.rs }
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
  # Slice B (FR-002): orchestrator takes a factory-contracts dependency for the
  # RunBudget* meter types consumed by budget_gate.rs.
  - aspect: "run-budget-metering"
    unit: { kind: file, path: crates/orchestrator/Cargo.toml }
  # Slice A (FR-001): factory-contracts re-exports the new RunBudget* types.
  - aspect: "run-budget-contract-exports"
    unit: { kind: file, path: crates/factory-contracts/src/lib.rs }
  # Slice A (FR-001): the schema-version-pin test tracks the 1.0.0 -> 1.1.0 bump.
  - aspect: "schema-version-pin"
    unit: { kind: file, path: crates/factory-contracts/src/build_spec.rs }
references:
  - role: enforcer
    unit: { kind: crate, id: factory-engine }
  - role: context
    unit: { kind: file, path: platform/services/stagecraft/api/factory/runsScheduler.ts }
  - role: context
    unit: { kind: file, path: docs/owasp-agentic-top-10-2026.md }
  # Surfaces surveyed by the 2026-06-12 refinement pass (non-owning;
  # implementation claims land with the implementation PR):
  - role: pause-channel-precedent
    unit: { kind: file, path: product/apps/opc/src-tauri/src/commands/run_governance.rs }
  - role: certificate-extension-target
    unit: { kind: file, path: crates/factory-engine/src/governance_certificate.rs }
  - role: trip-pattern-library
    unit: { kind: file, path: crates/orchestrator/src/circuit_breaker.rs }
---

# Feature Specification: Run Blast-Radius Governor

**Feature Branch**: `202-run-blast-radius-governor`
**Created**: 2026-06-11
**Refined**: 2026-06-12 (sketch FRs hardened against the surveyed metering
surfaces; see §Code reality)
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

## Code reality: the metering surfaces (surveyed 2026-06-12)

The refinement pass grounded each FR in what exists today. Five findings
shape the design:

1. **The instruction boundary exists and is step-shaped.** The
   orchestrator dispatch loop consults the `PreStepGate` trait
   (`crates/orchestrator/src/lib.rs`) once per step, before dispatch, on
   both the persisted and non-persisted paths; a gate `Err` fails the
   step closed, cascades `Skipped` to dependents, and persists the run
   summary. The live implementation is `GrantRenewalGate`
   (`product/apps/opc/src-tauri/src/commands/run_governance.rs`), the
   spec 198 FR-005 pause channel this spec reuses.
2. **Per-step actuals are already collected; run-level accumulation is
   not.** Every dispatched step records `tokens_used`, `cost_usd`,
   `duration_ms`, `num_turns`, and `retry_count` into the run summary.
   Nothing accumulates these across steps, and nothing compares any of
   them to a ceiling. `FactoryPipelineState.total_tokens` is the one
   cross-phase accumulator, and it too is never checked.
3. **One ceiling is a dead stub.** `FactoryEngineConfig.max_total_tokens`
   (annotated NF-002) is declared, hardcoded `None` at the CLI
   entrypoint, and read nowhere. FR-002 activates or retires it; after
   this spec no declared-but-unenforced ceiling remains (AC-6).
4. **Within a step the run is opaque.** The executor runs the agent as a
   subprocess and observes tool calls only from post-exit accounting.
   Budget checks therefore fire at step entry against accumulated
   actuals; a breaching step can overshoot its run-level ceiling by at
   most one step's work, with in-step wall-clock already bounded by the
   executor's per-step, effort-scaled timeout. This bounded overshoot is
   an accepted contract of the design: it is metered, surfaced, and
   certificate-visible, not silently absorbed.
5. **The platform scheduler is a sweeper, not a queue.**
   `platform/services/stagecraft/api/factory/runsScheduler.ts` flips
   stale runs to `failed` on a cron using `last_event_at`; there is no
   runs-in-flight counter and no enqueue-rate observation. FR-003's
   queue-storm lever is a new count gate at run submission, not a
   modification of the sweeper.

Supporting surfaces: the oscillation-trip pattern already exists as an
unwired library (`crates/orchestrator/src/circuit_breaker.rs`, spec 102
FR-032/FR-035, consecutive-failure trip; a library today, not wired into
dispatch); the certificate extension point is `CertificateBuilder` plus
`verify_certificate` (`crates/factory-engine/src/governance_certificate.rs`,
schema v1.5.0), with `SandboxExecutionRecord.resource_peak` as the existing
precedent for quantitative per-stage binding. Naming note:
`crates/factory-contracts/src/budget.rs` already exports `AssumptionBudget`
(spec 121, claim provenance, unrelated); the governor's types take a
distinct `RunBudget*` prefix to avoid collision.

## Functional requirements

- **FR-001 — Declared budgets in the envelope.** The governance envelope
  gains an additive `budgets:` section, every field ASI08-tagged inline
  (the spec 198 FR-002 idiom: the brief cites its statutes). Shape
  discipline: the envelope is predicate-shaped, never topology-shaped
  (198 P-2), so budgets are declared at two scopes only: `per_run`
  (ceiling on the whole run) and `per_stage` (a uniform ceiling no single
  stage may exceed); a stage-id-keyed map is inadmissible, since stage
  ids are run topology OAP does not model. Axes, each optional:
  - `tool_invocations` — as reported by the executor's post-step turn
    accounting today; a finer per-tool-call count is adopted when the
    executor surfaces one.
  - `file_mutations` — requires a new post-step workspace-accounting
    observation point (the executor does not report mutations today).
  - `spawned_agents` — dispatched step count, including the dynamically
    generated Phase-2 scaffold manifest, whose step count is bounded at
    generation time, not only during dispatch.
  - `wall_clock_secs` — run-level elapsed time (per-step timeouts
    already exist and are unchanged; see §Overlaps).
  - `tokens` / `cost_usd` — the existing per-step actuals, accumulated.

  Budgets are admitted like every other envelope field (two-sided
  validation, 198 FR-001/FR-003). An absent budget does NOT mean
  unlimited: platform defaults apply fail-closed, and the admission
  record shows each defaulted axis with `source: platform-default`. Org
  policy may tighten, never loosen, the platform defaults (the spec 047
  five-tier merge direction). Versioning: the schema bumps 1.0.0 → 1.1.0
  with the Rust twin (`GOVERNANCE_ENVELOPE_SCHEMA_VERSION`,
  `crates/factory-contracts/src/governance_envelope.rs`) updated in the
  same PR; the version is a compile-time const, so a mismatch fails at
  parse. Lockstep posture: `governance-envelope.schema.yaml` is Tier B in
  the spec 212 checker, so the addition is OAP-unilateral and
  factory adopts on its own schedule.
- **FR-002 — Engine-side metering and circuit break.** A run-level
  `RunBudgetMeter` accumulates the per-step actuals the orchestrator
  already collects (§Code reality 2), one accumulator per admitted axis.
  A budget gate implementing `PreStepGate` checks the meter against the
  admitted ceilings at every step boundary, ordered after
  `GrantRenewalGate` and composing with it, never replacing it. Exceeding
  any ceiling pauses the run at the next step boundary with the same
  pause semantics as a failed grant renewal (198 FR-005): the step fails
  closed before dispatch, the breach surfaces via introspection (172)
  with an attributable record naming axis, ceiling, and actual, and
  resume requires an explicit human actor with either a raised ceiling
  (a new admission or an audited override, AC-5) or an abort. The engine
  never silently resumes and never auto-raises a budget. Granularity
  contract: checks fire at step entry, so a single step may overshoot by
  at most its own work (§Code reality 4); the overshoot is metered and
  certificate-visible. This FR also discharges the dead stub:
  `max_total_tokens` is either enforced through the meter or removed
  (AC-6).
- **FR-003 — Fan-out and feedback-loop detection.** Beyond static
  ceilings, the governor detects the ASI08 propagation signatures, each
  with thresholds declared as `budgets:` fields, never hardcoded:
  - *(a) Repeated near-identical intents* within a window, engine-side.
    Intent identity is the run's goal id plus a step signature: a
    content hash over the step's normalized instruction text, mirroring
    the `goal_id` construction in
    `crates/factory-engine/src/intent_capsule.rs`. The step id is
    excluded from the hash so dynamically generated near-twin steps
    collide; the normalization rule is fixed at implementation.
  - *(b) Oscillating retry/compensation loops between stages.* Inputs
    are the per-step `retry_count` and the step failure/retry sequence
    the orchestrator already records; the detector reuses the
    consecutive-trip pattern of `circuit_breaker.rs` (102
    FR-032/FR-035), wiring the today-unwired library into dispatch.
  - *(c) Queue storms, platform-side.* Runs-in-flight per org
    (`queued` + `running`) counted at run submission against a
    configurable ceiling; a new count gate, the staleness sweeper is
    unchanged (§Code reality 5).

  Response order: detection throttles first (rate cap), breaks second
  (pause via the FR-002 channel).
- **FR-004 — Approval-velocity (governance-drift) counter.** Bulk
  rubber-stamping is ASI08's governance-drift vulnerability. The governor
  counts approvals per actor per window and surfaces anomalous approval
  velocity on the approval surfaces, composing with spec 201's
  fact-grounded `ApprovalSummary` evidence rows and `summaryHash` audit
  trail; it records, it does not block — blocking on human behaviour is
  an org-policy decision, not a platform hardcode (the 198 FR-013
  posture: depth of policy is org-filed).
- **FR-005 — Certificate binding of consumption actuals.** The
  governance certificate gains a `budget_consumption` record inside the
  signed payload (covered by the content hash and Ed25519 signature;
  certificate schema bumps 1.5.0 → 1.6.0): per admitted axis, `{ceiling,
  actual, source: declared | platform-default, breached}` at termination
  (success or halt). `verify-certificate` gains two checks: every
  admitted axis is present, and the actuals are internally consistent
  with the per-stage records; tampering with an actual fails
  verification with an axis-specific diagnostic (the spec 102 FR-007
  posture: the verifier does not trust the producer). A run that
  circuit-broke is visibly marked, mirroring how halts already surface.

## Overlaps with existing machinery (refine, do not duplicate)

- **Per-step wall-clock timeouts** (executor, effort-scaled) and
  **per-step retry caps** (`max_retries` in manifest generation) stay as
  they are: they bound a single step. The governor's axes bound the run
  and the uniform per-stage ceiling; FR-003 (b)'s oscillation detection
  is inter-stage, orthogonal to the intra-step retry cap.
- **Approval-gate `timeout_ms`** (process-phase HITL wait with
  escalation) is an operational timeout, not a budget; FR-004 counts
  approval velocity, a different quantity.
- **`FactoryPipelineState.total_tokens`** is the prototype accumulator;
  FR-002 subsumes it into the per-axis meter rather than adding a second
  token counter.

## Acceptance criteria

- **AC-1.** A run whose accumulated tool-invocation actuals exceed the
  admitted budget pauses fail-closed at the next step boundary: the next
  step does not dispatch, the attributable error names the axis, the
  ceiling, and the actual, and resume requires a human actor id.
- **AC-2.** A seeded feedback-loop fixture (two stages retrying each
  other) trips the oscillation detector before exhausting the run's
  wall-clock budget.
- **AC-3.** A factory admitted with no `budgets:` section runs under
  platform defaults; the admission record shows every defaulted axis
  with `source: platform-default`.
- **AC-4.** The emitted certificate carries `{ceiling, actual, source,
  breached}` per admitted axis inside the signed payload; tampering with
  an actual fails `verify-certificate` with an axis-specific diagnostic.
- **AC-5.** No code path raises a budget without a new admission or an
  audited human override.
- **AC-6.** No declared-but-unenforced ceiling remains in the engine
  configuration: `max_total_tokens` is enforced through the meter or
  removed, and `FactoryPipelineState.total_tokens` is subsumed by the
  meter's `tokens` axis or removed — no two independent token
  accumulators coexist after implementation.
- **AC-7.** The envelope schema 1.1.0 and its Rust twin land in the same
  PR; schema-parity (125/191) and the factory-schema-lockstep lane (212)
  are green.

## Out of scope

- The kill switch (org-wide halt is spec 208; the governor bounds a single
  run, it does not stop the fleet).
- Org risk-budget *values* — what the right ceilings are is filed org
  policy (the spec 198 FR-013 posture: depth of policy is org-filed, not
  OAP-hardcoded).
- Model-quality or semantic judgment of whether work is "useful" — the
  governor counts, it does not evaluate.
- HITL gate declaration and presentation (specs 198 FR-008 and 201).
- Per-tool-call interception inside a step. Sub-step granularity is
  executor-internal; the governor's boundary is the orchestrator step,
  and the bounded overshoot that follows is a stated contract (§Code
  reality 4), not a gap this spec closes.

## Sequencing

Gated on spec 198 reaching `implementation: complete` (the budgets section
rides the envelope schema and the pause semantics ride grant renewal,
198 FR-005). Two parts may land earlier behind a non-blocking flag because
they do not ride the envelope: FR-003 (c)'s platform-side runs-in-flight
counter (detection-only, thresholds from platform config until the
envelope carries them) and FR-004's approval-velocity counter (records,
never blocks). The 2026-06-12 refinement is intentionally pre-gate: the
FRs are implementable the day 198 flips.
