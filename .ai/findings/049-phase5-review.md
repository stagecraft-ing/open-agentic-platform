# 049 Phase 5 Review — CLI Permissions Management

**Reviewer**: claude
**Date**: 2026-03-31
**Verdict**: **APPROVED** — no blockers for Phase 6

## Scope

Phase 5 targets:
- **FR-008**: Permission entries can be listed, revoked, and edited through a CLI subcommand (`claude permissions list`, `claude permissions revoke <pattern>`).
- **SC-005**: `claude permissions list` displays all stored permission entries with their patterns, scopes, and timestamps.
- **SC-006**: `claude permissions revoke "Bash(git commit:*)"` removes the entry and subsequent git commit commands prompt again.

## Deliverables

- `packages/permission-system/src/cli.ts` — `runPermissionsCli()` with `list`, `revoke`, `clear` subcommands
- `packages/permission-system/src/cli.test.ts` — 7 unit tests
- `package.json` subpath export `./cli` registered

## Requirement satisfaction

### FR-008 — CLI subcommands

**Satisfied.** Three subcommands implemented:
- `permissions list` — lists all stored entries across scopes (`store.list()` with no scope filter), sorted by `createdAt` then `tool` then `pattern` then `scope`. Output includes header row and pipe-delimited columns: PATTERN | TOOL | DECISION | SCOPE | CREATED_AT | EXPIRES_AT.
- `permissions revoke <pattern>` — delegates to `store.revoke(pattern, scope?)` with optional `--scope` flag. Reports count of removed entries.
- `permissions clear --expired` — delegates to `store.clearExpired(nowIso, scope?)` with optional `--scope` flag. Reports count of removed entries.

FR-008 also mentions "edited" — no `edit` subcommand is present, but the plan (Phase 5 section) scoped this phase to list/revoke/clear only, consistent with finding F-003 from the plan review which noted the edit scope was deferred. The store format is human-readable (NF-002), so manual editing via a text editor is the intended path for now.

### SC-005 — list output includes patterns, scopes, and timestamps

**Satisfied.** Test at `cli.test.ts:92–115` asserts the header row format and verifies each data row contains pattern (`Bash(git commit:*)`), decision (`allow`), scope (`project`), and timestamps (`2026-03-30T10:00:00Z`). Both `createdAt` and `expiresAt` are present in output.

### SC-006 — revoke removes entry and subsequent calls re-prompt

**Satisfied.** Test at `cli.test.ts:117–135` revokes `Bash(git commit:*)`, asserts exit code 0, confirms removal message, and verifies the store no longer contains that entry. The re-prompt behavior follows automatically from the evaluator: with no stored grant, the evaluator falls through bypass → disallowed → remembered (no match) → prompt layer.

## Implementation quality

### CLI contract

`runPermissionsCli(args, options)` accepts raw string args and an options object with injectable `store`, `nowIso`, `writeLine`, and `writeError`. This makes the CLI fully testable without filesystem or stdout side effects. The function returns an exit code (0 success, 1 error), which is the expected contract for CLI subcommand functions.

### Flag parsing

`parseFlags()` handles `--scope <value>`, `--scope=value`, `-s <value>`, and `--expired`. Positional args are consumed before flags (e.g., `revoke <pattern> --scope project`). The parser is intentionally simple — no external dependency, no complex option grammar.

### Output formatting

`formatEntryLine()` produces pipe-delimited output with `.trim()` on each segment. The header row and data rows use the same column order. `expiresAt` defaults to empty string when null. This is straightforward, human-readable output consistent with NF-002.

### Help output

Empty args, `help`, `--help`, and `-h` all produce the usage message with exit code 0. Unknown subcommands produce an error with exit code 1.

## Test coverage

25/25 tests pass (all 5 test files). 7 CLI-specific tests:

| Test | What it covers |
|------|---------------|
| 1 | Help message on empty args |
| 2 | List output with header, patterns, decisions, scopes, timestamps (SC-005) |
| 3 | Revoke by pattern across scopes, entry removed from store (SC-006) |
| 4 | Revoke without pattern → exit code 1 |
| 5 | Clear --expired removes expired entries, retains valid ones |
| 6 | Clear without --expired → exit code 1 |
| 7 | Unknown subcommand → exit code 1 |

## Build validation

- `pnpm --filter @opc/permission-system test` — 25/25 pass
- `pnpm --filter @opc/permission-system build` (`tsc`) — clean, no errors

## Findings

### P5-001 — `cli.ts` not re-exported from `index.ts` (LOW)

The CLI module has a `./cli` subpath export in `package.json` but is not re-exported from `index.ts`. This is likely intentional — keeping the CLI entry point separate from the library API prevents CLI-specific code from being bundled with the core permission system. The subpath export `@opc/permission-system/cli` is sufficient. No action needed unless consumers expect `import { runPermissionsCli } from '@opc/permission-system'`.

### P5-002 — No `--scope` flag test for `list` subcommand (LOW)

`list` always calls `store.list()` without a scope argument — it doesn't pass through the `--scope` flag even though revoke and clear support it. If a user wants to list only project-scoped entries, they can't. The flag parsing infrastructure exists but isn't wired for `list`. Not a blocker — could be added as a follow-up.

### P5-003 — No empty store test for `list` subcommand (LOW)

While the implementation handles `entries.length === 0` with a "No stored permission entries" message (line 88–89), there is no test exercising this path. The in-memory store always seeds 2 entries. Minor coverage gap.

### P5-004 — `revoke` matches pattern only, not tool+pattern (INFO)

`store.revoke(pattern)` filters by `entry.pattern !== pattern`. This means revoking `Bash(git commit:*)` will match on the full pattern string. This is consistent with the store's revoke API design (noted in P2-001 from Phase 2 review). The pattern string is the primary identifier for user-facing operations.

### P5-005 — No `--scope` flag test for `revoke` subcommand (INFO)

While the `--scope` flag is parsed and forwarded to `store.revoke(pattern, scope)`, no test exercises scope-filtered revocation. The flag parsing is shared and tested implicitly through the `clear --expired` test path.

### P5-006 — `formatEntryLine` coupling to pipe-delimited format (INFO)

The output format uses `" | "` as a delimiter. This is adequate for human consumption but not machine-parseable. If a `--json` or `--format` flag is needed later, the format function would need refactoring. Not a concern for the current phase.

## Summary

Phase 5 delivers all three CLI subcommands (list, revoke, clear --expired) with proper output formatting, error handling, and test coverage. FR-008, SC-005, and SC-006 are all satisfied. The implementation is clean, injectable, and consistent with the prior phases' patterns. No blockers for Phase 6.
