# 053 Verification Profiles — Pre-Implementation Readiness Review

**Reviewer:** claude
**Date:** 2026-03-31
**Base commit:** b1f9b28
**Verdict:** Spec is well-defined. Dependencies (048, 054, 052) are feature-complete and provide reusable patterns. Six Phase 1–6 sub-tasks scoped below with 8 findings (3 LOW, 5 INFO).

## Dependency Readiness

| Dep | Package | Status | What 053 uses |
|-----|---------|--------|---------------|
| 048 — Hookify Rule Engine | `@opc/hookify-rule-engine` | ✅ feature-complete | Pattern: YAML-with-frontmatter parsing, `evaluate()` engine, `RuleRuntime` hot-reload, `Diagnostic` error format, markdown rule files. 053 skills share the same "YAML config → validate → evaluate" pattern. |
| 054 — Agent Frontmatter Schema | `@opc/agent-frontmatter` | ✅ feature-complete | Pattern: `parseFrontmatter()` for YAML extraction, `Tier1/Tier2` progressive loader, `LintSummary` structured output, diagnostic codes (`AFS_*`). 053 skill files follow a similar frontmatter-driven schema. |
| 052 — State Persistence | `crates/orchestrator` (Rust) | ✅ feature-complete | Integration point: `dispatch_manifest_persisted` emits `workflow_completed` event — post-session gates hook in before this event fires (Phase 4). `SqliteWorkflowStore` + `EventBroadcaster` available for persisting verification results. |

## Spec Assessment

### Strengths

- **Clear schema examples** — Profile and skill YAML schemas are fully specified with realistic examples (lint, security-scan).
- **Well-scoped phases** — 6 phases follow the established pattern (schema → library → engine → integration → selection → bundled defaults).
- **Reusable patterns** — FR-001 through FR-003 map directly to the 048/054 parsing patterns; the execution engine (FR-007) mirrors the orchestrator dispatch loop.
- **Test-friendly design** — Step constraints (timeout, read_only, network) are individually testable properties.

### Architecture Decision: TypeScript Package (not Rust crate)

053 should be a TypeScript package (`@opc/verification-profiles`) because:

1. **Pattern alignment** — The dependency chain is TypeScript: 048 (`@opc/hookify-rule-engine`), 054 (`@opc/agent-frontmatter`), both in `packages/`.
2. **YAML parsing** — Both 048 and 054 use the `yaml` npm package with established error-position-preserving patterns.
3. **Shell execution** — Steps run shell commands; Node's `child_process.spawn` with timeout/signal handling is the natural fit.
4. **Orchestrator integration** — FR-004 (post-session gates) needs a bridge. The Rust orchestrator calls through `GovernedExecutor` trait; the TypeScript verification runner would be invoked at the session boundary in the TypeScript agent layer (e.g., `@opc/worktree-agents` or `@opc/claude-code-bridge`), not inside the Rust dispatch loop. The Rust dispatch emits `workflow_completed`; a TypeScript listener can gate on that event via SSE before marking delivery.

**Finding R-001 (INFO):** The spec's FR-004 says "the orchestrator blocks delivery" — but the Rust orchestrator has no concept of "delivery" beyond emitting `workflow_completed`. The actual delivery block (preventing merge/deploy/publish) must happen at the TypeScript layer that consumes orchestrator events. The Phase 4 implementation should clarify this boundary.

## Phase-by-Phase Implementation Scope

### Phase 1 — Schema Definition & Validation

**Deliverables:**
- `packages/verification-profiles/` package scaffold (`@opc/verification-profiles`)
- `src/types.ts` — `VerificationProfile`, `VerificationSkill`, `VerificationStep`, `SkillResult`, `ProfileResult`, `Determinism`, `SafetyTier`, `NetworkPolicy` types
- `src/schema.ts` — JSON Schema definitions for profile and skill YAML; `validateProfile()` and `validateSkill()` functions
- `src/parser.ts` — `parseProfileFile(content, filePath)` and `parseSkillFile(content, filePath)` using 054's `parseFrontmatter` pattern (or plain YAML since these aren't markdown files — see R-002)
- Tests: schema validation with valid/invalid YAML, error messages with file paths and line numbers (NF-001)

**Finding R-002 (LOW):** The spec shows skill/profile files as plain YAML (not markdown with frontmatter). This diverges from 048 (markdown rules) and 054 (markdown agents). The implementer must decide: (a) plain `.yaml` files as shown in spec, or (b) markdown with YAML frontmatter for consistency with the rest of the platform. Plain YAML is simpler and matches the spec literally. Recommendation: follow the spec — use plain `.yaml`.

**Finding R-003 (INFO):** The `yaml` package (v2.8.x) used by 048/054 provides `parseDocument()` with source position tracking. Reuse this for line-number error reporting (NF-001).

### Phase 2 — Skill Library & Resolution

**Deliverables:**
- `src/loader.ts` — `loadSkillLibrary(projectRoot)` discovers and parses `.verification/skills/*.yaml`; `resolveSkillRef(name, library)` resolves a skill reference
- `src/defaults.ts` — bundled platform default skills (can be stubs in Phase 2, fleshed out in Phase 6)
- Tests: discovery from filesystem, resolution by name, missing skill errors, duplicate name handling

**Finding R-004 (LOW):** FR-006 says skills resolve from "local project `.verification/skills/` directory or platform defaults." The spec doesn't define precedence when both exist with the same name. Recommendation: local overrides platform defaults (consistent with how `.claude/` overrides platform defaults elsewhere in OAP).

