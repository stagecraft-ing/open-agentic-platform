# 046 Context Compaction — Phase 4 Review

> Reviewed preserve-vs-compress refinements and interruption heuristic tightening in `apps/desktop/src/lib/contextCompaction.ts` and `apps/desktop/src/lib/contextCompaction.test.ts` against `specs/046-context-compaction/spec.md` and `.ai/plans/046-context-compaction-phased-plan.md`.
> Reviewer: **claude** | Date: 2026-03-30

## Scope of Review

Phase 4 deliverables per plan:
- Preserve verbatim: system prompt, most recent N turns, pinned messages, tool results in active operation
- Interruption detection signals: tool call missing result, uncommitted edits, question/next-step phrasing, partial plan
- Gate `<interruption>` insertion on multi-signal policy
- SC-004 byte-identical preservation tests for pinned + recent turns
- SC-003 true-positive and false-positive interruption fixtures

Additionally, review checks requested in handoff:
- P3-001 fully resolved (parameterized `compacted_at`)
- P3-002 fully resolved (scoped active tool preservation)
- Non-alternating turn handling improved vs `* 2` heuristic

## P3-001 Resolution — `compacted_at` Parameterization

**Code:** Line 163–167:
```typescript
compact(
  history: CompactionHistory,
  gitSnapshot: GitSnapshot,
  compactedAt: Date = new Date(),
): ProgrammaticCompactionOutput {
```

**Assessment:** Optional `Date` parameter defaults to `new Date()` for production correctness. Tests pass `new Date(0)` for determinism (line 248). The `buildSessionContextXml` receives the real `compactedAt` value (line 201) and serializes via `.toISOString()` (line 354).

**P3-001 RESOLVED.** ✅

## P3-002 Resolution — Active Tool Preservation Scoping

**Code:** Lines 178–189:
```typescript
const activeToolCallId = findLatestUnresolvedToolCallId(messages);
if (activeToolCallId) {
  for (const message of messages) {
    if (
      message.tool_call_id === activeToolCallId ||
      messageContainsToolUseId(message, activeToolCallId) ||
      messageContainsToolResultId(message, activeToolCallId)
    ) {
      preserveIds.add(message.id);
    }
  }
}
```

**Assessment:** The blanket `|| message.role === "tool"` from Phase 3 is removed. Preservation now scopes to three precise matchers:

| Matcher | Function | Purpose |
|---------|----------|---------|
| `message.tool_call_id === activeToolCallId` | Direct match | Preserves tool result message linked by `tool_call_id` |
| `messageContainsToolUseId()` (lines 618–623) | Content block scan for `tool_use.id` | Preserves assistant message that initiated the call |
| `messageContainsToolResultId()` (lines 625–633) | `tool_call_id` on `role=tool` + content block `tool_result.tool_use_id` | Preserves result message via both legacy and content-block paths |

This matches FR-005 ("tool results from the current active operation") exactly — only messages linked to the unresolved call are preserved.

**P3-002 RESOLVED.** ✅

## Non-Alternating Turn Handling

**Code:** `collectRecentTurnMessageIds()` (lines 594–616):
```typescript
for (let index = messages.length - 1; index >= 0; index -= 1) {
  if (messages[index].role !== "user") continue;
  userTurnsSeen += 1;
  if (userTurnsSeen === preserveRecentTurns) {
    cutoffIndex = index;
    break;
  }
}
```

**Assessment:** The previous `* 2` heuristic assumed strict user/assistant alternation. The new implementation walks backward counting actual `role === "user"` messages, then preserves everything from the Nth user turn onward (inclusive). This correctly handles:

- Multiple consecutive assistant messages within one turn
- Interleaved tool messages between user turns
- Tool prelude messages within a turn

**Verified in test** (lines 294–352): Messages m-5 ("Recent assistant turn A") and m-6 ("Tool prelude within same turn") are both consecutive assistant messages, yet both are preserved because they fall after the 2nd-from-last user turn (m-4). The expected preserved IDs `["m-1", "m-3", "m-4", "m-5", "m-6", "m-7", "m-8"]` confirm this.

