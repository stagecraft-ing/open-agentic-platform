---
id: "075-elucid-workflow-engine"
title: "Elucid Workflow Engine — Orchestrator Fan-Out, Manifest Generation, and Verification"
feature_branch: "feat/075-elucid-workflow-engine"
status: draft
kind: platform
created: "2026-04-04"
authors: ["open-agentic-platform"]
language: en
summary: >
  Extends the OAP orchestrator to natively support Elucid pipeline workflows
  including two-phase manifest execution (process stages + dynamic scaffold fan-out),
  Build Spec to manifest generation, post-step verification hooks, and Rust-native
  verification harness integration.
code_aliases: ["ELUCID_WORKFLOW", "ELUCID_FANOUT"]
---

# Feature Specification: Elucid Workflow Engine

## Purpose

The OAP orchestrator (`crates/orchestrator/`) already handles DAG manifests, artifact passing, gate evaluation, and state persistence. Elucid's 7-stage pipeline maps naturally onto these primitives — but requires two capabilities the orchestrator doesn't yet have:

1. **Two-phase execution**: Process stages (1-5) produce a Build Spec; scaffolding (stage 6) fans out into N dynamic sub-steps derived from the Build Spec content. The orchestrator must support generating a second manifest mid-workflow.

2. **Post-step verification hooks**: After each scaffolding step, adapter-specific commands (compile, test) must run automatically. The current orchestrator dispatches agents but has no post-step hook mechanism.

This spec extends the orchestrator to handle Elucid workflows natively, including manifest generation from Build Specs, fan-out orchestration, verification hook execution, and integration with Elucid's contract types (spec 074).

## Scope

### In scope

- Elucid workflow type recognition in orchestrator
- `generate_scaffold_manifest()` — Build Spec + Adapter Manifest → Phase 2 WorkflowManifest
- Post-step verification hook mechanism (adapter `feature_verify` commands)
- Build-test-fix retry loop for scaffolding steps
- Elucid-specific policy shard generation for axiomregent
- Resume-from-checkpoint for Elucid pipelines (extends spec 052)
- Elucid agent registration bridge (Elucid agents → AgentRegistry)

### Out of scope

- Desktop UI (spec 076)
- Platform API (spec 077)
- Contract type definitions (spec 074)

## Requirements

### Functional Requirements

**FR-001: Elucid Workflow Type**
The orchestrator SHALL recognize an `elucid` workflow type in manifests. An Elucid workflow is a `WorkflowManifest` with metadata indicating two-phase execution:

```yaml
metadata:
  type: elucid
  adapter: next-prisma
  build_spec_step: s5-ui-specification  # step that produces the frozen Build Spec
  build_spec_output: build-spec.yaml     # output artifact name
```

When the orchestrator completes the `build_spec_step` and its gate is approved, it SHALL automatically invoke manifest generation for Phase 2.

**FR-002: Phase 1 Manifest Generation**
A function SHALL generate the Phase 1 (process stages) manifest from an adapter name and input document paths:

```rust
pub fn generate_process_manifest(
    adapter: &AdapterManifest,
    business_doc_paths: &[PathBuf],
    elucid_root: &Path,
) -> Result<WorkflowManifest, ManifestError>;
```

This produces stages s0-s5 with:
- Agent references to process agent IDs (e.g., `elucid-business-analyst`)
- Input paths pointing to business documents and prior-stage artifact references
- Output declarations matching Elucid's stage output schemas
- Gate configurations (checkpoint for stages 1-4, approval with timeout for stage 5)

**FR-003: Phase 2 Manifest Generation (Dynamic Fan-Out)**
After Build Spec freeze, the orchestrator SHALL generate the Phase 2 (scaffolding) manifest:

```rust
pub fn generate_scaffold_manifest(
    build_spec: &BuildSpec,
    adapter: &AdapterManifest,
    elucid_root: &Path,
) -> Result<WorkflowManifest, ManifestError>;
```