### Phase 3 — Execution Engine

**Deliverables:**
- `src/runner.ts` — `executeSkill(skill, opts)` runs steps in order; `executeProfile(profile, library, opts)` runs skills in order
- Step execution: `child_process.spawn` with `timeout` enforcement (SIGTERM → SIGKILL after grace period), `read_only` flag (advisory — log warning if step modifies tracked files), `network` policy (advisory on non-Linux — see R-005)
- `SkillResult` and `ProfileResult` structured output with per-step timing, stdout/stderr capture, exit codes
- Tests: successful execution, timeout killing, step failure stops skill, skill failure in gated profile

**Finding R-005 (LOW):** SC-004 requires "Skills with `network: deny` cannot make outbound network requests." The spec acknowledges (R-001) this needs OS-level sandboxing. On macOS, there's no `unshare` equivalent. Implementation options: (a) best-effort with env var hints (e.g., `NO_PROXY=*`), (b) skip enforcement and log advisory, (c) use `sandbox-exec` on macOS (deprecated but functional). Recommendation: implement as advisory with clear logging; document as best-effort. This is consistent with the spec's own risk mitigation.

### Phase 4 — Post-Session Gates

**Deliverables:**
- `src/gate.ts` — `evaluatePostSessionGate(profileName, projectRoot)` loads profile, resolves skills, executes, returns gate result
- Integration point in the TypeScript agent session layer — after an agent session completes, check for configured profile and run verification before marking delivered
- `GateResult` type: `{ passed: boolean; profile: string; results: SkillResult[]; failedSkills: string[] }`
- Tests: gated profile blocks on failure (FR-004, FR-008, SC-002), ungated profile passes through

**Finding R-006 (INFO):** The exact TypeScript integration point for FR-004 depends on how `@opc/worktree-agents` or `@opc/claude-code-bridge` manage session lifecycle. The implementer should check the `runAgent` / session completion path in those packages. The Rust orchestrator's `dispatch_manifest_persisted` is NOT the right place — it operates at the workflow level, not the individual agent session level.

### Phase 5 — Profile Selection

**Deliverables:**
- `src/selector.ts` — `selectProfile(context)` with context-based detection (PR branch → `pr`, release branch → `release`, explicit `--verify=<name>` override)
- `ProfileContext` type: `{ branch?: string; isPR?: boolean; isRelease?: boolean; explicit?: string }`
- Tests: context detection, explicit override precedence, fallback when no context matches

**Finding R-007 (INFO):** FR-005 mentions "detect PR context." In a local CLI environment, PR detection requires either git branch naming conventions or GitHub API calls. The implementer should keep detection simple: branch name pattern matching (`feature/*` → `pr`, `release/*` → `release`, `hotfix/*` → `hotfix`), with explicit `--verify` always winning.

### Phase 6 — Bundled Skills

**Deliverables:**
- `.verification/skills/` directory with bundled defaults: `lint.yaml`, `type-check.yaml`, `unit-tests.yaml`, `security-scan.yaml`, `license-check.yaml`
- `.verification/profiles/pr.yaml` and `.verification/profiles/release.yaml` as reference profiles
- `src/defaults.ts` updated with bundled skill definitions
- Verification: SC-001 (pr profile executes all steps), SC-006 (skill reuse across profiles)
- `specs/053-verification-profiles/execution/verification.md` — full FR/NF/SC evidence matrix

**Finding R-008 (INFO):** Bundled skills reference commands like `npm run lint`, `npx tsc --noEmit`, `npm test`. These assume a Node.js project. For a Rust-heavy monorepo like OAP, bundled defaults should also include Rust-appropriate variants (e.g., `cargo clippy`, `cargo test`). The implementer should provide both Node and Rust skill variants, or make the bundled defaults generic enough to detect the project type.

## Carry-Forward Findings from Dependencies

| Finding | Source | Impact on 053 |
|---------|--------|---------------|
| P4-001: `current_step_index` not persisted | 052 Phase 4 | None — 053 doesn't depend on step index |
| P5-008: Lagged subscriber recovery | 052 Phase 5 | None — 053 doesn't use SSE directly |
| P3-001: Epoch timestamps not ISO-8601 | 052 Phase 3 | INFO — if 053 persists verification timestamps via 052's store, format mismatch possible |

## Summary of Findings

| ID | Severity | Description |
|----|----------|-------------|
| R-001 | INFO | FR-004 "orchestrator blocks delivery" — delivery blocking happens at TypeScript layer, not Rust orchestrator |
| R-002 | LOW | Skill/profile files are plain YAML (not markdown frontmatter) — diverges from 048/054 pattern but matches spec literally |
| R-003 | INFO | Reuse `yaml` package `parseDocument()` for line-number error reporting (NF-001) |
| R-004 | LOW | Skill name collision precedence undefined — recommend local overrides platform defaults |
| R-005 | LOW | Network policy enforcement is best-effort on macOS — implement as advisory with logging |
| R-006 | INFO | TypeScript integration point for FR-004 needs session lifecycle discovery in worktree-agents/bridge |
| R-007 | INFO | PR context detection should use branch name patterns, not API calls |
| R-008 | INFO | Bundled skills assume Node.js — consider Rust variants for this monorepo |

## Verdict

**Ready for Phase 1 implementation.** No blockers. The spec is clear, dependencies are complete, and established package patterns (048, 054) provide a template for the new `@opc/verification-profiles` package. The implementer should start with the package scaffold and type definitions.
