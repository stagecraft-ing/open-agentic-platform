# Implementation Plan: spec 202 (Run Blast-Radius Governor)

**Spec**: `specs/202-run-blast-radius-governor/spec.md`
**Gate cleared**: spec 198 `implementation: complete` (confirmed in task prompt)
**Plan date**: 2026-06-24

---

## Surface-drift findings (2026-06-24 survey vs 2026-06-12 spec refinement)

| Finding | Impact |
|---------|--------|
| `CERTIFICATE_VERSION` is already `"1.6.0"` (spec 218 FR-001 landed it). The spec text says `1.5.0 -> 1.6.0`; this is stale. The correct bump for spec 202 is `1.6.0 -> 1.7.0`. | **AC-4/FR-005 lockstep bump must be corrected in the PR.** The `CERTIFICATE_VERSION` const, the version check in `verify_certificate`, all test fixtures that assert the version string, and the doc comment on the const all need updating to `"1.7.0"`. |
| `FactoryPipelineState.total_tokens` accumulates tokens via `add_tokens()` in `pipeline_state.rs`. `FactoryEngineConfig.max_total_tokens` is declared and defaulted `None` in `engine.rs`, read nowhere, not enforced. Both confirmed present. | AC-6 is fully landable: subsume `total_tokens` into the meter's `tokens` axis, remove `max_total_tokens` or enforce it via the meter (the meter is simpler; retire the stub and wire the meter). |
| `GovernanceEnvelope` (and the schema file) are at `schema_version: "1.0.0"`. The Rust twin const is `GOVERNANCE_ENVELOPE_SCHEMA_VERSION = "1.0.0"`. The 1.1.0 bump is clean. | No drift; plan proceeds as written. |
| `crates/orchestrator/src/circuit_breaker.rs` is a complete, tested library. It is not imported anywhere in the orchestrator dispatch loops (confirmed by reading `lib.rs`: no `circuit_breaker` import in the dispatch path). | FR-003(b) wires this library. The precedent was correctly described in the spec. |
| `IntentCapsule.derive_goal_id()` uses `"goal-" + first 16 hex chars of SHA-256(goal_text)`. FR-003(a) says the step signature hashes `goal_id + normalized instruction text` with step id excluded. The normalization rule ("fixed at implementation") is intentionally left to the implementer. | No drift; the hash construction is derivable from the existing `derive_goal_id` pattern. |
| `GrantRenewalGate` in `run_governance.rs` is the sole wired `PreStepGate` today. The dispatch loop checks `options.pre_step` as a single `Option<Arc<dyn PreStepGate>>`. To compose the budget gate with `GrantRenewalGate` a chain wrapper is needed, or `DispatchOptions.pre_step` must become a `Vec`. | See Slice B design note. |
| `runs.ts::reserveRunCore` performs the admission check and writes the row. There is no existing runs-in-flight count at this call site. FR-003(c) adds one. `runsScheduler.ts` is confirmed unchanged (sweeper only). | Platform-side gate is additive to `reserveRunCore`. |
| Spec 202 `extends` frontmatter declares two edges. The `refines` edge points at `crates/orchestrator/src/lib.rs`. `references` edges point at the cert file, circuit-breaker, run_governance, and runsScheduler. The coupling gate enforces `refines` and `establishes` (non-`references`) edge paths exist and are owned by this spec. Any file edited in implementation that is NOT covered by the frontmatter graph will trip the coupling gate. | The `extends` edges cover the envelope schema and featuregraph golden. The `refines` edge covers `lib.rs`. But edits in Slice A (`factory-contracts/src/budget.rs`, `factory-contracts/src/governance_envelope.rs`) and Slice C (`factory-engine/src/governance_certificate.rs`) are not covered. The frontmatter must gain `establishes:` entries for those files before or in the same PR as each slice. See coupling note in each slice. |

---

## Coupling-graph gap: required frontmatter additions

The spec's current frontmatter has no `establishes:` entries. The coupling gate requires that every file edited or created by this spec either (a) sits under a path covered by an `establishes:` edge, or (b) is declared as a `refines:` unit. The files below need coverage:

| File to be edited | Required frontmatter edge |
|---|---|
| `crates/factory-contracts/src/governance_envelope.rs` | `establishes:` unit (kind: file) |
| `standards/schemas/factory/governance-envelope.schema.yaml` | already covered by `extends: 198` unit (the `extends` edge owns the path) |
| `crates/factory-contracts/src/lib.rs` (re-exports) | `establishes:` unit (kind: crate, id: factory-contracts) OR a file-level entry |
| `crates/factory-contracts/src/run_budget.rs` (new file) | `establishes:` unit (kind: file) |
| `crates/orchestrator/src/budget_gate.rs` (new file) | `establishes:` unit (kind: file) |
| `crates/orchestrator/src/lib.rs` | covered by `refines:` already |
| `crates/factory-engine/src/governance_certificate.rs` | `establishes:` unit (kind: file); currently only a `references:` role |
| `crates/factory-engine/src/lib.rs` (re-exports) | `establishes:` unit (kind: crate, id: factory-engine) OR file-level |
| `platform/services/stagecraft/api/factory/runs.ts` | currently a `references: context` unit; needs promotion to `establishes:` or `refines:` |
| `crates/featuregraph/tests/golden/features_graph.json` | covered by `extends: 034` unit |

**Rule of thumb for each PR**: update `specs/202-run-blast-radius-governor/spec.md` frontmatter in the same commit as the implementation files, or the coupling gate will fail. Do not add edges speculatively; add them in the PR whose diff touches that file.

---

## Slice decomposition

### Slice A: FR-001 Budget types + envelope schema 1.0.0 -> 1.1.0 (AC-3, AC-7)

**Land this first.** It is the dependency root: FR-002 (meter + gate), FR-003(b/c) thresholds, and FR-005 (certificate binding) all depend on the `RunBudget*` type shape being stable in `factory-contracts`.

**FR/AC coverage**: FR-001 fully, AC-3 (platform defaults visible at admission), AC-7 (schema-parity and lockstep in same PR).

**Files created/edited**:

- `crates/factory-contracts/src/run_budget.rs` (new)
  - `RunBudgetAxis` enum: `ToolInvocations`, `FileMutations`, `SpawnedAgents`, `WallClockSecs`, `Tokens`, `CostUsd`
  - `RunBudgetCeiling` struct: `{ axis: RunBudgetAxis, per_run: Option<BudgetValue>, per_stage: Option<BudgetValue> }` where `BudgetValue` is an enum over integer and float to handle token counts vs dollar amounts
  - `BudgetSource` enum: `Declared`, `PlatformDefault`
  - `AdmittedBudget` struct: `{ axis: RunBudgetAxis, ceiling_per_run: BudgetValue, ceiling_per_stage: Option<BudgetValue>, source: BudgetSource }`, one entry per axis (populated with defaults when absent from the envelope)
  - `PLATFORM_DEFAULT_BUDGETS: &[AdmittedBudget]` compile-time platform defaults (values are policy, picked at implementation; must be non-zero and conservative)
  - `fn apply_defaults(declared: &[RunBudgetCeiling]) -> Vec<AdmittedBudget>` merges declared ceilings with defaults; any declared value tightens but cannot exceed the platform default (the spec 047 five-tier direction: policy tightens, never loosens)

- `crates/factory-contracts/src/governance_envelope.rs`
  - Add `budgets: Vec<RunBudgetCeiling>` field to `GovernanceEnvelope` (optional/default empty, so existing instances deserialize cleanly)
  - Bump `GOVERNANCE_ENVELOPE_SCHEMA_VERSION` from `"1.0.0"` to `"1.1.0"`
  - Update `version_const_anchor` test

- `crates/factory-contracts/src/lib.rs`
  - `pub mod run_budget;` and re-exports

- `standards/schemas/factory/governance-envelope.schema.yaml`
  - Bump header comment version from `1.0.0` to `1.1.0`
  - Add `budgets:` section after `overrides:`:

```yaml
# BUDGETS -- run blast-radius ceilings  [ASI08 m6/m7; spec 202 FR-001]
#
# Optional. When absent, platform defaults apply fail-closed. Per-axis
# ceilings at two scopes only (predicate-shaped, never topology-shaped, P-2):
#   per_run:   ceiling on the whole run
#   per_stage: uniform ceiling no single stage may exceed
# Absent budget does NOT mean unlimited (AC-3).
budgets:
  - axis: string            # "tool_invocations" | "file_mutations" |
                            # "spawned_agents" | "wall_clock_secs" |
                            # "tokens" | "cost_usd"                 [ASI08 m6]
    per_run: number         # Optional ceiling for the whole run
    per_stage: number       # Optional uniform per-stage ceiling
```

