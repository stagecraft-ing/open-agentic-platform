# 053 Verification Profiles — Phase 1 Review (Schema Definition & Package Scaffold)

**Reviewer:** claude
**Date:** 2026-03-31
**Base commit:** ab8202e
**Verdict:** Phase 1 approved. Schema types, validation, and parsing faithfully implement FR-001, FR-002, FR-003, and NF-001. 25/25 tests pass, `tsc` clean.

## Requirement Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| FR-001 (profile schema: name, optional description, gate, skills) | **Satisfied** | `types.ts:44-53` — `VerificationProfile` with `name`, `description?`, `gate`, `skills`. `schema.ts:40-88` — `validateProfileObject` checks all fields. Tests: valid profile, minimal profile, missing name/gate/skills, empty skills, bad skill ref. |
| FR-002 (skill schema: name, description, determinism, safety_tier, steps) | **Satisfied** | `types.ts:30-41` — `VerificationSkill` with all required fields. `schema.ts:94-155` — `validateSkillObject` checks all fields + enum validation for determinism/safety_tier. Tests: valid skill, cautious skill, missing name/description, invalid enums. |
| FR-003 (step properties: command, timeout, read_only, network) | **Satisfied** | `types.ts:18-27` — `VerificationStep` with all 4 properties. `schema.ts:157-202` — `validateStepObject` validates each. Tests: missing command, bad timeout (0), missing read_only, invalid network policy, multiple errors at once. |
| NF-001 (line-number errors on malformed YAML) | **Satisfied** | `parser.ts:19-26` — `lineColFromYamlError` extracts line/column from `yaml` library. `parser.ts:38-45` — first parse error reported with position. Tests: "rejects invalid YAML syntax with line number" asserts `line` is defined. File path included in all diagnostics. |

## Architecture Assessment

### Strengths

1. **Spec fidelity** — Types match the YAML schemas in the spec exactly (profile: lines 77-87, skill: lines 93-106). Field names, types, and optionality all align.
2. **Clean separation** — `types.ts` (data), `schema.ts` (validation), `parser.ts` (YAML→typed objects). Each layer has a single responsibility.
3. **Composable diagnostics** — `VerificationDiagnostic` with `VP_`-prefixed codes follows the established 048/054 pattern. 13 distinct codes cover all failure modes.
4. **Source-position-aware parsing** — `parseDocument()` from the `yaml` package (R-003 from readiness review) correctly used for line-number errors.
5. **Result types ready** — `StepResult`, `SkillResult`, `ProfileResult` types defined for Phase 3 execution engine, avoiding a later schema change.
6. **Defensive validation** — Each step validated individually (`validateStepObject`), array element types checked, empty arrays rejected. Multiple errors collected and returned together (tested: "reports multiple step errors at once").

### Package Scaffold

- ESM (`"type": "module"`), strict TypeScript, vitest — consistent with 048/054 patterns.
- `yaml` ^2.8.1 as sole runtime dependency — appropriate.
- Subpath exports (`.`, `./types`, `./parser`, `./schema`) allow consumers to import selectively.
- `main`/`types` point to `src/index.ts` (source, not dist) — fine for monorepo internal consumption.

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| P1-001 | LOW | `diag()` helper (`schema.ts:26-34`) hardcodes `severity: "error"`. All validation diagnostics are errors today, but `DiagnosticSeverity` supports `"warning"` and `"info"`. Phase 2+ may want warnings (e.g., deprecated skill field). If needed, add a severity parameter later — no action now. |
| P1-002 | LOW | `objectToProfile` (`parser.ts:75-82`) trims skill names (`.map(s => s.trim())`), but validation (`schema.ts:74-84`) only checks `trim().length === 0` for rejection. A skill name like `" lint "` would pass validation and be silently trimmed to `"lint"`. Consistent behavior, but callers should be aware that resolved names won't match the raw YAML exactly. No action needed — trimming is defensive and correct. |
| P1-003 | INFO | No `tsconfig.json` `include` for test files — tests excluded via `"exclude": [".test.ts"]` in the build config but still type-checked by vitest. This is the expected setup. |
| P1-004 | INFO | The `diag` helper is exported from `schema.ts` and re-exported by `index.ts` (implicitly via `validateProfileObject`/`validateSkillObject`). It's also imported by `parser.ts`. The function is useful as-is but not documented as a public API. Phase 2 `loader.ts` will likely need it. No action. |
| P1-005 | INFO | YAML `parseDocument` warnings (not just errors) are not checked. The `yaml` library separates `doc.errors` (fatal) from `doc.warnings` (non-fatal). Currently only errors are surfaced. Warnings could be surfaced as `DiagnosticSeverity: "warning"` in a future enhancement. |
| P1-006 | INFO | `validateProfileObject` accepts `Record<string, unknown>` — extra/unknown keys are silently ignored. A profile YAML with `name: pr, gate: true, skills: [lint], typo_field: 123` would validate without complaint. This is intentional (forward-compatible) but worth noting: no strict-mode "unknown key" detection. |

## Test Coverage

25 tests across 2 `describe` blocks:
- **Profile parsing** (12 tests): valid full, valid minimal, YAML syntax error with line number, non-object YAML (array), null/empty YAML, missing name, missing gate, missing skills, empty skills, non-string skill ref, file path in diagnostics.
- **Skill parsing** (13 tests): valid multi-step, cautious skill, missing name, missing description, invalid determinism, invalid safety_tier, empty steps, missing command, bad timeout, missing read_only, invalid network, multiple errors, invalid YAML, file path in diagnostics.

**Coverage gap (LOW):** No test for `dangerous` safety tier or `non_deterministic` determinism. The `VALID_SKILL_CAUTIOUS` covers `mostly_deterministic`/`cautious`, but the third values in each enum are only validated via the `ReadonlySet` check. Low risk — enum validation is mechanical.

## Verdict

**Phase 1 approved.** FR-001, FR-002, FR-003, NF-001 satisfied at schema layer. Types, validation, and parsing are clean, well-tested, and faithfully implement the spec's YAML schemas. No blockers for Phase 2 (skill library & resolution).

Carry-forward to Phase 2: R-004 from readiness review (skill name collision precedence — local overrides platform defaults).