**Edge case:** If fewer user turns exist than `preserveRecentTurns`, all messages are preserved (line 611–613). Correct behavior — avoids over-compaction on short histories.

**IMPROVED vs `* 2` heuristic.** ✅

## SC-004 — Byte-Identical Preservation

**Test:** "preserves pinned messages and recent turns byte-identically" (lines 294–352)

| Message | Category | Preserved? | Reason |
|---------|----------|------------|--------|
| m-1 (system) | System prompt | ✅ | `role === "system"` |
| m-2 (user "Older context") | Old turn | ❌ | Before cutoff, not pinned |
| m-3 (assistant, pinned) | Pinned | ✅ | `pinned: true` |
| m-4 (user "Recent A") | Recent turn | ✅ | 2nd-from-last user turn (cutoff) |
| m-5 (assistant "Recent A") | Recent turn | ✅ | After cutoff |
| m-6 (assistant "Tool prelude") | Recent turn | ✅ | After cutoff (non-alternating) |
| m-7 (user "Recent B") | Recent turn | ✅ | After cutoff |
| m-8 (assistant "Recent B") | Recent turn | ✅ | After cutoff |

Content assertions verify actual string values: `preservedMessages[1]?.content === "Pinned and important"` (line 350) and `preservedMessages[5]?.content === "Recent user turn B"` (line 351).

**SC-004 SATISFIED.** ✅

## SC-003 — Interruption Heuristic Fixtures

### True-Positive Test (lines 392–416)

**Setup:** Assistant says "Do the next step now?" + git has 1 staged change.

**Signals fired:**
- `uncommittedChanges`: `stagedChanges: 1` → true (1 signal)
- `asksQuestion || explicitNextStep`: "next step" + "?" both match → true (1 combined signal)
- Total: 2 signals ≥ 2 threshold → interruption emitted

**Assertions:** `interruption !== null`, `<interruption detected="true">` present in XML. ✅

### False-Positive Test (lines 418–442)

**Setup:** Assistant says `"- [ ] update README"` + clean git.

**Signals fired:**
- `hasIncompletePlan`: `pendingStepsCount > 0` (the `[ ]` checkbox) → true (1 signal)
- `uncommittedChanges`: 0 staged + 0 unstaged → false
- `asksQuestion`: no trailing `?` → false
- `explicitNextStep`: no "next step" → false
- `unresolvedToolCallId`: no tool calls → null
- Total: 1 signal < 2 threshold → null (no interruption)

**Assertions:** `interruption === null`, no `<interruption` in XML. ✅

### Signal Counting Design

`countActiveInterruptionSignals()` (lines 657–664) treats `asksQuestion` and `explicitNextStep` as a single combined signal (`if (signals.asksQuestion || signals.explicitNextStep) count += 1`). This prevents a single message like "next step?" from inflating the count to 2 — correct and intentional.

**SC-003 SATISFIED.** ✅

## Active Tool Test (lines 354–390)

**Setup:** Completed tool call (m-1 tool_use "tool-old" + m-2 tool result) followed by unresolved tool call (m-3 tool_use "tool-active") and a recent user turn (m-4).

| Message | Preserved? | Reason |
|---------|------------|--------|
| m-1 (tool_use "tool-old") | ❌ | Completed tool — not active, not recent |
| m-2 (tool result "tool-old") | ❌ | Completed tool result — not active |
| m-3 (tool_use "tool-active") | ✅ | `messageContainsToolUseId(m-3, "tool-active")` matches |
| m-4 (user "continue") | ✅ | Recent turn (1 user turn preserved) |

This directly validates P3-002's resolution: old completed tool messages are compacted, only the active (unresolved) tool call is preserved. ✅

## Findings

### P4-001 — `<!-- pin -->` content annotation not yet wired (LOW)

**Spec:** FR-005 says "any pinned messages explicitly marked with a `<!-- pin -->` annotation."
**Implementation:** Preservation checks `message.pinned` boolean (line 175), not content scanning.
**Phase 3 review noted** this as clean separation — the integration layer (Phase 5) must parse `<!-- pin -->` from content and set the boolean.
**Status:** Deferred to Phase 5. Not a regression. Phase 4 tests use `pinned: true` directly, which is correct for unit-level testing.

