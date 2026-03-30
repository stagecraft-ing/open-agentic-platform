# 046 Context Compaction — phased implementation plan

> **Non-authoritative.** Planning scratch for agent coordination only. Canonical contract remains `specs/046-context-compaction/spec.md`.

## Goal

Implement Feature 046 so long-running sessions compact deterministically into a structured `<session_context>` block before context exhaustion, while preserving recent working turns and active-operation continuity.

## Pre-implementation decisions (F-001 to F-004)

- **F-001 (architecture):** implement compaction in TypeScript under `apps/desktop/src/lib/` because session message history already lives in the TypeScript runtime (`ClaudeCodeSession` / `AgentExecution`) and this avoids unnecessary Tauri IPC serialization cost.
- **F-002 (`<task_summary>` determinism):** construct summary using deterministic templates only:
  - first non-empty user prompt as goal sentence (truncated to fixed char ceiling),
  - fixed progress sentence: `Completed X step(s); Y pending/in-progress.`,
  - no LLM-generated prose in this feature.
- **F-003 (`<git_state>` source):** caller-provided `git_snapshot` object from session runtime integration layer (TypeScript), initially populated via existing git-facing APIs/commands in the app layer; compactor remains pure and does not shell out.
- **F-004 (token source):** `TokenBudgetMonitor` is a passive accumulator fed by per-turn usage emitted by runtime events (`message.usage` / `usage`), via `reportUsage(promptTokens, completionTokens)`.

## Implementation slices

### Phase 1 — Core domain + config wiring (FR-002, FR-003 shape, NF-002)

Deliverables:
- Add `ContextCompactionConfig` (threshold, preserve_recent_turns) with defaults:
  - `threshold = 0.75`
  - `preserve_recent_turns = 4`
- Parse threshold from `OAP_COMPACTION_THRESHOLD` and `compaction.threshold`.
- Validate range `[0.5, 0.95]` and fall back to defaults on invalid values.
- Add deterministic message/history model types used by compactor pipeline.

Validation:
- Unit tests for default config, env/config override precedence, boundary acceptance/rejection.
- Determinism test for config + model serialization.

Status:
- In progress. Added `apps/desktop/src/lib/contextCompaction.ts` with:
  - `ContextCompactionConfig` resolution and threshold validation,
  - deterministic message/history model types,
  - deterministic `stableSerializeHistory()` helper.
- Added `apps/desktop/src/lib/contextCompaction.test.ts` covering defaults, precedence, boundaries, and deterministic serialization.

### Phase 2 — Token budget monitor and trigger logic (FR-001, SC-006)

Deliverables:
- Introduce `TokenBudgetMonitor` that tracks cumulative input/output tokens per turn.
- Add `should_compact(context_window_tokens)` with threshold comparison.
- Expose trigger result and reason for observability.

Validation:
- Unit tests for below/at/above-threshold behavior.
- Explicit tests for threshold overrides (`0.5`, `0.95`) to satisfy SC-006.

### Phase 3 — Programmatic compactor + XML block builder (FR-003, FR-004, NF-003, NF-002)

Deliverables:
- Implement `ProgrammaticCompactor::compact(history, git_snapshot, config)` returning:
  - one `<session_context ...>` block
  - preserved recent/pinned/active-operation messages
- Implement deterministic extraction for:
  - `<task_summary>`
  - `<completed_steps>`
  - `<pending_steps>`
  - `<file_modifications>`
  - `<git_state>`
  - `<key_decisions>`
  - optional `<interruption>`
- Keep format parseable via simple XML/regex rules (no dependency on full XML parser).

Validation:
- Golden tests asserting exact XML output for fixed fixtures.
- Coverage for inclusion/omission of `<interruption>`.
- Parseability test using lightweight regex parser expectations.

### Phase 4 — Preserve-vs-compress and interruption heuristics (FR-005, FR-006, SC-004, SC-003)

Deliverables:
- Preserve verbatim:
  - system prompt
  - most recent N turns
  - pinned messages marked `<!-- pin -->`
  - tool results in active operation
- Implement interruption detection signals:
  - tool call missing result
  - uncommitted file edits without commit
  - final assistant message ends in question/next-step phrasing
  - partial multi-step plan completion
- Gate `<interruption>` insertion on detected signal policy.

Validation:
- Byte-identical preservation tests for recent turns and pinned messages (SC-004).
- Heuristic fixtures for true-positive and false-positive control cases (SC-003).

### Phase 5 — Message rewrite integration + session init hydration (FR-007, FR-008, SC-001, SC-002)

Deliverables:
- Integrate compactor into message rewrite path used by runtime session history.
- Enforce post-compaction token ceiling:
  - compacted payload <= 40% of context window
  - if exceeded, collapse older file-modification entries to summary counts.
- On session init, detect `<session_context>` and elevate it immediately after system prompt.
- Add init hint to prioritize interruption resumption when present.

Validation:
- End-to-end compaction test proving trigger at >75% and output <40% (SC-001).
- Session restart test proving resumed agent can identify completed/pending/files from context block (SC-002).

### Phase 6 — Performance and round-trip behavior (NF-001, SC-005)

Deliverables:
- Add benchmark-style test for <=2s compaction at 100k-token equivalent fixture.
- Add 50-turn round-trip scenario:
  - compact
  - resume
  - continue remaining steps without repeating completed work

Validation:
- Performance assertion for compaction runtime target (NF-001).
- Scenario assertion for no duplicate-completion regressions (SC-005).

## Proposed file touchpoints (initial)

- `apps/desktop/src/lib/contextCompaction.ts` (core domain + config + deterministic serializers)
- `apps/desktop/src/lib/contextCompaction.test.ts` (phase tests)
- `apps/desktop/src-tauri/src/` command/session integration paths that own manifest/session dispatch context
- additional integration points in `apps/desktop/src/components/ClaudeCodeSession.tsx` and `apps/desktop/src/components/AgentExecution.tsx` for Phase 2+ token/trigger wiring

## Risks and guardrails

- Keep compactor deterministic: no LLM summarization path in this feature slice.
- Avoid overfitting interruption heuristics to one workflow shape; use mixed fixtures.
- Maintain strict separation: `.ai/` plan guides implementation, but canonical status/progress belongs under `specs/046-context-compaction/` once implementation starts.
