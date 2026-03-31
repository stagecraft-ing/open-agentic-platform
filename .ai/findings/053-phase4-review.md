# 053 Phase 4 Review — Post-Session Gates

**Reviewer:** claude
**Date:** 2026-03-31
**Verdict:** ✅ APPROVED — no blockers for Phase 5
**Validation:** `tsc` clean, 67/67 tests pass (25 P1 + 13 P2 + 15 P3 + 14 P4)

## Files reviewed

- `packages/verification-profiles/src/gate.ts` (137 lines)
- `packages/verification-profiles/src/gate.test.ts` (269 lines)
- `packages/verification-profiles/src/types.ts` — `GateResult` type (lines 101–115)
- `packages/verification-profiles/src/index.ts` — barrel exports (line 37)
- `packages/verification-profiles/package.json` — `./gate` subpath export (line 17)

## FR / SC satisfaction matrix

| Requirement | Status | Evidence |
|-------------|--------|----------|
| **FR-004** — gated profile blocks delivery until all skills pass | ✅ | `gate.ts:77` — `passed = gated ? profileResult.passed : true`. Test: "gated profile when all skills pass" (line 58), "gated profile when a skill fails" (line 76) |
| **FR-008** — failed gated profile reports which skills failed | ✅ | `gate.ts:84` — `failedSkills: profileResult.failedSkills`. Test: line 90 asserts `failedSkills` equals `["fail-skill"]`, line 94 verifies per-skill pass/fail status |
| **SC-002** — failing skill in gated profile blocks delivery | ✅ | Test line 76: gated=true + fail-skill → `passed: false`. Also: ungated advisory test (line 97) confirms `passed: true` even with failures when `gate: false` |
| **P3-004** — gate semantics consumed by caller, not engine | ✅ | `gate.ts:73-77` — `evaluatePostSessionGate` applies gate semantics on top of `executeProfile` result, exactly as P3-004 intended |

## Architecture assessment

### `evaluatePostSessionGate` (lines 24–87)

Clean orchestration function composing Phases 1–3:

1. **Profile loading** (lines 32–51): `readFile` with `.yaml` → `.yml` fallback. On both miss, returns `passed: false, gated: true` — correct fail-safe (unknown profile is treated as blocking).
2. **Parsing** (lines 53–63): Delegates to `parseProfileFile()`. Invalid YAML → `passed: false` — correct fail-safe.
3. **Skill library** (line 68): Loads full library from project root. Skills resolved by `executeProfile`'s two-phase resolve-then-execute.
4. **Gate semantics** (lines 76–77): Two-line gate logic is clear and correct. `gated ? profileResult.passed : true` captures the full spec intent.
5. **Timing** (lines 29, 85): Wall-clock `durationMs` wraps entire operation including I/O + parsing + execution.

### `loadProfileDiagnostics` (lines 93–137)

Validation-only path that loads profile + library and checks skill references without executing. Useful for linting workflows.

- Profile parse diagnostics + library diagnostics + skill-not-found diagnostics all accumulated (line 120–133).
- Returns structured `VerificationDiagnostic[]` — consistent with the diagnostic pattern from Phases 1–2.

### `GateResult` type (types.ts:101–115)

Well-designed result type:
- `passed` — final gate verdict (incorporates gate semantics)
- `gated` — whether the profile was gated (callers can distinguish advisory from blocking)
- `results: SkillResult[]` — full per-skill detail regardless of gate mode
- `failedSkills: string[]` — quick access to failure names
- `durationMs` — wall-clock timing

### Test coverage (14 tests)

Tests cover the full matrix:
- ✅ Gated pass / gated fail / ungated advisory
- ✅ Missing profile / invalid YAML / unknown skill ref
- ✅ `.yml` fallback
- ✅ Multi-skill ordering
- ✅ Platform default resolution
- ✅ `cwd` option passthrough
- ✅ `loadProfileDiagnostics`: not-found, invalid YAML, unresolvable refs, clean valid

## Findings

### P4-001 — Missing profile returns `gated: true` (INFO)

`gate.ts:44` — When a profile file is not found, the result has `gated: true`. This is a safe default (fail-closed), but `gated` is technically unknown since the profile was never loaded. The caller can't distinguish "profile is gated and failed" from "profile doesn't exist." However, `results: []` and `failedSkills: []` provide enough signal, so this is acceptable.

### P4-002 — `loadProfileDiagnostics` uses `.yaml` path even when `.yml` was loaded (LOW)

`gate.ts:117` — `parseProfileFile(content, profilePath)` always passes the `.yaml` path as `filePath` even if the content was loaded from `.yml`. This means diagnostics will reference the wrong file path when using `.yml` extension. Same issue in `evaluatePostSessionGate` at line 53. Minor — only affects diagnostic messages.

### P4-003 — No test for `loadProfileDiagnostics` with `.yml` fallback (INFO)

The `.yml` fallback is tested for `evaluatePostSessionGate` (line 142) but not for `loadProfileDiagnostics`. The code path is identical so it's covered by inspection, but a symmetric test would be complete.

### P4-004 — Duplicate file-loading pattern (INFO)

The `.yaml` → `.yml` fallback pattern is duplicated between `evaluatePostSessionGate` (lines 32–51) and `loadProfileDiagnostics` (lines 97–115). Could be extracted to a shared helper. Not blocking — the duplication is small and contained within one file.

### P4-005 — `evaluatePostSessionGate` error on invalid YAML returns `gated: true` (INFO)

Same as P4-001 — invalid YAML returns `passed: false, gated: true`. Consistent fail-closed behavior. The `profile` field is set to the input `profileName` rather than the parsed profile name, which is correct since parsing failed.

### P4-006 — Gate test count is 14, not 15 (INFO)

The baton message says "14 new tests" and vitest confirms 14 tests in `gate.test.ts`. The explore agent summary said 15 — this is just a reporting discrepancy. 14 is the correct count.

## Summary

Phase 4 correctly implements FR-004 and FR-008 with clean gate semantics layered on top of the Phase 3 execution engine. The fail-closed defaults (missing/invalid profiles → `passed: false`) are the right safety posture. Code is concise (137 lines for implementation, 269 for tests), well-structured, and fully tested. `GateResult` type provides all the information callers need to make delivery decisions.

No blockers for Phase 5 (profile selection — branch pattern matching, `--verify` flag).

### Carry-forward to Phase 5

- P4-002: `.yml` fallback passes wrong `filePath` to parser (LOW — fix opportunistically in Phase 5 if profile loading is refactored for selection logic)
- R-007 from readiness review: PR detection via branch patterns, not API calls
