# 046 Phase 6 Review — Performance, Git Snapshot, Context Window

**Reviewer:** claude
**Date:** 2026-03-30
**Commit:** `cef59c1 feat(046): Phase 6 compaction perf, git snapshot, context window`
**Files reviewed:**
- `apps/desktop/src/lib/contextCompaction.ts` (lines 90–173 new: `getContextWindowTokensForModel`, `fetchGitSnapshotFromRepo`)
- `apps/desktop/src/lib/contextCompaction.test.ts` (lines 566–693 new: Phase 6 describe block)
- `apps/desktop/src-tauri/src/commands/git.rs` (lines 253–278 new: `git_last_commit`)
- `apps/desktop/src-tauri/src/types.rs` (line 139: `GitHeadCommit`)
- `apps/desktop/src-tauri/src/bindings.rs` (line 39: registration)
- `apps/desktop/src-tauri/src/lib.rs` (line 424: registration)
- `apps/desktop/src/components/ClaudeCodeSession.tsx` (lines 335–345: integration)

## Phase 6 Deliverable Checklist

| Deliverable | Status | Evidence |
|---|---|---|
| NF-001 benchmark (< 2s for 100k tokens) | ✅ | Test at line 588: 400 × 1000-char messages, `performance.now()` delta asserted `< 2000` |
| SC-005 50-turn round-trip | ✅ | Test at line 611: 25 user + 25 assistant turns, asserts completed/pending steps + file paths preserved in `<session_context>` |
| P5-001 regression test | ✅ | Test at line 661: null entry between valid records, asserts compaction succeeds with both messages present |
| P5-002 resolved — real git snapshot | ✅ | `fetchGitSnapshotFromRepo()` calls Tauri `gitStatus`, `gitDiff`, `gitCurrentBranch`, `gitLastCommit`; fallback on error |
| P5-003 resolved — model-based context window | ✅ | `getContextWindowTokensForModel()` returns 1M for `*1m*` model strings, 200k for Claude family |
| `git_last_commit` Tauri command | ✅ | `git.rs:256–278` via `git2` crate, returns `GitHeadCommit { hash, message }` |
| `ClaudeCodeSession` wired to real snapshot + model window | ✅ | Lines 335–345: `await fetchGitSnapshotFromRepo(...)`, `getContextWindowTokensForModel(lastModel)` |

## Prior Finding Resolutions

| Finding | Status | Notes |
|---|---|---|
| P5-001 (index mismatch normalizeRuntimeMessages vs composeCompactedHistory) | ✅ Resolved | `normalizeRuntimeMessages` now uses `recordIndex` (line 468) that only increments for valid records, matching the filtered-array indexing in `composeCompactedHistory`. Regression test at line 661 confirms. |
| P5-002 (dummy git snapshot) | ✅ Resolved | `fetchGitSnapshotFromRepo()` calls 4 Tauri git commands (status, diff, branch, last_commit). Graceful fallback returns placeholder on error. FR-006(b) interruption signal for uncommitted changes is now live in production. |
| P5-003 (hardcoded 200k context window) | ✅ Resolved | `getContextWindowTokensForModel()` dispatches on model string. `ClaudeCodeSession.tsx:345` passes model from session metrics. Tests at lines 567–576 cover default, known UI models, and 1M-window detection. |

## NF-001 Validation

Test creates 400 messages × 1000 characters each = 400k chars ≈ 100k tokens at the 4-chars/token estimator. Runs `ProgrammaticCompactor.compact()` and asserts wall-clock elapsed < 2000ms. The hot path exercises step extraction, file modification regex, interruption detection, and XML generation — the same code paths that run in production. **NF-001 satisfied.**

## SC-005 Validation

Test generates 50 raw messages (25 turn pairs). Turn 0 contains a structured plan with completed step ("Ship feature A"), pending step ("Ship feature B"), and file references (`src/app/feature.ts`, `packages/api/handler.rs`). After `rewriteSessionHistoryForCompaction()`:
- `result.compacted === true`
- Output contains both step descriptions (completed and pending work preserved)
- Output contains both file paths (modification history preserved)
- Output contains `<session_context` (structured summary emitted)

This is a structural round-trip — a resumed agent reading the compacted output can identify what was done, what remains, and which files were touched without re-reading or re-asking. **SC-005 satisfied** (structural verification; LLM behavioral verification is out of scope per F-007 from Phase 1 review).

## `git_last_commit` Command Review

`git.rs:256–278`: Clean implementation using `git2::Repository::find_commit()`. Returns full OID as string + `commit.summary()` (first line). Uses the same `open_repo()` error-handling pattern as siblings (`git_diff`, `git_status`, `git_current_branch`). Registered in both `bindings.rs:39` and `lib.rs:424`. Type `GitHeadCommit { hash: String, message: String }` is derive-tagged with `Serialize, Deserialize, Type` for Specta type generation. Sound.

## `fetchGitSnapshotFromRepo` Review

Lines 106–173: Async function that dynamically imports Tauri bindings and calls four git commands in sequence:
1. `gitStatus(repoPath)` → counts staged vs unstaged entries
2. `gitDiff(repoPath, null, null)` → HEAD-to-workdir diff stats
3. `gitCurrentBranch(repoPath)` → branch name or "detached"
4. `gitLastCommit(repoPath)` → hash (truncated to 12 chars) + message

Fallback: Returns a placeholder `GitSnapshot` with `branch: "unknown"` and contextual message on any error. The `try/catch` around the entire Tauri call sequence ensures non-Tauri environments (tests, web builds) degrade gracefully.

## `getContextWindowTokensForModel` Review