This produces steps for:
- **s6a**: Scaffold initialization (copy adapter base project, install deps, verify compile)
- **s6b-***: One step per entity, dependency-ordered (entities with references come after their targets)
- **s6c-***: One step per API operation, grouped by resource
- **s6d-***: One step per UI page
- **s6e**: Configuration (project identity, auth wiring, env vars)
- **s6f**: Trim (remove unused scaffold artifacts)
- **s6g**: Final validation (full build + all tests + lint + type check + invariants)

Each scaffolding step's `instruction` SHALL contain:
- The specific feature to generate (entity name, operation spec, page spec)
- Paths to relevant pattern files (resolved via PatternResolver from spec 074)
- The adapter's directory conventions for file placement

**FR-004: Entity Dependency Ordering**
`generate_scaffold_manifest` SHALL topologically sort entities by their `reference` fields. If entity `Site` has a field referencing entity `Organization`, `s6b-data-Organization` SHALL precede `s6b-data-Site` in the manifest. Circular references SHALL produce a `ManifestError`.

**FR-005: Post-Step Verification Hooks**
The orchestrator SHALL support a `post_verify` field on workflow steps:

```rust
pub struct WorkflowStep {
    // ... existing fields ...
    pub post_verify: Option<Vec<VerifyCommand>>,
}

pub struct VerifyCommand {
    pub command: String,        // e.g., "npx tsc --noEmit"
    pub working_dir: String,    // relative to project root
    pub timeout_ms: u64,
}
```

After a scaffolding step's agent completes successfully, the orchestrator SHALL execute each `post_verify` command in sequence. If any command fails, the step is marked as verification-failed (distinct from agent-failed).

For Elucid scaffolding, `post_verify` is populated from the adapter's `feature_verify` commands.

**FR-006: Build-Test-Fix Retry Loop**
When a scaffolding step fails verification (FR-005), the orchestrator SHALL retry by re-dispatching the agent with error context prepended:

```
PREVIOUS ATTEMPT FAILED. Fix the following errors:

--- Verification Output ---
{stderr from failed command}
---

Original instruction: {original instruction}
```

Retry behavior:
- Maximum retries: 3 (configurable via policy shard)
- Each retry increments `retry_count` in step state
- After max retries, step is marked `Failed` and the manifest continues to the next independent step
- Failed steps do NOT cascade-fail dependent steps unless they are direct data dependencies

**FR-007: Elucid Agent Registration Bridge**
The orchestrator SHALL bridge Elucid agent definitions to OAP's `AgentRegistry`:

```rust
pub struct ElucidAgentBridge {
    process_agents: Vec<AgentPrompt>,    // from spec 074
    adapter_agents: Vec<AgentPrompt>,    // from spec 074
}

impl AgentRegistry for ElucidAgentBridge {
    async fn has_agent(&self, agent_id: &str) -> bool;
}
```

Agent ID mapping:
- Process agents: `elucid-{role}` (e.g., `elucid-business-analyst`, `elucid-data-architect`)
- Scaffold agents: `elucid-{role}-{adapter}` (e.g., `elucid-api-scaffolder-next-prisma`)

**FR-008: Elucid Policy Shard Generation**
For each Elucid workflow, the orchestrator SHALL generate an axiomregent policy shard:

```rust
pub fn generate_elucid_policy_shard(
    adapter: &AdapterManifest,
    build_spec: &BuildSpec,
) -> PolicyShard;
```

The policy shard SHALL:
- Set `file_write` allowed paths from adapter's `directory_conventions` (expanded with actual entity/resource names from Build Spec)
- Set `execute` allowed commands from adapter's `commands` (compile, test, lint, etc.)
- Block dangerous patterns (`rm -rf`, `DROP`, `git push`, etc.)
- Set token budgets per step effort level

**FR-009: Elucid Pipeline State Mapping**
The orchestrator SHALL maintain Elucid-specific state alongside `WorkflowState`:

```rust
pub struct ElucidPipelineState {
    pub pipeline_id: String,
    pub adapter: AdapterRef,
    pub build_spec_hash: Option<String>,  // SHA-256, set after stage 5 freeze
    pub phase: ElucidPhase,               // Process | Scaffolding | Complete
    pub scaffolding: Option<ScaffoldingProgress>,
}

pub struct ScaffoldingProgress {
    pub entities_completed: Vec<String>,
    pub entities_failed: Vec<FailedFeature>,
    pub operations_completed: Vec<String>,
    pub operations_failed: Vec<FailedFeature>,
    pub pages_completed: Vec<String>,
    pub pages_failed: Vec<FailedFeature>,
}
```

This state is persisted via the `WorkflowStore` trait (spec 052) alongside the generic `WorkflowState`.

**FR-010: Verification Harness Integration**
The orchestrator SHALL integrate Elucid's verification checks at stage gates. When a process stage completes, the gate evaluation SHALL run the corresponding verification checks from `elucid/contract/schemas/verification.schema.yaml`:

- Stage gate checks: artifact existence, schema validation, cross-reference integrity
- Final validation checks: full build, all tests, lint, type check, invariants

The Python harness (`elucid/process/harness/`) SHALL be invoked as a subprocess:

```rust
pub async fn run_elucid_gate_check(
    stage_id: &str,
    project_path: &Path,
    elucid_root: &Path,
) -> Result<GateCheckResult, VerifyError>;
```

Long-term, gate checks migrate to Rust; short-term, the Python harness is the reference implementation.

### Non-Functional Requirements

**NF-001: Fan-Out Performance**
Scaffold steps with no data dependencies SHALL be dispatchable in parallel. The orchestrator SHALL identify independent steps in the Phase 2 DAG and dispatch them concurrently (bounded by a configurable concurrency limit, default 4).

**NF-002: Token Budget Tracking**
Every agent dispatch SHALL record `prompt_tokens` and `completion_tokens`. The orchestrator SHALL track cumulative token spend per pipeline and halt if the policy shard's `total_pipeline` budget is exceeded.

**NF-003: Idempotent Resume**
If the orchestrator crashes mid-pipeline, restarting with the same `run_id` SHALL:
- Load persisted state from `WorkflowStore`
- Skip completed steps (including verified scaffolding steps)
- Resume from the first pending/failed step
- Re-run failed steps (retry count preserved)

## Architecture

### Execution Flow

```
User clicks "Start Pipeline" in OPC
  │
  ▼
OPC calls Tauri command: start_elucid_pipeline(project_id, adapter, docs)
  │
  ├─ 1. AdapterRegistry.get(adapter) → AdapterManifest
  ├─ 2. generate_process_manifest(adapter, docs, elucid_root) → Phase 1 manifest
  ├─ 3. ElucidAgentBridge.new(process_agents, adapter_agents)
  ├─ 4. generate_elucid_policy_shard(adapter, build_spec_stub)
  ├─ 5. dispatch_manifest_persisted(manifest, bridge, policy_shard)
  │     │
  │     ├─ s0: Pre-flight → artifacts
  │     ├─ s1: Business Requirements → gate (checkpoint) → user confirms
  │     ├─ s2: Service Requirements → gate (checkpoint) → user confirms
  │     ├─ s3: Data Model → gate (checkpoint) → user confirms
  │     ├─ s4: API Specification → gate (checkpoint) → user confirms
  │     └─ s5: UI Specification → gate (approval, 1h timeout)
  │           │
  │           ▼ Build Spec frozen (hash recorded)
  │
  ├─ 6. validate_build_spec(artifact_path) → BuildSpec
  ├─ 7. generate_scaffold_manifest(build_spec, adapter, elucid_root) → Phase 2 manifest
  ├─ 8. dispatch_manifest_persisted(phase2_manifest, bridge, policy_shard)
  │     │
  │     ├─ s6a: Scaffold init → post_verify: [install, compile]
  │     ├─ s6b-*: Data per entity → post_verify: [compile]
  │     ├─ s6c-*: API per operation → post_verify: [compile, test]
  │     ├─ s6d-*: UI per page → post_verify: [compile]
  │     ├─ s6e: Configure → post_verify: [compile]
  │     ├─ s6f: Trim → post_verify: [compile]
  │     └─ s6g: Final validation → gate (checkpoint)
  │
  └─ 9. Pipeline complete → notify Stagecraft
```

