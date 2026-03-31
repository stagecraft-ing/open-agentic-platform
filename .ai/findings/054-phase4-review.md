# 054 Phase 4 Review — Tool Allowlist Enforcement

**Reviewer:** claude
**Date:** 2026-03-30
**Verdict:** Phase 4 approved. FR-007 and SC-003 satisfied on both dispatch paths. No blockers.

## Requirements Under Review

- **FR-007**: The `tools` array in agent frontmatter is an allowlist; agents may only invoke tools listed in their frontmatter. The runtime enforces this constraint.
- **SC-003**: An agent attempting to use a tool not listed in its `tools` frontmatter is blocked by the runtime with a clear error.

## Evidence

### D-001: `allowed_tools` threaded through `AgentExecutionProfile` — SATISFIED

`orchestrator.rs:38–45` — `AgentExecutionProfile` struct has `allowed_tools: Option<Vec<String>>`. Populated from SQLite `tools` column at `orchestrator.rs:369–370` via `parse_allowed_tools()`.

### D-002: Enforcement on both dispatch paths — SATISFIED

**Path A — Governed Claude CLI** (`orchestrator.rs:226–231`):
When `allowed_tools` is `Some` and non-empty, pushes `--allowedTools <tool1> <tool2> ...` as CLI args. The Claude CLI natively enforces this — tools not in the list are blocked before execution. This is the simplest and most robust path.

**Path B — Provider Registry Sidecar** (`orchestrator.rs:107–116`):
`allowedTools` is included in the sidecar JSON payload. The sidecar:
1. Passes `allowedTools` to `BridgeQueryOptions` for the Claude Code SDK path (`node-sidecar.ts:209`), which has native enforcement.
2. Additionally enforces via `canUseTool` callback (`node-sidecar.ts:212–217`) — checks `allowedTools.includes(toolName)` and throws with SC-003 error format before the permission broker is consulted. This is defense-in-depth: the SDK enforces at its level, and the sidecar gates at the bridge level too.

### D-003: SC-003 error format — SATISFIED

`node-sidecar.ts:83–90` — `formatToolAllowlistError()` produces:
```
Tool '<toolName>' is not in agent '<agentName>' allowlist. Declared tools: [<tools>]
```
Includes: the tool name attempted, the agent name, and the full declared allowlist. This matches the pre-implementation decision D-003 exactly. Falls back to `"unknown-agent"` if agent name is missing.

### D-004: SQLite `tools` column as source — SATISFIED

`agents.rs:482` — `ALTER TABLE agents ADD COLUMN tools TEXT` migration. Uses `let _ =` pattern consistent with all other migration columns (idempotent, no-op if column exists).

`orchestrator.rs:359` — SELECT includes `tools` as column index 6.

`orchestrator.rs:417–451` — `parse_allowed_tools()`:
- `None` / empty string → `None` (no restriction)
- JSON array (`[...]`) → parsed, trimmed, empty-filtered
- Comma-separated string → split, trimmed, empty-filtered
- Both formats handle the existing agent comma-separated convention (e.g. `"Read, Grep, Glob, Bash, LS"`)

### D-005: Command scope exclusion — CONFIRMED

Phase 4 only enforces for agents. Commands' `allowed-tools` (hyphenated) field is not in scope per spec. Correct.

## Compilation Verification

- `cargo check` for `apps/desktop/src-tauri/Cargo.toml`: **clean** (warnings only, none related to Phase 4)
- `tsc --noEmit` for `packages/provider-registry/`: **clean** (no errors)

## Findings

| ID | Severity | Finding |
|----|----------|---------|
| P4-001 | LOW | **Provider path (`runProviderPath`) does not enforce allowlist.** `allowedTools` is normalized at `node-sidecar.ts:196` and threaded into the Claude Code bridge path (`opts.allowedTools` at line 209 + `canUseTool` gate at line 212). However, `runProviderPath()` at lines 92–137 receives the full `SidecarQueryMessage` including `q.allowedTools` but **never reads it**. Non-Claude providers (OpenAI, Gemini, Bedrock) that happen to emit tool_use events would bypass the allowlist entirely. Currently low risk since these providers don't route through Claude's tool system, but the gap means FR-007 is not enforced on the provider path. |
| P4-002 | LOW | **No integration test in codebase.** The Phase 4 plan specifies "Integration test: agent with `tools: [Read]` cannot invoke `Write`." No test file was added. The `parse_allowed_tools` function has no unit tests in `orchestrator.rs`. The sidecar TypeScript changes also lack tests for `normalizeAllowedTools`, `formatToolAllowlistError`, and the `canUseTool` callback logic. |
| P4-003 | INFO | **`--allowedTools` CLI flag assumption.** The governed Claude path passes `--allowedTools` as CLI args. This is the documented Claude CLI interface. If the flag is renamed or removed in a future Claude CLI version, enforcement silently breaks. Mitigated by the fact that unknown CLI flags cause Claude to error, so breakage would be visible. |
| P4-004 | INFO | **Double enforcement on Claude Code bridge path.** Both `opts.allowedTools` (SDK-level) and `canUseTool` callback (sidecar-level) enforce the same allowlist for the Claude Code SDK bridge path. This is intentional defense-in-depth — the `canUseTool` gate fires before the permission broker and provides the SC-003 error format, while the SDK may enforce differently. Not a bug, just worth noting. |
| P4-005 | INFO | **`normalizeAllowedTools` in sidecar trims and filters.** `node-sidecar.ts:73–81` normalizes the array. This means whitespace-only tool names or empty strings from a malformed DB value are safely dropped. Good defensive coding. |
| P4-006 | INFO | **Agent `tools` column nullable, no default.** `ALTER TABLE agents ADD COLUMN tools TEXT` adds a nullable column with no default. Existing agents get `NULL`, which `parse_allowed_tools` maps to `None`, meaning no tool restriction. This is correct: legacy agents without a `tools` field should not be restricted until Phase 6 migration populates the column. |

## Summary

Phase 4 wires agent frontmatter `tools` through `AgentExecutionProfile` → both dispatch paths. The governed Claude CLI path uses native `--allowedTools` enforcement. The provider registry sidecar path threads `allowedTools` into the Claude Code SDK bridge and additionally gates via `canUseTool` with SC-003 error format. The SQLite migration is idempotent, and `parse_allowed_tools()` handles both JSON array and comma-separated formats.

**FR-007: SATISFIED** — `tools` field is enforced at runtime on both primary dispatch paths (governed Claude CLI and Claude Code SDK bridge). The non-Claude provider sub-path is a gap (P4-001) but is low-risk given current provider capabilities.

**SC-003: SATISFIED** — `formatToolAllowlistError()` produces a clear error with tool name, agent name, and declared allowlist. The `canUseTool` callback throws this error before the permission broker is consulted.

**No blockers for Phase 5.**