### P4-002 — True-positive test does not assert interruption type (LOW)

**Code:** Line 414: `expect(output.interruption).not.toBeNull()` — checks existence only.
**Impact:** The test proves an interruption is emitted under multi-signal conditions, but doesn't verify the priority ordering (uncommittedChanges fires before asksQuestion in `detectInterruption` priority chain). Adding `expect(output.interruption?.operation).toContain("git")` would pin the priority behavior.
**Recommendation:** Add a targeted assertion in Phase 6 when round-trip scenarios exercise more signal combinations.

### P4-003 — Multiple unresolved tool calls: only latest tracked (INFO)

**Code:** `findLatestUnresolvedToolCallId()` (lines 563–592) walks backward and returns the first unresolved call found.
**Impact:** If multiple tool calls are pending simultaneously, only the latest is preserved. FR-005 says "tool results from the current active operation" (singular), so this matches spec. In practice, well-formed conversations shouldn't have multiple unresolved calls — the API enforces call-then-result sequencing.

### P4-004 — `messageContainsToolResultId` role guard (INFO)

**Code:** Line 628: `if (message.tool_call_id === toolUseId && message.role === "tool") return true`
**Note:** The `role === "tool"` guard prevents false matches on non-tool messages that might happen to have a `tool_call_id` field. This is defensive and correct. Content-block path (lines 629–632) has no role guard, which is also correct — `tool_result` blocks can appear in any message role per the Anthropic API.

### P4-005 — P2-001 (TokenBudgetMonitor reset) still open (INFO)

No changes to `TokenBudgetMonitor` in Phase 4. P2-001 remains open for Phase 5 integration, where post-compaction token baseline must be reset. The recommended `resetTo()` approach from Phase 3 review is still valid.

### P4-006 — R-004 (runtime message IDs) still open (INFO)

`CompactionMessage.id` remains required. Phase 5 must verify that the runtime session provides stable string IDs or synthesize deterministic IDs at the integration boundary.

## Test Summary

23 tests total (17 from Phase 1–3, 6 new in Phase 4):

| Test | Spec Requirement | Status |
|------|-----------------|--------|
| Preserves pinned messages and recent turns byte-identically | SC-004 | ✅ |
| Preserves only active-operation tool messages | FR-005, P3-002 resolution | ✅ |
| Emits interruption for true-positive multi-signal case | SC-003 (positive) | ✅ |
| Does not emit interruption for false-positive single-signal case | SC-003 (negative), R-002 | ✅ |
| Deterministic XML with `compactedAt` parameter | P3-001 resolution, NF-002 | ✅ (updated) |
| Interruption omission (no signals) | FR-006 (negative) | ✅ (existing) |

## Verdict

**Phase 4 approved.** All Phase 4 deliverables are spec-faithful:

- **P3-001 resolved:** `compacted_at` parameterized via optional `Date` argument with `new Date()` default.
- **P3-002 resolved:** Active tool preservation scoped to `tool_use.id` / `tool_result.tool_use_id` / `tool_call_id` match — no blanket `role=tool` preservation.
- **SC-004 satisfied:** Byte-identical preservation confirmed for system, pinned, and recent-turn messages with non-alternating pattern coverage.
- **SC-003 satisfied:** 2-signal threshold with true-positive (multi-signal → interruption) and false-positive (single-signal → null) fixtures.
- **Non-alternating turns:** User-turn counting replaces `* 2` heuristic, correctly handling consecutive assistant/tool messages.

**No blockers for Phase 5.** Open items for Phase 5: P2-001 (monitor reset), R-004 (runtime message IDs), P4-001 (`<!-- pin -->` content scanning at integration boundary).

---

**Files created:** `.ai/findings/046-phase4-review.md`
**Baton:** → **cursor** for Phase 5 (message rewrite integration + session init hydration). Priority items: P2-001 (TokenBudgetMonitor reset), R-004 (runtime message ID verification), P4-001 (`<!-- pin -->` content annotation parsing), FR-007 (session init `<session_context>` detection), FR-008 (40% post-compaction ceiling).
