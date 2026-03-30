# 043 Agent Organizer — Phase 1 Review

> Reviewer: **claude** · Date: 2026-03-30
> Files: `crates/agent/src/plan.rs`, `crates/agent/src/complexity.rs`, `crates/agent/src/lib.rs`
> Spec: `specs/043-agent-organizer/spec.md`
> Plan: `.ai/plans/043-agent-organizer-phased-plan.md`

## Signal table parity (spec § Architecture vs implementation)

| Signal | Spec cap | Impl cap | Measurement match | Key |
|--------|----------|----------|-------------------|-----|
| Prompt length | 0-20 | 0-20 | Linear ≤50→0, ≥2000→20 ✅ | `prompt_length` |
| Action verb count | 0-20 | 0-20 | 15 verbs, 1=5/2=10/3=15/4+=20 ✅ | `action_verbs` |
| Multi-step connectors | 0-20 | 0-20 | 10 patterns, each +5, cap 20 ✅ | `multi_step_connectors` |
| Technology breadth | 0-15 | 0-15 | 8 domains distinct, each +5, cap 15 ✅ | `technology_breadth` |
| Scope indicators | 0-15 | 0-15 | 5 phrases, each +5, cap 15 ✅ | `scope_indicators` |
| File/path references | 0-10 | 0-10 | 1-2=3, 3-5=6, 6+=10 ✅ | `file_path_references` |

**All 6 signals match spec.** Max possible score = 20+20+20+15+15+10 = 100. Caps enforced individually per signal. `BTreeMap` ensures deterministic JSON key order.

### Verb list (complexity.rs:20-26 vs spec)

Spec lists: create, build, implement, refactor, fix, add, remove, update, migrate, deploy, test, review, analyze, design, optimize (15 verbs).
Regex contains all 15. ✅

### Connector list (complexity.rs:28-34 vs spec)

Spec lists: then, after, next, finally, first, also, additionally, followed by, once, before (10 patterns).
Two regexes cover all 10 (multi-word "followed by" in separate pattern). ✅

### Technology domains (complexity.rs:36-43 vs spec)

Spec lists: frontend, backend, database, API, infrastructure, testing, security, CI/CD (8 domains).
Regex contains all 8. `\bapi\b` correctly word-bounded. ✅

### Scope phrases (complexity.rs:45-53 vs spec)

Spec lists: "across all", "entire codebase", "end-to-end", "full-stack", "comprehensive" (5 phrases).
Five separate regexes, one per phrase. ✅

## ExecutionPlan JSON field names vs TypeScript contract

| Spec TypeScript field | Rust type | Serde output | Match |
|-----------------------|-----------|-------------|-------|
| `request_id: string` | `String` | `"request_id": "..."` | ✅ |
| `mode: "direct" \| "delegated"` | `PlanMode` (snake_case) | `"direct"` / `"delegated"` | ✅ |
| `complexity.score: number` | `u8` | `42` | ✅ |
| `complexity.band: "simple" \| ...` | `ComplexityBand` (snake_case) | `"highly_complex"` | ✅ |
| `complexity.signals: Record<string, number>` | `BTreeMap<String, f64>` | `{"key": 5.0}` | ✅ |
| `complexity.mandatory_trigger?: string` | `Option<String>` (skip_serializing_if) | absent when None | ✅ |
| `team?: { agents: [...] }` | `Option<TeamBlock>` (skip_serializing_if) | absent when None | ✅ |
| `team.agents[].agent_id` | `String` | `"agent_id": "..."` | ✅ |
| `team.agents[].role` | `AgentRole` (snake_case) | `"lead"` / `"support"` / `"reviewer"` | ✅ |
| `team.agents[].justification` | `String` | `"justification": "..."` | ✅ |
| `team.agents[].model` | `ModelTier` (snake_case) | `"haiku"` / `"sonnet"` / `"opus"` | ✅ |
| `workflow?.phases[].*` | `WorkflowPhase` | all 8 fields present | ✅ |
| `warnings?: string[]` | `Option<Vec<String>>` (skip_serializing_if) | absent when None | ✅ |

**All fields match.** `skip_serializing_if = "Option::is_none"` correctly omits optional fields when absent, matching TypeScript optional (`?`) semantics.

## Plan review findings resolution

| Finding | Status | Evidence |
|---------|--------|----------|
| F-004 (lib.rs module declarations) | ✅ Resolved | `lib.rs:7` `pub mod complexity;`, `lib.rs:11` `pub mod plan;` |
| F-005 (PlanContext undefined) | ✅ Resolved | `plan.rs:9-14` `PlanContext { request_id: Option<String> }` with serde |

## Test coverage assessment

### Tests in `complexity.rs` (10 tests)

