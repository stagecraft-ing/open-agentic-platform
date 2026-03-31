# 049 Permission System — Phase 2 review

> Reviewer: **claude** | Date: 2026-03-31 | Verdict: **APPROVED** for Phase 3 start

## Scope

Phase 2 deliverables per plan: `store.ts` with `list`, `upsert`, `revoke`, `clearExpired`, and scoped load/save; schema versioning (`version: 1`); readable pretty JSON persistence; atomic write strategy (temp file + rename); unit tests for path resolution, create-on-first-write, revoke by exact pattern, expired-entry clearing, and hand-edited JSON round-trip.

## Requirements coverage

| Requirement | Phase 2 scope | Status |
|-------------|---------------|--------|
| FR-005 | "Allow & remember" writes granted pattern and scope to permission store | **Satisfied** — `upsert()` writes entry with tool, pattern, decision, scope, createdAt, expiresAt to scoped JSON file; dedup by id or by (pattern + tool + decision) |
| FR-006 | JSON file at `.claude/permissions.json` (project) and `~/.claude/permissions.json` (global); schema records tool, pattern, scope, timestamp, decision | **Satisfied** — `resolveScopedStorePath` maps project → `<projectRoot>/.claude/permissions.json`, global → `<homeDir>/.claude/permissions.json`; schema includes all 7 fields |
| NF-002 | Human-readable and hand-editable store format | **Satisfied** — `JSON.stringify(..., null, 2)` with trailing newline; `readDocumentContent` tolerant parser skips malformed entries without crashing; round-trip test proves hand-edited JSON is loadable |

## Plan deliverable checklist

| Deliverable | Status |
|-------------|--------|
| `store.ts` with `list`, `upsert`, `revoke`, `clearExpired` | **Present** — all 4 operations implemented |
| Schema versioning (`version: 1`) | **Present** — `PERMISSION_STORE_VERSION = 1`; reader rejects unknown versions with `PERM_STORE_UNSUPPORTED_VERSION` |
| Pretty JSON persistence | **Present** — 2-space indent, trailing newline |
| Atomic write (temp file + rename) | **Present** — `atomicWriteStoreDocument` writes to `${storePath}.tmp-${pid}-${timestamp}-${random}` then `fs.rename` |
| Scoped load/save (project + global) | **Present** — `loadScopeEntries` and `atomicWriteStoreDocument` both dispatch on scope |

## Phase 1 review findings status

| Finding | Status |
|---------|--------|
| P1-001 (LOW): Regex caching for NF-001 | **Still open** — expected, relevant at Phase 3/6 evaluator + benchmark |
| P1-002 (LOW): No tool-wildcard test | **Still open** — Phase 3 evaluator test scope |
| P1-003 (LOW): `escapeRegExpChar` dash safety | **Still open** — no action needed |

## Implementation assessment

### store.ts — API surface

- **`createPermissionStore(overrides?)`**: Factory accepts `projectRoot` and `homeDir` overrides for testability — clean dependency injection. Returns `PermissionStore` interface with 4 operations. Sound.
- **`list(scope?)`**: When scope is omitted or `"session"`, loads both project and global in parallel via `Promise.all`, returns merged. When scoped, loads only that scope. Correct per P-005 (merged view for reads).
- **`upsert(input)`**: Rejects `"session"` scope with `PERM_STORE_SCOPE_NOT_PERSISTED`. Deduplicates by `id` first, then by `(pattern, tool, decision)` triple — prevents accumulation of identical grants. Generates UUID for new entries. `createdAt` defaults to `new Date().toISOString()` if not supplied. Sound.
- **`revoke(pattern, scope?)`**: Removes all entries matching exact pattern string across target scopes. When scope is omitted or `"session"`, targets both project and global. Returns count of removed entries. Correct per SC-006.
- **`clearExpired(nowIso?, scope?)`**: Filters entries where `expiresAt` is non-null, parseable, and <= `nowIso`. Retains entries with `null`/absent `expiresAt` or unparseable dates (safe default — don't discard entries you can't interpret). Sound.

### store.ts — Persistence mechanics