Lines 90–100: Simple substring-match dispatcher:
- Empty/undefined → 200k (DEFAULT_CONTEXT_WINDOW_TOKENS)
- Contains `"1m"`, `"1-m"`, or `"1000000"` → 1M
- Contains `"opus"`, `"sonnet"`, `"haiku"` → 200k
- Fallback → 200k

Correct for the current Claude model family. The 1M detection covers extended-context variants (`claude-3-5-sonnet-1m`, etc.).

## Findings

### P6-001 — `fetchGitSnapshotFromRepo` has no direct unit test (LOW)

The function performs dynamic `import("./bindings")` and calls Tauri commands, making it difficult to test without an integration harness or mock. It's exercised indirectly through `ClaudeCodeSession.loadSessionHistory()` but has no isolated test for success paths, error fallbacks, or detached-HEAD handling.

**Impact:** Low — the function is an I/O boundary with a comprehensive fallback. The risk is in silent regressions if the Tauri binding API changes.
**Mitigation:** Acceptable as-is. Consider adding a mock-based test if the binding surface changes.

### P6-002 — `getContextWindowTokensForModel` model matching is substring-based (LOW)

`model.includes("1m")` would match unrelated strings (e.g., a hypothetical model containing "1m" in another context). Similarly, `model.includes("opus")` would match non-Claude models named "opus-something".

**Impact:** Low — OAP only sees Claude model strings from its own session metrics. The risk is theoretical.
**Mitigation:** Acceptable. If multi-provider model strings flow through this function, tighten with prefix-anchored patterns.

### P6-003 — SC-005 test is structural, not behavioral (INFO)

The spec's SC-005 wording says "the resumed agent completes the remaining steps without re-doing completed work." The test verifies that the compacted output structurally contains the right information but doesn't run an LLM to prove non-rework. This was flagged in Phase 1 (F-007) as an intentional design choice — structural verification is deterministic and doesn't depend on model behavior.

### P6-004 — `composeCompactedHistory` index coupling remains implicit (INFO)

The P5-001 fix correctly aligns `normalizeRuntimeMessages` (using `recordIndex`) with `composeCompactedHistory` (using filtered-array index). However, this alignment is not enforced by a shared abstraction — if either function's filtering logic is modified independently, the IDs could diverge again. The regression test at line 661 guards against this, which is the right approach.

### P6-005 — `fetchGitSnapshotFromRepo` sequential Tauri calls (INFO)

The four git commands run sequentially (`await` one after another). They could be parallelized with `Promise.all()` for ~2-3x speedup since they're independent read-only operations. Not a correctness issue; the latency is bounded by the git2 crate's speed on local repos (typically < 50ms per call).

### P6-006 — `createSessionContextRuntimeEntry` timestamp non-deterministic (INFO — carried from P5-004)

Still uses `new Date().toISOString()` instead of the parameterized `compactedAt`. Not tested against, so not a correctness issue, but the timestamp will differ from the `compacted_at` attribute in the XML block itself.

## Test Summary

29 tests passing across all phases:
- Config: 7 tests
- Env threshold: 2 tests
- Serialization: 1 test
- TokenBudgetMonitor: 6 tests (including resetTo for P2-001)
- ProgrammaticCompactor: 6 tests (deterministic XML, preservation, active-tool, interruption TP/FP)
- Phase 5 integration: 2 tests (SC-001 trigger+ceiling, SC-002 session restart)
- Phase 6 new: 5 tests
  - `getContextWindowTokensForModel`: 2 tests (defaults, 1M detection)
  - NF-001 benchmark: 1 test (100k tokens < 2s)
  - SC-005 50-turn round-trip: 1 test (structural preservation)
  - P5-001 regression: 1 test (non-record entry alignment)

## Spec Coverage Summary

| Requirement | Status | Phase |
|---|---|---|
| FR-001 (token budget monitor) | ✅ | Phase 2 |
| FR-002 (configurable threshold) | ✅ | Phase 1 |
| FR-003 (ProgrammaticCompactor) | ✅ | Phase 3 |
| FR-004 (session_context sections) | ✅ | Phase 3 |
| FR-005 (preserve policy) | ✅ | Phase 4 |
| FR-006 (interruption detection) | ✅ | Phase 4 + Phase 6 (git snapshot now live) |
| FR-007 (session init elevation) | ✅ | Phase 5 |
| FR-008 (40% ceiling) | ✅ | Phase 5 |
| NF-001 (< 2s for 100k tokens) | ✅ | Phase 6 |
| NF-002 (deterministic) | ✅ | Phase 1 + Phase 3 |
| NF-003 (regex-parseable XML) | ✅ | Phase 3 |
| SC-001 (trigger > 75%, output < 40%) | ✅ | Phase 5 |
| SC-002 (resumable session) | ✅ | Phase 5 |
| SC-003 (interruption detection) | ✅ | Phase 4 |
| SC-004 (byte-identical preservation) | ✅ | Phase 4 |
| SC-005 (50-turn round-trip) | ✅ | Phase 6 |
| SC-006 (threshold config respected) | ✅ | Phase 2 |

## Verdict

**Phase 6 approved.** All prior findings (P5-001, P5-002, P5-003) resolved. NF-001 benchmark and SC-005 round-trip tests are present and meaningful. `git_last_commit` Tauri command is correctly implemented and registered. `fetchGitSnapshotFromRepo` provides real git data with graceful fallback. `getContextWindowTokensForModel` replaces the hardcoded 200k. `ClaudeCodeSession.loadSessionHistory` awaits real git snapshot and uses model-based context window.

All 17 spec requirements (8 FR + 3 NF + 6 SC) are satisfied. 29/29 tests pass. **046 Context Compaction is feature-complete.**

6 findings: P6-001 (no unit test for fetchGitSnapshotFromRepo, LOW), P6-002 (substring model matching, LOW), P6-003–P6-006 (INFO). No blockers.
