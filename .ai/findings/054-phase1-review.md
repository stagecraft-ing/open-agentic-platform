# 054 Phase 1 Review — Schema Definition

**Reviewer**: claude
**Date**: 2026-03-30
**Verdict**: Phase 1 **approved** — no blockers for Phase 2.

## Artifacts reviewed

| Artifact | Path |
|----------|------|
| Agent frontmatter schema | `schemas/agent-frontmatter.schema.json` |
| Skill frontmatter schema | `schemas/skill-frontmatter.schema.json` |
| Phased plan | `.ai/plans/054-agent-frontmatter-schema-phased-plan.md` |
| Spec | `specs/054-agent-frontmatter-schema/spec.md` |
| Existing agents | `.claude/agents/{architect,explorer,implementer,reviewer}.md` |

## Requirement coverage

| Requirement | Status | Notes |
|-------------|--------|-------|
| FR-005 (JSON Schema for both) | **Satisfied** | Both schemas present at `schemas/`, draft-07, valid JSON |
| FR-001 (required fields) | **Satisfied** | `required: ["name", "description", "tools", "model"]` matches spec |
| FR-002 (optional fields) | **Satisfied** | All 7 optional fields present: `category`, `color`, `displayName`, `version`, `author`, `tags`, `priority` |
| FR-003 (skill required fields) | **Satisfied** | `required: ["name", "description"]` matches spec |
| NF-003 (forwards-compatible) | **Satisfied** | `additionalProperties: true` on both schemas; A-002 decision sound |

## Plan assessment

- **Phase ordering**: Matches spec's implementation approach exactly (schema → parser → loader → allowlist → linter → migration). Sound.
- **A-001..A-004 decisions**: All reasonable and spec-faithful. A-003 (bounded frontmatter reads for NF-001) is the right approach. A-004 (threading into 035/042) correctly identifies integration points.
- **Phase 1 spot-check note**: Plan correctly identifies that existing agents use comma-separated `tools` (e.g., `tools: Read, Grep, Glob, Bash, LS`) which YAML parses as a single string, not an array. Migration to array format is tracked for Phase 6.

## Findings

### P1-001 — `tools` schema allows empty array (LOW)

The agent schema defines `tools` as `type: array` with no `minItems` constraint. FR-008 requires "non-empty `tools` list for agents that use tools." This is correctly deferred to the Phase 5 linter (schema validates structure, linter enforces policy), but the schema could optionally include `minItems: 1` as a first-pass guard.

**Recommendation**: Leave as-is for now; Phase 5 linter is the right enforcement point since "agents that use tools" is a conditional rule JSON Schema can't express cleanly.

### P1-002 — Existing agents use comma-separated `tools` string (LOW)

All four `.claude/agents/*.md` files currently use `tools: Read, Grep, Glob, Bash, LS` which YAML interprets as a single string, not an array. Validating current agents against the schema would fail on `tools` type.

**Status**: Acknowledged in plan Phase 1 validation notes and Phase 6 migration. No action needed now.

### P1-003 — `description` minLength 1 vs FR-008 minimum 50 (INFO)

Schema uses `minLength: 1`; FR-008 specifies minimum 50 characters. Correctly separated: schema ensures non-empty, linter enforces quality threshold.

### P1-004 — Skill schema extends beyond FR-003 minimum (INFO)

Skill schema includes optional `tags`, `version`, `author` not explicitly listed in FR-003. These appear in the spec's skill example (line 128–130 shows `tags`) and are reasonable extensions consistent with NF-003 forwards-compatibility. Not a deviation.

### P1-005 — `color` pattern validation (INFO)

Schema correctly constrains `color` to `^#[0-9A-Fa-f]{6}$`. Good — prevents invalid hex values at schema level rather than deferring to linter.

### P1-006 — `LS` tool in existing agents (INFO)

Existing agents list `LS` as a tool name. This is not a standard Claude Code tool (the tool is accessed via `Bash`). Phase 6 migration should audit tool names against the actual tool catalog.

## Summary

Both schemas are spec-faithful, well-structured, and correctly scoped for Phase 1. The plan's 6-phase structure mirrors the spec's implementation approach with sound pre-implementation decisions. The key gap (comma-separated `tools` in existing agents) is tracked for Phase 6 migration. No blockers for Phase 2 (parser).
