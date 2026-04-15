---
id: "092-workspace-runtime-threading"
title: "Workspace Runtime Threading"
status: approved
implementation: complete
owner: bart
created: "2026-04-11"
risk: high
depends_on:
  - "087"
  - "090"
summary: >
  Make workspace_id flow as a first-class value through every execution path:
  desktop UI, ClaudeExecutionRequest, orchestrator manifests, checkpoints,
  factory contracts, and spawned Claude processes. Currently workspace_id exists
  only in Stagecraft (DB + JWT) and two narrow read sites.
code_aliases: ["WORKSPACE_THREADING"]
implements:
  - path: crates/orchestrator
  - path: crates/factory-contracts
---

# 092 — Workspace Runtime Threading

Parent plan: [089 Governed Convergence Plan](../089-governed-convergence-plan/spec.md)

## Problem

Workspace_id exists only in Stagecraft (DB + JWT) and flows into exactly two
places: grants fetch (`OPC_WORKSPACE_ID` env var in `governed_claude.rs`) and
factory API bodies (optional on 10 of 11 endpoints). The orchestrator,
checkpoints, factory contracts, and Claude execution all ignore it.

Without workspace threading, governance boundaries are meaningless — any
execution can access any workspace's resources, and checkpoints cannot be
scoped to the workspace that created them.

| Gap | Location |
|-----|----------|
| No Tauri command for workspace selection | `commands/stagecraft_client.rs` |
| `ClaudeExecutionRequest` lacks workspace_id | `web_server.rs` |
| `WorkflowManifest` lacks workspace_id | `crates/orchestrator/src/manifest.rs` |
| `CheckpointInfo` lacks workspace_id | `crates/axiomregent/src/checkpoint/types.rs` |
| Factory API accepts missing workspaceId | `platform/services/stagecraft/api/factory/factory.ts` |
| `WorkspaceTools` name collision | `crates/axiomregent/src/workspace/mod.rs` |

## Implementation Slices

### 1. Programmatic workspace selection in desktop (2 days)

- Add Tauri command `set_active_workspace(workspace_id)` that:
  - Sets `StagecraftClient.workspace_id`
  - Sets `OPC_WORKSPACE_ID` process env var
  - Fetches grants from platform and updates `SidecarState.grants_json`
  - Emits `workspace-changed` event
- Files: `commands/stagecraft_client.rs`, `apps/desktop/src-tauri/src/commands/agents.rs`

### 2. Thread workspace_id into ClaudeExecutionRequest (1 day)

- Add `workspace_id: Option<String>` to `ClaudeExecutionRequest`
- Pass through to governed_claude as `OPC_WORKSPACE_ID` env on spawned process
- Files: `web_server.rs`, `commands/claude.rs`

### 3. Thread workspace_id into orchestrator (1 day)

- Add `workspace_id: Option<String>` to `WorkflowManifest`
- Persist in `WorkflowState.metadata["workspace_id"]`
- Pass to `DispatchRequest` → inject as env var on spawned claude processes
- Files: `crates/orchestrator/src/manifest.rs`, `crates/orchestrator/src/state.rs`,
  `crates/orchestrator/src/lib.rs`

### 4. Thread workspace_id into checkpoints (1 day)

- Add `workspace_id: Option<String>` to `CheckpointInfo`
- Populate from `OPC_WORKSPACE_ID` env in `do_create`
- Add workspace_id filter to `list` and `timeline` queries
- Files: `crates/axiomregent/src/checkpoint/types.rs`,
  `crates/axiomregent/src/checkpoint/provider.rs`,
  `crates/axiomregent/src/checkpoint/store.rs`

### 5. Make factory workspace_id mandatory (1 day)

- Change `workspaceId` from optional to required in all Stagecraft factory API
  request types (currently required only on `InitRequest`)
- `verifyProjectInWorkspace()` runs unconditionally
- Add `workspace_id` to factory contract `build-spec.schema.yaml`
- Files: `platform/services/stagecraft/api/factory/factory.ts`,
  `factory/contract/schemas/build-spec.schema.yaml`

### 6. Rename axiomregent WorkspaceTools (0.5 day)

- Rename `WorkspaceTools` to `RepoMutationTools` to eliminate name collision
  with the workspace-as-container concept
- Update MCP tool names from `workspace.*` to `repo.*`
- Add backward-compat aliases so `workspace.*` calls still work
- Files: `crates/axiomregent/src/workspace/mod.rs`,
  `crates/axiomregent/src/router/legacy_provider.rs`

## Acceptance Criteria

- SC-092-1: Changing workspace via `set_active_workspace` updates grants, env
  var, and emits `workspace-changed` event
- SC-092-2: `WorkflowState` persists `workspace_id` in metadata
- SC-092-3: Checkpoints list filters by `workspace_id` when provided
- SC-092-4: Factory API rejects requests without `workspaceId`
- SC-092-5: `workspace.*` tool calls still work (backward compat alias)

## Dependencies

| Spec | Relationship |
|------|-------------|
| 087-unified-workspace-architecture | Workspace entity model |
| 090-governance-non-optionality | Governance bypass closure (prerequisite) |
| 093-spec-driven-preflight | Consumes workspace-scoped context (downstream) |
| 094-unified-artifact-store | Workspace-scoped artifact storage (downstream) |
| 095-checkpoint-branch-of-thought | Checkpoint workspace filtering (downstream) |
