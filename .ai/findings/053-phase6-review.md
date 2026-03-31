# 053 Phase 6 Review — Bundled Skills and Profiles

**Date:** 2026-03-31
**Reviewer:** claude
**Verdict:** APPROVED — 053 feature-complete, all 6 phases delivered.

## Validation

- `pnpm --filter @opc/verification-profiles test` — **81/81 pass** (25 P1 + 13 P2 + 15 P3 + 14 P4 + 10 P5 + 1 P6-profiles + 3 P6-gate)
- `pnpm --filter @opc/verification-profiles build` — `tsc` clean, zero errors

## Files reviewed

| File | Change |
|------|--------|
| `src/defaults.ts` | Added `security-scan` and `license-check` skills (Phase 6) |
| `src/profiles.ts` | New file — bundled `pr`, `release`, `hotfix` profiles |
| `src/profiles.test.ts` | New file — tests bundled profile map |
| `src/gate.ts` | Updated `loadProfile` to fall back to bundled profiles; updated `loadProfileDiagnostics` to recognise bundled profiles |
| `src/gate.test.ts` | Added bundled hotfix test, bundled profile diagnostics test |
| `src/loader.test.ts` | Updated default count assertions (3 -> 5) |
| `src/index.ts` | Added `getDefaultProfiles` barrel export |
| `package.json` | Added `./profiles` subpath export |

## Spec requirement satisfaction

### FR-006 — skill resolution from library or platform defaults

**Satisfied.** `getDefaultSkills()` now returns 5 bundled skills: `lint`, `type-check`, `unit-tests`, `security-scan`, `license-check`. This matches the Phase 6 spec list exactly ("lint, type-check, test, security-scan, license-check" — `unit-tests` is the implementation name for `test`). Local overrides still take precedence per R-004 (verified by existing `loader.test.ts` "local skill overrides platform default" test).

### Phase 6 — bundled skills

**Satisfied.** All five skills specified in the implementation approach are present:

| Skill | Command | Safety | Network | Timeout |
|-------|---------|--------|---------|---------|
| `lint` | `npm run lint` | safe | deny | 120s |
| `type-check` | `npx tsc --noEmit` | safe | deny | 180s |
| `unit-tests` | `npm test` | safe | deny | 300s |
| `security-scan` | `npm audit --audit-level=high` | cautious | allow | 180s |
| `license-check` | `npx license-checker --summary` | safe | allow | 120s |

The `security-scan` skill correctly uses `safety_tier: "cautious"` (it queries external APIs) and `network: "allow"` (needs registry access). `license-check` also correctly uses `network: "allow"`. All other skills use `network: "deny"` and `read_only: true` — correct safe posture.

### Bundled profiles (beyond spec — carry-forward P5-002)

Three bundled profiles were added to satisfy the P5-002 carry-forward (hotfix profile needed for selector completeness):

| Profile | Gate | Skills |
|---------|------|--------|
| `pr` | true | lint, type-check, unit-tests, security-scan |
| `release` | true | lint, type-check, unit-tests, security-scan, license-check |
| `hotfix` | true | lint, type-check, unit-tests, security-scan |

**Assessment:** Sensible defaults. `release` adds `license-check` on top of `pr`/`hotfix` — correct since license compliance is most critical at release time. All three are `gate: true` (delivery-blocking), which matches the intended use with the profile selector from Phase 5.

### Gate fallback to bundled profiles

`gate.ts:loadProfile` correctly implements two-tier loading:
1. Try local `.verification/profiles/<name>.yaml` (then `.yml` fallback)
2. If no local file, resolve from `getDefaultProfiles()` map

Local profiles always take precedence — bundled profiles are a fallback, not an override. This is consistent with R-004 precedence semantics applied to skills.

`loadProfileDiagnostics` also updated: returns empty diagnostics for bundled profiles (no `VP_PROFILE_NOT_FOUND`), correctly reflecting that the profile exists as a bundled default. Test at `gate.test.ts:262` confirms.

### Carry-forward resolution

| ID | Description | Status |
|----|-------------|--------|
| P5-002 | No bundled `hotfix` profile | **RESOLVED** — `hotfix` profile in `profiles.ts`, tested in `gate.test.ts:228` |
| R-008 | Bundled skills assume Node.js | **Acknowledged** — all 5 bundled skills use `npm`/`npx`. This is documented in prior reviews and is acceptable for the initial implementation. Projects using other toolchains override via local `.verification/skills/`. |

## Architecture assessment

- **`profiles.ts`**: Clean, minimal module. Fresh `Map` per call (same pattern as `defaults.ts`). Profiles reference skills by name only — no circular dependency with `defaults.ts`.
- **`gate.ts` refactor**: `loadProfileContent` helper (introduced in Phase 5 for P4-002 fix) is reused for the two-tier loading, keeping the code DRY. `loadProfile` composes `loadProfileContent` + `parseProfileFile` + `getDefaultProfiles()` cleanly.
- **Export hygiene**: `getDefaultProfiles` exported from barrel, `./profiles` subpath in `package.json`. Consistent with existing pattern.

## Test coverage

- `profiles.test.ts`: 1 test — verifies map size (3) and presence of `pr`, `release`, `hotfix`
- `gate.test.ts:228`: Bundled `hotfix` profile resolves without local file, executes skills, returns `gated: true`
- `gate.test.ts:262`: `loadProfileDiagnostics` returns empty for bundled profile
- `loader.test.ts:79`: Updated to verify 5 default skills (was 3, now includes `security-scan` + `license-check`)

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| P6-001 | INFO | `profiles.test.ts` only checks map size and key presence — doesn't verify profile contents (skill lists, gate values). Acceptable given profiles are simple static data and gate tests exercise them end-to-end. |
| P6-002 | INFO | `hotfix` and `pr` profiles have identical skill lists. If these diverge in the future, the distinction already exists structurally. No action needed. |
| P6-003 | INFO | No test for local profile overriding a bundled profile (analogous to the R-004 skill override test). The code path is covered by the `loadProfile` precedence logic and existing tests that write local profile files which implicitly shadow bundled ones. |
| P6-004 | INFO | Bundled profile skill references (`"lint"`, `"type-check"`, etc.) are strings that must match bundled skill names exactly. No compile-time or load-time validation of this coupling. Acceptable: any mismatch would be caught at execution time by `resolveSkillRef` returning `VP_SKILL_NOT_FOUND`. |

## Summary

Phase 6 delivers exactly what the spec requires: five bundled default skills shipped out of the box. The implementation goes slightly beyond spec by also adding three bundled profiles (`pr`, `release`, `hotfix`), which is a natural extension that completes the P5-002 carry-forward and makes the system usable without any local configuration.

All 81 tests pass. `tsc` clean. No blockers. **053 is feature-complete across all 6 phases.**
