# Architecture

## System Overview

Open Agentic Platform (OAP) is a governed operating system for AI-native software delivery. Three layers:

1. **OPC Desktop** (`apps/desktop/`) — Local Tauri v2 + React cockpit for inspection, governance, and git context
2. **Spec Spine** (`specs/`, `tools/`, `build/`) — Canonical contract system: markdown specs compile to JSON registries
3. **Platform** (`platform/`) — Organisational control plane: identity, policy, approvals, deployments, audit

## Crate Map

### Library Crates (`crates/`)

| Crate | Path | Purpose |
|-------|------|---------|
| `orchestrator` | `crates/orchestrator/` | Multi-agent workflow dispatch: manifest parsing, DAG validation, artifact passing, gates, verify+retry, state persistence (SQLite), SSE events |
| `factory-engine` | `crates/factory-engine/` | Factory two-phase pipeline engine: process stages (s0-s5), Build Spec freeze, scaffold fan-out (s6a-s6g), verification harness |
| `factory-contracts` | `crates/factory-contracts/` | Rust types for Factory schemas: BuildSpec, AdapterManifest, PipelineState, Verification, AdapterRegistry, AgentLoader |
| `policy-kernel` | `crates/policy-kernel/` | 5-tier settings merge, proof chains, permission tracking, policy evaluation |
| `axiomregent` | `crates/axiomregent/` | Unified MCP agent: GitHub tools, semantic search, checkpoint subsystem (absorbed gitctx, blockoli, stackwalk) |
| `xray` | `crates/xray/` | Repository analysis: complexity scoring, incremental scanning, call graphs, dependency extraction, semantic embeddings, structural fingerprinting |
| `featuregraph` | `crates/featuregraph/` | Feature registry scanner: maps specs to code locations |
| `agent` | `crates/agent/` | Agent framework: executor, verification, ID generation |
| `skill-factory` | `crates/skill-factory/` | Skill and Command Factory (spec 071) |
| `tool-registry` | `crates/tool-registry/` | ToolDef trait + registry with permission gates (spec 067) |
| `run` | `crates/run/` | Run lifecycle management |

### CLI Tools (`tools/`)

| Tool | Path | Purpose |
|------|------|---------|
| `spec-compiler` | `tools/spec-compiler/` | Compiles `specs/` markdown to `build/spec-registry/registry.json` |
| `registry-consumer` | `tools/registry-consumer/` | Queries the compiled registry (list, show, status) |
| `spec-lint` | `tools/spec-lint/` | Conformance linter (W-xxx warnings) |
| `policy-compiler` | `tools/policy-compiler/` | Compiles governance policies |

## Factory Pipeline

The Factory delivery engine transforms business requirements into production code through a two-phase pipeline.

### Phase 1: Process (s0-s5)

Six sequential stages, each driven by a specialized AI agent:

```
s0-preflight          -> Validate docs, adapter, dependencies
s1-business-reqs      -> Extract entities, use cases, audiences    [CHECKPOINT]
s2-service-reqs       -> Design services, rules, integrations      [CHECKPOINT]
s3-data-model         -> Generate data model from entities         [CHECKPOINT]
s4-api-spec           -> Specify API operations                    [CHECKPOINT]
s5-ui-spec            -> Specify UI pages, produce Build Spec      [APPROVAL]
```

After s5 approval, the Build Spec is frozen (SHA-256 hashed) and becomes the input for Phase 2.

### Phase 2: Scaffolding (s6a-s6g)

Dynamic fan-out generated from Build Spec content:

```
s6a: Scaffold init    -> Copy adapter base, install deps, verify compile
s6b-*: Data entities  -> One step per entity (topologically sorted)
s6c-*: API operations -> One step per resource/operation
s6d-*: UI pages       -> One step per page
s6e: Configure        -> Wire auth, env vars, deployment config
s6f: Trim             -> Remove unused scaffold/placeholder files
s6g: Final validation -> Full build + test + lint + type-check     [CHECKPOINT]
```

Each step runs post-verification commands (compile, test) and retries up to 3 times on failure.

### Adapters

Four pluggable tech adapters in `factory/adapters/`:

| Adapter | Stack |
|---------|-------|
| `aim-vue-node` | Express 5 + Vue 3 |
| `next-prisma` | Next.js 15 + Prisma 5 |
| `rust-axum` | Axum + HTMX |
| `encore-react` | Encore.ts + React |

Each adapter provides: manifest, scaffold template, code patterns, agent prompts, validation invariants.

## Orchestrator

The orchestrator (`crates/orchestrator/`) executes multi-step workflows:

- **WorkflowManifest** — Declares steps with inputs, outputs, agents, effort levels, gates, and verification commands
- **DAG validation** — Topological sort ensures correct execution order
- **Artifact passing** — File-based: each step reads from absolute paths, writes to declared output paths
- **Gates** — Checkpoint (pause for review) and Approval (with timeout and escalation)
- **Verify+Retry** — Post-step commands (compile, test); on failure, rebuild prompt with stderr and retry
- **State persistence** — SQLite backend with crash-resume support
- **SSE events** — Real-time event stream for the desktop UI

Key traits:

| Trait | Implementations |
|-------|----------------|
| `GovernedExecutor` | `ClaudeCodeExecutor`, noop mock |
| `AgentRegistry` | Checks agent availability by ID |
| `GateHandler` | `CliGateHandler`, `AutoApproveGateHandler` |

## Platform Services

Located in `platform/`:

- **Stagecraft** (`platform/services/stagecraft/`) — Encore.ts SaaS: auth, admin, monitoring, Slack integration, GitHub webhooks, Factory lifecycle API
- **deployd-api-rs** (`platform/services/deployd-api-rs/`) — Rust (axum + hiqlite) Kubernetes deployment orchestration
- **Rauthy** — OIDC identity provider
- **Infrastructure** (`platform/infra/`) — Terraform modules for Azure AKS, ACR, KeyVault
- **Helm charts** (`platform/charts/`) — stagecraft, deployd-api, rauthy

## Spec System

The spec-first workflow (defined in spec 000):

1. **Author** — Write a spec in `specs/NNN-slug/spec.md` with YAML frontmatter
2. **Compile** — `spec-compiler compile` produces `build/spec-registry/registry.json`
3. **Query** — `registry-consumer list|show` reads the compiled registry
4. **Lint** — `spec-lint` checks conformance (W-xxx warnings)

Status lifecycle: `draft` -> `active` (ratified and delivered) -> `superseded` | `retired`

Human-authored truth is always markdown. Machine-consumed truth is always compiler-emitted JSON.

## Desktop App (OPC)

Tauri v2 + React desktop application in `apps/desktop/`:

- **Rust backend** (`src-tauri/`) — Tauri commands, axiomregent sidecar, state management
- **React frontend** (`src/`) — Panels for inspection, governance, git, Factory pipeline visualization
- **Factory panel components** — PipelineDAG, ArtifactInspector, BuildSpecStructuredView, GateDialog, ScaffoldMonitor, TokenDashboard

## Claude Code Integration

The `.claude/` directory ships development infrastructure as a first-class part of the workflow:

| Path | Contents |
|------|---------|
| `.claude/agents/` | architect, explorer, implementer, reviewer |
| `.claude/commands/` | init, commit, code-review, validate-and-fix, research, implement-plan, cleanup |
| `.claude/rules/` | Orchestrator behavioral rules (6 rules for multi-step workflows) |
| `CLAUDE.md` | Project conventions, build commands, extension points |
| `AGENTS.md` | Agent protocol and session initialization |
