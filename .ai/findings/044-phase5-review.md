# 044 Phase 5 Review — Real Governed Executor + Sidecar Protocol

**Reviewer:** claude
**Date:** 2026-03-30
**Scope:** `apps/desktop/src-tauri/src/commands/orchestrator.rs` (cc8ec88) — `RealGovernedExecutor` replacing `FileBackedGovernedExecutor`; `packages/provider-registry/src/node-sidecar.ts` — `systemPrompt` protocol addition
**Verdict:** Phase 5 is spec-faithful. The executor correctly implements dual-path dispatch (provider-registry sidecar + governed Claude CLI) with permission grants, token tracking, and error propagation. One sidecar protocol gap addressed in this review.

## What Phase 5 delivers

1. **`AgentExecutionProfile`** — per-agent runtime config (model, system_prompt, permission flags) loaded from SQLite (`orchestrator.rs:37–44`)
2. **`RealGovernedExecutor`** — replaces `FileBackedGovernedExecutor` with two real dispatch paths (`orchestrator.rs:46–325`)
3. **Provider-registry path** — spawns Node sidecar for `providerId:apiModel` models (e.g., `openai:gpt-4`), stdin/stdout JSONL protocol (`orchestrator.rs:81–202`)
4. **Governed Claude path** — spawns `claude` CLI with 035 MCP governance (`--mcp-config` + axiomregent) or bypass fallback (`orchestrator.rs:204–324`)
5. **`SnapshotRegistry`** upgraded — now stores `HashMap<String, AgentExecutionProfile>` instead of `HashSet<String>` (`orchestrator.rs:26–35`)
6. **Full agent profile SQL** — `SELECT name, system_prompt, model, enable_file_read, enable_file_write, enable_network FROM agents` (`orchestrator.rs:340`)
7. **`systemPrompt` sidecar protocol** — added to `SidecarQueryMessage` and piped through to both provider and bridge paths (sidecar fix in this review)

## Spec contract verification

### Architecture flow (spec § "Orchestrator dispatch flow")

| Spec step | Implemented? | Evidence |
|-----------|-------------|----------|
| Resolve agent from registry (042) | **Yes** | `SnapshotRegistry::has_agent()` in async dispatcher + `RealGovernedExecutor` looks up `AgentExecutionProfile` from the same map |
| Build agent prompt (instruction + input paths + output paths + effort) | **Yes** | `build_step_system_prompt()` (Phase 3+4) + `full_prompt` assembly in `dispatch_step()` |
| Dispatch agent execution (governed, via 035) | **Yes** | `dispatch_via_governed_claude()` uses `axiomregent_mcp_config_json()` + `--mcp-config` |
| Agent writes output artifacts to filesystem | **Yes** | Agents receive absolute output paths in prompt; `dispatch_manifest()` verifies existence post-dispatch |
| Orchestrator marks step complete, unlocks dependents | **Yes** | Inherited from `dispatch_manifest()` (Phase 4) |

### Error semantics (spec § "Error semantics")

| Error | Implemented? | Evidence |
|-------|-------------|----------|
| `StepFailed { step_id, reason }` | **Yes** | From executor errors (both paths) + output verification in `dispatch_manifest()` |
| `DependencyMissing { step_id, artifact_path }` | **Yes** | Inherited from `dispatch_manifest()` |
| `CycleDetected { cycle }` | **Yes** | Inherited from `validate_and_order()` |
| `AgentNotFound { agent_id }` | **Yes** | `SnapshotRegistry::has_agent()` + `RealGovernedExecutor` profile lookup |

### Governance integration (035)

| 035 requirement | Implemented? | Evidence |
|----------------|-------------|----------|
| Permission grants from DB | **Yes** | `enable_file_read`, `enable_file_write`, `enable_network` read from SQLite, passed to `grants_json` |
| `--mcp-config` with axiomregent | **Yes** | `bundled_axiomregent_binary_path()` + `axiomregent_mcp_config_json()` (`orchestrator.rs:227–234`) |
| `max_tier: 3` for agents | **Yes** | Hardcoded in grants JSON (`orchestrator.rs:214`) — matches 035 spec (agents get tier 3) |
| Bypass fallback | **Yes** | `--dangerously-skip-permissions` when axiomregent binary missing (`orchestrator.rs:236, 239`) |

