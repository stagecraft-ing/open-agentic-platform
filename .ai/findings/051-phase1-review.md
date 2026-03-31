# 051 — Worktree Agents Phase 1 review

Date: 2026-03-30
Reviewer: claude
Phase: 1 — Worktree manager core
Spec: `specs/051-worktree-agents/spec.md`
Plan: `.ai/plans/051-worktree-agents-phased-plan.md`
Code: `packages/worktree-agents/src/worktree-manager.ts`

## Verdict

**Phase 1 approved.** FR-001 (worktree creation + branch), FR-010 (shared `.git`, isolated working tree), NF-003 (idempotent cleanup) all satisfied. F-001 and F-003 from the plan review are both addressed. No blockers for Phase 2.

## Requirement assessment

### FR-001 — spawn creates worktree from HEAD on new branch

**Satisfied.** `createAgentWorktree()` (worktree-manager.ts:154–175):
- Creates a new git worktree via `git worktree add <rel> -b <branch> <startPoint>` with `startPoint` defaulting to `"HEAD"`.
- Branch naming follows spec pattern `agent/<id>-<slug>` via `branchNameForAgent()` (line 39).
- Worktree path is deterministic under `.worktrees/<sanitized-id>` via `defaultWorktreePath()` (line 43).
- Calls `ensureWorktreesDirectoryIgnored()` before creation (line 158) — addresses **F-001**.
- Guards against existing path with `PATH_EXISTS` error (lines 163–172).

### FR-010 — shared .git, isolated working tree

**Satisfied.** Standard `git worktree add` behavior gives each worktree its own working tree and index while sharing the repository's `.git` directory. Test confirms isolation: writing `wt-only.txt` in the worktree does not appear in the main checkout (test lines 59–62).

### NF-003 — idempotent cleanup

**Satisfied.** `removeAgentWorktree()` (worktree-manager.ts:182–206):
- Lists all worktrees and matches by resolved real path via `sameRealPath()` (line 189).
- If registered: tries `git worktree remove --force`, falls back to `fs.rm` (lines 192–196).
- If not registered: `fs.rm` with `force: true` and swallowed errors (line 198).
- Prunes with `git worktree prune` (line 201).
- Optional branch deletion via `git branch -D` (lines 203–205).
- Double-remove test passes (test lines 76–96).

## Plan findings resolution

### F-001 — `.worktrees/` directory gitignore handling

**Resolved.** Two mechanisms:
1. **Root `.gitignore`**: Repo `.gitignore` line 299 contains `.worktrees/` — prevents tracking the directory entirely.
2. **Inner `.gitignore`**: `ensureWorktreesDirectoryIgnored()` (lines 94–103) writes `.worktrees/.gitignore` containing `*` as a belt-and-suspenders measure. Called automatically in `createAgentWorktree()`.

### F-003 — Orphaned worktree reconciliation

**Resolved.** `reconcileOrphanWorktreeDirs()` (lines 212–232) scans `.worktrees/` for directories not in the `knownDirectoryNames` set, returning their paths. Test confirms detection of unknown `ghost` directory (test lines 98–109). Note: the function only *detects* orphans — callers decide whether to remove them. This is the right separation of concerns for Phase 1.

## Findings

### P1-001 — `sanitizeSegment` may collide agent IDs (LOW)

`sanitizeSegment()` strips all non-`[a-z0-9_-]` characters and truncates to 64 chars. Two different `agentId` values could produce the same sanitized directory name (e.g., `a.b.c` and `a-b-c` both become `a-b-c`). In practice, agent IDs are expected to be UUIDs or short alphanumeric strings, so collision is unlikely. If agent IDs with special characters are used, `createAgentWorktree` would fail with `PATH_EXISTS` — a safe failure. Not a blocker.

### P1-002 — No `startPoint` test coverage (LOW)

`createAgentWorktree` accepts an optional `startPoint` parameter (default `"HEAD"`) that is passed directly to `git worktree add`. No test exercises branching from a specific commit or tag. Add a test in a later phase when more complex scenarios are needed.

### P1-003 — `parseWorktreeListPorcelain` exported but not directly tested (LOW)

The porcelain parser is exercised indirectly through `listOapWorktrees` in the create test, but there is no direct unit test with crafted porcelain output (detached HEAD, bare worktree, edge cases). Consider adding parser-specific tests if parsing bugs surface.

### P1-004 — `reconcileOrphanWorktreeDirs` skips `.git` but not `.gitignore` (INFO)

The function skips `.git` directories but not files like `.gitignore` (which isn't a directory, so the `d.isDirectory()` filter handles it). Correct behavior — non-directory entries are ignored.

### P1-005 — `--force` flag on worktree removal (INFO)

`removeAgentWorktree` uses `git worktree remove --force` which removes the worktree even if it has uncommitted changes. This is intentional per W-006 (idempotent cleanup) and aligns with the discard semantics from FR-008. The force flag is appropriate because agent work is preserved via its branch; uncommitted worktree state is not a concern at cleanup time.

### P1-006 — `sameRealPath` symlink fallback (INFO)

`sameRealPath()` falls back to `path.resolve()` if `realpathSync` throws (e.g., when one path doesn't exist). This is sound — the fallback handles the "already deleted" case during idempotent removal without throwing.

## Test coverage

4/4 tests pass:
1. `createAgentWorktree adds an isolated checkout under .worktrees` — FR-001, FR-010
2. `ensureWorktreesDirectoryIgnored writes .worktrees/.gitignore` — F-001
3. `removeAgentWorktree is idempotent` — NF-003
4. `reconcileOrphanWorktreeDirs flags unknown directories` — F-003, R-003

## Summary

6 findings: P1-001 LOW (sanitize collision), P1-002 LOW (no startPoint test), P1-003 LOW (no direct porcelain parser test), P1-004–P1-006 INFO. F-001 and F-003 from plan review both resolved. Phase 1 approved — baton to **cursor** for Phase 2 (concurrency + FIFO queue).
