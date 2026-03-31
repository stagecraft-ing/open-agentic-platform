# 053 Phase 5 Review — Profile Selection

**Reviewer:** claude
**Date:** 2026-03-31
**Verdict:** ✅ APPROVED — no blockers for Phase 6
**Validation:** `tsc` clean, 78/78 tests pass (25 P1 + 13 P2 + 15 P3 + 14 P4 + 10 P5 + 1 carry-forward)

## Files reviewed

- `packages/verification-profiles/src/selector.ts` (91 lines)
- `packages/verification-profiles/src/selector.test.ts` (59 lines)
- `packages/verification-profiles/src/gate.ts` (139 lines — refactored from Phase 4)
- `packages/verification-profiles/src/gate.test.ts` (280 lines — 1 new test for .yml diagnostic path)
- `packages/verification-profiles/src/index.ts` — barrel exports (lines 39–41)
- `packages/verification-profiles/package.json` — `./selector` subpath export (line 18)

## FR / SC satisfaction matrix

| Requirement | Status | Evidence |
|-------------|--------|----------|
| **FR-005** — explicit invocation (`--verify=release`) and automatic context detection | ✅ | `selector.ts:36` — `selectProfile(context)` implements three-tier precedence: explicit > context flags > branch patterns. `parseVerifyFlag(args)` handles `--verify=<val>` and `--verify <val>` CLI forms. |
| **R-007** — PR detection via branch patterns, not API | ✅ | `selector.ts:60-68` — Branch regex covers `pull/<n>/(head|merge)`, `pr/*`, and conventional PR branch prefixes (`feature/*`, `fix/*`, `chore/*`, etc.) without any external API calls. |

## Architecture assessment

### `selectProfile` (lines 36–71)

Clean three-tier precedence implementation:

1. **Explicit override** (lines 37–40): `normalizeProfileName(context.explicit)` — trims, lowercases, returns null on empty. Correctly short-circuits before checking any other context.
2. **Boolean context flags** (lines 42–47): `isRelease` before `isPR` — correct ordering since release is more specific. Test at line 16 confirms release takes precedence over PR when both are true.
3. **Branch patterns** (lines 49–68): `normalizeBranch` strips `refs/heads/` prefix, lowercases. Pattern matching order is correct:
   - `release/*` and `rel/*` → `"release"` (most specific delivery branch)
   - `hotfix/*` → `"hotfix"` (distinct from release)
   - `pull/<n>/(head|merge)` → `"pr"` (GitHub-style PR refs)
   - `pr/*` → `"pr"` (generic PR prefix)
   - conventional prefixes (`feature`, `feat`, `bugfix`, `fix`, `chore`, `refactor`, `docs`, `test`, `perf`, `ci`, `build`) → `"pr"`
4. **Fallback** (line 70): Returns `null` for unrecognized branches (e.g., `main`, `develop`) — correct; no profile should be auto-selected for trunk branches.

### `parseVerifyFlag` (lines 79–90)

Minimal CLI flag parser. Handles both `--verify=value` and `--verify value` forms. Delegates to `normalizeProfileName` for empty/whitespace-only values. Does not consume `--verify` as a boolean flag (no value → returns null) — correct since a bare `--verify` with no profile name is meaningless.

### `normalizeProfileName` / `normalizeBranch` (lines 12–26)

Good defensive normalization:
- `normalizeProfileName`: handles `undefined`, empty string, whitespace-only strings. Lowercases for case-insensitive matching.
- `normalizeBranch`: strips `refs/heads/` for CI environments that provide full refs. Lowercases for consistent matching.

### `ProfileContext` interface (lines 1–10)

All fields optional — correct for a context object that may come from different sources (CLI args, CI environment, caller code). JSDoc comments provide clear examples.

### Carry-forward fix: P4-002 `.yml` diagnostic path

The `loadProfileContent` shared helper (`gate.ts:12–29`) now returns `{ content, filePath }` with the actual resolved path. Both `evaluatePostSessionGate` and `loadProfileDiagnostics` use this correctly — `loaded.filePath` is passed to `parseProfileFile` and used in diagnostics. This fully resolves P4-002.

