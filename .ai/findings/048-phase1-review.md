# 048 Hookify Rule Engine — Phase 1 review

> **Reviewer:** claude | **Date:** 2026-03-30
> **Spec:** `specs/048-hookify-rule-engine/spec.md`
> **Plan:** `.ai/plans/048-hookify-rule-engine-phased-plan.md`
> **Implementation:** `packages/hookify-rule-engine/src/` (types.ts, parser.ts, index.ts, parser.test.ts)

## Verdict

**Phase 1 approved.** FR-001, FR-009, NF-002, NF-003 all satisfied. F-002 from plan review resolved (flat condition arrays explicitly normalized as implicit AND with test coverage). 10/10 tests pass. Package scaffold is clean with correct workspace structure. No blockers for Phase 2.

## Requirement verification

| Req | Status | Evidence |
|-----|--------|----------|
| FR-001 | ✅ | `parseRuleFile()` extracts all 5 frontmatter fields (`event`, `matcher`, `conditions`, `action`, `priority`) + markdown body. Validated: `parser.test.ts:20-28` |
| FR-009 | ✅ | Malformed YAML → `HKY_YAML_PARSE_ERROR` diagnostic, rule skipped. Missing fields → diagnostics, rule skipped. Invalid action/event → diagnostics, rule skipped. Duplicate IDs in `parseRuleSet()` → `HKY_DUPLICATE_RULE_ID` diagnostic, duplicate skipped. `parser.test.ts:42-54, 56-68, 70-88, 90-108, 132-140` |
| NF-002 | ✅ | Markdown body preserved verbatim as `rationale` field on Rule type. `parser.test.ts:30-33` |
| NF-003 | ✅ | Parser is a pure function — no runtime session dependency, no filesystem access, no side effects. `parseRuleFile(content, filePath)` takes strings, returns data. Determinism test: `parser.test.ts:142-147` |
| F-002 (plan) | ✅ Resolved | Flat condition arrays normalized as implicit AND via `normalizeConditionNode()` at `parser.ts:145-161`. Dedicated test at `parser.test.ts:35-39` confirms `conditions: [{...}]` → `{ all: [{...}] }` |

## Types assessment

All core types defined in `types.ts`:

- `HookEventType`: 4 spec events (PreToolUse, PostToolUse, UserPromptSubmit, Stop) ✅
- `Rule`: all spec fields + `rationale` (NF-002) + `sourcePath` (traceability) ✅
- `ConditionNode`: discriminated union supporting `ConditionLeaf | ConditionAllNode | ConditionAnyNode | ConditionNotNode` — covers FR-006 AND/OR/NOT ✅
- `ConditionLeaf`: operators `==`, `!=`, `contains`, `matches`, `glob` — covers all FR-006 comparisons ✅
- `Action`: `type` (block/warn/modify) + optional `transform` for modify — covers FR-003/FR-004/FR-005 ✅
- `Diagnostic`: structured with `code`, `severity`, `message`, `filePath`, `ruleId` — matches H-006 ✅
- `EvaluationResult`, `HookEvent`, `ParseRuleResult`: forward-compatible types for Phases 2-4 ✅

## Parser assessment

`parser.ts` implements a two-stage parse:

1. **Frontmatter extraction** (`parseFrontmatter()`): regex-based `---` boundary detection, YAML parsing via `yaml` package, type validation (must be object, not array/scalar). Structured diagnostics for missing frontmatter, malformed boundaries, and YAML parse errors.

2. **Field validation** (`parseRuleFile()`): validates `id` (non-empty string), `event` (in supported set), `matcher` (is record), `conditions` (via `normalizeConditionNode()`), `action` (via `validateAction()`), `priority` (finite number). Any diagnostic → rule is null (all-or-nothing validation).

3. **Condition normalization** (`normalizeConditionNode()`): recursively normalizes flat arrays → `{ all: [...] }`, validates `all`/`any` arrays, `not` single node, and leaf nodes (requires exactly one operator). Handles nested trees correctly.

4. **Ruleset dedup** (`parseRuleSet()`): first-occurrence-wins on duplicate IDs with diagnostic for subsequent duplicates.