**Hook-in**: The `apply_defaults` function is called by the platform admission path (in `factory-contracts`); the admitted budget list is what the engine receives alongside the envelope.

**Frontmatter additions to `spec.md`** (same PR):
- Add `establishes:` entry for `crates/factory-contracts/src/run_budget.rs` (kind: file)
- Add `establishes:` entry for `crates/factory-contracts/src/governance_envelope.rs` (kind: file)
- The `extends: 198` edge already covers `standards/schemas/factory/governance-envelope.schema.yaml`

**Tests**:
- `run_budget.rs` unit tests: `apply_defaults` with no declared budgets produces all six axes with `source: PlatformDefault`; a declared `tokens` ceiling tighter than the default passes; a declared ceiling looser than the platform default is clamped (AC-3)
- `governance_envelope.rs` tests: `version_const_anchor` updated to `"1.1.0"`; round-trip YAML with `budgets:` section; `budgets: []` (absent) round-trips cleanly
- Existing `envelope_yaml_round_trips` must still pass

**Lockstep verification**:
- `make ci-schema-parity` must pass (schema-parity checker walks the YAML schema and the Rust twin)
- `make factory-schema-lockstep` must pass or gracefully skip if no factory checkout is present (Tier B: OAP-unilateral addition)

**Schema-parity constraint (AC-7)**: The `GOVERNANCE_ENVELOPE_SCHEMA_VERSION` const and the YAML schema version header must change atomically in the same commit. The schema-parity checker (spec 125/191) will fail if they diverge. Both files are in this slice.

---

### Slice B: FR-002 Run-level meter + budget PreStepGate + AC-6 dead-stub discharge (AC-1, AC-5, AC-6)

**Depends on**: Slice A (needs `AdmittedBudget`, `RunBudgetAxis`).

**FR/AC coverage**: FR-002 fully, AC-1 (budget-exceed pauses the run), AC-5 (no silent resume), AC-6 (dead stubs retired).

**Files created/edited**:

- `crates/orchestrator/src/budget_gate.rs` (new)

  `RunBudgetMeter`:
  - `struct RunBudgetMeter { ceilings: Vec<AdmittedBudget>, axes: HashMap<RunBudgetAxis, f64>, run_start: Instant }`
  - `fn record_step(&mut self, step: &StepSummaryEntry)` adds `tokens_used`, `cost_usd`, `duration_ms` (converted to secs), `retry_count` (tool invocations proxy for now; `file_mutations` recorded as 0 until executor surfaces them; `spawned_agents` is +1 per dispatched step)
  - `fn check(&self) -> Option<BudgetBreach>` returns `Some(BudgetBreach { axis, ceiling, actual })` for the first exceeded ceiling; `None` if all clear

  `BudgetGate` implementing `PreStepGate`:
  - `struct BudgetGate { meter: Arc<Mutex<RunBudgetMeter>> }`
  - `async fn before_step(&self, step_id: &str) -> Result<(), String>` locks meter, calls `check()`, formats the attributable error message (`"budget ceiling exceeded: axis={axis} ceiling={ceiling} actual={actual} step={step_id}"`) and returns `Err` if breached
  - AC-1: the `Err` propagates to `dispatch_manifest`'s `pre_step` branch, which already fails the step closed, cascades `Skipped` to dependents, and persists the run summary; no new dispatch-loop changes needed
  - AC-5: the meter has no `raise_ceiling` method; raising requires a new `AdmittedBudget` slice (a new envelope or an audited override); the gate is structurally incapable of self-raising

  Gate composition with `GrantRenewalGate`:
  - Add `struct ChainedPreStepGate { gates: Vec<Arc<dyn PreStepGate>> }` implementing `PreStepGate` where `before_step` calls each gate in order and short-circuits on the first `Err`
  - Callers construct `ChainedPreStepGate { gates: vec![grant_renewal_gate, budget_gate] }`; `DispatchOptions.pre_step` stays `Option<Arc<dyn PreStepGate>>` (no breaking change to the dispatch API)