### Provider registry integration (042)

| 042 requirement | Implemented? | Evidence |
|----------------|-------------|----------|
| Model routing via `providerId:apiModel` | **Yes** | `profile.model.contains(':')` gates sidecar path (`orchestrator.rs:70`) |
| Sidecar spawn with JSONL protocol | **Yes** | `dispatch_via_provider_registry()` spawns `node node-sidecar.js`, writes query to stdin, reads stdout JSONL |
| Token extraction from result event | **Yes** | Parses `total_input_tokens` + `total_output_tokens` from `type: "result"` event |
| Error extraction | **Yes** | Parses `type: "error"` events and returns `StepFailed` |

## Tests: 14/14 pass

No new tests added in Phase 5 (the `dispatch_manifest()` tests from Phase 4 still exercise the trait interface with `MockExecutor`). The `RealGovernedExecutor` is an integration-level implementation that requires a running Node sidecar / Claude CLI — unit testing is appropriately delegated to the trait boundary.

## Findings

### P5-001: System prompt embedded in user prompt, not system field (LOW)

**File:** `orchestrator.rs:63–68`
The executor concatenates `profile.system_prompt` + `request.system_prompt` + execution requirements into a single `full_prompt` and passes it as the `prompt` field to both the sidecar query and the Claude CLI `-p` flag. This means the system prompt lands in the user message position, not the LLM's system message slot.

**Impact:** For the provider-registry path, all 5 adapters support `systemPrompt` as a proper system message (Anthropic `system` param, OpenAI system role, Gemini `systemInstruction`, etc.). For the Claude CLI path, `--system-prompt` flag exists. Using these would improve instruction following fidelity, especially for longer prompts.

**Addressed:** The sidecar protocol now supports `systemPrompt` (added in this review). The Rust side can be updated to use it in a follow-up by splitting the prompt fields.

### P5-002: `working_directory` defaults to `current_dir()` not project path (LOW)

**File:** `orchestrator.rs:381–383`
```rust
working_directory: std::env::current_dir()
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|_| ".".to_string()),
```
This uses the Tauri process's CWD, which may be the app bundle location (e.g., `/Applications/OPC.app/Contents/MacOS/`), not the user's project directory. The `execute_claude_bridge` Tauri command takes an explicit `project_path` parameter from the frontend, but `orchestrate_manifest` doesn't accept one. Agents dispatched by the orchestrator will run in a potentially wrong directory.

**Recommended fix:** Add `project_path: String` parameter to `orchestrate_manifest` and pass it through to `RealGovernedExecutor.working_directory`.

### P5-003: Model routing heuristic uses `contains(':')` (LOW)

**File:** `orchestrator.rs:70`
```rust
if profile.model.contains(':') {
```
This routes to the provider-registry sidecar if the model string contains a colon. Models like `claude-sonnet-4-20250514` go to the Claude CLI path. This matches the `providerId:apiModel` convention from 042, but a model like `us.anthropic.claude-3-5-sonnet-20241022-v2:0` (Bedrock ARN format with colons) would incorrectly route to the sidecar as a provider-prefixed model when the sidecar's `parseProviderModel()` might not recognize it.

**Impact:** LOW — in practice, agents in the SQLite DB are configured by users who follow the `providerId:apiModel` convention. Edge case only.

### P5-004: Stdin dropped immediately after query write in sidecar path (INFO)

**File:** `orchestrator.rs:127`
```rust
drop(stdin);
```
After writing the query, stdin is dropped, which closes the pipe. This means:
- No abort messages can be sent mid-execution
- No permission responses can be sent

For orchestrator steps that need tool permissions (e.g., file writes), the sidecar's permission broker will deny all requests because the control loop stdin is closed. The governed Claude path avoids this by using `--dangerously-skip-permissions` or `--mcp-config` (which pre-authorizes via axiomregent).

