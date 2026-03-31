# 054 Phase 5 Review — Quality Linter

**Reviewer**: claude
**Date**: 2026-03-30
**Verdict**: **Approved** — no blockers for Phase 6.

## Requirements checked

### FR-008 — Contributing quality criteria enforced via linter

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Required fields present | **PASS** | `readStringField()` checks type + emptiness; `lintAgent()` additionally checks `model` and `tools` presence (`linter.ts:91-145`) |
| `description` minimum length (50 chars) | **PASS** | `DESCRIPTION_MIN_LENGTH = 50`, checked in `lintCommon()` (`linter.ts:7, 108-115`) |
| `name` follows kebab-case convention | **PASS** | `KEBAB_CASE = /^[a-z0-9]+(?:-[a-z0-9]+)*$/` in `lintCommon()` (`linter.ts:8, 99-106`) |
| `tools` list non-empty for agents | **PASS** | `lintAgent()` checks `normalizeToolsField()` result for empty array (`linter.ts:136-144`); correctly handles both YAML arrays and comma-separated strings via `normalizeToolsField` |

### SC-004 — Linter catches missing required fields, short descriptions, non-kebab names

| Sub-criterion | Status | Evidence |
|---------------|--------|----------|
| Missing required fields | **PASS** | `readStringField()` emits `AFS_LINT_REQUIRED_FIELD` for missing/empty strings; agent mode additionally requires `model` and `tools` |
| Descriptions shorter than 50 chars | **PASS** | `AFS_LINT_DESCRIPTION_TOO_SHORT` emitted; test `linter.test.ts:39-59` validates with "Too short." |
| Non-kebab-case names | **PASS** | `AFS_LINT_NAME_KEBAB_CASE` emitted; test validates with `Bad_Agent` |

## Validation against existing agent definitions

All 4 existing agents in `.claude/agents/` pass the linter cleanly:

```
$ agent-frontmatter-lint .claude/agents --kind agent
agent-frontmatter lint passed (4 files checked)
```

| Agent | name | description length | tools format | Lint result |
|-------|------|--------------------|--------------|-------------|
| architect.md | `architect` ✓ kebab | 242 chars ✓ | comma-separated ✓ (via normalizeToolsField) | PASS |
| explorer.md | `explorer` ✓ kebab | 219 chars ✓ | comma-separated ✓ | PASS |
| implementer.md | `implementer` ✓ kebab | 196 chars ✓ | comma-separated ✓ | PASS |
| reviewer.md | `reviewer` ✓ kebab | 200 chars ✓ | comma-separated ✓ | PASS |

Commands in `.claude/commands/` correctly fail (8 errors across 9 files — missing `name` field). This is expected pre-Phase 6 migration behavior.

## Implementation quality

- **`classifyKind` auto-detection** (`linter.ts:48-60`): Path-based (`/agents/` → agent, `/skills/` → skill) with metadata fallback (presence of `model`/`tools`/`category`). Sound heuristic.
- **Diagnostic surfacing**: `toLintIssue()` bridges parser `FrontmatterDiagnostic` (with line/col) into `LintIssue` — parse-time errors (e.g., `AFS_MISSING_FRONTMATTER`) propagate correctly to lint output.
- **CLI entrypoint** (`lint-cli.ts`): Clean `--kind` / `--help` / positional `rootDir` parsing. Exit code 0 on pass, 1 on failure, 2 on CLI error.
- **Package wiring**: `bin.agent-frontmatter-lint`, `exports["./linter"]`, `exports["./lint-cli"]`, `lint:definitions` script all correctly configured.
- **Index exports**: Linter types (`DefinitionKind`, `LintFileResult`, `LintIssue`, `LintOptions`, `LintSummary`) and functions (`formatLintSummary`, `lintDefinitionsInDir`) exported from `src/index.ts`.
- **Test coverage**: 4 golden tests covering pass, multi-failure (3 codes), skill mode, and formatter output. All 21/21 tests pass. Build (`tsc`) clean.

## Findings

| ID | Severity | Finding |
|----|----------|---------|
| P5-001 | LOW | **No warning severity level.** All lint issues are `severity: "error"`. FR-008 says "enforced via linter" but some checks (e.g., description length) may be better as warnings in practice. The current all-error approach is spec-compliant but inflexible for graduated adoption during Phase 6 migration. |
| P5-002 | LOW | **No `--json` CLI output mode.** The CLI only emits human-readable text via `formatLintSummary`. CI pipelines typically want JSON for programmatic processing. The `LintSummary` object is already structured — a `--json` flag would be trivial to add. Not required by spec. |
| P5-003 | LOW | **`classifyKind` path heuristic is not robust for nested structures.** A file at `foo/agents/bar/skills/baz.md` would match `/agents/` first due to `includes()` short-circuit. Edge case unlikely in practice given current repo layout. |
| P5-004 | INFO | **Skill linter does not check for body presence.** FR-003 says skill body contains instructions, but `lintSkill()` only validates common fields (name, description). A skill with frontmatter but no body would pass. Reasonable — body quality is subjective. |
| P5-005 | INFO | **Agent `tools` field is required but an agent with `tools: []` gets a separate error code (`AFS_LINT_TOOLS_EMPTY`) from missing tools (`AFS_LINT_REQUIRED_FIELD`).** Good distinction — agents that deliberately declare empty tools vs agents that forgot the field are different cases. |
| P5-006 | INFO | **FR-006 trigger condition clause in description not validated.** The spec says descriptions "must include a trigger condition clause" but the linter only checks length >= 50, not content. Validating natural language trigger clauses would require heuristics or NLP — deferring is reasonable. |

## Verdict

**Phase 5 approved.** FR-008 (all four contributing quality criteria) and SC-004 (catches missing fields, short descriptions, non-kebab names) are both satisfied. The linter correctly handles legacy comma-separated `tools` strings via `normalizeToolsField`, ensuring existing agent definitions pass. CLI, package wiring, exports, and tests are all clean. No blockers for Phase 6 migration.
