# 048 Phase 4 Review — Engine Core Orchestration

**Reviewer:** claude
**Date:** 2026-03-30
**Verdict:** Phase 4 approved
**Tests:** 31/31 passing (`pnpm --filter @opc/hookify-rule-engine test`)

## Requirements Satisfied

### FR-002 (priority-ordered evaluation)
`engine.ts:27-89`: `evaluate()` filters rules by event type via `matchesRuleEventType()`, then sorts ascending by `priority` (lowest number = highest priority). Tie-break is deterministic: `id` lexicographic, then `sourcePath` lexicographic (`compareRulesForEvaluationOrder`, lines 6-15). This matches the plan's Phase 4 deliverable and FR-002 ("executes matching rules in priority order").

### FR-003 (block short-circuit)
`engine.ts:67-78`: When `actionResult.terminalDecision === "blocked"`, the loop returns immediately with `allowed: false`, `blockedByRuleId`, and `blockRationale` set from the rule's trimmed rationale. Post-block rules never execute. P3-001 from the Phase 3 review is now resolved — `blockRationale` is populated from `rule.rationale.trim()` (line 71).

### FR-004 + FR-005 (warn + modify flow)
For non-blocking actions, `payload`, `warnings`, and `matchedRuleIds` are threaded from one rule's output to the next rule's input (lines 62-65). This means:
- `warn` rules accumulate warnings across multiple matching rules.
- `modify` rules transform the payload, and subsequent rules see the modified payload for both matcher evaluation (line 40) and condition evaluation (line 44).

### FR-009 (invalid rule resilience)
Condition diagnostics are collected (line 49) and included in the result regardless of match outcome. The engine never throws on condition evaluation — diagnostics propagate non-fatally. Combined with parser-level skip (Phase 1) and action-level skip (Phase 3), the full pipeline is FR-009 compliant.

### NF-003 (testable in isolation)
`evaluate()` is a pure function: `EvaluateInput` → `EvaluationResult`. No file I/O, no global state, no session dependency. The `EvaluateInput` interface (lines 17-20) accepts `rules` and `event` directly.

## P3-001 Resolution

Phase 3 finding P3-001 noted that `blockRationale` was absent from `ActionExecutionResult`. Phase 4 resolves this by sourcing the rationale directly from the rule object at the engine level (`rule.rationale.trim()`, line 71) and setting `blockRationale` on the `EvaluationResult`. The `EvaluationResult` type (types.ts:82) has the optional `blockRationale` field with JSDoc comment. Verified in test at `engine.test.ts:88`.

## Payload Immutability

`engine.ts:32`: `structuredClone(event.payload)` ensures the caller's original event payload is never mutated. Each rule operates on the running copy, and modifications are visible to subsequent rules. This is correct — the spec evaluation flow diagram shows modify rules transforming the payload "before passing it downstream."

## Exports

`engine.ts` exports `evaluate` and `EvaluateInput`. Both re-exported from `index.ts:22-23`. Package subpath `./engine` added to `package.json` exports (line 16).

## Test Coverage

6 tests in `engine.test.ts`:

| Test | Validates |
|------|-----------|
| ascending priority order | FR-002 — lower priority number fires first |
| tie-break by id then sourcePath | Plan tie-break determinism requirement |
| short-circuit on block | FR-003 — post-block rules do not execute |
| payload threading after modify | FR-005 + FR-002 — modified payload visible to later rules |
| matcher skip (non-fatal) | Non-matching tool rules silently skipped |
| condition skip (non-fatal) | Non-matching conditions silently skipped |

## Findings

### P4-001 — No event type filtering test (LOW)
All tests use `PreToolUse` for both rules and events. There is no test where a rule targets a different event type (e.g., `PostToolUse`) and is excluded by `matchesRuleEventType`. This path is exercised by the Phase 2 matcher tests, but an engine-level integration test would strengthen confidence in the filter-before-sort flow.

### P4-002 — No diagnostics content assertion (LOW)
Tests verify `warnings` and `matchedRuleIds` but never assert on `diagnostics` array contents. For example, a test with a rule whose condition references an undefined field would prove that condition diagnostics propagate through `evaluate()`. Currently only tested at the Phase 2 conditions level.

### P4-003 — Empty rules array path (INFO)
When `rules` is `[]`, `evaluate()` returns `allowed: true` with empty arrays. This is correct behavior but has no explicit test. Trivially correct from the loop structure.

### P4-004 — blockRationale includes leading/trailing whitespace handling (INFO)
`rule.rationale.trim()` (line 71) strips whitespace. This is good — Phase 1 noted (P1-002) that the parser preserves a leading newline in the rationale. The trim here normalizes that.

### P4-005 — terminalDecision always set in return paths (INFO)
Both return paths set `terminalDecision` explicitly — `"blocked"` (line 76) and `"allowed"` (line 86). The `EvaluationResult` type marks it as optional (`terminalDecision?: TerminalDecision`) for backward compatibility, but the engine always provides it. Clean.

### P4-006 — No multi-block scenario test (INFO)
If two block rules both match, only the first (by priority) fires due to short-circuit. This is implicitly proven by the short-circuit test, but a test with two block rules at different priorities would make the guarantee explicit.

## No blockers for Phase 5.
