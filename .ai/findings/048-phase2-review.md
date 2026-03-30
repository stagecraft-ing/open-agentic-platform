# 048 Phase 2 Review — Matcher + Condition Evaluator

**Reviewer**: claude
**Date**: 2026-03-30
**Commit**: `7e8625c feat(048): implement hook matcher and condition evaluator`
**Verdict**: **Approved** — FR-006, FR-002 (partial) satisfied. P1-001 resolved. 21/21 tests pass. No blockers for Phase 3.

---

## Requirement Verification

| Requirement | Status | Evidence |
|-------------|--------|----------|
| **FR-006** (condition language) | ✅ | `conditions.ts` implements all 5 operators (`==`, `!=`, `contains`, `matches`, `glob`) + 3 boolean combinators (`all`/AND, `any`/OR, `not`/NOT) via recursive `evaluateConditionNode()` |
| **FR-002** (event filtering, partial) | ✅ | `matcher.ts`: `matchesRuleEventType()` filters by lifecycle event type; `matchesRuleMatcher()` matches tool name, input subset, output subset |
| **P1-001** (combinator parse tests) | ✅ Resolved | `parser.test.ts:42-67` — explicit `any`/`not` combinator parse test added |

## Deliverable Assessment

### `matcher.ts` (74 lines)

Two exported functions, three helpers:

- **`matchesRuleEventType()`**: Exact string comparison between rule event and hook event type. Correct and minimal.
- **`matchesRuleMatcher()`**: Three-selector filter (tool, input, output). Empty matcher `{}` acts as catch-all within event type — spec-faithful.
- **`resolveToolName()`**: Defensively resolves tool name from 4 payload shapes (`payload.tool` string, `payload.tool.name` object, `payload.toolName`, `payload.name`). Handles Claude Code's varying event payload formats.
- **`deepSubsetMatch()`**: Recursive subset check — every key/value in `expected` must exist and match in `actual`. Nested objects recurse. Primitives use strict equality (no coercion). Clean and correct.

### `conditions.ts` (209 lines)

One exported function, five helpers:

- **`evaluateConditionNode()`**: Recursive evaluator over the `ConditionNode` discriminated union. Short-circuits: `all` stops on first false, `any` stops on first true. `not` inverts child result. Diagnostics accumulated through all branches.
- **`evaluateLeaf()`**: Handles all 5 FR-006 operators with type safety:
  - `==` / `!=`: strict equality/inequality against resolved value
  - `contains`: string `includes()` + array `some()` — dual-mode, spec-faithful
  - `matches`: `new RegExp()` with try/catch → `HKY_INVALID_REGEX` diagnostic on bad pattern
  - `glob`: converts via `globToRegExp()` + try/catch → `HKY_INVALID_GLOB` diagnostic
  - Unknown operator → `HKY_UNKNOWN_OPERATOR` error diagnostic
- **`getPathValue()`**: Dot-path field navigation (`tool.name`, `input.command`). Returns `{ found, value }` tuple. Handles empty paths, non-object intermediates, missing keys. Undefined field → `HKY_FIELD_UNDEFINED` info diagnostic + `matched: false`. This is the spec's "undefined field reads → false with diagnostic" behavior.
- **`globToRegExp()`**: Escapes regex metacharacters, replaces `*` → `.*` and `?` → `.`. Anchored `^...$`.

### `index.ts` (18 lines)

Updated exports: adds `evaluateConditionNode`, `matchesRuleEventType`, `matchesRuleMatcher`. Package `exports` field in `package.json` includes `./conditions` and `./matcher` subpaths. ✅

## Test Coverage

| File | Tests | Coverage |
|------|-------|----------|
| `matcher.test.ts` | 5 | Event type match/mismatch, tool+input+output selectors, tool mismatch, missing input field, empty matcher catch-all |
| `conditions.test.ts` | 5 | Full operator matrix (all 5 operators in single `all` node), nested AND/OR/NOT combinators, undefined field diagnostic, short-circuit OR (avoids invalid regex branch), array `contains` |
| `parser.test.ts` | 11 | Phase 1 tests + P1-001 resolved (explicit `any`/`not` combinator parse test at line 42) |

**Total: 21/21 pass** (242ms)

## Findings

### P2-001 — `globToRegExp` supports `*` and `?` only (LOW)

**File**: `conditions.ts:50-54`

The glob-to-regex conversion handles `*` (any chars) and `?` (single char) but not `**` (recursive/double-star), character classes `[abc]`, or negation `[!abc]`. The spec says "glob patterns" without specifying which features.

**Impact**: Rules using advanced glob syntax will silently fail to match.
**Recommendation**: Sufficient for Phase 2. If advanced globs are needed, consider a dedicated glob library (e.g., `minimatch`) in a later phase. Document supported glob subset in the built-in rule library (Phase 6).

### P2-002 — No test for `?` glob wildcard (LOW)

**File**: `conditions.test.ts`

The glob test uses `specs/*/spec.md` (testing `*`). No test exercises `?` (single-char wildcard). The implementation handles it correctly (`\\\?` → `.`), but coverage is incomplete.

**Recommendation**: Add a `?` wildcard test in Phase 3 or Phase 6 test expansion.

### P2-003 — Evaluator doesn't guard against multi-discriminator nodes (INFO)

**File**: `conditions.ts:174-207`

A node like `{ all: [...], any: [...] }` would match `all` first due to if-chain ordering. This is safe because the parser validates condition structure in Phase 1, but the evaluator itself doesn't independently reject malformed nodes.

**Impact**: None in practice — parser is the validation boundary. If `evaluateConditionNode` is ever called with unvalidated input, behavior would be deterministic (first-match) but potentially surprising.

### P2-004 — Matcher failures are silent (INFO)

**File**: `matcher.ts:50-73`

`matchesRuleMatcher()` returns `boolean` without diagnostics. When a matcher fails, there's no structured feedback about *why* (e.g., "tool name 'Bash' didn't match 'Read'"). This is appropriate — matchers are filters, not validators — but it means debugging rule non-matches requires manual inspection.

**Impact**: None for correctness. May affect debuggability in Phase 4 (engine core) if users wonder why rules didn't fire.

### P2-005 — `contains` operator type widening (INFO)

**File**: `conditions.ts:82-98`

`ConditionLeaf.contains` is typed as `string` but `evaluateLeaf` handles both string containment (`value.includes(leaf.contains)`) and array containment (`value.some(item => item === leaf.contains)`). This dual behavior is useful and spec-faithful (FR-006 says "contains"), but the type doesn't document the array case.

**Impact**: None — the implementation is correct. Type documentation could be improved with a JSDoc comment.

### P2-006 — `resolveToolName` fallback chain untested for all variants (INFO)

**File**: `matcher.ts:30-44`, `matcher.test.ts`

Tests exercise `payload.tool.name` (object form). The string form (`payload.tool = "Bash"`), `payload.toolName`, and `payload.name` fallbacks are implemented but not individually tested.

**Impact**: Low — the code is straightforward. Phase 4 integration tests will likely exercise more payload shapes.

## Summary

Phase 2 delivers spec-faithful matcher and condition evaluator modules. All FR-006 operators are implemented with proper type checking and diagnostic emission. The matcher cleanly handles event type filtering and payload subset matching. Short-circuit behavior is deterministic and tested. P1-001 (missing combinator parse tests) is resolved. No blockers — cursor may proceed with Phase 3 (action executors: block, warn, modify).
