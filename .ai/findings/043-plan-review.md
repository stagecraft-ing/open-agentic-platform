# 043 Agent Organizer — Plan Review

> Reviewer: **claude** · Date: 2026-03-30
> Plan: `.ai/plans/043-agent-organizer-phased-plan.md`
> Spec: `specs/043-agent-organizer/spec.md`

## Requirement coverage matrix

| Requirement | Plan phase | Covered? | Notes |
|-------------|-----------|----------|-------|
| FR-001 | Phase 4 | ✅ | `plan(request, context?)` exposed via `plan_request` in Phase 5; core logic in Phase 4 |
| FR-002 | Phase 1 | ✅ | `score_complexity()` with per-signal breakdown |
| FR-003 | Phase 2 | ✅ | Score ≤25 → `direct` |
| FR-004 | Phase 2 | ✅ | Score >25 → `delegated` with team + workflow |
| FR-005 | Phase 2 | ✅ | Mandatory delegate triggers |
| FR-006 | Phase 2 | ✅ | Mandatory direct triggers |
| FR-007 | Phase 3 | ✅ | 1–5 agents with role + justification |
| FR-008 | Phase 4 | ✅ | Phased workflow with depends_on, success_gate |
| FR-009 | Phase 4 | ✅ | Organizer on Haiku; per-phase model advisory |
| FR-010 | Phase 3 | ✅ | Empty registry → `direct` + warning |
| NF-001 | Phase 6 | ✅ | <3s p95 — documented, measured in verification |
| NF-002 | Phase 1 | ✅ | Deterministic scoring, no LLM |
| NF-003 | Phase 1 | ✅ | JSON serde round-trip tests |
| SC-001 | Phase 2 | ✅ | Simple request fixture |
| SC-002 | Phase 2 | ✅ | Complex request fixture |
| SC-003 | Phase 2 | ✅ | Mandatory delegate override |
| SC-004 | Phase 2 | ✅ | Mandatory direct override |
| SC-005 | Phase 1 | ✅ | Determinism test |
| SC-006 | Phase 6 | ✅ | Haiku latency measurement |
| SC-007 | Phase 3 | ✅ | Empty registry → direct + warning |
| SC-008 | Phase 6 | ✅ | `execution/verification.md` |

**All 10 FR, 3 NF, and 8 SC requirements are covered.** No spec requirement is missing from the plan.

## Phase ordering assessment

Phase 1 (types + scoring) → Phase 2 (dispatch) → Phase 3 (registry) → Phase 4 (Haiku planner) → Phase 5 (Tauri) → Phase 6 (verification)

This ordering is sound:
- Deterministic logic (Phases 1–3) built and testable before any LLM dependency (Phase 4)
- Registry integration (Phase 3) available before Haiku planner needs it (Phase 4)
- Tauri wiring (Phase 5) after all core logic is validated
- Verification (Phase 6) as final sweep

No reordering needed.

## Findings

### F-001 — Mandatory trigger list completeness (MEDIUM)

The spec defines two trigger lists:

**Mandatory DIRECT** (spec §Dispatch protocol):
- "single file edit", "what is", "how do I", "run command", "config change", "explain"
- Plus NEVER delegate list: conversational questions, single-command execution, simple lookups, configuration tweaks

**Mandatory DELEGATE** (spec §Dispatch protocol):
- "implement feature", "refactor", "debug across", "create test suite", "generate docs", "build", "review PR", "analyze architecture", "migrate"
- Plus ALWAYS delegate list: multi-file code gen, cross-module debugging, architecture design/review, full test suite creation, multi-component docs, frontend+backend features, security audits, performance analysis

The plan (Phase 2) says "spec lists + extensible config" but does not enumerate which patterns will be implemented. **Risk:** partial implementation of trigger lists could cause SC-003/SC-004 failures.

**Recommendation:** Phase 2 deliverables should explicitly commit to implementing the full spec trigger lists from both the dispatch protocol diagram AND the NEVER/ALWAYS delegate prose lists. Document the exact substring/pattern set in the plan or require the implementer to extract them all from the spec.