New test at `gate.test.ts:270–279` confirms `.yml` diagnostics use the `.yml` path (not `.yaml`), resolving P4-003.

### Barrel exports and subpath (index.ts:39–41, package.json:18)

- `selectProfile`, `parseVerifyFlag` exported as values
- `ProfileContext` exported as type
- `./selector` subpath correctly points to `./src/selector.ts`

### Test coverage (10 tests)

Tests cover:
- ✅ Explicit override precedence (explicit wins over branch + isPR)
- ✅ isRelease > isPR precedence
- ✅ isPR standalone
- ✅ Release branch patterns (`release/*`, `refs/heads/rel/*`)
- ✅ Hotfix branch pattern
- ✅ PR branch patterns (GitHub `pull/N/head`, conventional `feature/*`, `fix/*`)
- ✅ Null fallback for unrecognized branches (`main`) and empty context
- ✅ `parseVerifyFlag` with `=` form and space form
- ✅ Missing/empty flag values

## Findings

### P5-001 — No test for `--verify` with next arg being another flag (INFO)

`parseVerifyFlag(["--verify", "--other"])` would return `normalizeProfileName("--other")` → `"--other"` (a string starting with `--`). Unlikely in practice since profile names don't start with `--`, but a guard like `if (next?.startsWith("--")) return null` would be defensive. Not blocking — callers control the arg source.

### P5-002 — `hotfix` profile assumed to exist (INFO)

Branch pattern `hotfix/*` maps to `"hotfix"` profile, but no bundled `hotfix` profile exists in defaults. Phase 6 (bundled skills/profiles) should address this, or the caller should handle `selectProfile` returning a name that doesn't resolve to an existing profile file. `evaluatePostSessionGate` already handles this correctly (missing profile → `passed: false, gated: true`).

### P5-003 — No `develop`/`dev` branch detection (INFO)

Branches like `develop`, `dev`, or `staging` return `null` — no auto-selected profile. This is arguably correct (these are long-lived branches, not delivery contexts), but some teams might expect a profile for `develop`. Can be addressed in future by extending the regex or allowing project-level branch→profile mapping config. Not blocking.

### P5-004 — `normalizeBranch` doesn't strip `refs/remotes/` prefix (INFO)

Only `refs/heads/` is stripped. Remote refs like `refs/remotes/origin/release/1.0` would fail to match release patterns because the regex expects the branch to start with `release/`. Minor — remote refs are rarely passed as the branch context; CI systems typically provide `refs/heads/` or bare branch names.

### P5-005 — No integration between `parseVerifyFlag` and `selectProfile` (INFO)

`parseVerifyFlag` returns a profile name string, which the caller must then pass as `context.explicit` to `selectProfile`. The two functions are intentionally decoupled — correct design for composability — but there's no convenience function that does both. Not blocking; callers can compose trivially.

### P5-006 — P4-004 duplicate loading pattern resolved (RESOLVED)

The duplicate `.yaml`→`.yml` loading pattern identified in P4-004 has been extracted into the shared `loadProfileContent` helper (`gate.ts:12–29`). Both `evaluatePostSessionGate` and `loadProfileDiagnostics` now use it. Clean refactor.

## Summary

Phase 5 correctly implements FR-005 profile selection with a clean three-tier precedence model: explicit CLI flag > workflow context booleans > branch pattern matching. The `selectProfile` function is pure (no I/O, no side effects), making it trivially testable and composable. `parseVerifyFlag` provides a minimal but sufficient CLI integration point.

The carry-forward fixes from Phase 4 (P4-002 `.yml` diagnostic path, P4-004 duplicate loading) are both resolved in this phase.

No blockers for Phase 6 (bundled skills and profiles).

### Carry-forward to Phase 6

- P5-002: Bundled `hotfix` profile should be included if the selector maps `hotfix/*` branches to it
- R-008 from readiness review: Bundled skills assume Node.js toolchain (npm)
