# 043 Phase 4 Review — OrganizerPlanner Trait Hook + UTF-8 Safety

**Reviewer:** claude
**Date:** 2026-03-30
**Scope:** `crates/agent/src/registry.rs`, `crates/agent/src/lib.rs` — diff from `3ec449b..e4b6c90`
**Spec:** `specs/043-agent-organizer/spec.md` (FR-007, FR-008, FR-009)
**Prior review:** `.ai/findings/043-phase3-review.md`

## Verdict: **Phase 4 APPROVED**

All Phase 4 plan deliverables satisfied. P3-001 resolved. `OrganizerPlanner` trait is a clean injection point for a Haiku-backed planner. 36/36 tests pass.

---

## FR Alignment

### FR-007 — Team composition (1–5 agents, role labels, justifications) ✅

The `OrganizerPlanner` trait returns `(TeamBlock, WorkflowBlock, Vec<String>)`. `TeamBlock` contains `Vec<TeamAgent>`, where each `TeamAgent` has `agent_id`, `role: AgentRole` (Lead/Support/Reviewer), `justification: String`, and `model: ModelTier`. The deterministic planner caps teams at 1–5 via `band_team_bounds()`. A Haiku implementation receives the full `AgentRegistrySnapshot` and `ComplexityBreakdown`, giving it all information needed to select 1–5 agents with role labels and justifications per spec.

### FR-008 — Phased workflow (ordered phases with agents, task, dependencies, output, success gate) ✅

`WorkflowPhase` struct has all 7 fields required by the spec: `id`, `name`, `agents`, `task`, `depends_on`, `output`, `success_gate`, plus `model`. The deterministic planner builds 1–3 phases (Analysis → Implementation → Verification) matching the spec's example workflow pattern. A Haiku planner can produce arbitrary phase structures through the same `WorkflowBlock` return type.

### FR-009 — Organizer on Haiku, per-phase model tier advisory ✅

The trait design cleanly separates the organizer decision (which will run on Haiku) from the scoring/dispatch logic (deterministic, no LLM). The `plan_with_planner()` function takes any `P: OrganizerPlanner`, so the desktop/agent layer can inject a Haiku-backed implementation that calls the Haiku model for team assembly. `ModelTier` enum includes `Haiku`/`Sonnet`/`Opus` — consistent with spec's "advisory" model routing. Contract note "Model routing is advisory" is respected (no enforcement in the crate).

---

## P3-001 Resolution ✅

**UTF-8 byte-offset truncation fixed.** Two sites changed:

1. **`build_team_agents` → `truncate_for_justification()`** (registry.rs:175–193): Uses `char_indices()` to find the last valid boundary ≤ `max_bytes`. Defensive fallback returns full string if the first codepoint exceeds limit. Appends `…` (U+2026) only when truncated.

2. **`build_workflow` task_base** (registry.rs:213–228): Same `char_indices()` pattern for request truncation. Both previously used raw `&text[..120]` byte slicing — now safe.

3. **Test coverage** (registry.rs:518–546): `multi_byte_descriptions_and_requests_do_not_panic_or_split_chars` constructs multi-byte emoji strings repeated to exceed 120 bytes, verifies no panic and valid char boundaries.

Fix is correct and complete.

---

## Trait Design Assessment

### Is `OrganizerPlanner` sufficient for a Haiku-backed implementation?

**Yes.** The trait provides all necessary inputs:

| Input | Available | Purpose |
|-------|-----------|---------|
| `prompt` | ✅ `&str` | Raw user request for Haiku to analyze |
| `breakdown` | ✅ `&ComplexityBreakdown` | Score, band, per-signal values — Haiku can use these for calibration |
| `snapshot` | ✅ `&AgentRegistrySnapshot` | Full agent catalog with IDs and descriptions for selection |

The return type `(TeamBlock, WorkflowBlock, Vec<String>)` covers all FR-007/FR-008 output requirements. Warnings vector allows the Haiku planner to surface parse failures, fallback notices, or timeout degradation.

### Proposed Haiku prompt + JSON contract

A production `HaikuOrganizerPlanner` would:

