# 048 Phase 3 Review — Action Executors

**Reviewer:** claude
**Date:** 2026-03-30
**Verdict:** Phase 3 approved
**Tests:** 25/25 passing (`pnpm --filter @opc/hookify-rule-engine test`)

## Requirements Satisfied

### FR-003 (block action)
`executeRuleAction()` at `actions.ts:288-298`: when `action.type === "block"`, returns `terminalDecision: "blocked"` with `blockedByRuleId` set to the matching rule ID. Rule ID pushed to `matchedRuleIds`. Short-circuit of further evaluation is the engine core's responsibility (Phase 4).

### FR-004 (warn action)
`actions.ts:300-309`: when `action.type === "warn"`, trimmed rationale appended to `warnings[]`, returns `terminalDecision: "allowed"`. Operation proceeds.

### FR-005 (modify action)
`actions.ts:312-323`: delegates to `applyModifyTransform()` which dispatches on `transform.type`. Three safe transforms implemented: `append_arg` (line 80-118), `replace_regex` (line 121-182), `set_field` (line 184-232). Payload is `structuredClone`'d before mutation (line 284) — original input is never modified.

### H-003 (modify safety envelope)
Only `append_arg`, `replace_regex`, `set_field` accepted. Unknown types produce `HKY_UNKNOWN_TRANSFORM` diagnostic (line 267-278) and skip non-fatally. No dynamic code execution paths exist.

### SC-001 (block git push --force)
`actions.test.ts:20-34`: Verified — `terminalDecision: "blocked"`, `blockedByRuleId: "block-force-push"`.

### SC-002 (warn + allow)
`actions.test.ts:36-50`: Verified — `terminalDecision: "allowed"`, warning emitted with rationale.

### SC-003 (modify payload)
`actions.test.ts:52-74`: Verified — `append_arg` appends `--dry-run`, matched rule ID tracked.

### Invalid transform skip
`actions.test.ts:76-95`: Verified — unknown `shell_eval` transform produces `HKY_UNKNOWN_TRANSFORM` diagnostic, `matchedRuleIds` empty, pipeline continues.

## Type additions

`ActionExecutionResult` (types.ts:90-97) adds `terminalDecision`, `matchedRuleIds`, and optional `blockedByRuleId` to the result envelope. `TerminalDecision` type (line 88) is `"allowed" | "blocked"`. Both also added to `EvaluationResult` (line 78-86) for forward compatibility with the engine core (Phase 4).

Exports updated in `index.ts:21` and `package.json` subpath export `./actions`.

## Findings

### P3-001 — block rationale not in ActionExecutionResult (LOW)
FR-003 says "returns the rule's markdown body as the block reason". The `blockedByRuleId` is set, but the rationale string itself is not included in the result. The engine core (Phase 4) can look up the rule by ID to retrieve it, or the caller can. Acceptable for a Phase 3 library-level API, but the final engine output (Phase 4) should include the rationale text.

### P3-002 — replace_regex flags parameter allows arbitrary flags (LOW)
`applyReplaceRegex` passes `flags` directly to `new RegExp(pattern, flags)`. This could allow `g` (global) or `s` (dotall) which might produce unexpected results. Not a security issue since RegExp itself is safe, but worth documenting supported flags.

### P3-003 — no replace_regex or set_field test coverage (LOW)
`replace_regex` and `set_field` transforms have no dedicated test cases. Only `append_arg` (SC-003) and unknown transform are tested. Phase 4 or Phase 6 should add coverage for these two transforms.

### P3-004 — setPathValue auto-creates intermediate objects (INFO)
`setPathValue` (line 56-78) creates intermediate `{}` objects when path segments don't exist. This is a reasonable default for `set_field` but means transforms can add deeply nested structures that didn't exist before. Intentional and acceptable.

### P3-005 — append_arg separator logic (INFO)
`applyAppendArg` (line 116) uses `" "` separator unless the target is empty/whitespace-only. Clean behavior — empty fields get value without leading space.

### P3-006 — structuredClone safety (INFO)
`clonePayload` uses `structuredClone` which handles nested objects/arrays but will throw on functions or symbols in the payload. Acceptable — hook event payloads should be JSON-serializable.

## No blockers for Phase 4.
