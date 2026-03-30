# 046 Context Compaction — Plan Review

> Reviewed `.ai/plans/046-context-compaction-phased-plan.md` against `specs/046-context-compaction/spec.md`.
> Reviewer: **claude** | Date: 2026-03-30

## Coverage assessment

All spec requirements are mapped to plan phases:

| Requirement | Plan phase | Status |
|---|---|---|
| FR-001 (token budget monitor) | Phase 2 | Covered |
| FR-002 (configurable threshold) | Phase 1 | Covered |
| FR-003 (ProgrammaticCompactor) | Phase 1 (shape) + Phase 3 | Covered |
| FR-004 (session_context sections) | Phase 3 | Covered |
| FR-005 (preserve-vs-compress) | Phase 4 | Covered |
| FR-006 (interruption detection) | Phase 4 | Covered |
| FR-007 (session init hydration) | Phase 5 | Covered |
| FR-008 (40% ceiling) | Phase 5 | Covered |
| NF-001 (2s budget at 100k) | Phase 6 | Covered |
| NF-002 (determinism) | Phase 1 + Phase 3 | Covered |
| NF-003 (regex-parseable XML) | Phase 3 | Covered |
| SC-001 through SC-006 | Phases 2–6 | All covered |

No spec requirement is orphaned from the plan. **Coverage is complete.**

## Findings

### F-001 — Architectural mismatch: Rust crate vs TypeScript session history (HIGH)

**Plan proposes:** Implementation in `crates/orchestrator/src/` (Rust).

**Problem:** The orchestrator crate manages multi-agent workflow manifests (DAG validation, step dispatch, artifact chaining). Context compaction operates on a *single agent's conversation message history* — a fundamentally different domain. More critically, the conversation history lives in the TypeScript layer (`apps/desktop/`), not in Rust. The Tauri backend has no access to the raw message array that the compactor needs to analyze.

**Evidence:** `crates/orchestrator/src/lib.rs` has no conversation message model. Its types are `WorkflowManifest`, `WorkflowStep`, `StepResult`, `RunSummary` — all multi-agent orchestration primitives. The compactor needs `Message { role, content, tool_calls, tool_results, tokens }` types that don't exist in this crate.

**Recommendation:** Either (a) create a new `crates/compactor/` crate dedicated to context compaction, keeping orchestrator focused on workflow dispatch, or (b) implement compaction in TypeScript closer to the session history it operates on. The plan should make an explicit architectural decision here before Phase 1 begins. If Rust is chosen, the message history must be serialized and passed across the Tauri IPC boundary — the plan should account for this cost against NF-001 (2s budget).

### F-002 — Deterministic natural-language extraction undefined (MEDIUM)

**Spec says:** NF-002: "the same message history produces the same `<session_context>` block."
**Plan says:** "no LLM summarization path" and "deterministic extraction."

**Problem:** FR-004 requires `<task_summary>`: a "natural-language description of the overall goal and current progress." How is natural language produced deterministically without an LLM? Template-based extraction (e.g., concatenating the first user message with step counts) is deterministic but may produce low-quality summaries. The plan doesn't specify the extraction strategy.

**Recommendation:** Define the `<task_summary>` construction rule concretely before Phase 3. Options: (a) concatenate the first user message (truncated) with progress counts — deterministic, simple; (b) extract the system prompt's task description if one exists; (c) use the most recent assistant message that contains a plan/summary. Whichever is chosen, document it as a design decision so Phase 3 implementers don't improvise.

### F-003 — Git state extraction strategy missing (MEDIUM)

**Spec requires:** `<git_state>` with branch, staged/unstaged counts, last commit hash+message, diff stats.

**Plan mentions:** `git_snapshot` parameter to `ProgrammaticCompactor::compact()` but never specifies where it comes from.

**Problem:** The codebase has a `crates/gitctx/` crate that could provide git context, but the plan doesn't reference it. If git state is gathered by shelling out to `git`, that has implications for NF-001 (2s budget) — `git diff --stat` on a large repo can be slow. If it's gathered from the existing `gitctx` crate, the plan should say so.

**Evidence:** `crates/gitctx/` exists in the repo structure per CLAUDE.md.

