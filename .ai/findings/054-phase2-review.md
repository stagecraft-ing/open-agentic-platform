# 054 Phase 2 Review — Agent Frontmatter Parser

**Reviewer:** claude
**Date:** 2026-03-30
**Package:** `packages/agent-frontmatter/` (`@opc/agent-frontmatter` v0.1.0)
**Verdict:** ✅ Phase 2 approved — no blockers for Phase 3

## Scope

Review Phase 2 implementation (`parseFrontmatter`, `splitFrontmatterDelimiters`, `parseYamlMapping`, `normalizeToolsField`) against spec 054 requirements FR-001–FR-003, NF-002, NF-003, plus Phase 1 finding P1-002.

## Requirement coverage

| Req | Status | Evidence |
|-----|--------|----------|
| FR-001 (agent required: name, description, tools, model) | ✅ | Parser extracts all YAML keys into `Record<string, unknown>` — structural parsing is schema-agnostic, schema validation deferred to later phase (correct) |
| FR-002 (agent optional: category, color, displayName, etc.) | ✅ | Same `Record<string, unknown>` preserves all keys without filtering |
| FR-003 (skill required: name, description; body = instructions) | ✅ | `ParsedFrontmatter.body` captures markdown below closing `---`; metadata contains all keys |
| NF-002 (diagnostics with filePath + line + column) | ✅ | `FrontmatterDiagnostic` includes `filePath`, `line`, `column`; YAML parse errors use `lineColInFile()` to translate library positions to file-relative offsets; `yamlStartLine = 2` accounts for opening `---` on line 1. Test at `parser.test.ts:79` asserts line ≥ 2 and column defined |
| NF-003 (unknown fields preserved) | ✅ | Test at `parser.test.ts:45` adds `experimentalFlag: true` and asserts it survives parse. No field filtering anywhere in parser |
| P1-002 (comma-separated tools string) | ✅ | `normalizeToolsField()` splits on `,`, trims, filters empties. Test at `parser.test.ts:57` uses real agent format (`Read, Grep, Glob, Bash, LS`). Existing `.claude/agents/*.md` all use this format — confirmed compatible |

## Architecture assessment

- **Delimiter rules match Rust**: `splitFrontmatterDelimiters` uses identical boundary logic to `tools/shared/frontmatter` — `---` + newline open, `\n---\n` or `\r\n---\r\n` close. BOM stripped. This cross-language consistency is important for Phase 6 migration.
- **Diagnostic codes structured**: `AFS_MISSING_FRONTMATTER`, `AFS_MALFORMED_DELIMITERS`, `AFS_YAML_NOT_OBJECT`, `AFS_YAML_PARSE_ERROR` — four distinct codes cover all failure modes. All severity `"error"`.
- **`yaml` v2 dependency**: Single runtime dependency. `parseDocument()` used correctly (not `parse()`) to access `linePos` for NF-002 error positions.
- **Pure functions**: No side effects, no filesystem access — correct for a parser library. Loader (Phase 3) will add I/O.
- **`normalizeToolsField` is separate from `parseFrontmatter`**: Correct separation — the parser is format-agnostic; tools normalization is agent-specific. Caller composes as needed.

## Findings

| ID | Sev | Finding |
|----|-----|---------|
| P2-001 | LOW | No CRLF test in `splitFrontmatterDelimiters`. Code handles `\r\n` for both open and close delimiters, but all tests use LF only. Add a CRLF round-trip test for confidence, especially since Windows agents are a real use case. |
| P2-002 | LOW | `normalizeToolsField` silently drops non-string array elements (e.g., `[1, "Read", true]` → `["Read"]`). This is lenient but could mask authoring errors. Phase 5 linter should flag non-string elements in `tools` arrays. |
| P2-003 | INFO | Empty YAML body (`---\n\n---\nbody`) parses `null` via `doc.toJS()`, producing `AFS_YAML_NOT_OBJECT`. Correct behavior for agents/skills (required fields would be missing), but no test covers this path explicitly. |
| P2-004 | INFO | `lineColInFile` uses only `lp[0]` (start position). For multi-line YAML errors, end position (`lp[1]`) is available but unused. Not needed now — start position is sufficient for editor integration. |
| P2-005 | INFO | Parser does not validate against Phase 1 JSON Schemas. This is correct by design — structural parsing (Phase 2) is separate from schema validation (Phase 5 linter). Just noting for traceability. |
| P2-006 | INFO | `missing_newline_after_open` handles the `---foo` edge case (non-newline after opening delimiter). Good defensive check that prevents false-positive YAML parse attempts. |

## Test coverage

12 tests across 4 `describe` blocks:
- `splitFrontmatterDelimiters` (4): LF split, BOM strip, missing open, missing close
- `parseFrontmatter` (5): valid with unknown keys (NF-003), comma-separated tools (P1-002), missing delimiter, malformed YAML with line/col (NF-002), array root rejection
- `parseYamlMapping` (1): line offset relative to file
- `normalizeToolsField` (2): array trimming, unsupported type

Coverage is adequate for Phase 2. P2-001 recommends adding CRLF test.

## Real-world compatibility

Verified against all 4 existing agent definitions (`.claude/agents/architect.md`, `explorer.md`, `implementer.md`, `reviewer.md`):
- All use `tools: Read, Grep, Glob, Bash, LS` (comma-separated string)
- `normalizeToolsField()` correctly parses these to `["Read", "Grep", "Glob", "Bash", "LS"]`
- All have `name`, `description`, `model` as simple strings — parse cleanly

Verified against command/skill files (`.claude/commands/*.md`):
- Commands use `description`, `allowed-tools`, `argument-hint` — different field names from agent schema
- Parser preserves all these as unknown keys (NF-003) — no conflict

## Recommendation

**Phase 2 approved.** Parser is spec-faithful, well-structured, and compatible with existing definitions. Proceed to **Phase 3** (progressive loader with Tier 1–3 loading). P2-001 (CRLF test) and P2-002 (non-string tools element warning) are both LOW and can be addressed in Phase 3 or Phase 5 respectively.
