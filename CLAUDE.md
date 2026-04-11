# CLAUDE.md — Open Agentic Platform

## Project Overview

Open Agentic Platform (OAP) is a governed operating system for AI-native software delivery. It combines three layers:

- **OPC** (`apps/desktop/`) — local Tauri + React cockpit for inspect, governance, and git context
- **Platform** (`platform/`) — organisational control plane (identity, policy, approvals, deployments, audit)
- **Spec Spine** (`specs/`) — canonical contract system turning intent into traceable, machine-verifiable truth

## Repository Structure

```
specs/              — Feature specifications (000–088), the authoritative design record
tools/              — Rust CLI tools
  spec-compiler/    — Compiles specs → build/spec-registry/registry.json
  registry-consumer/— Reads and queries the compiled registry
  spec-lint/        — Conformance linter (W-xxx warnings)
  policy-compiler/  — Compiles governance policies
crates/             — Rust library crates
  agent/            — Agent framework: executor, verification, ID generation
  axiomregent/      — Unified MCP agent: GitHub tools, semantic search, checkpoint (spec 073)
  factory-engine/   — Factory two-phase pipeline engine (spec 075)
  factory-contracts/— Rust types for Factory contract schemas (spec 074)
  featuregraph/     — Feature registry scanner: maps specs to code
  orchestrator/     — Multi-agent workflow dispatch, DAG validation, state persistence
  policy-kernel/    — 5-tier settings merge, proof chains, policy evaluation
  run/              — Run lifecycle management
  skill-factory/    — Skill and Command Factory (spec 071)
  tool-registry/    — ToolDef trait + registry with permission gates (spec 067)
  xray/             — Repository analysis: complexity scoring, call graphs, structural fingerprinting
factory/            — Factory delivery engine
  contract/         — Formal schemas: Build Spec, Adapter Manifest, Pipeline State, Verification
  process/          — 7-stage pipeline: agents and stage definitions
  adapters/         — Pluggable tech adapters (aim-vue-node, next-prisma, encore-react, rust-axum)
  docs/             — Architecture, how-to, integration docs
apps/desktop/       — Tauri v2 + React desktop app (TypeScript + Rust)
platform/           — Organisational control plane (imported from stagecraft-ing/platform)
  services/
    stagecraft/     — Encore.ts SaaS (auth, admin, monitoring, Slack, GitHub webhook handling)
    deployd-api-rs/ — Rust (axum + hiqlite) K8s deployment orchestration
  infra/            — Terraform modules (Azure AKS, ACR, KeyVault)
  charts/           — Helm charts (stagecraft, deployd-api, rauthy)
  k8s/              — Baseline K8s policies (network deny, resource quotas)
build/              — Compiler output (registry.json, build-meta.json)
.claude/            — Claude Code agents, commands, rules (AI development infrastructure)
.specify/           — Spec Kit contract metadata and templates
```

## Orchestrator Behavioral Rules

All multi-step commands and agent workflows MUST follow the six rules defined in `.claude/rules/orchestrator-rules.md`:

1. Execute steps in order — no skipping, reordering, or merging
2. Write output files — file-based context passing between steps, not context window memory
3. Stop at checkpoints — wait for explicit user approval
4. Halt on failure — present errors, ask user how to proceed
5. Use only local agents — no cross-project dependencies
6. Never enter plan mode autonomously — the command is the plan

## Key Conventions

- **Specs are the source of truth.** Every feature starts as a spec in `specs/NNN-slug/spec.md` with YAML frontmatter.
- **Rust for tools and crates.** All CLI tools and library crates are Rust. Build with `cargo build --release --manifest-path <path>/Cargo.toml`.
- **TypeScript for the desktop app.** `apps/desktop/` uses Tauri v2, React, TypeScript.
- **TypeScript for platform services.** `platform/services/stagecraft/` uses Encore.ts with npm (NOT pnpm — excluded from the pnpm workspace). `deployd-api-rs` is the Rust deployment orchestrator (axum + hiqlite).
- **axiomregent is the unified MCP agent crate.** It now contains the `github/`, `search/`, and `checkpoint/` modules, absorbing the former `gitctx`, `blockoli`, and `stackwalk` crates.
- **Markdown for specs.** Human truth is markdown (with optional YAML frontmatter). Machine registries are compiler-emitted JSON only.
- **Spec compiler is the build system.** Run `./tools/spec-compiler/target/release/spec-compiler compile` from repo root to produce `build/spec-registry/registry.json`.

## Build Commands

```bash
# Compile specs
cargo build --release --manifest-path tools/spec-compiler/Cargo.toml
./tools/spec-compiler/target/release/spec-compiler compile

# Query registry
cargo build --release --manifest-path tools/registry-consumer/Cargo.toml
./tools/registry-consumer/target/release/registry-consumer list
./tools/registry-consumer/target/release/registry-consumer show <feature-id>

# Lint specs
cargo build --release --manifest-path tools/spec-lint/Cargo.toml

# Compile policies
cargo build --release --manifest-path tools/policy-compiler/Cargo.toml

# Platform services (local dev)
cd platform/services/stagecraft && npm run start   # Encore.ts on :4000

# deployd-api (Rust)
cargo build --release --manifest-path platform/services/deployd-api-rs/Cargo.toml

# Platform infrastructure
cd platform && make tf-init    # Init Terraform
cd platform && make tf-apply   # Full Azure deployment
```

## Claude Code Extension Points

- **`.claude/agents/`** — architect, explorer, implementer, reviewer, encore-expert
- **`.claude/commands/`** — /init, /commit, /code-review, /review-branch, /implement-plan, /research, /validate-and-fix, /cleanup, /factory-sync, /refactor-claude-md
- **`.claude/rules/`** — Reusable rule files (loaded automatically)
- **`AGENTS.md`** — Self-extending agent protocol and session init
- **`CLAUDE.md`** — Scoped at root, `platform/`, and `platform/services/stagecraft/`