- `crates/orchestrator/src/lib.rs`
  - Add `pub mod budget_gate;`
  - Re-export `BudgetGate`, `RunBudgetMeter`, `ChainedPreStepGate`, `BudgetBreach`
  - After step completion in `dispatch_manifest`, call `meter.lock().unwrap().record_step(&step_summary_entry)` (the meter is shared via `Arc<Mutex<RunBudgetMeter>>` threaded through `DispatchOptions`)

  `DispatchOptions` addition:
  - `pub budget_meter: Option<Arc<Mutex<RunBudgetMeter>>>`: when `Some`, the dispatcher records the completed step into the meter after each successful dispatch (the gate fires at step entry before the next step; the first-step check against pre-run defaults fires on step-0 entry)

- `crates/factory-engine/src/engine.rs` and `crates/factory-engine/src/bin/factory_run.rs`
  - Remove `max_total_tokens: Option<u64>` from `FactoryEngineConfig` (AC-6: retire the dead stub). Update the six call sites that set `max_total_tokens: None` to not pass the field.
  - The engine wires `AdmittedBudget` from the admitted envelope into a `RunBudgetMeter` and sets it into `DispatchOptions.budget_meter`

- `crates/factory-contracts/src/pipeline_state.rs`
  - Remove `total_tokens: u64` and `add_tokens()` from `FactoryPipelineState` (AC-6: no two independent token accumulators). Update callers that call `add_tokens()` to use the meter's `tokens` axis instead. Update `total_tokens: 0` field in `new()` and all test fixtures.

**Frontmatter additions to `spec.md`** (same PR):
- Add `establishes:` entry for `crates/orchestrator/src/budget_gate.rs` (kind: file)
- Existing `refines:` entry for `crates/orchestrator/src/lib.rs` covers that file

**Tests** (in `crates/orchestrator/tests/` or inline):
- AC-1: construct a `RunBudgetMeter` with a `tokens` ceiling of 100; call `record_step` with a step showing `tokens_used: 101`; assert `check()` returns `Some(BudgetBreach { axis: Tokens, .. })`; wire into a `BudgetGate` and confirm `before_step` returns `Err` with axis/ceiling/actual in the message
- AC-5: confirm `RunBudgetMeter` has no public method that raises a ceiling; `check()` uses the immutable ceiling from construction
- AC-6: confirm `FactoryEngineConfig` no longer has `max_total_tokens`; confirm `FactoryPipelineState` no longer has `total_tokens`
- `ChainedPreStepGate`: first gate `Err` short-circuits; second gate not called; both clear returns `Ok`

---

### Slice C: FR-005 Certificate binding of consumption actuals (AC-4)

