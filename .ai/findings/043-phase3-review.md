# 043 Phase 3 Review — Registry Integration + Team Size Rules

**Reviewer:** claude
**Date:** 2026-03-30
**Scope:** `crates/agent/src/registry.rs`, `crates/agent/src/lib.rs` exports
**Spec reference:** `specs/043-agent-organizer/spec.md` — FR-007, FR-010, SC-007
**Plan reference:** `.ai/plans/043-agent-organizer-phased-plan.md` Phase 3

## Verdict: APPROVED

Phase 3 delivers all planned deliverables. FR-007, FR-010, and SC-007 are satisfied. `plan()` accepts `AgentRegistrySnapshot`, team cardinality follows band rules, and `build_execution_plan` preserves backward compatibility via `legacy_stub()`. 35/35 tests pass (`cargo test --manifest-path crates/agent/Cargo.toml`).

## Requirement trace

| Requirement | Status | Evidence |
|---|---|---|
| **FR-007** (1–5 agents, role label, justification) | **PASS** | `build_team_agents` assigns `AgentRole` (Lead/Support/Reviewer) and justification string per entry. `band_team_bounds` enforces 1–5 range: Simple/Moderate 1–2, Complex 2–3, HighlyComplex 3–5. |
| **FR-010** (empty registry → direct + warning) | **PASS** | `plan()` checks `snapshot.agents.is_empty()` on both `MandatoryOutcome::Delegated` and `MandatoryOutcome::None` paths, returning `fr010_plan` with `PlanMode::Direct` and `FR010_WARNING`. |
| **SC-007** (empty registry → direct + warning) | **PASS** | Tests `fr010_empty_registry_forces_direct_with_warning` and `fr010_empty_registry_high_score_forces_direct` validate both mode and warning presence. |

## Phase 3 plan deliverables trace

| Deliverable | Status | Evidence |
|---|---|---|
| `AgentRegistrySnapshot` + `AgentRegistryEntry` | **DONE** | `registry.rs:18-28` — structs with serde derives, `Default` for empty snapshot |
| `plan()` accepts snapshot + request | **DONE** | `registry.rs:257` — `pub fn plan(prompt, ctx, snapshot)` |
| Deterministic team cardinality from band | **DONE** | `band_team_bounds` (line 53) + `desired_team_count` (line 63) + `pick_team_size` (line 72) |
| Placeholder keyword match selection | **DONE** | `keyword_overlap_score` (line 91) + `select_entries` sorted by `(score desc, id asc)` (line 127). Documented as Phase 4 replacement at line 100. |
| `build_execution_plan` backward compat | **DONE** | `registry.rs:317-319` — delegates to `plan()` with `AgentRegistrySnapshot::legacy_stub()` (6 stub agents) |
| `lib.rs` exports | **DONE** | `lib.rs:20` — re-exports `build_execution_plan`, `plan`, `AgentRegistryEntry`, `AgentRegistrySnapshot` |

## Code quality observations

1. **`plan()` dispatch structure is sound.** Three-way match on `MandatoryOutcome` × empty-registry × score threshold covers all paths. FR-010 degradation inserted at both delegation entry points (mandatory delegate and score-based delegate).

2. **Band-based team sizing is spec-faithful.** Moderate 1–2, Complex 2–3, HighlyComplex 3–5 from `band_team_bounds`. Simple band maps to 1–2 as a defensive fallback (only reachable when a mandatory delegate trigger fires on a low-score prompt — correct behavior).

3. **Deterministic keyword selection is well-structured.** `BTreeSet` for request tokens ensures stable iteration. Sort by `(score desc, id asc)` guarantees determinism for equal-score entries. Phase 4 Haiku planner will replace this with LLM-based selection.

4. **Workflow construction scales with team size.** 1 agent → single Execution phase; 2 agents → Analysis + Implementation; 3+ agents → Analysis + Implementation + Verification. Phase dependencies are correctly wired (`depends_on`).

5. **`legacy_stub()` provides clean Phase 2 parity.** The 6 fixed stub agents preserve deterministic agent IDs for existing callers, avoiding breaking changes.

## Findings

### P3-001: UTF-8 byte-offset truncation can panic (MEDIUM)

**File:** `registry.rs:151-155` and `registry.rs:169-172`

`&e.description[..120]` and `&request[..120]` index by byte offset. If the string contains multi-byte UTF-8 characters and byte 120 falls inside a character sequence, this will panic at runtime.

**Fix:** Use `e.description.char_indices().take_while(|(i, _)| *i < 120)` or `e.description.get(..120).unwrap_or(&e.description)` won't work either — use `floor_char_boundary` (nightly) or iterate `char_indices` to find the last valid boundary ≤ 120.

### P3-002: Mandatory-delegate + empty-registry path untested (LOW)

The `fr010_empty_registry_forces_direct_with_warning` test uses the OAuth prompt which goes through `MandatoryOutcome::None` (the prompt contains "frontend" but not "backend", so no `always_frontend_backend` trigger). The `MandatoryOutcome::Delegated` + empty-registry branch (line 276-278) is exercised by logic but has no dedicated test asserting that a mandatory delegate trigger still degrades to direct when the registry is empty.

### P3-003: Substring keyword matching may over-match (LOW)

`keyword_overlap_score` checks `desc.contains(token)` which is substring-based — "api" matches inside "capital", "test" matches inside "greatest". Acceptable since Phase 4 replaces this with Haiku-based selection, but could produce surprising team rankings in the interim.

### P3-004: Model assignment cycling doesn't match spec guidance (INFO)

`build_team_agents` cycles through `[Sonnet, Sonnet, Sonnet, Opus]` for model tiers. The spec says "review agents get sonnet" but the 4th agent (Support role) gets Opus. Acceptable as Phase 4 Haiku planner will produce spec-faithful model assignments.

### P3-005: `desired_team_count` is not used for team bounds validation (INFO)

`desired_team_count` returns 2/2/3/4 while `band_team_bounds` returns (1,2)/(1,2)/(2,3)/(3,5). The `clamp` in `pick_team_size` reconciles them correctly, but the two functions encode overlapping intent. Consider unifying after Phase 4 stabilizes.

### P3-006: Single-char token filter in `tokenize` (INFO)

`tokenize` filters `s.len() > 1`, so single-letter tokens like "R" (programming language) or "C" won't participate in keyword matching. Negligible impact for team selection, and Phase 4 replaces this.

## Test summary

- 26 unit tests + 9 integration tests = 35/35 pass
- Phase 3 tests: `sc001` through `sc004`, `direct_wins_before_delegate`, `build_project_is_direct`, `empty_string_is_score_only_direct`, `fr010_empty_registry_forces_direct_with_warning`, `fr010_empty_registry_high_score_forces_direct`, `team_size_respects_band_when_catalog_sufficient`
- No regressions in Phase 1/2 tests

## Blockers for Phase 4

**P3-001 (UTF-8 truncation) should be fixed before Phase 4** to avoid runtime panics when real registry data contains non-ASCII descriptions.

No other blockers. Phase 3 approved for Phase 4 start.
