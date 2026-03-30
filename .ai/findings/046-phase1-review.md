# 046 Context Compaction — Phase 1 + Design Decision Review

> Reviewed `.ai/plans/046-context-compaction-phased-plan.md` (F-001..F-004 resolutions) and Phase 1 code (`apps/desktop/src/lib/contextCompaction.ts` + `contextCompaction.test.ts`) against `specs/046-context-compaction/spec.md`.
> Reviewer: **claude** | Date: 2026-03-30

## F-001..F-004 Resolution Verdict

All four pre-implementation decisions are sound and spec-aligned. No regressions against the original plan review findings.

| Finding | Resolution | Assessment |
|---------|-----------|------------|
| F-001 (architecture) | TypeScript in `apps/desktop/src/lib/` | **Correct.** Session message history lives in TS (`ClaudeCodeSession` / `AgentExecution`). Avoids Tauri IPC serialization overhead that would eat into NF-001's 2s budget. History-local architecture is the right call. |
| F-002 (`<task_summary>` determinism) | First non-empty user prompt (truncated) + fixed progress sentence template | **Correct.** Satisfies NF-002 determinism. Pragmatic trade-off: FR-004 says "natural-language description" but a template that incorporates the user's own words is natural enough. No LLM dependency. |
| F-003 (`<git_state>` source) | Caller-provided `git_snapshot` object; compactor stays pure | **Correct.** Clean separation — compactor is a pure function of its inputs. Integration layer (Phase 5) owns git data gathering. No shell-out in hot path. |
| F-004 (token source) | Passive `TokenBudgetMonitor` fed by `reportUsage(promptTokens, completionTokens)` from runtime events | **Correct.** Standard pattern matching Claude API `usage` fields. Monitor doesn't need to estimate tokens — it receives actuals. |

**No outstanding design ambiguities from the original review remain unresolved.**

## Phase 1 Code Review

### Coverage against Phase 1 deliverables

| Deliverable | Status | Evidence |
|-------------|--------|----------|
| `ContextCompactionConfig` with threshold + preserve_recent_turns | ✅ | Lines 13–17 |
| Defaults: threshold=0.75, preserve_recent_turns=4 | ✅ | Lines 1–2 |
| Parse from `OAP_COMPACTION_THRESHOLD` | ✅ | `readCompactionThresholdFromEnv()` lines 41–49 |
| Parse from `compaction.threshold` config key | ✅ | `resolveContextCompactionConfig()` lines 51–69 |
| Validate range [0.5, 0.95] | ✅ | `isValidCompactionThreshold()` lines 116–118 |
| Fall back to defaults on invalid | ✅ | `chooseFirstValidThreshold()` lines 105–113 |
| Deterministic message/history model types | ✅ | `CompactionMessage`, `CompactionHistory` lines 18–39 |
| Deterministic serialization | ✅ | `stableSerializeHistory()` lines 71–90 |

**All Phase 1 deliverables are present and spec-aligned.**

### Config precedence

Env > config > default — correct per FR-002 intent (environment variable overrides config key). The `chooseFirstValidThreshold()` function evaluates `[envThreshold, configThreshold, DEFAULT]` in that order. Verified by test at line 22–28 of test file.

### Determinism (NF-002)

`stableSerializeHistory()` normalizes optional fields to explicit `null` values and sorts `meta` keys alphabetically. This eliminates the two main sources of JSON non-determinism (undefined-elision and key-ordering). The test at line 98–122 confirms key-order independence.

## Findings

### R-001 — `CompactionMessage.content` is string-only (LOW)

**Code:** `content: string` at line 28.

**Problem:** Claude API messages can contain content block arrays (`[{ type: "text", text: "..." }, { type: "tool_use", ... }]`), not just strings. Phase 3's `ProgrammaticCompactor` will need to iterate content blocks to extract tool calls, decisions, and completed steps. A string-only content field means either (a) the integration layer must pre-flatten blocks to string before compaction, or (b) the type needs broadening in Phase 3.

**Impact:** No Phase 1 breakage. Phase 3 must decide how content blocks are represented — flag for cursor when starting Phase 3.

**Recommendation:** Add a `// Phase 3: may need content block union type for tool_use/tool_result extraction` comment, or widen to `content: string | ContentBlock[]` now with a `ContentBlock` stub type.

### R-002 — `stableSortRecord` is shallow (INFO)

**Code:** Lines 128–137.

**Problem:** `stableSortRecord()` sorts top-level keys of `meta` but does not recursively sort nested objects. If `meta` contains `{ "a": { "z": 1, "b": 2 } }`, the inner `{ "z", "b" }` ordering depends on insertion order.

**Impact:** Negligible in practice — `meta` is unlikely to have deeply nested objects with non-deterministic key ordering. Noting for completeness.

### R-003 — Missing positive-path test for `readCompactionThresholdFromEnv` (INFO)

**Test file** line 82–95 tests missing, empty, invalid, and out-of-range env values — all returning `undefined`. No test asserts that a valid env value (e.g., `"0.8"`) returns `0.8`.

**Impact:** The positive path is indirectly tested via `resolveContextCompactionConfig` (test line 22–28), so coverage exists. A direct test would be clearer.

### R-004 — `CompactionMessage.id` is required — verify runtime has stable IDs (LOW)

**Code:** `id: string` at line 26 (not optional).

**Problem:** The compactor's message model requires an `id` on every message. Phase 5 integration must map runtime session messages to `CompactionMessage`. If the runtime's message objects don't carry stable IDs (e.g., only positional indices), the integration layer will need to synthesize them. This could affect SC-004 (byte-identical preservation) if ID synthesis isn't deterministic.

**Impact:** No Phase 1 breakage. Phase 5 integration must account for this.

### R-005 — Plan still references F-005 and F-006 as unresolved (INFO)

The updated plan resolves F-001..F-004 explicitly in the "Pre-implementation decisions" section but does not address F-005 (PreservePolicy trait for Phase 3) or F-006 (cascading-collapse priority for FR-008). Both are LOW severity from the original review and can be resolved when their respective phases begin, but cursor should be aware.

## Test summary

7 tests across 3 describe blocks. All test spec-relevant behavior:
- Default config: ✅
- Env > config precedence: ✅
- Config-only path: ✅
- Invalid env fallback: ✅
- Boundary acceptance (0.5, 0.95): ✅
- Boundary rejection (<0.5, >0.95): ✅
- preserve_recent_turns normalization: ✅
- Env reader negative cases: ✅
- Deterministic serialization (key-order independence): ✅

No missing critical test paths for Phase 1 scope.

## Verdict

**Phase 1 approved.** All F-001..F-004 resolutions are sound. Code is clean, spec-aligned, and deterministic. No blockers for Phase 2 (TokenBudgetMonitor + trigger logic).

Findings R-001 (content block type) and R-004 (message ID source) should be tracked for Phase 3 and Phase 5 respectively. Everything else is INFO.

---

**Files created:** `.ai/findings/046-phase1-review.md`
**Baton:** → **cursor** for Phase 2 (TokenBudgetMonitor, `should_compact()`, threshold trigger tests). Note R-001 for Phase 3 planning.
