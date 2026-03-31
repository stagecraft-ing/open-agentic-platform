# 053 Phase 2 Review — Skill Library & Resolution

**Reviewer:** claude
**Date:** 2026-03-31
**Verdict:** ✅ APPROVED — Phase 2 scope satisfied. No blockers for Phase 3.

## Scope Assessed

Phase 2: Skill library discovery, platform defaults, merge with local-overrides-default precedence, and skill reference resolution.

**Files reviewed:**
- `packages/verification-profiles/src/defaults.ts` (bundled platform default skills)
- `packages/verification-profiles/src/loader.ts` (discovery, merge, resolution)
- `packages/verification-profiles/src/loader.test.ts` (13 Phase 2 tests)
- `packages/verification-profiles/src/index.ts` (barrel exports)
- `packages/verification-profiles/package.json` (subpath exports)

## Requirements Verification

| Requirement | Status | Evidence |
|-------------|--------|----------|
| **FR-006** — Skills resolved by name from library (local `.verification/skills/` + platform defaults) | ✅ | `loadSkillLibrary()` at loader.ts:25–82 discovers local files, merges with defaults; `resolveSkillRef()` at loader.ts:88–105 resolves by name |
| **FR-002** — Skill schema (name, description, determinism, safety_tier, steps) | ✅ | Defaults at defaults.ts:10–53 conform to `VerificationSkill` type; local files parsed via `parseSkillFile` (Phase 1) |
| **FR-003** — Step properties (command, timeout, read_only, network) | ✅ | All 3 default skills specify all 4 step properties |
| **NF-001** — Malformed files produce clear error messages with line numbers | ✅ | Invalid files parsed via `parseSkillFile` which uses `yaml.parseDocument()` (Phase 1); errors flow into `diagnostics` array (loader.ts:61) |
| **R-004** — Local overrides platform defaults on name collision | ✅ | loader.ts:75–79: starts with defaults map, overlays local skills. Test at loader.test.ts:139–149 verifies "lint" override replaces default |

## Implementation Quality

### `defaults.ts`
- 3 sensible platform defaults: `lint` (`npm run lint`), `type-check` (`npx tsc --noEmit`), `unit-tests` (`npm test`).
- All marked `read_only: true`, `network: "deny"` — correct safety posture for default verification skills.
- `getDefaultSkills()` returns a fresh `Map` each call — no shared mutable state. Clean.

### `loader.ts`
- **Discovery:** `readdir` + filter for `.yaml`/`.yml` extensions + `.sort()` for deterministic order. Correct.
- **Error handling:** `readdir` failure (no directory) gracefully returns defaults only (loader.ts:36–38). Per-file `readFile` failure emits `VP_SKILL_READ_ERROR` diagnostic and continues (loader.ts:49–57). Invalid YAML emits parser diagnostics and continues (loader.ts:61). One bad file doesn't block others — correct resilience pattern.
- **Duplicate detection:** `VP_SKILL_DUPLICATE_NAME` warning on local name collision (loader.ts:63–70). Later file (alphabetically) wins due to `.sort()`. Deterministic behavior — good.
- **Merge:** `new Map(defaults)` then overlay local — simple, correct implementation of R-004.
- **Resolution:** `resolveSkillRef` returns skill or null + diagnostic. `VP_SKILL_NOT_FOUND` message helpfully lists available skills (loader.ts:101). Clean API.
- **`SkillLibrary` type:** Exported interface with `skills: Map<string, VerificationSkill>` and `diagnostics` — appropriate data structure.

### `loader.test.ts`
13 tests covering:
1. Default skills content verification
2. No-directory fallback → defaults only
3. Empty-directory fallback → defaults only
4. YAML discovery (2 local + 3 defaults = 5)
5. `.yml` extension support
6. Non-YAML files ignored
7. Local overrides default (R-004) — verifies description and command changed
8. Invalid file diagnostics without blocking valid files
9. Duplicate local name warning
10. Resolve hit
11. Resolve miss with diagnostic
12. Resolve miss lists available skills
13. Empty library returns "(none)"

All 13 meaningful, all pass. Good coverage of happy path, error paths, and edge cases.

### Barrel exports & package.json
- `index.ts` correctly re-exports `getDefaultSkills`, `loadSkillLibrary`, `resolveSkillRef`, and `SkillLibrary` type.
- `package.json` adds subpath exports for `./defaults` and `./loader` — consistent with Phase 1 pattern.

## Validation

- **Tests:** 38/38 pass (25 Phase 1 + 13 Phase 2)
- **Types:** `tsc --noEmit` clean, zero errors
- **No regressions:** Phase 1 parser tests unaffected

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| P2-001 | LOW | `getDefaultSkills()` allocates a new Map on every call. Not a problem now, but if called in a hot loop (e.g., resolving 50+ profiles), could be memoized. NF-003 (50 skills) is about library size not call frequency, so this is fine for now. |
| P2-002 | LOW | `resolveSkillRef` returns `filePath: ""` in the not-found diagnostic (loader.ts:103). This is a reasonable sentinel but differs from other VP_ diagnostics that point to a file. Could use the profile file path that triggered the resolution. Phase 3+ can address when profile execution calls this. |
| P2-003 | INFO | No test for `VP_SKILL_READ_ERROR` — the read-error path (loader.ts:49–57) is tested implicitly through the invalid-file test, but not via an actual filesystem read failure (e.g., permission denied). Low risk given the catch-all `catch` block. |
| P2-004 | INFO | `readdir` errors other than "directory doesn't exist" (e.g., permission denied on the directory itself) are silently treated as "no directory" (loader.ts:35–38). Acceptable for now — the catch block doesn't distinguish ENOENT from EACCES. |
| P2-005 | INFO | Duplicate name warning only fires for local-vs-local collisions. A local skill overriding a platform default is silent (by design per R-004). This is the correct behavior but worth noting — no diagnostic trail when a default is shadowed. |
| P2-006 | INFO | Default skills assume `npm`-based toolchain. Phase 6 will flesh these out per the spec. Noted for carry-forward. |

## Carry-Forward

- **P1-001** (diag() hardcodes severity "error") — still open, LOW
- **P2-002** (empty filePath on not-found diagnostic) — LOW, addressable in Phase 3
- **P2-006** (defaults assume npm) — INFO, Phase 6 scope

## Phase 3 Readiness

Phase 3 (Execution Engine) depends on:
1. ✅ Skill schema types (`VerificationSkill`, `VerificationStep`) — Phase 1
2. ✅ Skill resolution (`resolveSkillRef`) — Phase 2
3. ✅ Result types (`StepResult`, `SkillResult`) — Phase 1

Phase 3 will implement `child_process.spawn`-based step execution with timeout, read_only advisory logging, network policy enforcement, and structured results. No blockers from Phase 2.
