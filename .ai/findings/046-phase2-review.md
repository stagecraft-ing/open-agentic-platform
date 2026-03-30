# 046 Context Compaction — Phase 2 Review

> Reviewed `TokenBudgetMonitor` implementation in `apps/desktop/src/lib/contextCompaction.ts` (lines 56–97) and test coverage in `contextCompaction.test.ts` (lines 131–189) against `specs/046-context-compaction/spec.md`.
> Reviewer: **claude** | Date: 2026-03-30

## Scope of Review

Phase 2 deliverables per plan:
- `TokenBudgetMonitor` with cumulative input/output token tracking
- `shouldCompact(contextWindowTokens)` with threshold comparison
- Trigger result with observability reason string
- Tests for below/at/above threshold + SC-006 override tests

## FR-001 — Cumulative Token Accounting + Threshold Trigger

| Requirement | Status | Evidence |
|-------------|--------|----------|
| Tracks cumulative prompt + completion tokens | ✅ | `reportUsage()` uses `+=` on `promptTokens`/`completionTokens` fields (lines 66–68) |
| Compares against context window × threshold | ✅ | `shouldCompact()` computes `totalTokens / safeContextWindow` and compares `>= this.threshold` (lines 82–84) |
| Default threshold 75% | ✅ | `DEFAULT_COMPACTION_THRESHOLD = 0.75` (line 1), wired via `config.threshold` in constructor |
| Fires compaction when exceeded | ✅ | Returns `{ shouldCompact: true }` when ratio >= threshold |

**Semantic note:** FR-001 says "when usage *exceeds* the configured threshold" — code uses `>=` (line 84). This means compaction fires at exact equality, not strictly above. This is the safer/more conservative choice: at the boundary, it's better to compact than risk exceeding the window on the next turn. Correct engineering decision.

**Token sanitization:** `sanitizeTokenDelta()` (lines 198–200) floors fractional values and rejects NaN/negative/zero. `sanitizeContextWindow()` (lines 203–205) applies the same guard. Both prevent corruption from malformed API responses. Sound.

**Zero context window guard:** When `safeContextWindow === 0`, `usageRatio` is set to `0` (line 83), which means `shouldCompact` returns `false`. Correct — avoids division by zero and doesn't trigger compaction on nonsensical input.

## SC-006 — Threshold Override Behavior

| Test Case | Threshold | Input | Expected | Actual | Status |
|-----------|-----------|-------|----------|--------|--------|
| 0.5 override | 0.5 | 520/1000 = 0.52 | shouldCompact: true | true | ✅ |
| 0.95 override, below | 0.95 | 900/1000 = 0.90 | shouldCompact: false | false | ✅ |
| 0.95 override, above | 0.95 | 960/1000 = 0.96 | shouldCompact: true | true | ✅ |
| Boundary equality (default) | 0.75 | 750/1000 = 0.75 | shouldCompact: true | true | ✅ |

SC-006 is fully satisfied. Both early (0.5) and late (0.95) trigger points are explicitly tested. The 0.95 test additionally verifies multi-call accumulation (`reportUsage` called twice) correctly crosses the threshold on the second call.

## Observability Output

The `reason` string format: `"usage ratio 0.7500 >= threshold 0.7500 (750/1000 tokens)"`

- **Deterministic:** Uses `.toFixed(4)` for ratios and integer token counts. Same inputs always produce same string. ✅
- **Diagnostic value:** Includes ratio, comparison operator (`>=` vs `<`), threshold, and raw token counts. Sufficient for log/debug tracing. ✅
- **Structured fields:** `CompactionTriggerDecision` also exposes `usageRatio`, `thresholdRatio`, `usedTokens`, `contextWindowTokens` as numeric fields for programmatic consumers. ✅

## R-003 Resolution Check

Phase 1 review flagged R-003: missing positive-path test for `readCompactionThresholdFromEnv`. Phase 2 adds this at test lines 97–101: asserts `readCompactionThresholdFromEnv({ OAP_COMPACTION_THRESHOLD: "0.95" })` returns `0.95`. **Resolved.** ✅

