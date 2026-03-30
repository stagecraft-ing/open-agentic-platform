# 046 Context Compaction — Phase 3 Review

> Reviewed `ProgrammaticCompactor` + XML block builder in `apps/desktop/src/lib/contextCompaction.ts` (lines 148–631) and Phase 3 tests in `contextCompaction.test.ts` (lines 193–292) against `specs/046-context-compaction/spec.md`.
> Reviewer: **claude** | Date: 2026-03-30

## Scope of Review

Phase 3 deliverables per plan:
- `ProgrammaticCompactor::compact(history, git_snapshot)` producing `<session_context>` block + preserved/compacted message split
- Deterministic extraction for all 7 FR-004 sections (`task_summary`, `completed_steps`, `pending_steps`, `file_modifications`, `git_state`, `key_decisions`, optional `interruption`)
- Format parseable via simple XML/regex (NF-003)
- Tests for deterministic XML output and interruption inclusion/omission

Additionally, review checks requested in handoff:
- FR-004 section completeness and NF-002/NF-003 expectations
- Preserve-vs-compress boundary assumptions (FR-005/FR-006 readiness for Phase 4)
- R-004 follow-up (runtime message ID stability)
- P2-001 planning note (TokenBudgetMonitor reset strategy)

## FR-004 — Section Completeness

| Section | Status | Evidence |
|---------|--------|----------|
| `<task_summary>` | ✅ | `buildTaskSummary()` (line 414) — first user message truncated to 180 chars + fixed progress template |
| `<completed_steps>` | ✅ | `extractSteps(compactedMessages, "completed")` (line 177) — regex `[x]`/`[X]` checklist extraction |
| `<pending_steps>` | ✅ | `extractSteps(compactedMessages, "pending")` (line 178) — regex `[ ]` checklist extraction |
| `<file_modifications>` | ✅ | `extractFileModifications()` (line 179) — regex file-extension matching, deduplicated, sorted, capped at 20 |
| `<git_state>` | ✅ | Direct serialization from caller-provided `GitSnapshot` (lines 373–382) |
| `<key_decisions>` | ✅ | `extractKeyDecisions()` (line 180) — line-start keyword matching, capped at 10 |
| `<interruption>` | ✅ | `detectInterruption()` (line 181) — conditional; 4 signal types implemented per FR-006 |

All 7 FR-004 sections are present. The XML builder (`buildSessionContextXml`, lines 332–404) produces each section in spec-defined order with proper escaping. Empty sections emit placeholder entries (e.g., `"No completed steps captured yet."`) rather than empty tags, which aids parseability.

## NF-002 — Determinism

| Aspect | Deterministic? | Notes |
|--------|---------------|-------|
| `compacted_at` timestamp | ✅ (test-deterministic) | Uses `new Date(0).toISOString()` — constant epoch. See P3-001 below. |
| `task_summary` | ✅ | Template: `"<user goal>. Completed X step(s); Y pending/in-progress."` — no LLM prose |
| Step extraction | ✅ | Regex patterns, `dedupeStable()` preserves insertion order, capped at 12 |
| File modification extraction | ✅ | Regex + `Map` dedup + `localeCompare` sort. Deterministic for same input. |
| Key decisions | ✅ | Line-start keyword match, `dedupeStable()`, capped at 10 |
| Interruption detection | ✅ | Pure logic on message content + git snapshot state |

Test at lines 248–249 (`outputA === outputB`) confirms same-input determinism. ✅

## NF-003 — Regex-Parseable Format

The output is line-structured XML with:
- Proper `xmlEscape()` (line 623–630) handling `&`, `<`, `>`, `"`, `'`
- No CDATA sections, no mixed content, no namespaces
- Each section is a simple open/close tag pair with children on individual lines

Parseable by regex (e.g., `/<task_summary>\n\s*(.*?)\n\s*<\/task_summary>/s`). ✅

## R-001 Resolution

Phase 1 flagged `content: string` as needing block union for tool_use/tool_result extraction. Phase 3 resolves this:

- `CompactionContentBlock = CompactionContentTextBlock | CompactionContentToolUseBlock | CompactionContentToolResultBlock` (lines 25–46)
- `CompactionMessage.content: string | CompactionContentBlock[]` (line 51)
- `normalizeMessageContent()` (lines 586–600) handles both branches, extracting text from text blocks and serializing tool_use/tool_result into searchable strings

**R-001 resolved.** ✅

## FR-005/FR-006 — Preserve-vs-Compress Boundary (Phase 4 readiness)

Phase 3 already implements the core preserve logic (lines 155–175):

| Preserve category | Implemented? | Notes |
|-------------------|-------------|-------|
| System prompt | ✅ | Line 162: `if (message.role === "system") preserveIds.add(message.id)` |
| Most recent N turns | ✅ | Lines 158–159: `tailStart = messages.length - preserveRecentTurns * 2` |
| Pinned messages | ✅ | Line 163: `if (message.pinned) preserveIds.add(message.id)` |
| Active operation tool results | ⚠️ | Lines 166–170: **overly broad** — see P3-002 |