- **Path resolution**: `PROJECT_RELATIVE_PATH` = `.claude/permissions.json`, `GLOBAL_RELATIVE_PATH` = `.claude/permissions.json`. Project joins with `projectRoot`, global joins with `homeDir`. Matches FR-006 spec paths exactly.
- **Atomic write**: Temp file uses `pid-timestamp-random` suffix for uniqueness. `fs.writeFile` then `fs.rename` — rename is atomic on POSIX filesystems. `fs.mkdir({ recursive: true })` ensures parent directory exists. Sound.
- **Schema validation**: `readDocumentContent` checks `version === 1`, validates `entries` is array, validates each entry has correct types for all required fields (`id`, `tool`, `pattern`, `decision`, `scope`, `createdAt`), normalizes `expiresAt` to `null` if absent. Malformed entries are silently skipped — this supports the hand-editability goal (NF-002) by being tolerant of partial corruption.

### store.test.ts — Test adequacy

5 tests covering all plan-required scenarios:

| Test | Plan requirement | Verdict |
|------|-----------------|---------|
| "resolves project and global scope paths correctly" | Path resolution | **Pass** — confirms files created at expected locations |
| "create-on-first-write creates schema v1 pretty JSON" | Create-on-first-write, schema v1, pretty JSON | **Pass** — asserts 2-space indent, version 1, trailing newline, entry content |
| "revoke removes entries by exact pattern across scopes" | Revoke by exact pattern | **Pass** — inserts in both scopes, revokes pattern, verifies count=2 and remaining entry |
| "clearExpired removes expired entries while retaining active" | Expired-entry clearing | **Pass** — expired vs active entries, asserts count=1 removed, 1 retained |
| "round-trips hand-edited JSON documents" | Hand-edited JSON round-trip | **Pass** — pre-writes JSON, reads it back, upserts additional entry, verifies both present |

All 5 tests pass. Coverage is adequate for Phase 2 deliverables.

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| P2-001 | LOW | `revoke` matches by exact `pattern` string only, not by tool+pattern. Two entries with the same pattern but different tools (e.g., `Bash(git:*)` with tool `"Bash"` vs a hypothetical `"Read"` with same pattern string) would both be revoked. This is unlikely to be a real issue since patterns encode the tool name, but `revoke` could also filter on `tool` for precision. Monitor at Phase 5 CLI when `revoke <pattern>` is surfaced to users. |
| P2-002 | LOW | `list()` with no scope or `"session"` scope loads both project and global but returns project entries first. No sorting by `createdAt` or any other key. Phase 3 evaluator will iterate these entries for matching — if order matters for precedence (first match wins), the project-before-global ordering should be documented as intentional. |
| P2-003 | LOW | No test for `upsert` deduplication behavior — the test suite writes distinct entries but never tests the path where an existing entry with the same `(pattern, tool, decision)` triple is updated rather than appended. Adding a test would prevent regressions in the dedup logic. |
| P2-004 | INFO | `readDocumentContent` silently skips malformed entries (missing required fields). This is correct for NF-002 tolerance but means data loss is invisible. Phase 5 CLI `list` could surface a warning count if entries were dropped during parsing. |
| P2-005 | INFO | No concurrency protection beyond atomic rename. Two concurrent `upsert` calls on the same scope could race (both read, both write). Single-process Node.js event loop makes this unlikely in practice, and the store is not designed for multi-process concurrent access. Acceptable. |
| P2-006 | INFO | `upsert` with `scope: "session"` throws synchronously rather than returning a structured error. Consistent with other error paths in the module (all throw `Error`). Acceptable for internal package. |

## Verdict

**Phase 2 APPROVED.** All Phase 2 deliverables present and correct. FR-005 (persist granted patterns), FR-006 (JSON store at specified paths with full schema), NF-002 (human-readable/hand-editable) all satisfied. Atomic write semantics sound. Schema versioning with rejection of unknown versions correct. Test coverage matches all plan-required scenarios. 5/5 store tests pass, 12/12 total tests pass, `tsc` clean. No blockers for Phase 3.

Baton → **cursor** for Phase 3 (layered evaluator and prompt contract).