**Recommendation:** Specify whether `git_snapshot` is provided by `gitctx`, by raw git commands, or by the caller (TypeScript layer passing it in). This affects Phase 3's function signature and Phase 5's integration wiring.

### F-004 — Token count sourcing unspecified (MEDIUM)

**Spec says:** FR-001: "tracks cumulative token usage (prompt + completion) against the model's context window size."

**Problem:** The plan's `TokenBudgetMonitor` needs a token count source. Options: (a) model API response headers/fields (accurate but only available after each turn), (b) local tokenizer estimate (fast but approximate), (c) running total passed in by the caller. The plan doesn't specify which. This affects whether the monitor is a passive accumulator or an active counter.

**Recommendation:** Clarify that the monitor is a passive accumulator fed token counts from the model response (the standard pattern). Document the expected input: `report_usage(prompt_tokens: u64, completion_tokens: u64)` called after each turn by the session runtime.

### F-005 — Phase 3/4 ordering creates rework risk (LOW)

**Phase 3** builds `ProgrammaticCompactor::compact()` that returns preserved messages + session_context.
**Phase 4** adds preserve-vs-compress policy and interruption heuristics.

**Problem:** The compactor's `compact()` method must know what to preserve vs compress to produce correct output. Building the compactor in Phase 3 without the policy means either (a) Phase 3 hardcodes a temporary policy that Phase 4 replaces, or (b) Phase 3 builds a compactor that can't actually run correctly until Phase 4.

**Recommendation:** Phase 3 should define a `PreservePolicy` trait/interface with a default implementation (preserve system prompt + last N turns only). Phase 4 then extends this with pinned messages, active-operation tool results, and interruption detection. This avoids rework and keeps Phase 3 testable in isolation.

### F-006 — 40% ceiling overflow strategy incomplete (LOW)

**Spec says:** FR-008: compacted payload must not exceed 40% of context window. R-003: "if exceeded, older file entries are collapsed into a count summary."

**Plan says:** "if exceeded, collapse older file-modification entries to summary counts."

**Problem:** What if the session_context block still exceeds 40% after collapsing file entries? A session with hundreds of key decisions, many completed steps, and a long task summary could exceed the ceiling even without file entries. The plan doesn't address this cascading-collapse scenario.

**Recommendation:** Define a priority-ordered collapse strategy: (1) collapse file entries to counts, (2) truncate completed_steps to last N with a count prefix, (3) trim key_decisions to most recent K, (4) truncate task_summary to a character limit. This ensures FR-008 is always satisfiable.

### F-007 — SC-005 round-trip test feasibility (LOW)

**Plan says:** Phase 6 includes a "50-turn round-trip scenario: compact, resume, continue remaining steps without repeating completed work."

**Problem:** Verifying "without repeating completed work" requires either a real LLM (non-deterministic, expensive, slow) or a mock agent that parses session_context and makes decisions (complex test infrastructure). The plan doesn't specify the test strategy.

**Recommendation:** Implement SC-005 as a structural test: (a) construct a 50-turn fixture, (b) compact it, (c) verify the resumed context contains all completed step IDs, (d) verify no completed step appears in pending_steps. This tests the contract the agent would rely on without needing an actual LLM.

## Pre-implementation corrections (ordered by priority)

1. **Resolve F-001** — Decide: new `crates/compactor/` crate, or TypeScript implementation, or orchestrator with IPC bridge. This determines the entire implementation stack.
2. **Resolve F-002** — Define the `<task_summary>` construction rule (template-based extraction strategy).
3. **Resolve F-003** — Specify git state source (`gitctx` crate, raw git, or caller-provided).
4. **Resolve F-004** — Clarify token count sourcing (passive accumulator from API responses).
5. **Address F-005** — Add `PreservePolicy` trait to Phase 3 deliverables.
6. **Address F-006** — Define cascading-collapse priority order for FR-008.

## Verdict

**Plan covers all spec requirements but has one HIGH-severity architectural gap (F-001) that must be resolved before coding starts.** The remaining findings are design clarifications that should be pinned down during Phase 1 to prevent rework in later phases. No spec contract misses — only implementation-strategy ambiguities.

---

**Files created:** `.ai/findings/046-plan-review.md`
**Baton:** → **cursor** to resolve F-001 (architectural decision) and F-002–F-004 (design clarifications) before starting Phase 1 implementation.