1. Serialize `breakdown` and `snapshot.agents` into a structured prompt section.
2. Ask Haiku to return JSON matching `{ "team": TeamBlock, "workflow": WorkflowBlock }`.
3. Parse with `serde_json::from_str`, catching errors into warnings and falling back to `DeterministicOrganizerPlanner` on failure.

The contract is already defined by `TeamBlock` + `WorkflowBlock` serde schemas — no new types needed. This is a clean boundary.

---

## Architecture Quality

- **Backward compatibility preserved:** `plan()` still works identically (uses `DeterministicOrganizerPlanner`). `build_execution_plan()` unchanged. All existing tests pass without modification.
- **Generic dispatch clean:** `plan_inner<P: OrganizerPlanner>` avoids dynamic dispatch overhead while keeping the trait object option open (trait is object-safe).
- **lib.rs exports updated:** `OrganizerPlanner`, `DeterministicOrganizerPlanner`, `plan_with_planner` all re-exported from crate root.

---

## Findings

### P4-001: No test exercising `plan_with_planner` with a non-default planner (LOW)

All tests use `build_execution_plan` or `plan()`, which both use `DeterministicOrganizerPlanner`. There is no test injecting a custom `OrganizerPlanner` implementation to verify the trait dispatch path. This is LOW because the generic dispatch is trivially correct from the code, but a smoke test with a mock planner would increase confidence.

**Suggested fix:** Add a test with a `struct MockPlanner` implementing `OrganizerPlanner` that returns a known team/workflow, then verify `plan_with_planner` produces those exact values.

### P4-002: `truncate_for_justification` returns full string when first char exceeds max_bytes (INFO)

The `end == 0` fallback (registry.rs:186–189) returns the entire untruncated string. For a single multi-byte codepoint > 120 bytes this is impossible in practice (max UTF-8 char is 4 bytes), so this is purely defensive. Acceptable.

### P4-003: Duplicate truncation logic in `build_team_agents` and `build_workflow` (INFO)

The `char_indices()` truncation pattern appears twice — once as `truncate_for_justification()` (a named function) and once inline in `build_workflow`. Minor inconsistency but no functional impact. Could extract to a shared helper, but not worth a change.

### P4-004: `OrganizerPlanner` trait is not `Send + Sync` bounded (INFO)

The trait has no `Send + Sync` bounds. If used in async Tauri commands, the caller may need `Send + Sync`. Since the trait is generic (not `dyn`), this is handled at the call site — no issue for the crate itself.

### P4-005: `multi_byte_descriptions_and_requests_do_not_panic_or_split_chars` checks `is_char_boundary(len)` (INFO)

The assertion `justification.is_char_boundary(justification.len())` is always true for any valid Rust `String` — `len()` is always a char boundary. A stronger test would assert that the truncated string doesn't end mid-character by checking `justification.len() <= 120 + "Registry selection (deterministic keyword match): ".len() + "…".len()` or similar. However, the test still validates the no-panic property which is the primary concern.

### P4-006: SC-006 planner timeout budget deferred to Phase 6 (INFO)

Per the plan, SC-006 timeout measurement is Phase 6 scope. Noted for tracking.

---

## Test Results

```
27 unit tests + 9 integration tests = 36/36 passing
cargo test --manifest-path crates/agent/Cargo.toml ✅
```

---

## Summary

| Requirement | Status | Evidence |
|-------------|--------|----------|
| FR-007 (team 1–5, roles, justifications) | ✅ | `TeamAgent` struct, `OrganizerPlanner` return type |
| FR-008 (phased workflow with all fields) | ✅ | `WorkflowPhase` 7-field struct, `WorkflowBlock` return |
| FR-009 (Haiku organizer, advisory model) | ✅ | Trait injection point, `ModelTier` enum, contract note respected |
| P3-001 (UTF-8 truncation) | ✅ Fixed | `char_indices()` in both sites + test |
| Backward compat | ✅ | `plan()`, `build_execution_plan()` unchanged |
| Tests | ✅ | 36/36 |

**No blockers.** Recommend handing baton to **claude-opus** for next-slice prioritization. If 043 Phase 5 (Tauri `plan_request` command) is next, the `plan_with_planner` entrypoint is ready for wiring.
