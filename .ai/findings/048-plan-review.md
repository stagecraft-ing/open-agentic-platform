# 048 Hookify Rule Engine — plan review

> **Reviewer:** claude | **Date:** 2026-03-30
> **Plan:** `.ai/plans/048-hookify-rule-engine-phased-plan.md`
> **Spec:** `specs/048-hookify-rule-engine/spec.md`

## Verdict

**Plan approved for Phase 1 start.** All 18 requirements (9 FR + 3 NF + 6 SC) are covered across 6 phases. Phase ordering is sound (types → conditions → actions → engine core → loader → manifest+evidence). One MEDIUM finding (F-001) requires resolution before Phase 6; no blockers for Phase 1.

## Requirement coverage matrix

| Req | Phase(s) | Status |
|-----|----------|--------|
| FR-001 | 1 | ✓ Markdown + YAML frontmatter parser, all fields |
| FR-002 | 2, 4 | ✓ Matcher in Phase 2, priority ordering + evaluate loop in Phase 4 |
| FR-003 | 3, 4 | ✓ Block action in Phase 3, short-circuit in Phase 4 |
| FR-004 | 3 | ✓ Warn action with markdown body |
| FR-005 | 3 | ✓ Modify action with transform |
| FR-006 | 2 | ✓ All operators, glob, regex, AND/OR/NOT |
| FR-007 | 6 | ✓ hooks.json generation — but see F-001 |
| FR-008 | 5 | ✓ Configurable directory + hot-reload |
| FR-009 | 1, 4 | ✓ Validation in Phase 1, skip-invalid in Phase 4 |
| NF-001 | 6 | ✓ 100-rule benchmark |
| NF-002 | 1 | ✓ Markdown body preserved as rationale |
| NF-003 | 1 | ✓ Pure function tests, no runtime dependency |
| SC-001 | 3 | ✓ Block force push fixture |
| SC-002 | 3 | ✓ Warn fixture |
| SC-003 | 3 | ✓ Modify payload fixture |
| SC-004 | 5 | ✓ Hot-reload integration test |
| SC-005 | 4 | ✓ Invalid rule skip + diagnostic |
| SC-006 | 6 | ✓ All 4 lifecycle events registered |

## Pre-implementation decisions (H-001 to H-006)

All six decisions are sound and spec-faithful:
- **H-001** (packages/ location) consistent with existing monorepo structure (provider-registry, claude-code-bridge, etc.)
- **H-002** (flat + tree condition model) correctly supports FR-006 boolean combinators while preserving the flat shorthand shown in the spec example
- **H-003** (deterministic transforms only) aligns with R-001 mitigation and out-of-scope "custom action plugins"
- **H-004** (atomic snapshots) directly implements R-002 mitigation
- **H-005** (generate + check mode) supports FR-007 "generated or maintained"
- **H-006** (structured diagnostics, skip-never-fatal) matches FR-009

## Findings

### F-001 — CLI entry point and stdin/stdout protocol missing from plan (MEDIUM)

The spec's hooks.json integration section shows:
```json
{ "command": "hookify-rule-engine evaluate --event PreToolUse" }
```

Claude Code hooks invoke commands as shell processes. The event payload is passed via **stdin** as JSON; the hook response is read from **stdout**. The plan describes a TypeScript library with programmatic API (`loadRules()`, `evaluate()`, `getRulesSnapshot()`) but does not mention:

1. A CLI binary or `bin` entry in package.json
2. stdin JSON parsing of the event payload
3. stdout JSON formatting of the evaluation result (block reason, modified payload, warnings)
4. The exit code contract (non-zero for block? zero always?)

**Impact:** Without a CLI wrapper, the hooks.json manifest has no command to point at. This is likely Phase 6 scope but should be explicitly planned — it affects package.json `bin` field and the evaluate function's input/output contract.

**Recommendation:** Add a CLI entry point deliverable to Phase 6 (or split into Phase 5 alongside the loader). Define the stdin→evaluate→stdout protocol: input schema (event type + payload JSON), output schema (action result JSON), and exit code semantics.

### F-002 — Flat condition list semantics unspecified (LOW)

H-002 says the plan supports both flat list and boolean tree conditions. The spec example shows only the flat form:
```yaml
conditions:
  - field: input.command
    matches: "git push.*--force"
```

The plan should clarify: a flat conditions array is implicitly AND (all conditions must match). This is the natural reading but should be stated explicitly in the types or parser, since it affects Phase 2 normalization from flat list → AST.

### F-003 — Tie-breaking determinism is a plan-level addition (INFO)

Plan Phase 4 specifies tie-break by "rule ID then source path" for equal priority. The spec only says "lowest number = highest priority" and doesn't define tie-breaking. This is a reasonable addition that strengthens NF-003 (isolation/determinism). No action needed — just noting it as plan-originated, not spec-mandated.

### F-004 — `modify` transform set completeness (INFO)

H-003 defines three transforms: `append_arg`, `replace_regex`, `set_field`. The spec's FR-005 says "a transformation defined in the rule's `action.transform` field" without enumerating specific transforms. SC-003 requires appending `--dry-run` to a command, which `append_arg` covers. The set seems sufficient for Phase 3 but may need extension — the plan correctly gates this via "reject unknown transform kinds with diagnostics."

### F-005 — hooks.json location undecided (LOW)

The plan (line 152) says "location decided during implementation integration." The spec shows `hooks.json` at the repo root (implied by the Architecture section). Claude Code hook configuration typically lives in `.claude/settings.json` under a `hooks` key, or in a standalone hooks file. The plan should decide the canonical output path before Phase 6 — likely `.claude/hooks.json` or project root `hooks.json`.

### F-006 — No mention of the Stop event's payload shape (INFO)

The spec lists four events (PreToolUse, PostToolUse, UserPromptSubmit, Stop) but the `Stop` event has a different payload shape than tool events (no `tool.name`, no `input.command`). The plan's Phase 2 matcher tests should include a Stop event fixture to ensure the condition evaluator handles missing tool-specific fields gracefully (per H-006 / FR-009 — undefined field reads → false with diagnostic).

## Phase ordering assessment

The ordering is sound:
- **Phase 1** (types/parser) has no dependencies — correct starting point
- **Phase 2** (matcher/conditions) builds on Phase 1 types — correct
- **Phase 3** (actions) builds on Phase 2 matching — correct
- **Phase 4** (engine core) wires Phases 1–3 — correct orchestration phase
- **Phase 5** (loader/hot-reload) adds filesystem layer — correct late placement
- **Phase 6** (manifest/rules/evidence) is integration + verification — correct final phase

No phase reordering needed. Cursor may begin Phase 1 as planned.
