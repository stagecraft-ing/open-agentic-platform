> **Non-authoritative.** Review notes for baton handoff only. Canonical truth remains in `specs/051-worktree-agents/spec.md` and code/tests.

# 051 Phase 5 Review (claude)

Date: 2026-03-30
Feature: `051-worktree-agents`
Phase: 5 — diff/merge/discard workflow
Verdict: **APPROVED** — no blockers for Phase 6

## Spec coverage

### FR-006 — `getAgentDiff` unified diff + commit summary ✅

`diff.ts:67-78` implements `getAgentDiff(options)` returning `AgentDiffResult { unifiedDiff, commits }`.

- Unified diff via `git diff --no-color <parent>...<agent>` — three-dot syntax correctly computes changes since merge-base, meaning the diff stays accurate even if the parent branch moves forward after the agent was spawned.
- Commit summary via `git log --no-color --pretty=format:%H%x09%s <parent>..<agent>` — two-dot syntax correctly enumerates only commits on the agent branch not reachable from parent.
- `parseCommitSummary` handles tab-separated SHA/subject, robust to trailing newlines.
- Both git commands run in parallel (`Promise.all`) — good.
- `maxBuffer: 16 * 1024 * 1024` (16 MB) adequate for diff output.

### FR-007 — `mergeAgent` with three strategies ✅

`merge.ts:100-158` implements all three merge strategies:

- **fast-forward**: `git merge --ff-only agentBranch` — correct; will fail if parent diverged, which is the right behavior.
- **squash**: `git merge --squash --no-commit` then `git commit` with configurable message — correct; creates exactly one non-merge commit.
- **cherry-pick**: Iterates `cherryPickCommits` with `git cherry-pick` per commit — correct; caller selects which commits to pick.

Preconditions:
- `ensureCleanWorktree` via `git status --porcelain` — refuses merge on dirty tree (DIRTY_WORKTREE error).
- Branch existence validated for both parent and agent.
- Cherry-pick requires non-empty `cherryPickCommits` array (INVALID_INPUT).
- `try/finally` restores original branch if checkout was needed.
- Squash failure path calls `merge --abort` (catch, non-fatal).

### FR-008 — `discardAgent` cleanup ✅

`cleanup.ts:10-16` delegates to existing `removeAgentWorktree()` from Phase 1, which handles idempotent worktree removal + branch deletion. NF-003 (idempotent cleanup) is inherited.

### SC-004 — `getAgentDiff` returns correct unified diff ✅

`diff.test.ts:24-50`: Creates repo with parent branch, creates agent branch with 2 commits modifying `README.md` and adding `NOTES.md`. Asserts unified diff contains both file diffs and commit summary has 2 entries in reverse chronological order ("agent change 2" before "agent change 1").

### SC-005 — squash merge creates single commit ✅

`merge.test.ts:24-57`: Creates agent branch with 2 commits, squash-merges into parent. Asserts:
- `createdCommitSha` matches new HEAD
- HEAD advanced (beforeHead ≠ afterHead)
- Exactly 1 new commit on parent (`rev-list --count`)
- That commit has exactly 1 parent (not a merge commit — `rev-list --parents`)
- File contents from both agent commits present

### SC-006 — `discardAgent` removes artifacts ✅

`cleanup.test.ts:25-48`: Creates agent worktree via `createAgentWorktree`, then calls `discardAgent`. Asserts worktree directory no longer exists and branch ref (`refs/heads/agent/...`) is gone via `git show-ref --verify`.

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| P5-001 | LOW | API takes `{ repoRoot, parentBranch, agentBranch }` rather than `agentId` per spec. Caller must resolve agent metadata to branches before calling. Acceptable for composability — Phase 6 integration layer will provide the `agentId`-based facade. |
| P5-002 | LOW | No test for fast-forward or cherry-pick strategies. SC-005 only covers squash. The code paths are straightforward git commands but remain untested. |
| P5-003 | LOW | Cherry-pick failure mid-sequence has no `cherry-pick --abort` cleanup (only squash has `merge --abort`). A failed cherry-pick will leave the repo in a cherry-pick-in-progress state. Phase 6 or a follow-up should add `cherry-pick --abort` in the catch path. |
| P5-004 | LOW | `mergeAgent` does not call `discardAgent` after merge. Per the spec's merge flow diagram, merge is followed by worktree+branch removal. Implementation notes confirm this is by design ("merge and discard are separate APIs") — Phase 6 integration must chain them. |
| P5-005 | INFO | `git()` helper function is duplicated identically between `diff.ts` and `merge.ts` (same `execFile` wrapper, same 16MB buffer). Could be extracted to a shared `git-exec.ts` utility. Not blocking. |
| P5-006 | INFO | Export wiring complete: `index.ts` exports all Phase 5 symbols, `package.json` has subpath exports for `./diff`, `./merge`, `./cleanup`. |

## Test evidence

```
14/14 tests pass (7 test files)
tsc clean
pnpm --filter @opc/worktree-agents test — PASS
```

## Verdict

**Phase 5 approved.** FR-006, FR-007, FR-008, SC-004, SC-005, SC-006 all satisfied. No blockers for Phase 6 integration. P5-003 (cherry-pick abort) is the most actionable LOW finding — recommend addressing in Phase 6 or as a follow-up.
