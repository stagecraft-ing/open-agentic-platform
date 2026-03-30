# 048 Hookify Rule Engine — phased implementation plan

> **Non-authoritative.** Planning scratch for agent coordination only. Canonical contract remains `specs/048-hookify-rule-engine/spec.md`.

## Goal

Implement Feature 048 as a deterministic, markdown-authored hook rule engine that evaluates Claude Code lifecycle events (PreToolUse, PostToolUse, UserPromptSubmit, Stop) with ordered rule execution and safe action handling (`block`, `warn`, `modify`), plus plugin manifest integration via `hooks.json`.

## Pre-implementation decisions (H-001 to H-006)

- **H-001 (package location):** implement as a standalone workspace package at `packages/hookify-rule-engine/` to keep runtime hook behavior decoupled from app-specific UI and aligned with the spec architecture.
- **H-002 (condition model):** support both a flat list form (`conditions: [{ field, ... }]`) and explicit boolean trees (`all`, `any`, `not`) in parsed rule structures; normalize internally to one AST for deterministic evaluation.
- **H-003 (modify safety envelope):** ship deterministic built-in transforms first (`append_arg`, `replace_regex`, `set_field`) and reject unknown transform kinds with diagnostics; no dynamic code execution in rules.
- **H-004 (hot-reload model):** use atomic ruleset snapshots per evaluation cycle (`Arc`-style immutable reference semantics in TS via copy-on-write replacement) so in-flight evaluations never observe partial reloads.
- **H-005 (hooks manifest strategy):** generate `hooks.json` from one canonical event map emitted by the package and support check/update mode to avoid drift between generated and committed manifest.
- **H-006 (diagnostics contract):** all parse/validation/evaluation errors are surfaced as structured diagnostics with rule `id`, file path, severity, and code; invalid rules are skipped, never fatal to the pipeline (FR-009).

## Implementation slices

### Phase 1 — Types, parser, and rule schema validation (FR-001, FR-009, NF-002, NF-003)

Deliverables:
- Scaffold `packages/hookify-rule-engine/` with:
  - `src/index.ts`, `types.ts`, `parser.ts`
  - test harness and fixtures for markdown rules
- Define core types:
  - `HookEventType`, `Rule`, `Matcher`, `ConditionNode`, `Action`, `EvaluationResult`, `Diagnostic`
- Implement markdown + YAML frontmatter parser:
  - parse frontmatter fields (`event`, `matcher`, `conditions`, `action`, `priority`)
  - map markdown body to human rationale text
- Add validation rules:
  - missing required fields
  - invalid action type
  - invalid event name
  - malformed condition object/tree
  - duplicate rule IDs in loaded set

Validation:
- Unit tests for valid parse, malformed YAML, missing fields, and invalid action/event variants.
- Fixtures proving markdown body is preserved as user-facing rationale (NF-002).
- Pure function tests for parser/validator without runtime session dependency (NF-003).

### Phase 2 — Matcher + condition language evaluator (FR-006, FR-002 partial)

Deliverables:
- Implement `matcher.ts` for event filtering by:
  - lifecycle event type
  - payload selectors (tool name, input/output fields)
- Implement `conditions.ts` evaluator:
  - field access (`tool.name`, `input.command`, etc.)
  - operators (`==`, `!=`, `contains`, `matches`)
  - glob support
  - regex support
  - boolean combinators (`AND`/`OR`/`NOT`) via normalized AST
- Deterministic truth evaluation:
  - undefined field reads evaluate false with diagnostic trace (no throw)
  - stable short-circuit behavior for boolean nodes

Validation:
- Operator matrix tests for all comparisons.
- Boolean expression tests covering nested `AND`/`OR`/`NOT`.
- Event matcher tests for each lifecycle event and payload field path.

### Phase 3 — Action executors (`block`, `warn`, `modify`) (FR-003, FR-004, FR-005, FR-002 partial)

Deliverables:
- Implement `actions.ts` executors:
  - `block`: return denied outcome with markdown rationale and stop flag
  - `warn`: append warning output while preserving allow flow
  - `modify`: apply transform in `action.transform` and emit modified payload