## Findings

### P1-001 — No test for `any`/`not` boolean combinator parsing (LOW)

Tests cover flat arrays (implicit AND) and leaf conditions but not explicit `any`, `all`, or `not` nodes. The parser correctly handles these paths (`parser.ts:176-217`) but there is no test proving they parse through. Phase 2 will exercise these via condition evaluation tests, but Phase 1 should ideally validate the parse path independently.

**Recommendation:** Add 1-2 tests in Phase 2 that parse rules with explicit `any`/`not` conditions before testing evaluation.

### P1-002 — Rationale includes leading newline (LOW)

The frontmatter regex captures the body after `---\s*\n?` as `match[2]`. A standard rule file with a blank line between the closing `---` and body text will have a leading `\n` in `rationale`. Example:

```
---
...
---

Force-pushing rewrites remote history.
```

Produces `rationale: "\nForce-pushing rewrites remote history.\n"` rather than trimmed text. This affects FR-003/FR-004 display quality when the rationale is used as the block reason or warning message.

**Recommendation:** Trim the rationale (`rationale.trim()`) in `parseRuleFile()`, or defer to Phase 3 action executors. Not a blocker.

### P1-003 — Matcher validation is shallow (INFO)

`isRecord(matcher)` accepts any object including `{}`. An empty matcher has no `tool`, `input`, or `output` fields. This is correct for Phase 1 since matcher semantics are Phase 2 scope. The parser's job is structural validation only — field semantics belong to the evaluator.

### P1-004 — All-or-nothing validation on any diagnostic (INFO)

If any single field produces a diagnostic, the entire rule is rejected (`diagnostics.length > 0 → rule: null`). This means a rule with a valid structure but, say, an invalid condition leaf will be fully rejected even if other fields are fine. This is the correct behavior per FR-009 (skip invalid rules) — a partially valid rule is not a valid rule.

### P1-005 — `parseRuleSet` first-occurrence-wins on duplicate IDs (INFO)

When duplicate IDs are detected, the first occurrence is kept and the second is discarded with a `HKY_DUPLICATE_RULE_ID` diagnostic. This is a reasonable and deterministic choice. File ordering in the input array determines which occurrence wins — the loader (Phase 5) will need to define a canonical file ordering (alphabetical by path) to ensure determinism.

### P1-006 — `conditions` field is effectively required (INFO)

A rule with no `conditions` field passes `undefined` to `normalizeConditionNode()`, which produces `HKY_INVALID_CONDITION_NODE` and rejects the rule. This means match-all rules (no conditions, just event + matcher) are not expressible. The spec's FR-001 lists conditions as a required frontmatter field, so this is spec-faithful. If match-all rules are desired later, `conditions` could be made optional with a default of `{ all: [] }` (trivially true).

## Test coverage assessment

10/10 tests pass:

| Test | Req |
|------|-----|
| parses a valid markdown rule | FR-001 |
| preserves markdown body as rationale | NF-002 |
| normalizes flat conditions arrays as implicit AND | F-002 |
| reports malformed YAML and skips rule | FR-009 |
| reports missing required fields | FR-009 |
| rejects invalid action type | FR-009 |
| rejects invalid event name | FR-009 |
| rejects malformed condition objects | FR-009 |
| detects duplicate rule IDs | FR-009 |
| is a pure parser/validator | NF-003 |

Coverage is adequate for Phase 1 scope. P1-001 (boolean combinator parse tests) is the main gap.

## Package scaffold

- `package.json`: correct workspace name (`@opc/hookify-rule-engine`), private, ESM (`type: module`), `yaml` dependency for YAML parsing, vitest for testing ✅
- `tsconfig.json`: strict, ES2022 target, bundler resolution ✅
- `vitest.config.ts`: node environment, src test glob ✅
- `index.ts`: re-exports types and parser functions ✅

## Summary

Phase 1 delivers a sound, spec-faithful parser and type system. All Phase 1 plan deliverables are present. F-002 is resolved. No blockers. Cursor may proceed with Phase 2 (matcher + condition evaluator).
