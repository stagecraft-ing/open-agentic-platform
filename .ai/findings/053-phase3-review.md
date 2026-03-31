# 053 Phase 3 Review — Execution Engine

**Reviewer:** claude
**Date:** 2026-03-31
**Verdict:** ✅ APPROVED — no blockers for Phase 4

## Scope

Phase 3 delivers the execution engine in `packages/verification-profiles/src/runner.ts` with three functions: `executeStep`, `executeSkill`, `executeProfile`. Reviewed against FR-007 (execution engine), FR-008 (failed gated profile blocks delivery), SC-001 (ordered execution with structured report), SC-003 (timeout enforcement), SC-004 (network deny), and findings R-005 (best-effort network on macOS).

## FR/NF Satisfaction

| Requirement | Status | Evidence |
|-------------|--------|----------|
| **FR-007** — execution engine runs skills in declared order, enforces timeouts, respects constraints, structured results | ✅ Satisfied | `executeSkill` runs steps sequentially with fail-fast (L121–127). `executeProfile` runs resolved skills in declared order (L178–184). `executeStep` enforces timeout via SIGTERM→SIGKILL (L64–70). `network: "deny"` sets `NO_PROXY=*` (L41–44, advisory per R-005). `read_only` advisory (documented L30). `StepResult` captures command, exitCode, passed, durationMs, timedOut, stdout, stderr — fully structured. |
| **FR-008** — failed skill in gated profile blocks delivery and reports details | ✅ Satisfied | `executeProfile` collects `failedSkills` (L183), `passed = failedSkills.length === 0` (L186). Profile continues through all skills after failure to produce complete report (L178–184). |
| **SC-001** — profile executes all steps in order with structured pass/fail report | ✅ Satisfied | Test "runs all skills and reports pass when all succeed" (runner.test.ts L146–159): two skills resolved and run in order, `ProfileResult` returned with skills array, failedSkills, profile name. |
| **SC-003** — timeout enforcement kills long-running steps | ✅ Satisfied | Test "kills process on timeout and marks timedOut" (runner.test.ts L57–63): 0.3s timeout with 200ms grace, timedOut=true, passed=false. SIGTERM at timeout, SIGKILL after grace (runner.ts L64–70). |
| **SC-004** — `network: deny` skills cannot make outbound requests | ✅ Partial (advisory) | `NO_PROXY=*` and `no_proxy=*` set in environment (L41–44). Test verifies env var propagation (runner.test.ts L66–71). Per R-005, this is best-effort on macOS — no OS-level enforcement. Acceptable per readiness review consensus. |
| **NF-002** — < 1s overhead beyond step execution | ✅ Satisfied | Test suite runs 15 tests in 696ms total including two 300ms timeout tests. Overhead per step is negligible (spawn + Buffer concat). |

## Architecture Assessment

### `executeStep` (L32–108)
- Correct use of `child_process.spawn` with `shell: true` for command string execution.
- `stdio: ["ignore", "pipe", "pipe"]` — stdin ignored, stdout/stderr piped. Correct.
- Buffer-based capture avoids string encoding issues during collection; `.toString("utf-8")` at close. Good.
- Timeout chain: `setTimeout(timeoutMs)` → SIGTERM → `setTimeout(killGraceMs)` → SIGKILL. Both timers cleaned on close and error. No leak.
- `error` event handler resolves with `passed: false`, `exitCode: null`, stderr set to error message. Correct — prevents unhandled promise rejection.
- `env` merge: `{ ...process.env, ...opts?.env }` then conditional `NO_PROXY` addition. Order correct — explicit env overrides inherited, then network policy applied last.

### `executeSkill` (L114–135)
- Sequential loop with early break on `!result.passed`. Correct fail-fast behavior per FR-007.
- `passed` derived from `steps.every(s => s.passed)` — consistent with break logic but also handles edge case of empty steps array (returns true, which is correct: vacuously passing).

### `executeProfile` (L145–195)
- Two-phase design: resolve all refs first, then execute. Good — fails early on missing skills without partial execution side effects.
- Resolution errors: pushes to `failedSkills` and `resolutionErrors`, returns immediately with `passed: false` and empty `skills` array. Clean.
- Execution phase: continues through failures, collecting `failedSkills`. Correct for complete report generation.
- `passed = failedSkills.length === 0` — straightforward gate logic.

### Exports
- Barrel `index.ts` exports all three functions and `ExecutionOptions` type. ✅
- `package.json` has `"./runner": "./src/runner.ts"` subpath export. ✅

## Test Coverage Assessment

15 tests covering:
- **executeStep** (7): success, failure, timeout, network deny, network allow, cwd override, spawn error
- **executeSkill** (3): all-pass sequential, fail-fast at second step, timeout-stop
- **executeProfile** (5): all-pass, failed-skill recording, continue-after-failure, unresolved-ref early exit, empty-skills edge case

Coverage is thorough for the execution engine phase. Key behavioral contracts (fail-fast, continue-through-failure, timeout chain, env propagation) all have dedicated tests.

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| **P3-001** | LOW | `exitCode` may be `null` on signal kill (runner.ts L79: `const exitCode = code`) but `StepResult.exitCode` is typed `number \| null` — correct. However, on some platforms `code` can be `null` and `signal` is the informative field. Signal is not captured in `StepResult`. |
| **P3-002** | LOW | No `maxBuffer` equivalent — stdout/stderr Buffers grow unbounded. A pathological step could OOM the host. Not a blocker at current scale but worth noting for Phase 6 bundled skills discussion. |
| **P3-003** | INFO | `read_only` is declared advisory but has zero runtime effect — not even logged. The comment says "logged but not enforced" but there's no logging. Cosmetic discrepancy in comment at L30. |
| **P3-004** | INFO | `executeProfile` doesn't use `profile.gate` in its logic — `passed` is always `failedSkills.length === 0` regardless of gate value. The gate semantics (blocking delivery) are presumably consumed by the caller (Phase 4 post-session gates), not the engine itself. This is acceptable separation of concerns. |
| **P3-005** | INFO | The test for spawn error (runner.test.ts L89–94) checks `passed: false` and `exitCode !== 0` but `__nonexistent_command_xyz__` with `shell: true` returns exit code 127 (command not found) via the shell — it's not actually a spawn `error` event. A true spawn error test would need e.g. an invalid `cwd`. Low impact — the error handler at L93–106 is still structurally correct. |
| **P3-006** | INFO | `env` type cast `as NodeJS.ProcessEnv` at L54 is necessary because `opts?.env` is `Record<string, string>` but `process.env` values are `string | undefined`. The merged type is `Record<string, string | undefined>` which doesn't match spawn's `ProcessEnv`. Cast is safe — spawn handles undefined values by omitting them. |

## Validation

- `tsc --noEmit`: clean
- `vitest run`: 53/53 pass (25 P1 + 13 P2 + 15 P3)
- No new dependencies introduced
- No regressions in prior phase tests

## Verdict

**Approved.** FR-007, FR-008, SC-001, SC-003 satisfied. SC-004 satisfied at advisory level per R-005. All 6 findings are LOW/INFO severity with no blockers. Phase 4 (post-session gates) may proceed.