- Implement safe modify transforms (H-003):
  - `append_arg` (string payload paths)
  - `replace_regex`
  - `set_field`
- Add action result envelope:
  - resulting payload
  - emitted warnings
  - terminal decision (`blocked`/`allowed`)
  - matched rule IDs for observability

Validation:
- SC-001 fixture: `git push --force` blocked with rule rationale.
- SC-002 fixture: warning emitted, operation allowed.
- SC-003 fixture: command payload modified (e.g., appends `--dry-run`).
- Negative tests for invalid transform definitions -> diagnostic + skip rule.

### Phase 4 — Engine core orchestration + priority ordering (FR-002, FR-003, FR-004, FR-005, FR-009)

Deliverables:
- Implement `evaluate()` in `index.ts`:
  - filter by event type
  - sort by `priority` ascending (lowest first)
  - evaluate matcher + conditions
  - execute action
  - short-circuit on `block`
- Preserve deterministic rule ordering on equal priority:
  - tie-break by rule ID then source path
- Surface evaluation report:
  - matched rules
  - skipped-invalid rules with diagnostic codes
  - warnings and final payload state

Validation:
- Multi-rule ordering tests (priority + tie-break determinism).
- Short-circuit test proving post-block rules do not execute.
- Invalid rule skip tests confirming non-fatal pipeline behavior (FR-009, SC-005).

### Phase 5 — Loader, configurable rule directory, and hot-reload (FR-008, FR-009, SC-004)

Deliverables:
- Implement `loader.ts`:
  - configurable directory input (default `.claude/hooks/rules/`)
  - recursive markdown rule discovery
  - parse + validate to ruleset snapshot
- Implement file-watch reload loop:
  - add/remove/change triggers rebuild of immutable ruleset snapshot
  - diagnostics retained for skipped invalid files
- Add API entrypoints:
  - `loadRules(config)`
  - `getRulesSnapshot()`
  - `startHotReload() / stopHotReload()`

Validation:
- SC-004 integration test: adding a new rule file takes effect on next event.
- Change and delete tests proving snapshot replacement behavior.
- Race-safety tests ensuring in-flight evaluation uses prior stable snapshot.

### Phase 6 — `hooks.json` manifest + built-in starter rules + verification artifact (FR-007, SC-006, NF-001)

Deliverables:
- Implement `hooks-json.ts`:
  - generate manifest entries for all 4 lifecycle events
  - `writeHooksManifest({ check?: boolean })` for enforce/check workflows
- Add built-in rule library under `packages/hookify-rule-engine/rules/`:
  - `block-force-push.md`
  - `block-credential-read.md`
  - `warn-large-write.md`
- Add benchmark path for rule evaluation performance:
  - 100-rule fixture target, p99 <10ms/event (NF-001)
- Produce canonical evidence doc:
  - `specs/048-hookify-rule-engine/execution/verification.md`

Validation:
- SC-006 test verifying manifest registers all four lifecycle events.
- End-to-end test path showing event -> rules -> action result through generated manifest command shape.
- Performance test/benchmark showing p99 latency target compliance (NF-001).

## Proposed file touchpoints (initial)

- `packages/hookify-rule-engine/` (new package: parser, matcher, conditions, actions, loader, hooks manifest)
- `.claude/hooks/rules/` (default runtime rule directory if repo-level install is used)
- `hooks.json` (generated/maintained plugin manifest, location decided during implementation integration)
- `specs/048-hookify-rule-engine/execution/verification.md` (verification evidence once phases complete)

## Risks and guardrails

- Keep the condition grammar intentionally bounded in Phase 1-4; avoid introducing scripting semantics.
- Treat hot reload as snapshot replacement, not mutable shared state, to avoid race-driven nondeterminism.
- Ensure modify transforms are explicit and typed; no eval/dynamic code paths.
- Keep `.ai/` artifacts coordination-only; promote durable implementation truth to `specs/.../execution` and code.
