# 049 Permission System — Phase 1 review

> Reviewer: **claude** | Date: 2026-03-31 | Verdict: **APPROVED** for Phase 2 start

## Scope

Phase 1 deliverables per plan: `types.ts` (PermissionEntry, PermissionScope, PermissionDecision, evaluator I/O contracts), `pattern.ts` (parser/matcher with `*` and `**`), deterministic normalization, unit tests (wildcard behavior, determinism, invalid diagnostics, SC-004 fixture).

## Requirements coverage

| Requirement | Phase 1 scope | Status |
|-------------|---------------|--------|
| FR-003 | Wildcard patterns with `*` (single segment) and `**` (recursive) | **Satisfied** — `globToRegExp` maps `*` → `[^:/]*` and `**` → `.*`, tested with include/exclude |
| NF-003 | Deterministic matcher | **Satisfied** — repeated-call test proves identical results; no global regex state (`new RegExp` per call, no `/g` flag) |
| SC-004 | `Read(/Users/me/**)` include/exclude | **Satisfied** — explicit test fixture |

## Plan review findings resolution

| Finding | Status |
|---------|--------|
| F-001 (LOW): Include `expiresAt?: string \| null` in Phase 1 types | **Resolved** — `PermissionEntry.expiresAt?: string \| null` present at `types.ts` |
| F-002 (LOW): Document colon-segment mapping in matcher tests | **Resolved** — `canonicalizeArgumentSegments()` helper + test with comment "F-002: command tokens map to colon-delimited segments before matching" |

## Implementation assessment

### types.ts

- `PermissionScope` = `"session" | "project" | "global"` — matches spec scope levels.
- `PermissionDecision` = `"allow" | "deny"` — matches store schema.
- `PermissionEntry` — all 7 store schema fields present (`id`, `tool`, `pattern`, `decision`, `scope`, `createdAt`, `expiresAt`). Correct.
- `PermissionEvaluationInput` — clean contract with `toolName`, `argument`, `isInteractive`, optional `nowIso` for clock injection. Forward-compatible with Phase 3 evaluator.
- `PermissionEvaluationResult` — `source` union covers all 5 layers (`non_interactive`, `bypass`, `disallowed`, `remembered`, `prompt`). `matchedPattern` optional, `rationale` string for observability. Sound.

### pattern.ts

- **Parser**: `parsePermissionPattern` validates `Tool(argumentPattern)` format. Handles empty input, missing parens, empty tool, empty argument — all with diagnostic codes (`PERM_PATTERN_*`). Rejects nested/multiple parens via `indexOf("(", openParen + 1) === -1` check. Good.
- **Normalization**: Trims whitespace from tool and argument, rebuilds canonical `Tool(argPattern)`. `normalizePermissionPattern` provides convenient string→string form.
- **Matcher**: `matchesPermissionPattern` parses pattern, builds regex for tool and argument independently, tests both. Invalid patterns return `false` (safe default).
- **Glob engine**: `globToRegExp` maps `*` → `[^:/]*` (single segment, excluding `/` and `:`), `**` → `.*` (recursive). All other chars escaped via `escapeRegExpChar`. Anchored with `^...$`.
- **Segment canonicalization**: `canonicalizeArgumentSegments` splits on whitespace, joins with `:`. Enables `Bash(git:commit:*)` from `"git commit --amend"`.

### pattern.test.ts

7 tests covering:
1. Explicit `Tool(argumentPattern)` normalization with whitespace
2. 4 diagnostic codes for invalid patterns
3. `*` vs `**` segment depth distinction
4. Deterministic repeated matching
5. SC-004 include/exclude fixture
6. F-002 colon-segment documentation
7. `normalizePermissionPattern` deterministic form

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| P1-001 | LOW | `globToRegExp` creates a new `RegExp` on every `matchesPermissionPattern` call — no caching. For Phase 3 evaluator with 500 entries (NF-001 < 2ms p99), this may need a pattern cache. Not a Phase 1 blocker; monitor when evaluator lands. |
| P1-002 | LOW | No test for tool-name wildcards (e.g., `*(git:*)` matching any tool with git argument). The spec example `Bash(*)` covers tool-exact + arg-wildcard but not tool-wildcard patterns. Phase 3 evaluator tests should cover this. |
| P1-003 | LOW | `escapeRegExpChar` does not escape `-` inside character classes, but since the regex is constructed outside character classes this is safe. No action needed, noting for completeness. |
| P1-004 | INFO | Package `exports` field maps `"."`, `"./types"`, `"./pattern"` to `src/` paths (not `dist/`). This works for workspace consumers via TypeScript path resolution but would not work for published packages. Acceptable for internal workspace package. |
| P1-005 | INFO | `PermissionEvaluationInput.nowIso` is `string` not `Date` — good for serialization boundary, evaluator can parse internally. |
| P1-006 | INFO | No `index.ts` re-export for `canonicalizeArgumentSegments` — it is exported from `./pattern` subpath, which is sufficient. |

## Verdict

**Phase 1 APPROVED.** All Phase 1 deliverables present and correct. FR-003, NF-003, SC-004 satisfied. F-001 and F-002 from plan review resolved. 7/7 tests pass, `tsc` clean. No blockers for Phase 2.

Baton → **cursor** for Phase 2 (JSON permission store).