## Findings

### P2-001 — No `reset()` or post-compaction token adjustment (MEDIUM)

**Code:** `TokenBudgetMonitor` accumulates tokens monotonically — there is no way to reset or adjust totals after compaction occurs.

**Problem:** After Phase 5 wires compaction into the runtime, a successful compaction will replace old messages with a smaller `<session_context>` block. The actual token count in the conversation drops, but the monitor's cumulative total still reflects pre-compaction usage. On the next turn, `shouldCompact()` will immediately return `true` again because the ratio is still above threshold — causing a compaction loop.

**Impact:** Phase 5 integration must either:
- (a) Call a `reset(newBaseline)` method that sets totals to the post-compaction token count, or
- (b) Construct a new `TokenBudgetMonitor` after each compaction, seeded with the post-compaction token count.

**Recommendation:** Add a `resetTo(promptTokens: number, completionTokens: number): void` method now. It's trivial and prevents Phase 5 from having to work around the missing API. Alternatively, document the intended reset strategy as a comment for cursor.

### P2-002 — `contextWindowTokens` is a per-call parameter, not constructor config (INFO)

**Code:** `shouldCompact(contextWindowTokens: number)` at line 79.

**Problem:** The context window size is passed on every `shouldCompact()` call rather than being fixed at construction time. This is flexible (supports dynamic context windows, multiple models) but means the integration layer must consistently pass the correct value. Passing different values across calls would cause inconsistent trigger behavior.

**Impact:** Not a bug — just an integration contract that Phase 5 must honor. The current design is reasonable for a monitor that may outlive a single model context.

### P2-003 — `getTotals()` is public but not tested directly (INFO)

**Code:** `getTotals()` at lines 70–77.

**Problem:** The method is exercised indirectly through `shouldCompact()`, but there's no test that calls `getTotals()` directly and asserts the breakdown of `promptTokens` vs `completionTokens` after multiple `reportUsage()` calls. If someone refactors the method, there's no regression gate on the individual breakdown.

**Impact:** Low — `shouldCompact` tests verify the total implicitly. A direct test would be a nice-to-have.

## Test Summary

15 tests across 4 describe blocks. Phase 2 adds 5 tests in the `TokenBudgetMonitor` block:

| Test | Spec Requirement | Status |
|------|-----------------|--------|
| Below threshold → no trigger | FR-001 | ✅ |
| At threshold boundary → trigger | FR-001 (boundary) | ✅ |
| Above threshold → trigger | FR-001 | ✅ |
| 0.5 override → earlier trigger | SC-006 | ✅ |
| 0.95 override → later trigger | SC-006 | ✅ |
| Positive env parse | R-003 fix | ✅ |

All spec-relevant paths for Phase 2 scope are covered.

## Verdict

**Phase 2 approved.** `TokenBudgetMonitor` correctly implements FR-001 cumulative accounting and SC-006 threshold overrides. Observability output is deterministic and diagnostic. 15/15 tests pass.

**One MEDIUM finding:** P2-001 (no reset mechanism) should be addressed before or during Phase 5 integration to prevent compaction loops. Not a Phase 2 blocker — the monitor's API is correct for its current scope — but cursor should be aware when planning Phase 5.

No blockers for Phase 3 (`ProgrammaticCompactor` + XML block builder). Phase 1 findings R-001 (content block type) and R-004 (message ID source) remain open and relevant to Phase 3.

---

**Files created:** `.ai/findings/046-phase2-review.md`
**Baton:** → **cursor** for Phase 3 (`ProgrammaticCompactor`, `<session_context>` XML builder, deterministic extraction). Notes for Phase 3:
- R-001: `content: string` may need widening to `string | ContentBlock[]` for tool_use/tool_result extraction.
- R-004: Verify runtime message IDs exist and are stable.
- P2-001: Plan the reset strategy for post-compaction token accounting.
