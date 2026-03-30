# 047 Phase 4 Review — Coherence Scheduler

> Reviewer: **claude** | Date: 2026-03-30 | Verdict: **APPROVED**

## Scope

Phase 4 deliverables per plan: rolling-window coherence score with decay, privilege assignment with monotonic degradation (Full → Restricted → Read-only → Suspended), human-restore guard path. Requirements: FR-008, SC-007, SC-008.

## Requirement-by-requirement assessment

### FR-008 — Privilege levels from coherence score ✅

Spec defines four bands:
| Level | Spec threshold | Code threshold (`level_from_score`) |
|---|---|---|
| Full | >= 0.8 | `score >= 0.8` ✅ |
| Restricted | 0.5 <= c < 0.8 | `score >= 0.5` ✅ |
| Read-only | 0.2 <= c < 0.5 | `score >= 0.2` ✅ |
| Suspended | < 0.2 | else ✅ |

All four thresholds match the spec verbatim. `PrivilegeLevel` enum with `Full`, `Restricted`, `ReadOnly`, `Suspended` directly maps to the spec names.

**Coherence formula**: Spec says `(actions_aligned / total_actions) * decay_factor` with `lambda = 0.95` over a window of 50. Implementation (`coherence_score()`) computes weighted aligned/total where each position i gets weight `lambda^(n-1-i)` (newest = weight 1.0, oldest = `lambda^(n-1)`). This is a sound weighted-average interpretation of the spec formula — the spec's `decay_factor` is implemented as per-position exponential decay, which is a more rigorous version of the same intent.

**Rolling window**: Default `window_size = 50`, `decay_lambda = 0.95` — both match spec defaults.

### SC-007 — Configurable violation floor ✅

Spec: "degrades privilege from full to restricted after a configurable number of policy-violating actions in the rolling window."

Implementation: `violation_count_for_restricted` (default 3) in `CoherenceSchedulerConfig`. `raw_privilege_level()` calls `violation_count()` (counts `false` entries in window) and applies `max_severity(level, Restricted)` when count >= threshold. This floors the privilege at Restricted regardless of the coherence score — faithful to spec.

Test `sc007_violations_force_at_least_restricted` verifies: 8/10 aligned (score = 0.8) with 2 violations (threshold = 2) → forced to Restricted even though score alone would yield Full.

### SC-008 — Monotonic degradation + human restore ✅

Spec: "Privilege degradation is monotonic — no path from restricted back to full without human intervention."

Implementation:
- `stuck_severity: u8` tracks the worst (highest) severity seen in the session.
- `update_stuck()` called after every `record_action()` — ratchets `stuck_severity` to `max(current, raw.severity())`.
- `effective_privilege_level()` returns `max(raw.severity(), stuck_severity)` — ensures the effective level never improves beyond the worst seen.
- `human_restore()` resets `stuck_severity = 0` and clears the window — the only path back to Full.

Test `sc008_monotonic_no_self_promotion` verifies: after hitting Restricted, flushing the window with 40 aligned actions recovers raw to Full but effective stays Restricted. Only `human_restore()` unlocks Full.

## Integration with evaluate() path

`record_from_policy_outcome(&PolicyOutcome)` bridges the evaluation pipeline to the coherence scheduler — `Allow` maps to aligned, `Deny`/`Degrade` map to non-aligned. This is correct: the spec defines aligned actions as those requiring no policy intervention.

The coherence module is properly exported from `lib.rs`: `pub mod coherence` + `pub use coherence::{CoherenceScheduler, CoherenceSchedulerConfig, PrivilegeLevel}`.

## Validation results

- **cargo test** (policy-kernel): 9/9 ✅ (5 kernel + 4 coherence)
- **cargo check --target wasm32-unknown-unknown** (policy-kernel): clean ✅
- No I/O, no wall clock (`std::time`) — deterministic given action sequence ✅

## Test coverage assessment

| Test | What it covers |
|---|---|
| `sc007_violations_force_at_least_restricted` | SC-007 violation floor overrides score |
| `sc008_monotonic_no_self_promotion` | SC-008 latch + human_restore clearing |
| `threshold_crossing_read_only_and_suspended` | FR-008 ReadOnly (30% aligned) and Suspended (0% aligned) bands |
| `weighted_window_favors_recent` | Decay weighting correctness with lambda=0.5 |

## Findings

### P4-001 — No test for `record_from_policy_outcome` bridge (LOW)

`record_from_policy_outcome()` is the expected integration point between `evaluate()` and the coherence scheduler. It has no dedicated test. All coherence tests use `record_action(bool)` directly. A test that feeds `PolicyOutcome::Deny` / `PolicyOutcome::Allow` through `record_from_policy_outcome` and verifies the score would confirm the bridge works end-to-end.

**Impact**: Low — the method is trivial (`matches!(outcome, Allow)`), but it's the only public API that connects the two subsystems.

### P4-002 — Empty window returns coherence 1.0 (INFO)

When no actions have been recorded, `coherence_score()` returns 1.0 (Full privilege). This is a reasonable default — a fresh session starts with maximum privilege. Noting for spec traceability: the spec doesn't explicitly define the initial state, but this aligns with the "privilege transitions are monotonically degrading within a session" narrative (start at Full, degrade from there).

### P4-003 — `CoherenceSchedulerConfig` configurable but no `.claude/governance.toml` wiring yet (INFO)

The spec mentions "configurable per-repository via `.claude/governance.toml` or equivalent." The config struct exists and is parameterizable, but no config-file loading exists yet. This is expected — runtime integration is Phase 6 scope.

### P4-004 — `Degrade` outcome treated same as `Deny` in coherence (INFO)

`record_from_policy_outcome` treats both `Deny` and `Degrade` as non-aligned. The spec says "actions that required no policy intervention" count as aligned — so both deny and degrade are correctly non-aligned. No issue, noting for completeness.

### P4-005 — P3-003 (allowlist BUILTIN rule ID) still open (INFO)

Phase 3 finding P3-003 flagged that the tool allowlist gate uses `KERNEL:BUILTIN-ALLOWLIST` instead of originating rule IDs. This was marked "fix before Phase 5." Still open — tracking forward.

### P4-006 — No boundary test for exact threshold values (LOW)

The tests verify bands (30% → ReadOnly, 0% → Suspended, 80% → Full/Restricted with SC-007) but don't test exact boundary values like score = 0.2 (should be ReadOnly, not Suspended) or score = 0.5 (should be Restricted, not ReadOnly). `level_from_score` uses `>=` which is correct, but a boundary test would lock this down.

## Summary

Phase 4 is **spec-faithful**. All three requirements (FR-008, SC-007, SC-008) are implemented correctly:
- Four privilege bands with spec-matching thresholds
- Configurable violation floor with `max_severity` enforcement
- Monotonic latch via `stuck_severity` with `human_restore()` as the only escape

6 findings: 0 HIGH, 0 MEDIUM, 2 LOW (P4-001, P4-006), 4 INFO (P4-002..P4-005). No blockers for Phase 5 (proof chains).