**Impact:** INFO for provider-registry path — raw LLM providers (OpenAI, Gemini) don't have tool use with permission requests. The Claude Code SDK provider does, but orchestrator steps using Claude should route through the governed Claude path anyway. This gap matters only if a provider-prefixed Claude model (e.g., `anthropic:claude-sonnet-4-20250514`) is used, where the sidecar would enter the provider path but the agent might need permission handling.

### P5-005: No new integration tests for the real executor (INFO)

The `RealGovernedExecutor` is not directly testable in the orchestrator crate's unit tests (it requires a running Node sidecar + Claude CLI). This is acceptable — the trait boundary is tested via `MockExecutor`, and end-to-end testing requires the full Tauri app stack. Consider adding integration test documentation to `execution/verification.md` when SC-007 is addressed.

### P5-006: stderr read after stdout in provider path could delay error reporting (INFO)

**File:** `orchestrator.rs:175–183`
Stderr is read in a sequential loop after stdout completes. If the sidecar writes a large amount to stderr (filling the OS pipe buffer, typically 64KB), stdout reading could block because the child process blocks on stderr writes. In practice, sidecar stderr is minimal (only error logs), so this is theoretical.

## FR/SC cumulative coverage

| FR | Status | Phase |
|----|--------|-------|
| FR-001 | Partial | P1 (step model); NL decomposition deferred |
| FR-002 | **Done** | P1 + P2 + P4 + P5 (input gating) |
| FR-003 | **Done** | P1 (path convention) |
| FR-004 | **Done** | P3 + P4 (prompt builder) |
| FR-005 | **Done** | P1 + P3 + P4 (effort text) |
| FR-006 | **Done** | P1 + P2 + P4 + P5 (token usage from real execution) |
| FR-007 | **Done** | P1 (YAML manifest) |
| FR-008 | **Done** | P1 + P2 + P4 (failure cascade) |

| SC | Status | Evidence |
|----|--------|----------|
| SC-001 | **Ready** | Full dispatch pipeline wired; requires deployed agents + sidecar for end-to-end validation |
| SC-002 | **Ready** | Token tracking from `total_input_tokens` + `total_output_tokens` in result events |
| SC-003 | **Done** | `DependencyMissing` + `Skipped` cascade |
| SC-004 | **Done** | YAML manifest validation + dispatch |
| SC-005 | **Ready** | Effort level propagated to agent prompts; different output sizes depend on agent behavior |
| SC-006 | **Done** | `cleanup_artifacts()` |
| SC-007 | Deferred | Verification doc not yet written |

## Prior findings status

| Finding | Status | Resolution |
|---------|--------|------------|
| P4-001 (cancel_run in-memory only) | Acknowledged | Still applies — cancel_run doesn't reach running sidecar/CLI processes |
| P4-002 (scaffold executor) | **Resolved** | `FileBackedGovernedExecutor` replaced by `RealGovernedExecutor` |
| P4-003 (process-global static) | Acknowledged | Still applies — acceptable for MVP |
| P4-004 (sync YAML read) | Acknowledged | Still applies |
| P4-005 (Cancelled status) | Acknowledged | Still applies |
| P4-006 (snapshot registry) | Acknowledged | Still applies |

## Summary

Phase 5 completes the orchestrator's real dispatch capability. `RealGovernedExecutor` correctly:
- Loads per-agent profiles (model, system_prompt, permissions) from SQLite
- Routes `providerId:apiModel` models through the Node sidecar (042 provider registry)
- Routes native models through governed Claude CLI with 035 MCP config + axiomregent permission grants
- Extracts token usage from JSONL result events
- Propagates errors with step context
- Falls back to `--dangerously-skip-permissions` when axiomregent unavailable

The sidecar protocol is extended with `systemPrompt` support (both provider and bridge paths), enabling proper system/user message separation in a follow-up.

14/14 tests pass. 6 findings — 3 LOW (system prompt in user slot, working_directory defaults to CWD, model routing heuristic), 3 INFO. The main actionable items are P5-001 (split system prompt) and P5-002 (accept project_path parameter).

**Recommendation:** Fix P5-002 (project_path parameter) before end-to-end testing, as agents running in `/Applications/` will fail. P5-001 (system prompt separation) is a quality-of-life improvement that can follow. After P5-002, the orchestrator is ready for end-to-end validation (SC-001, SC-002, SC-005).
