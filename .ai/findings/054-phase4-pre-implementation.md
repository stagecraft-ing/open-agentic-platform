# 054 Phase 4 Pre-Implementation Analysis — Tool Allowlist Enforcement

**Analyst:** claude
**Date:** 2026-03-30
**Purpose:** Trace runtime paths, identify integration points, surface architectural decisions needed before Phase 4 implementation of FR-007 and SC-003.

## Requirement Summary

- **FR-007**: The `tools` array in agent frontmatter is an allowlist; agents may only invoke tools listed in their frontmatter. The runtime enforces this constraint.
- **SC-003**: An agent attempting to use a tool not listed in its `tools` frontmatter is blocked by the runtime with a clear error.

## Current State

### What exists (Phases 1–3)

1. **JSON Schemas** at `schemas/agent-frontmatter.schema.json` — `tools` is a required array of strings.
2. **Parser** at `packages/agent-frontmatter/src/parser.ts` — `normalizeToolsField()` handles both array and comma-separated string formats.
3. **Loader** at `packages/agent-frontmatter/src/loader.ts` — Tier 1 metadata includes parsed `tools` field. Types exported from index (P3-001 resolved).
4. **Existing agents** use comma-separated string format:
   - `architect.md`: `tools: Read, Grep, Glob, Bash, LS`
   - `explorer.md`: `tools: Read, Grep, Glob, Bash, LS`
   - `implementer.md`: `tools: Read, Write, Edit, Grep, Glob, Bash, LS`
   - `reviewer.md`: `tools: Read, Grep, Glob, Bash, LS`

### What does NOT exist

- No runtime enforcement: agent `tools` field is parsed but never checked against actual tool invocations.

## Two Dispatch Paths Requiring Enforcement

### Path A — Governed Claude CLI (`dispatch_via_governed_claude`)

**File:** `apps/desktop/src-tauri/src/commands/orchestrator.rs:200–270`

Flow: `dispatch_step()` → `dispatch_via_governed_claude()` → spawns `claude` CLI with `--mcp-config` pointing to axiomregent MCP server.

- Agent tools flow through axiomregent's `preflight_tool_permission()` (`crates/axiomregent/src/router/mod.rs:121–207`).
- **Existing enforcement layers:**
  1. Tier + permission checks (`permissions::check_tool_permission()` — `permissions.rs:56–85`)
  2. Policy kernel `gate_tool_allowlist()` (`crates/policy-kernel/src/lib.rs:156–192`) — reads from compiled policy bundles
  3. Static executor allowlist (`crates/agent/src/executor.rs:14–23`) — hardcoded internal tool names

- **Integration option A1:** Add `--allowedTools` CLI args to the spawned `claude` command. The Claude CLI supports `--allowedTools` natively. This is the simplest path — add the agent's `tools` list as CLI args at `orchestrator.rs:214–222`.

- **Integration option A2:** Inject a dynamic policy rule into axiomregent with `gate: "tool_allowlist"` and `allowed_tools` matching the agent frontmatter. More complex, benefits from existing audit trail.

### Path B — Provider Registry Sidecar (`dispatch_via_provider_registry`)

**File:** `apps/desktop/src-tauri/src/commands/orchestrator.rs:76–197`

Flow: `dispatch_step()` → `dispatch_via_provider_registry()` → spawns `node packages/provider-registry/dist/node-sidecar.js` with JSON on stdin.

- The sidecar JSON payload (`orchestrator.rs:106–113`) currently sends `prompt`, `systemPrompt`, `workingDirectory`, `model`, `permissionMode` — but **no `allowedTools` field**.
- Inside the sidecar, the Claude Code SDK adapter passes `allowedTools` from bridge options (`packages/claude-code-bridge/src/sdk-adapter.ts:48`). The OpenAI/Gemini/Bedrock adapters don't have native tool restriction — enforcement would need to be at the event level.

- **Integration option B1:** Add `allowedTools` to the sidecar JSON payload. The Claude Code SDK adapter already supports it. For non-Claude providers, the sidecar can filter at the response level (reject tool_use events for unlisted tools).

- **Integration option B2:** Use the `canUseTool` callback on the bridge (`packages/claude-code-bridge/src/types.ts:136`). This provides a per-call gate.

## Recommended Architecture

### Decision D-001: Thread `tools` through `AgentExecutionProfile`

Add an `allowed_tools: Option<Vec<String>>` field to `AgentExecutionProfile` (`orchestrator.rs:38–44`). Populate from the SQLite agents table (requires schema migration or frontmatter parse at load time). This makes the allowlist available to both dispatch paths.

### Decision D-002: Enforce on both paths

