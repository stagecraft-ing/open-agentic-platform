# 046 Phase 5 Review — Message Rewrite Integration + Session Init Hydration

**Reviewer:** claude
**Date:** 2026-03-30
**Commit:** `97aa80b feat(046): implement phase 5 compaction rewrite integration`
**Files reviewed:**
- `apps/desktop/src/lib/contextCompaction.ts` (lines 241–627 new)
- `apps/desktop/src/lib/contextCompaction.test.ts` (lines 455–561 new)
- `apps/desktop/src/components/ClaudeCodeSession.tsx` (integration diff)

## Phase 5 Deliverable Checklist

| Deliverable | Status | Evidence |
|---|---|---|
| Integrate compactor into message rewrite path | ✅ | `rewriteSessionHistoryForCompaction()` wired into `ClaudeCodeSession.tsx:334–341` |
| Post-compaction token ceiling ≤ 40% (FR-008) | ✅ | `maxCompactedTokens = Math.floor(contextWindowTokens * 0.4)` with 2-stage cascading collapse |
| Session init elevate `<session_context>` after system prompt (FR-007) | ✅ | `elevateSessionContextForInit()` finds context block and splices after system entry |
| Init hint for interruption resumption | ✅ | `addInterruptionInitHint()` inserts "Prioritize interruption resumption first" when `<interruption detected="true">` present |
| `resetTo()` on `TokenBudgetMonitor` (P2-001) | ✅ | `resetTo(promptTokens, completionTokens = 0)` added and tested (line 194–200 in test) |
| Synthesize deterministic IDs for runtime messages (R-004) | ✅ | `resolveRuntimeMessageId()` returns `runtime-${index}-${stableHash(...)}` when no `id` field exists |
| Parse `<!-- pin -->` from content → `pinned: true` (P4-001) | ✅ | `resolvePinned()` checks both `raw["pinned"]` flag and `containsPinAnnotation()` regex |

## Prior Finding Resolutions

| Finding | Status | Notes |
|---|---|---|
| P2-001 (TokenBudgetMonitor reset) | ✅ Resolved | `resetTo()` called after compaction with `Math.min(estimatedTokens, maxCompactedTokens)` |
| R-004 (runtime message IDs) | ✅ Resolved | FNV-1a hash of stable-sorted record produces deterministic fallback IDs |
| P4-001 (`<!-- pin -->` annotation) | ✅ Resolved | Regex `/<!--\s*pin\s*-->/` scans full message text |

## SC-001 Validation (Trigger at >75%, Output <40%)

Test at line 465: `rawMessages` with usage summing to ~1440 tokens against a 1400-token context window (ratio ≈ 1.03, well above 0.75 threshold). After compaction, the test asserts rewritten payload ≤ 560 tokens (40% of 1400) and confirms `result.compacted === true`. The cascading collapse strategy (collapse file entries → minify XML) ensures the ceiling is met progressively. **SC-001 satisfied.**

## SC-002 Validation (Session Restart Resumption)

Test at line 534: `elevateSessionContextForInit` receives a history with system prompt, an assistant message, and a session_context block containing `<completed_steps>`, `<pending_steps>`, `<file_modifications>`, and `<interruption detected="true">`. The test verifies:
- System prompt remains first
- Hint entry ("Prioritize interruption resumption first") inserted second
- Session context block third, with all structured sections intact

A resumed agent reading this ordering receives: system policy → interruption priority hint → full session state. **SC-002 satisfied.**

## FR-008 Cascading Collapse Strategy

The 40% ceiling is enforced with a 3-tier approach:
1. **Baseline**: full session_context + preserved messages
2. **Stage 1** (`collapseFileModificationSection`): replaces individual `<file>` entries with summary counts (created/modified/deleted)
3. **Stage 2** (`minifySessionContextXml`): strips all whitespace/indentation from the XML

This matches the spec's "collapse older file-modification entries to summary counts" language. After collapse, `monitor.resetTo()` is called with the capped token count to reset the baseline for continued monitoring. Sound design.