**Depends on**: Slice A (needs `AdmittedBudget`), Slice B (needs the meter's output shape for the `budget_consumption` record).

**FR/AC coverage**: FR-005 fully, AC-4 (certificate carries actuals; tampering fails verify).

**Files edited**:

- `crates/factory-engine/src/governance_certificate.rs`
  - Bump `CERTIFICATE_VERSION` from `"1.6.0"` to `"1.7.0"` (see drift finding above)
  - Add `BudgetAxisRecord` struct: `{ axis: String, ceiling: f64, actual: f64, source: String, breached: bool }` (using `String` for axis and source to keep the JSON schema human-readable and version-independent)
  - Add `pub budget_consumption: Option<Vec<BudgetAxisRecord>>` to `GovernanceCertificate` with `#[serde(default, skip_serializing_if = "Option::is_none")]` so pre-1.7.0 fixtures stay byte-identical
  - Add `CertificateBuilder::budget_consumption(mut self, record: Vec<BudgetAxisRecord>) -> Self` builder method
  - Update `verify_certificate`:
    - New check: if `budget_consumption` is `Some`, verify that every admitted axis is present in the record (axis completeness); if any axis is missing, push an error naming the axis
    - New check: for each `BudgetAxisRecord`, verify `actual >= 0.0` and `ceiling > 0.0` (internal consistency); verify `breached == (actual > ceiling)` (consistency between the flag and the numbers); tampering with `actual` to hide a breach will either flip the `breached` flag (caught by the consistency check) or leave it `true` (the certificate accurately records the breach, which is acceptable)
    - Both checks add axis-specific diagnostics (e.g. `"budget_consumption: axis 'tokens' actual -1 is not valid"`)
    - Existing signature check catches any other tampering

  Update `CERTIFICATE_VERSION` docstring:
  - Add `/// 1.7.0 (spec 202 FR-005) added the optional 'budgetConsumption' record inside the signed payload.`

**Frontmatter additions to `spec.md`** (same PR):
- Add `establishes:` entry for `crates/factory-engine/src/governance_certificate.rs` (kind: file); currently only a `references:` role

**Tests**:
- AC-4 happy path: build a cert with `budget_consumption` record; verify round-trip JSON; call `verify_certificate` and confirm `valid: true`
- AC-4 tamper: build a cert; tamper with `actual` on one axis (increase it above `ceiling` while setting `breached: false`); call `verify_certificate`; confirm the `breached` consistency check fires with an axis-specific diagnostic
- AC-4 tamper via signature: tamper with `actual` directly in the JSON bytes; the Ed25519 signature check (which runs first) catches it before the budget checks are reached; the existing signature test pattern applies
- Pre-1.7.0 fixture: a cert with no `budget_consumption` must still verify cleanly (`absent = None`; the axis-presence check is skipped when the field is absent)
- `version_const_anchor` test updated to `"1.7.0"`

**Second lockstep constraint**: `CERTIFICATE_VERSION` is in `factory-engine` (not `factory-contracts`). The spec 212 factory-schema-lockstep checks `standards/schemas/factory/` against the factory source repo's `contract/schemas/`. The certificate schema is not currently a checked schema in that lane (the governance-certificate is an OAP-internal artifact, not a factory-filed contract). Confirm with `make factory-schema-lockstep`; if the cert schema is absent from the factory's `contract/schemas/`, no lockstep action is required for Slice C.

---

### Slice D: FR-003(b) Oscillation detector (AC-2)

**Depends on**: Slice B (budget gate and `DispatchOptions.budget_meter` exist; the oscillation detector reports through the same pause channel).

**FR/AC coverage**: FR-003(b) fully, AC-2 (seeded feedback-loop fixture trips the detector).

**Files created/edited**:

- `crates/orchestrator/src/budget_gate.rs` (extend)
  - Add `OscillationDetector` struct:
    - Wraps `CircuitBreakerState` from `circuit_breaker.rs` (now wired for the first time)
    - Config: `oscillation_threshold: u32` from the envelope's `budgets` section (see design note)
    - `fn record_step_outcome(&mut self, step_id: &str, success: bool)` calls `cb.record_failure()` on a failed step, `cb.record_success()` on a successful step
    - `fn check_oscillation(&self) -> Option<OscillationBreach>` returns `Some` if the circuit breaker is tripped
  - `OscillationBreach` struct: `{ consecutive_failures: u32, threshold: u32 }`
  - Wire `OscillationDetector` into `BudgetGate.before_step` alongside the axis check; breach returns `Err` with the same pause semantics
  - Add `oscillation_state: OscillationDetector` to `RunBudgetMeter` or as a peer field in `BudgetGate`

- `crates/orchestrator/src/lib.rs`
  - After step completion, call `meter.record_step_outcome(step_id, success)` on the oscillation detector

**Design note on threshold declaration**: FR-003(b) says thresholds are "declared as `budgets:` fields, never hardcoded." The simplest fit is a new `budgets.axis = "consecutive_failures"` entry with `per_run` holding the trip threshold. This avoids inventing a new schema concept. The `OscillationDetector` reads `per_run` from the `AdmittedBudget` for this axis.

**Tests** (AC-2):
- Construct a fixture `WorkflowManifest` with two steps. Directly call `record_step_outcome` in alternation (fail, fail, fail) until the oscillation detector trips. Assert `check_oscillation()` returns `Some` before the run's `wall_clock_secs` budget would be exceeded.
- Full integration: set `oscillation_threshold: 2`; simulate two consecutive step failures; assert `before_step` returns `Err` on the third step call.

---

### Slice E: FR-003(a) Intent-hash deduplication detector (independent)

**Depends on**: Slice A (needs `RunBudgetAxis` for the threshold declaration pattern). Does NOT depend on Slice B or C.

**FR/AC coverage**: FR-003(a).

**Files created/edited**:

- `crates/orchestrator/src/budget_gate.rs` (extend)
  - `StepSignatureCache`:
    - `fn step_signature(goal_id: &str, instruction: &str) -> String`: SHA-256 of `goal_id + normalize(instruction)` where `normalize` collapses runs of whitespace and lowercases (the normalization rule fixed at implementation per spec)
    - `struct StepSignatureCache { seen: HashMap<String, u32>, threshold: u32 }`
    - `fn record(&mut self, goal_id: &str, instruction: &str) -> Option<IntentRepeatBreach>`: inserts/increments count; returns `Some(IntentRepeatBreach)` when count exceeds threshold
  - `IntentRepeatBreach` struct: `{ signature: String, count: u32, threshold: u32 }`
  - Wire into `BudgetGate.before_step` or into the step-dispatch path via `RunBudgetMeter`

- `crates/orchestrator/src/lib.rs`
  - Before dispatching a step, record the step's instruction in the cache

**Tests**:
- Two steps with identical instructions and the same `goal_id`: `record` on step 2 returns `Some` when threshold is 1.
- Step id excluded: two steps with identical instructions but different step ids collide (step id is excluded from the hash per spec).
- Normalization: `"  foo  bar  "` and `"foo bar"` produce the same signature.

---

### Slice F: FR-003(c) Platform queue-storm gate (independent)

**Depends on**: Slice A is helpful for the ceiling type pattern, but this slice can land before Slice A; it reads a `STAGECRAFT_FACTORY_MAX_RUNS_IN_FLIGHT` env var or a platform config value until the envelope carries the threshold.

**FR/AC coverage**: FR-003(c).

**Files edited**:

- `platform/services/stagecraft/api/factory/runs.ts`
  - In `reserveRunCore`, after the admission check and before the INSERT, add a runs-in-flight count gate:

```typescript
const [{ count: inFlight }] = await db
  .select({ count: sql<number>`count(*)::int` })
  .from(factoryRuns)
  .where(
    and(
      eq(factoryRuns.orgId, auth.orgId),
      inArray(factoryRuns.status, ["queued", "running"]),
    ),
  );
const maxInFlight = runsInFlightCeiling();
if (inFlight >= maxInFlight) {
  throw APIError.resourceExhausted(
    `org has ${inFlight} runs in flight (ceiling: ${maxInFlight}); ` +
    `wait for a run to complete or raise the ceiling via org policy (spec 202 FR-003c)`
  );
}
```

  - `runsInFlightCeiling()` reads `STAGECRAFT_FACTORY_MAX_RUNS_IN_FLIGHT` env var (default conservative value, e.g. 5); when the envelope carries the threshold (post-Slice A), admission reads it from the admitted budget. The function is pure and injectable for tests.
  - The sweeper (`runsScheduler.ts`) is NOT modified (confirmed per spec §Code reality 5).

**Frontmatter additions to `spec.md`** (same PR):
- Promote `platform/services/stagecraft/api/factory/runs.ts` from `references: context` to `refines:` or `establishes:` unit so the coupling gate accepts the edit

**Tests** (`runs.test.ts` or a new `runsQueueGate.test.ts`):
- Insert `maxInFlight` rows in `queued` for the same org; assert `reserveRunCore` throws `resourceExhausted`
- Insert `maxInFlight - 1` rows; assert the reservation succeeds

---

### Slice G: FR-004 Approval-velocity counter (independent)

**Depends on**: Nothing from the above slices. Can land any time after spec 198 is complete.

**FR/AC coverage**: FR-004 (records, does not block).

**Files to edit**: The approval surfaces in the platform (`approvalSummary.ts` and `approvalSummaryEndpoint.ts` in `platform/services/stagecraft/api/factory/`). The gate predicates are filed via the envelope; the velocity counter counts approvals per actor per window and surfaces anomalous velocity.

Since FR-004 is detection-only (records, never blocks) and it composes with spec 201's `ApprovalSummary` evidence rows, this slice requires surveying spec 201's surfaces to understand the exact integration point. That survey is deferred to PR time. Scope: add a velocity accumulator per `(actor_id, org_id, window_start)` at the approval-grant path, surface via a new diagnostic field in `ApprovalSummary`, and add a test fixture with N approvals in a short window asserting the `anomalous_velocity: true` flag appears.

Note: This slice is the least constrained by the dependency tree and the least risky (no blocking gate). It can land last without blocking any other AC.

---

## Dependency order and PR recommendation

```
Slice A (FR-001, AC-3, AC-7) -- LAND FIRST
    |
    +---> Slice B (FR-002, AC-1, AC-5, AC-6)
    |         |
    |         +---> Slice C (FR-005, AC-4)
    |         |
    |         +---> Slice D (FR-003b, AC-2)
    |         |
    |         +---> Slice E (FR-003a)  [can also start from A alone]
    |
    +---> Slice F (FR-003c)  [independent of B/C/D/E]
    |
    +---> Slice G (FR-004)   [fully independent]
```

**First PR: Slice A.** Rationale: it is the dependency root; nothing else ships cleanly without stable `RunBudget*` types. It also carries the mandatory AC-7 schema-parity lockstep, which CI enforces and which is the highest-risk gate to fail silently. Landing Slice A first surfaces any schema-parity tool friction before the larger PRs arrive.

---

## Summary

| Slice | FRs | ACs | First PR? |
|-------|-----|-----|-----------|
| A | FR-001 | AC-3, AC-7 | **Yes** |
| B | FR-002 | AC-1, AC-5, AC-6 | No (needs A) |
| C | FR-005 | AC-4 | No (needs A + B) |
| D | FR-003(b) | AC-2 | No (needs B) |
| E | FR-003(a) | (none stated) | No (needs A) |
| F | FR-003(c) | (none stated) | No (independent) |
| G | FR-004 | (none stated) | No (independent) |

All seven slices together discharge all FRs and all ACs. After Slice C lands, `spec.md` `implementation:` can flip to `complete`.

---

## Risks and open questions

1. **`CERTIFICATE_VERSION` drift (critical)**: The spec says `1.5.0 -> 1.6.0` but 1.6.0 has already landed for spec 218. Slice C must bump to `1.7.0` not `1.6.0`. If spec 218 is not yet in `main` at PR time, confirm with `git log` on the const; otherwise this is a firm correction.

2. **`DispatchOptions.pre_step` is a single `Option`**: Composing `GrantRenewalGate` and `BudgetGate` requires either (a) the `ChainedPreStepGate` wrapper proposed in Slice B, or (b) changing the field to `Vec`. Option (a) is non-breaking and preferred. Option (b) is a larger surface change and needs coupling review against all `DispatchOptions` construction sites.

3. **`file_mutations` axis is zero-valued today**: The executor does not report workspace mutations post-step; the axis records 0 until the executor is updated (a future spec). The axis is admitted and metered (with ceiling `None` or a conservative default meaning "no limit until metered"), but no ceiling breach is ever triggered on it today. This is correct per the spec's bounded-overshoot contract; document in code comments.

4. **`spawned_agents` vs. step count**: The spec says `spawned_agents` is "dispatched step count, including the dynamically generated Phase-2 scaffold manifest, whose step count is bounded at generation time, not only during dispatch." The Phase-2 step count is known at manifest generation (`manifest.steps.len()`). The meter should accept a `record_phase_plan(step_count: usize)` call from the engine at Phase-2 manifest generation to pre-charge the `spawned_agents` axis, not wait for each step to dispatch.

5. **Coupling gate on `runs.ts` (Slice F)**: The current `references: context` role does not grant ownership. Before editing `runs.ts`, the Slice F PR must update `spec.md` to add a `refines:` or `establishes:` edge for that file. Failure to do this will cause the coupling gate to fire in CI.

6. **Featuregraph golden**: The golden at `crates/featuregraph/tests/golden/features_graph.json` already has spec 202's row with `impl_files: []`. After any slice lands implementation files, the `impl_files` list in the golden will be out of date. Run `UPDATE_GOLDEN=1 cargo test -p featuregraph` in each PR's `make pr-prep` step to regenerate the golden before pushing.

7. **CONST-005 risk**: None identified. The implementation adds budget machinery and certificate fields; it does not propose modifying a spec to justify an action that contradicts the spec's design. The certificate version correction (1.6.0 -> 1.7.0) is a factual alignment, not a retroactive justification.