| Test | Requirement | Verdict |
|------|-------------|---------|
| `sc005_identical_prompts_identical_breakdown` | SC-005 | ✅ Multi-signal prompt, two calls, full equality |
| `nf002_scoring_is_pure_no_side_effects` | NF-002 | ✅ Minimal prompt, purity check |
| `cap_prompt_length_max_20` | Signal cap | ✅ 2500-char input, asserts ≤20 |
| `cap_action_verbs_max_20` | Signal cap | ✅ All 15+ verbs, asserts =20 |
| `cap_connectors_max_20` | Signal cap | ✅ Doubled connector set, asserts =20 |
| `cap_technology_breadth_max_15` | Signal cap | ✅ All 8 domains, asserts =15 |
| `cap_scope_max_15` | Signal cap | ✅ All 5 phrases, asserts =15 |
| `cap_file_paths_max_10` | Signal cap | ✅ 6+ paths, asserts =10 |
| `band_boundary_scores_via_constructed_totals` | Band thresholds | ✅ 25→S, 26→M, 50→M, 51→C, 75→C, 76→HC |
| `score_fits_band_from_breakdown` | Consistency | ✅ score_complexity band matches band_from_score |

### Tests in `plan.rs` (2 tests)

| Test | Requirement | Verdict |
|------|-------------|---------|
| `execution_plan_round_trips_json` | NF-003 | ✅ Serialize + deserialize, full equality |
| `band_boundaries_match_spec` | Band thresholds | ✅ Same 6 boundary checks (redundant with complexity, but harmless) |

### Test run result

```
16 passed; 0 failed (crate unit tests)
9 passed; 0 failed (golden/integration tests)
```

All green. ✅

## Findings

### P1-001 — action_verb_signal counts total occurrences, not distinct verbs (INFO)

`action_verb_signal` uses `VERB_RE.find_iter(prompt).count()` which counts total regex matches. If a prompt repeats the same verb ("create X, then create Y"), it counts as 2 verbs. The spec says "Count of imperative verbs" without specifying distinct-vs-total.

Compare: `technology_breadth_signal` explicitly deduplicates via `BTreeSet` because the spec says "Count of *distinct* technology domains."

The difference in wording ("imperative verbs" vs "distinct technology domains") suggests total count is the intended interpretation for verbs. **No action needed** — documenting for spec fidelity.

### P1-002 — No empty-string edge case test (LOW)

`score_complexity("")` should produce score 0 with all signals at 0.0. This is the floor case and works correctly (verified by code inspection: `chars().count()` = 0 ≤ 50 → 0.0, no regex matches → 0.0 for all other signals). A test would prevent future regressions.

**Recommendation:** Add `score_complexity("").score == 0` assertion in Phase 2 or later.

### P1-003 — No boundary test for prompt_length at exact 50/2000 char thresholds (LOW)

`prompt_length_signal` has boundary behavior at exactly 50 chars (→0.0) and 2000 chars (→20.0). These exact boundaries are not tested. The linear interpolation midpoint is implicitly covered by the cap test (2500 chars), but the precise threshold values are not.

**Recommendation:** Consider adding: `prompt_length_signal(50) == 0.0`, `prompt_length_signal(51) > 0.0`, `prompt_length_signal(2000) == 20.0`.

### P1-004 — f64 signals serialize as decimal in JSON (INFO)

`BTreeMap<String, f64>` values like `5.0` serialize as `5.0` in JSON (serde_json preserves the decimal). TypeScript `number` parses both `5` and `5.0` identically, so this is a non-issue for the consumer. Documenting for completeness.

### P1-005 — ComplexityBreakdown vs ComplexityBlock separation is clean (INFO)

`ComplexityBreakdown` (output of `score_complexity`) lacks `mandatory_trigger`. `ComplexityBlock` (in `ExecutionPlan`) adds it via `From<ComplexityBreakdown>` which sets `mandatory_trigger: None`. This cleanly separates Phase 1 (scoring only) from Phase 2 (dispatch with triggers). Good design — no action needed.

### P1-006 — PlanContext is minimal but sufficient (INFO)

`PlanContext { request_id: Option<String> }` satisfies FR-001's `context?: PlanContext` parameter. The struct is extensible for future phases (e.g., adding registry config or model overrides). Correctly derives `Default` for the optional-context case. No action needed.

## Summary

| Severity | Count | IDs |
|----------|-------|-----|
| HIGH | 0 | — |
| MEDIUM | 0 | — |
| LOW | 2 | P1-002, P1-003 |
| INFO | 4 | P1-001, P1-004, P1-005, P1-006 |

**Phase 1 approved.** All 6 spec signals implemented with correct caps and measurements. ExecutionPlan JSON contract matches TypeScript schema field-for-field. NF-002 (determinism) and SC-005 (identical inputs → identical scores) directly tested. NF-003 (valid JSON) verified by serde round-trip. Plan review findings F-004 and F-005 resolved. 12 Phase 1 tests pass, 25 total crate tests pass. No blockers for Phase 2.

**Next:** cursor implements Phase 2 (dispatch.rs) with full F-001 trigger lists from both the dispatch diagram patterns AND the NEVER/ALWAYS prose lists.