**Phase 4 implications:**
- The `* 2` turn pair heuristic (line 158) assumes alternating user/assistant. Consecutive assistant messages, interleaved tool messages, or multi-turn tool sequences could cause the preserved window to be misaligned. Phase 4 should refine this to count actual user-initiated turns rather than raw message pairs. (LOW — acceptable for Phase 3.)
- `<!-- pin -->` detection is via `message.pinned` boolean, not by scanning message content for the annotation. The integration layer (Phase 5) must parse `<!-- pin -->` from content and set the boolean. This is a clean separation. ✅
- FR-008 (40% ceiling) is not implemented yet — expected in Phase 5 per plan. No premature implementation needed. ✅

## FR-006 — Interruption Detection

`detectInterruption()` (lines 497–543) implements all 4 spec signals:

| Signal (FR-006) | Implemented? | Evidence |
|-----------------|-------------|----------|
| (a) Tool call without result | ✅ | `findLatestUnresolvedToolCallId()` (lines 545–574) scans backward |
| (b) Uncommitted file modifications | ✅ | `gitSnapshot.stagedChanges + unstagedChanges > 0` (line 503) |
| (c) Final assistant ends with question/next-step | ✅ | Regex `\?\s*$` and `/\bnext step\b/i` (lines 505–507) |
| (d) Incomplete multi-step plan | ✅ | `pendingStepsCount > 0` (line 508) |

**Priority ordering:** The function returns the first matching signal (tool call > uncommitted > question > incomplete plan). This is reasonable — tool calls are the most concrete signal. Spec doesn't mandate priority ordering, so this is an acceptable implementation choice.

**Conservative gating (R-002 mitigation):** The early-return at line 510 requires ALL signals to be absent before returning `null`. This means if *any* signal fires, an interruption is emitted. This matches the spec's intent but could produce false positives (e.g., a clean session with 1 pending step always triggers interruption). Phase 4 should consider requiring 2+ signals for higher-confidence interruption. LOW.

## Findings

### P3-001 — `compacted_at` uses epoch zero, not current time (MEDIUM)

**Code:** Line 335: `new Date(0).toISOString()` → `"1970-01-01T00:00:00.000Z"`

**Problem:** The spec example shows a real timestamp (`2026-03-29T14:30:00Z`). Using epoch zero is deterministic for testing but incorrect for production — the `compacted_at` attribute should reflect when compaction actually occurred. A resumed agent seeing `1970-01-01` would be confused or ignore the timestamp entirely.

**Recommendation:** Accept an optional `compactedAt?: Date` parameter in `compact()` or `buildSessionContextXml()`, defaulting to `new Date()`. Tests can pass `new Date(0)` for determinism. Phase 5 integration passes real time.

### P3-002 — Active tool preservation is overly broad (MEDIUM)

**Code:** Lines 166–170:
```typescript
if (activeToolCallId) {
  for (const message of messages) {
    if (message.tool_call_id === activeToolCallId || message.role === "tool") {
      preserveIds.add(message.id);
    }
  }
}
```

**Problem:** When an unresolved tool call exists, this preserves (a) the message with the matching `tool_call_id` AND (b) **every message with `role === "tool"`** in the entire history. FR-005 says "tool results from the current active operation" — not all tool results ever. In a tool-heavy session with 50+ tool result messages, this could preserve most of the history and defeat compaction.

**Recommendation:** Remove the `|| message.role === "tool"` branch. Only preserve messages directly related to the active tool call:
```typescript
if (message.tool_call_id === activeToolCallId) {
  preserveIds.add(message.id);
}
```
Also preserve the assistant message that initiated the tool call (scan content blocks for matching `tool_use.id`).

### P3-003 — `inferFileAction` operates per-message, not per-mention (LOW)

**Code:** Lines 475–480. `extractFileModifications()` calls `inferFileAction(text)` with the full message text for every file path found in that message.

**Problem:** If a message says "Created `foo.ts` and deleted `bar.ts`", both files receive the same action — whichever pattern matches first in the priority chain (delete > create > modified). The `deduped.set(pathEntry.path, pathEntry)` at line 469 then keeps only the last occurrence, so earlier correct actions can be overwritten by later incorrect ones.

**Impact:** File action accuracy in `<file_modifications>` may be degraded. Not critical — the `description` field is generic ("Referenced during X workflow") anyway. But worth noting for Phase 4 refinement.

### P3-004 — Test coverage below plan expectations (LOW)

**Plan Phase 3 validation says:** "Golden tests asserting exact XML output for fixed fixtures" and "Parseability test using lightweight regex parser expectations."