- **Governed Claude path:** Pass `--allowedTools <tool1> <tool2> ...` to the spawned `claude` CLI. This is the native, Claude-supported enforcement. Simplest and most robust — the SDK itself blocks disallowed tools before they execute.
- **Provider registry path:** Add `allowedTools` to the sidecar JSON payload. The Claude Code SDK adapter already wires it. For other providers, add a post-response filter in the sidecar that emits an error event when a disallowed tool is called.

### Decision D-003: Error format for SC-003

SC-003 requires "a clear error." Recommendation: `"Tool '<toolName>' is not in agent '<agentName>' allowlist. Declared tools: [<tools>]"`. Both paths should produce a user-facing message with the agent name, the attempted tool, and the declared allowlist for debuggability.

### Decision D-004: Where to source the allowlist

The agent frontmatter `tools` field is the source of truth (FR-007). Two options for getting it into the runtime:

1. **Parse frontmatter at dispatch time** — Load the agent `.md` file via `@opc/agent-frontmatter` parser, extract `tools`, thread into dispatch. This requires knowing the file path.
2. **Store in agents table** — Add a `tools` column to the SQLite agents table. The registration/migration (Phase 6) populates it. The orchestrator query (`orchestrator.rs:350`) already loads agent profiles from this table.

**Recommendation:** Option 2 (SQLite column) is more aligned with the existing architecture — agent profiles already come from the DB. Phase 6 migration will ensure all agents have their `tools` column populated. For Phase 4, the column can be added with the current 4 agents migrated inline.

### Decision D-005: What about commands?

Commands in `.claude/commands/*.md` use `allowed-tools` (hyphenated) which is a different field name from agents' `tools`. Phase 4 scope per spec is agent enforcement only. Command enforcement is out of scope but the patterns should be compatible for future alignment.

## Integration Checklist for Phase 4

1. **Schema:** Add `tools TEXT` column to SQLite agents table (or parse frontmatter on load).
2. **Profile:** Add `allowed_tools: Option<Vec<String>>` to `AgentExecutionProfile` struct.
3. **DB query:** Extend `orchestrator.rs:350` SELECT to include `tools` column; parse comma-separated or JSON array.
4. **Governed path:** At `orchestrator.rs:214–222`, if `profile.allowed_tools.is_some()`, push `--allowedTools` args.
5. **Provider path:** At `orchestrator.rs:106–113`, add `"allowedTools"` to sidecar JSON payload.
6. **Sidecar:** In `packages/provider-registry/src/node-sidecar.ts`, forward `allowedTools` to bridge options.
7. **Test:** Integration test: agent with `tools: [Read]` attempts `Write` → blocked with SC-003 error message.
8. **Package export:** Ensure `normalizeToolsField` from `@opc/agent-frontmatter` is available for any TypeScript-side normalization.

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Claude CLI `--allowedTools` flag may not exist or behave differently than expected | MEDIUM | Verify with `claude --help` or test before relying on it |
| Non-Claude providers (OpenAI, Gemini, Bedrock) have no native tool restriction | LOW | Post-response filtering in sidecar; these providers don't currently use Claude Code tools |
| SQLite schema migration for `tools` column | LOW | Simple ALTER TABLE; existing agents have 0 rows or can be re-inserted |
| Agent `LS` tool name doesn't match any standard tool | INFO | May be a legacy artifact — Phase 6 migration should audit |

## Existing Enforcement Layers (Context)

Phase 4 adds a **fourth enforcement layer** to the existing stack:

| Layer | Scope | Mechanism | Location |
|-------|-------|-----------|----------|
| 1. Tier + permissions | Per-lease | `check_tool_permission()` | `axiomregent/router/permissions.rs:56–85` |
| 2. Policy kernel | Per-repo | `gate_tool_allowlist()` from policy bundles | `policy-kernel/src/lib.rs:156–192` |
| 3. Executor allowlist | Global | Hardcoded `ALLOWLIST` | `agent/src/executor.rs:14–23` |
| **4. Agent frontmatter** | **Per-agent** | **FR-007: tools field** | **Phase 4 deliverable** |

Layer 4 is the most granular — it restricts tools per individual agent definition, complementing the broader policy and permission layers.

## Summary

Phase 4 requires threading the parsed `tools` field from agent frontmatter through two dispatch paths (governed Claude CLI and provider registry sidecar). The infrastructure for both paths already supports tool allowlists — the governed path can use `--allowedTools` CLI args, and the provider path can use `allowedTools` in bridge options. The main implementation work is adding the `tools` field to `AgentExecutionProfile`, populating it from the agents table, and wiring it into both dispatch code paths. SC-003 test should verify end-to-end: agent declares limited tools → attempts disallowed tool → gets blocked with descriptive error.