## Findings

### P5-001 — Index mismatch between `normalizeRuntimeMessages` and `composeCompactedHistory` (LOW)

`normalizeRuntimeMessages` uses the original `rawMessages` array index for `resolveRuntimeMessageId(raw, index)`. In `composeCompactedHistory`, `normalizedEntries` is filtered (non-records removed) and `.filter((entry, index) => ...)` uses the *filtered* array index. If `rawMessages` contains non-record entries (null, number, etc.), the synthesized IDs will differ between the two functions, causing preserved messages to be silently dropped from the rewritten output.

**Impact:** Low — runtime messages are expected to be objects. But this is a correctness gap.
**Fix:** Track original indices through `normalizedEntries`, or use a shared normalization pass.

### P5-002 — `buildRuntimeGitSnapshot` returns dummy data (MEDIUM)

`ClaudeCodeSession.tsx:387–400` returns a placeholder git snapshot with `branch: "unknown"`, `stagedChanges: 0`, `unstagedChanges: 0`. This means:
- The interruption signal for "uncommitted changes" will **never fire** from the integration path
- The `<git_state>` section in session_context will always contain dummy data
- FR-006(b) ("uncommitted file modifications") is effectively dead in production

**Impact:** Medium — one of four interruption signals is disabled; the git_state section is decorative.
**Fix:** Invoke Tauri backend (`git status --porcelain` / `git log -1` / `git diff --stat`) asynchronously before compaction, or accept this as a known Phase 6 gap.

### P5-003 — `getContextWindowTokens` hardcoded to 200000 (LOW)

`ClaudeCodeSession.tsx:385–387` returns a static 200000. This is a reasonable default for Claude models but means:
- The 40% ceiling is calculated against a fixed assumption, not the actual model's context window
- A session using a smaller-context model would get an incorrectly generous ceiling

**Impact:** Low — 200k is the most common deployment target, and the compaction logic itself is model-agnostic.
**Fix:** Source from model metadata or session configuration. Acceptable as Phase 6 / follow-up.

### P5-004 — `createSessionContextRuntimeEntry` timestamp is non-deterministic (INFO)

Uses `new Date().toISOString()` (line 517). The test doesn't assert on the timestamp so this is fine, but it means session_context entries created during compaction have a wall-clock timestamp rather than the parameterized `compactedAt` used elsewhere.

### P5-005 — Hint idempotency is text-equality based (INFO)

`addInterruptionInitHint` checks for duplicate hints by comparing the full hint text (`normalizeUnknownMessageText(entry) === hintText`). If the hint wording changes between code versions, a stale hint from a prior compaction won't be detected as a duplicate. Acceptable for now since hints are ephemeral.

### P5-006 — `elevateSessionContextForInit` called in non-compaction path (INFO — positive)

When `shouldCompact` is false, the function still calls `elevateSessionContextForInit(input.rawMessages)`, meaning a session that was previously compacted but doesn't need compaction again still gets its context block elevated. This is correct and ensures consistent init behavior regardless of whether compaction triggers in the current cycle.

## Test Summary

24 tests passing across all phases:
- Config: 7 tests
- Env threshold: 2 tests
- Serialization: 1 test
- TokenBudgetMonitor: 6 tests (including `resetTo` for P2-001)
- ProgrammaticCompactor: 6 tests (deterministic XML, preservation, active-tool, interruption)
- Phase 5 integration: 2 tests (SC-001 trigger+ceiling, SC-002 session restart)

## Verdict

**Phase 5 approved.** All 7 deliverables implemented. SC-001 and SC-002 validated by tests. FR-007 and FR-008 satisfied. All prior open findings (P2-001, R-004, P4-001) resolved.

P5-002 (dummy git snapshot) is the most significant gap — it disables one interruption signal and makes git_state decorative. Recommend addressing in Phase 6 alongside P5-003 (context window sourcing).

No blockers for Phase 6 (performance benchmarking + round-trip scenario).