**Actual tests:** Two tests — one checks `contains` for section tag names (not exact output), one checks interruption omission. No golden-output snapshot test, no regex-parseability test.

**Impact:** NF-002 determinism is tested (same input → same output), but there's no assertion on the *structure* of that output beyond tag presence. A Phase 4 or Phase 6 test should add:
- A golden snapshot for a known fixture (exact string match)
- A regex extraction test proving `<completed_steps>` etc. can be parsed by the patterns described in NF-003

### P3-005 — `extractSteps` only scans compacted messages (INFO)

**Code:** Lines 177–178 pass `compactedMessages` (not full history) to step extraction.

**Problem:** Steps mentioned only in preserved (recent) messages won't appear in `<completed_steps>` or `<pending_steps>`. This is arguably correct — preserved messages are kept verbatim, so the agent sees them directly. But it means the `<session_context>` block alone may undercount progress.

**Impact:** SC-002 says "a compacted session can be resumed... from the `<session_context>` block alone." If a step was completed in a recent (preserved) turn, it won't be in `<completed_steps>`. The agent must scan both the context block and preserved messages. Acceptable given preserved messages are present, but the block isn't self-contained for progress tracking.

### P3-006 — `extractFileModifications` regex matches false positives (INFO)

**Code:** Line 457: `/\b([A-Za-z0-9_.\/-]+\.(ts|tsx|js|jsx|rs|md|json|yaml|yml|toml))\b/g`

**Problem:** Matches "Node.js" as a `.js` file, version strings like "2.0.json", and other non-path text that happens to end with a supported extension. The 20-entry cap and dedup mitigate noise, but false paths dilute signal.

**Impact:** Low in practice — most assistant messages in code sessions reference real file paths. Could be tightened by requiring a `/` in the path or a minimum segment count.

## R-004 Follow-Up — Runtime Message IDs

`CompactionMessage.id` remains required (line 49). Phase 3 doesn't change this contract. The question is whether the runtime (`ClaudeCodeSession` / `AgentExecution`) provides stable string IDs on every message.

**Investigation needed before Phase 5:** Check `apps/desktop/src/components/ClaudeCodeSession.tsx` and the bridge event types for a stable `id` field on messages. If IDs are positional indices or absent, the integration layer must synthesize deterministic IDs (e.g., `${role}-${index}-${hash(content.slice(0,64))}`).

**Status:** Still open. Not a Phase 3 blocker.

## P2-001 Follow-Up — TokenBudgetMonitor Reset Strategy

No changes to `TokenBudgetMonitor` in Phase 3. P2-001 remains open.

**Recommended strategy for Phase 5:** Option (a) from P2-001 — add `resetTo(promptTokens, completionTokens)` that overwrites the cumulative totals to the post-compaction baseline. This is simpler than constructing a new monitor and preserves the instance reference. Trivial implementation:
```typescript
resetTo(promptTokens: number, completionTokens: number): void {
  this.promptTokens = sanitizeTokenDelta(promptTokens);
  this.completionTokens = sanitizeTokenDelta(completionTokens);
}
```

**Status:** Documented. Not a Phase 3 blocker.

## Test Summary

17 tests total (15 from Phase 1+2, 2 new in Phase 3):

| Test | Spec Requirement | Status |
|------|-----------------|--------|
| Deterministic XML with all required sections | FR-004, NF-002 | ✅ |
| Interruption omission when no signals | FR-006 (negative case) | ✅ |

Phase 3 tests confirm section presence and interruption conditionality. Missing: golden output snapshot, regex parseability, multi-signal interruption priority, file modification accuracy. These are gaps against the plan but not blockers.

## Verdict

**Phase 3 approved.** `ProgrammaticCompactor` correctly implements all 7 FR-004 sections with deterministic extraction (NF-002) and regex-parseable XML output (NF-003). R-001 (content block union) is resolved. Interruption detection covers all 4 FR-006 signals.

**Two MEDIUM findings:**
- P3-001: `compacted_at` epoch placeholder must be parameterized by Phase 5.
- P3-002: Active tool preservation is overly broad (`|| message.role === "tool"`) — should be scoped to only the active tool call to prevent compaction bypass in tool-heavy sessions.

**Phase 4 notes for cursor:**
- P3-002 should be fixed early in Phase 4 (preserve-vs-compress policy refinement).
- The `* 2` turn heuristic (line 158) may need refinement for non-alternating message patterns.
- Consider requiring 2+ interruption signals for higher-confidence detection (current: any 1 signal triggers).
- P2-001 (reset strategy) and R-004 (message IDs) remain open for Phase 5.

---

**Files created:** `.ai/findings/046-phase3-review.md`
**Baton:** → **cursor** for Phase 4 (preserve-vs-compress policy + interruption heuristics). Priority fixes: P3-002 (scope active tool preservation), P3-001 (parameterize timestamp). Plan-level validation gaps (P3-004) can be addressed in Phase 4 or Phase 6.