### F-002 — Team assembly uses LLM but Phase 3 is pre-Haiku (LOW)

Phase 3 introduces registry integration + team size rules with "placeholder agent selection (first-N or keyword match) acceptable only behind `OrganizerPlanner` stub." The spec's team assembly rules (§Team assembly rules) require capability matching, role assignment (`lead`/`support`/`reviewer`), justification, and model assignment — all of which need the Haiku planner from Phase 4.

The plan correctly defers real selection to Phase 4 and documents this. This is a sound phasing decision — Phase 3 validates cardinality and registry plumbing; Phase 4 adds intelligence. No action needed, just confirming this is intentional and acceptable.

### F-003 — `OrganizerPlanner` trait fallback path (LOW)

Phase 4 specifies: "if LLM unavailable, degrade to stub team + minimal single-phase workflow with warning." This is good defensive design. However, the fallback needs to produce a valid `ExecutionPlan` that still satisfies NF-003 (valid JSON conforming to schema). Ensure the fallback path includes proper `team` and `workflow` fields (not `None`/absent) when `mode` is `delegated`.

**Recommendation:** Add a test in Phase 4 validation: "LLM unavailable → fallback plan is valid JSON matching ExecutionPlan schema with populated team/workflow."

### F-004 — Existing `crates/agent/` module collision (LOW)

`crates/agent/src/` already contains: `agent.rs`, `canonical.rs`, `executor.rs`, `id.rs`, `lib.rs`, `safety.rs`, `schemas.rs`, `validator.rs`. The plan adds `plan.rs`, `complexity.rs`, `dispatch.rs`, `registry.rs`. No name collisions exist, and the spec's Architecture table explicitly names these files. However, `lib.rs` will need module declarations and re-exports.

**Recommendation:** Phase 1 deliverables should include updating `crates/agent/src/lib.rs` with `mod plan; mod complexity;` and public re-exports. Phase 2 and 3 similarly for `dispatch` and `registry`.

### F-005 — FR-001 `PlanContext` type undefined in plan (LOW)

The spec defines `plan(request: string, context?: PlanContext)` but the plan does not define what `PlanContext` contains. The spec doesn't define it either — it's an opaque optional parameter. The plan's Phase 5 mentions "optional plan context" for the Tauri command.

**Recommendation:** Phase 1 should define a `PlanContext` struct (even if initially empty or containing just `request_id: Option<String>`) so the API signature is complete from the start.

### F-006 — Score band boundary behavior unspecified (LOW)

The spec says "0-25: Simple" and "26-50: Moderate" — the boundaries are clear (≤25 vs >25). The plan Phase 2 says "Map numeric score to band enum per spec table." This is fine, but boundary tests should explicitly cover the exact thresholds: 25→simple, 26→moderate, 50→moderate, 51→complex, 75→complex, 76→highly_complex.

Phase 1 validation already says "Boundary tests for score bands (25/50/75)" — good. Confirming this is covered.

### F-007 — No mention of `packages/agents/orchestration/agent-organizer.md` (INFO)

The spec's Architecture integration table lists `packages/agents/orchestration/agent-organizer.md` as a new agent definition file with a Haiku-targeted prompt. The plan does not mention creating this file. This may be intentional (the prompt template could live inline in the `OrganizerPlanner` implementation), or it may be an oversight.

**Recommendation:** Clarify in Phase 4 whether the organizer prompt template is a standalone `.md` file (per spec table) or embedded in Rust code. If standalone, add it to Phase 4 deliverables.

## Summary

| Severity | Count | IDs |
|----------|-------|-----|
| HIGH | 0 | — |
| MEDIUM | 1 | F-001 |
| LOW | 4 | F-002, F-003, F-004, F-005 |
| INFO | 1 | F-007 |

**Plan approved for Phase 1 start.** All 21 spec requirements mapped to phases. Phase ordering is sound. F-001 (trigger list completeness) should be addressed before Phase 2 implementation begins — either by expanding the plan or by instructing the implementer to extract all triggers from both the dispatch diagram and the NEVER/ALWAYS prose lists. No blockers.