### New Tauri Commands

```rust
#[tauri::command]
pub async fn start_elucid_pipeline(
    project_path: String,
    adapter_name: String,
    business_doc_paths: Vec<String>,
) -> Result<String, String>;  // returns run_id

#[tauri::command]
pub async fn get_elucid_pipeline_status(
    run_id: String,
) -> Result<ElucidPipelineState, String>;

#[tauri::command]
pub async fn confirm_elucid_stage(
    run_id: String,
    stage_id: String,
) -> Result<(), String>;

#[tauri::command]
pub async fn reject_elucid_stage(
    run_id: String,
    stage_id: String,
    feedback: String,
) -> Result<(), String>;
```

## Implementation Approach

### Phase 1: Post-Step Verification Hooks (3 days)

1. Add `post_verify` field to `WorkflowStep`
2. Implement `run_verify_commands()` in orchestrator dispatch loop
3. Implement retry-with-error-context logic (FR-006)
4. Unit tests with mock commands

### Phase 2: Manifest Generation (4 days)

1. Implement `generate_process_manifest()` — static 6-step DAG
2. Implement `generate_scaffold_manifest()` — dynamic fan-out from Build Spec
3. Implement entity dependency ordering (topological sort)
4. Implement `generate_elucid_policy_shard()`
5. Tests against `cfs-womens-shelter.build-spec.yaml` example

### Phase 3: Two-Phase Execution (3 days)

1. Add Elucid metadata to `WorkflowManifest`
2. Implement phase transition: detect Build Spec freeze → generate Phase 2
3. Wire `ElucidPipelineState` alongside `WorkflowState`
4. Implement Tauri commands

### Phase 4: Verification Harness Integration (2 days)

1. Implement `run_elucid_gate_check()` — subprocess invocation of Python harness
2. Wire gate checks into stage gate evaluation
3. Integration test: full pipeline with mock agents

### Phase 5: Agent Bridge & Policy (2 days)

1. Implement `ElucidAgentBridge`
2. Wire to `dispatch_manifest_persisted()` as `AgentRegistry` implementation
3. Generate and apply policy shards at pipeline start

## Success Criteria

- **SC-001**: `generate_process_manifest()` produces a valid 6-step manifest from adapter + doc paths
- **SC-002**: `generate_scaffold_manifest()` produces correct fan-out from `cfs-womens-shelter.build-spec.yaml` (19 entities × operations × pages = ~50+ steps)
- **SC-003**: Entity dependency ordering is correct (topological sort verified)
- **SC-004**: Post-step verification runs adapter `feature_verify` commands after each scaffold step
- **SC-005**: Retry loop prepends error output and re-dispatches (max 3 retries)
- **SC-006**: Phase transition from process → scaffolding occurs automatically after stage 5 approval
- **SC-007**: Pipeline resumes correctly from persisted state after simulated crash

## Dependencies

| Spec | Relationship |
|------|-------------|
| 074-elucid-ingestion | Provides contract types, adapter registry, agent loader |
| 052-state-persistence | Provides WorkflowStore + EventNotifier for Elucid state |
| 044-multi-agent-orchestration | Base orchestrator types (WorkflowManifest, etc.) |
| 068-permission-runtime | Policy shard enforcement via axiomregent |

## Risks

| Risk | Mitigation |
|------|-----------|
| Python harness subprocess adds latency to gates | Gate checks are infrequent (6 process stages); latency acceptable. Long-term: Rust port |
| Fan-out produces 100+ steps for large Build Specs | Configurable concurrency limit (default 4); token budget cap |
| Entity circular references | Detect and reject at manifest generation time |
| Adapter command failure on host (missing Node, etc.) | Pre-flight check validates all adapter commands are available |
