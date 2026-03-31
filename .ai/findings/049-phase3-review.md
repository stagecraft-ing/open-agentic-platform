# 049 Permission System — Phase 3 Review

**Reviewer**: claude | **Date**: 2026-03-31 | **Verdict**: **Phase 3 APPROVED** for Phase 4 start

## Phase 3 scope

FR-001 (canUseTool hook contract), FR-002 (5-layer ordered evaluation), FR-004 (three-choice prompt), FR-007 (disallowed precedence over allowed). Deliverables: `evaluator.ts`, `defaults.ts`, `prompt.ts`.

## Requirements coverage

### FR-001 — canUseTool hook contract

**Satisfied.** `PermissionEvaluator.evaluate(input: PermissionEvaluationInput)` accepts `toolName`, `argument`, `isInteractive`, and optional `nowIso` — all fields needed for the hook. Full hook wiring deferred to Phase 4 (`createPermissionHandler`) as planned.

### FR-002 — 5-layer ordered evaluation

**Satisfied.** `evaluator.ts:86-184` implements all five layers in spec order:
1. **Non-interactive** (line 87-89): checks `isInteractive` first, routes to `evaluateNonInteractive()` with deny-all or allow-list modes.
2. **Bypass** (line 91-103): checks default + custom bypass patterns.
3. **Disallowed** (line 113-129): checks default disallowed + stored deny entries. Evaluated before remembered allows — spec-faithful.
4. **Remembered** (line 131-143): checks stored allow entries from `store.list()`.
5. **Prompt** (line 145-183): invokes prompt handler, handles all three choices.

Each layer returns immediately on match — first-match-wins semantics correct.

### FR-004 — Three-choice prompt

**Satisfied.** `prompt.ts` defines `PermissionPromptChoice` as `"allow_once" | "allow_remember" | "deny"`. `PermissionPromptRequest` provides `toolName`, `argument`, and `suggestedPattern`. `PermissionPromptResponse` allows the prompt to override the pattern and scope. The evaluator handles all three choices correctly:
- `allow_once` (line 152-158): returns allow without persistence.
- `allow_remember` (line 161-175): normalizes pattern, calls `store.upsert()`, returns allow.
- `deny` (line 178-183): implicit fall-through, returns deny.

### FR-007 — Disallowed precedence over allowed

**Satisfied.** Disallowed layer (line 113-129) is evaluated *before* remembered allows (line 131-143). SC-002 test directly validates this: a `Bash(git:push:**)` disallowed pattern blocks even when a broader `Bash(git:**)` allow exists in the store.

### FR-009 — Non-interactive mode

**Satisfied.** `NonInteractivePolicy` supports `deny_all` (default) and `allow_list` with configurable patterns. `evaluateNonInteractive()` (line 47-70) cleanly separates the two modes.

## Defaults review

`defaults.ts` defines:
- **Bypass**: `Read(**)`, `Glob(**)`, `rg(**)` — read-only tools, sensible baseline.
- **Disallowed**: `Bash(rm:-rf:*)`, `Bash(git:push:--force*)` — destructive operations, sensible baseline.

Both lists are `Object.freeze()` immutable arrays. Custom patterns are appended via `bypassPatterns` / `disallowedPatterns` options.

## Prompt contract review

`prompt.ts` is a clean callback interface:
- `PermissionPromptHandler` is an async function — allows UI-layer async resolution.
- `PermissionPromptResponse.scope` excludes `"session"` via `Exclude<PermissionScope, "session">` — correct, since remembered permissions must be persistent.
- `PermissionPromptResponse.pattern` is optional — falls back to `suggestedPattern` when absent.

## Test coverage

4/4 evaluator tests pass. 16/16 total package tests pass. `tsc` clean.

| Test | Validates | Result |
|------|-----------|--------|
| SC-001 bypass short-circuit | FR-002 layer 2, prompt not called | PASS |
| SC-002 disallowed over allow | FR-007 precedence | PASS |
| SC-003 remember flow | FR-004 + FR-005 persist then suppress | PASS |
| Non-interactive deny-all | FR-009 deny before other layers | PASS |

## Phase 1/2 review findings status

- **P1-001** (LOW): Regex caching for NF-001 — still open. `findMatchingPattern` iterates all patterns calling `matchesPermissionPattern` per invocation. At scale (500 entries per NF-001), this will matter. Phase 6 benchmark scope.
- **P1-002** (LOW): No tool-wildcard test — still open. No test for `Tool(*)` matching any argument.
- **P2-001** (LOW): `revoke` matches pattern only — unchanged, store behavior.
- **P2-002** (LOW): No list ordering — evaluator calls `store.list()` and filters by decision, so ordering doesn't affect correctness (all matching entries are checked).

## Findings

### P3-001 (LOW) — Store read on every evaluation

`evaluate()` calls `options.store.list()` on every invocation that passes the bypass check (line 105). For the JSON file-backed store, this means a disk read per tool call. At high invocation rates this could exceed NF-001's 2ms budget. Mitigation: Phase 4 or 6 can introduce a cached snapshot that invalidates on write.

### P3-002 (LOW) — No ambiguous overlap precedence test

The plan's validation section asks for "tests for ambiguous overlaps and explicit precedence guarantees." The evaluator correctly handles precedence by layer ordering (disallowed before remembered), but there is no test for overlapping *within* a single layer — e.g., two remembered patterns where one is more specific than the other. Not a correctness issue since `find()` returns the first match and patterns are insertion-ordered, but an explicit test would document this behavior.

### P3-003 (LOW) — No non-interactive allow-list test

There is a test for `deny_all` non-interactive mode but no test for `allow_list` mode with matching and non-matching patterns. Both paths exist in `evaluateNonInteractive()` but only the deny path has test coverage.

### P3-004 (INFO) — Suggested pattern construction

`suggestedPattern` at line 145 is constructed as `` `${input.toolName}(${input.argument})` `` — a raw concatenation. This is fine since `normalizePermissionPattern` is applied before storage (line 162), but the suggested pattern shown to the user at prompt time is unnormalized. Minor UX consideration.

### P3-005 (INFO) — Deny at prompt does not persist

When the user chooses "deny" at the prompt (line 178-183), the decision is not persisted to the store. This means the same tool invocation will prompt again on next call. This is a design choice — the spec doesn't require persist-on-deny. If desired later, it's a one-line addition.

### P3-006 (INFO) — `MemoryPermissionStore` test double is faithful

The in-memory store test double correctly implements `list`, `upsert`, `revoke`, and `clearExpired` with scope filtering and ID-based dedup. The `insert()` helper method for seeding test data is clean.

## Summary

Phase 3 delivers a correct, spec-faithful 5-layer evaluator with clean separation between evaluation logic (`evaluator.ts`), default patterns (`defaults.ts`), and prompt contract (`prompt.ts`). All three success criteria (SC-001, SC-002, SC-003) are directly tested. FR-007 precedence is structurally guaranteed by evaluation order. Non-interactive mode (FR-009) is clean with two policy modes. 3 LOW findings (store reads, overlap tests, allow-list test coverage) — none are blockers. No blockers for Phase 4.

**Baton**: -> **cursor** for Phase 4 (canUseTool hook integration).
