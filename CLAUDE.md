# CLAUDE.md — Open Agentic Platform

## Project Overview

Open Agentic Platform (OAP) is a governed operating system for AI-native software delivery. It combines three layers:

- **OPC** (`product/apps/desktop/`) — local Tauri + React cockpit for inspect, governance, and git context
- **Platform** (`platform/`) — organisational control plane (identity, policy, approvals, deployments, audit)
- **Spec Spine** (`specs/`) — canonical contract system turning intent into traceable, machine-verifiable truth

## Repository Structure

```
specs/              — Feature specifications (000–088), the authoritative design record
tools/              — Rust CLI tools (subdivided by ownership)
  spec-spine/       — Generic spec-spine binaries (spec-compiler,
                      registry-consumer, codebase-indexer, spec-lint,
                      spec-code-coupling-check)
  oap/              — OAP-specific tools (oap-registry-enrich,
                      oap-code-index-enrich, policy-compiler,
                      adapter-scopes-compiler, assumption-cascade-check,
                      ci-parity-check, schema-parity-check (JS),
                      stakeholder-doc-lint)
  shared/spec-types/— Shared frontmatter / spec-shape types
  vendor/           — Vendored third-party (e.g. tree-sitter grammars)
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
product/            — End-user product layer (npm workspace root post-I7)
  apps/desktop/     — Tauri v2 + React desktop app (TypeScript + Rust)
  packages/         — Shared npm packages (provider-registry, ui, etc.)
  package.json, pnpm-workspace.yaml — workspace root
platform/           — Organisational control plane (imported from stagecraft-ing/platform)
  services/
    stagecraft/     — Encore.ts SaaS (auth, admin, monitoring, Slack, GitHub webhook handling)
    deployd-api-rs/ — Rust (axum + hiqlite) K8s deployment orchestration
  infra/            — Terraform modules (Azure AKS, ACR, KeyVault)
  charts/           — Helm charts (stagecraft, deployd-api, rauthy)
  k8s/              — Baseline K8s policies (network deny, resource quotas)
.derived/           — Compiler output (registry.json, index.json, build-meta.json)
.claude/            — Claude Code agents, commands, rules (AI development infrastructure)
standards/spec/     — Graduated spec-spine standard: constitution.md, contract.md, templates/
```

## Orchestrator Behavioral Rules

All multi-step commands and agent workflows MUST follow the six rules defined in `.claude/rules/orchestrator-rules.md`:

1. Execute steps in order — no skipping, reordering, or merging
2. Write output files — file-based context passing between steps, not context window memory
3. Stop at checkpoints — wait for explicit user approval
4. Halt on failure — present errors, ask user how to proceed
5. Use only local agents — no cross-project dependencies
6. Never enter plan mode autonomously — the command is the plan

In addition, all orchestrated workflows load `.claude/rules/governed-artifact-reads.md` (spec 103) and `.claude/rules/adversarial-prompt-refusal.md` (CONST-005, spec 131) automatically. The latter codifies the prompt-time refusal pattern for instructions that would engineer drift between spec spine and code.

## Key Conventions

- **Specs are the source of truth.** Every feature starts as a spec in `specs/NNN-slug/spec.md` with YAML frontmatter.
- **Rust for tools and crates.** All CLI tools and library crates are Rust. Build with `cargo build --release --manifest-path <path>/Cargo.toml`.
- **TypeScript for the desktop app.** `product/apps/desktop/` uses Tauri v2, React, TypeScript.
- **TypeScript for platform services.** `platform/services/stagecraft/` uses Encore.ts with npm (NOT pnpm — excluded from the pnpm workspace). `deployd-api-rs` is the Rust deployment orchestrator (axum + hiqlite).
- **axiomregent is the unified MCP agent crate.** It now contains the `github/`, `search/`, and `checkpoint/` modules, absorbing the former `gitctx`, `blockoli`, and `stackwalk` crates.
- **Markdown for specs.** Human truth is markdown (with optional YAML frontmatter). Machine registries are compiler-emitted JSON only.
- **Spec compiler is the build system.** Run `./tools/spec-spine/spec-compiler/target/release/spec-compiler compile` from repo root to produce `.derived/spec-registry/registry.json`.
- **Traceability via `[package.metadata.oap]`.** Rust crates that implement a spec declare `spec = "<spec-id>"` under `[package.metadata.oap]` in their Cargo.toml; npm packages do the same via top-level `"oap": { "spec": "<spec-id>" }` in package.json. The codebase-indexer uses these to build spec-to-code traceability mappings in `.derived/codebase-index/index.json`.
- **Per-crate documentation lives in the spec, not in per-crate READMEs.** The spec id declared in each manifest (above) is the canonical "what is this crate." `.derived/codebase-index/CODEBASE-INDEX.md` renders this as a Spec column linking each crate/package to its spec. Do not add new per-crate or per-package READMEs; route prose into the owning spec, the root [`README.md`](README.md), [`docs/DEVELOPERS.md`](docs/DEVELOPERS.md), or [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). Existing tool READMEs that document binary-specific behavior beyond the spec (e.g. `tools/spec-spine/registry-consumer/README.md` whose fenced examples are fixture-tested verbatim) are exceptions, not the pattern.

## Build Commands

Prefer the root `Makefile` for common flows; the raw cargo invocations below are the authoritative underlying commands.

```bash
# Primary entry points (Makefile)
make setup        # install deps, build spec compiler + codebase indexer, compile both
make dev          # start OPC desktop (Vite + Tauri, hot-reload)
make dev-platform # stagecraft + deployd-api in background
make ci           # parallel local validation (~5 min warm) — daily dev loop (spec 135)
make ci-strict    # full parity mirror (~90 min) — pre-merge / parity-investigation
make registry     # recompile spec registry + codebase index
make pr-prep      # pre-commit refresh: regenerate codebase index + run coupling gate
```

### `make pr-prep` — pre-PR / pre-commit gate

Run `make pr-prep` before `git commit` on a PR. It rebuilds the codebase index and runs the spec-code coupling gate against `origin/main` — the same two checks that fail first in CI when forgotten.

The codebase index hashes more than `spec.md`. Its inputs (see `tools/spec-spine/codebase-indexer/src/lib.rs::collect_input_files`) include `Cargo.toml`, `package.json`, `pnpm-workspace.yaml`, `specs/*/spec.md`, `factory/adapters/*/manifest.yaml`, `factory/process/stages/*`, `.claude/{agents,commands,rules}/**/*.md`, `standards/schemas/**/*.{json,yaml,yml}`, and `.github/workflows/*.yml`. Editing any of these without committing the regenerated `.derived/codebase-index/index.json` fails the staleness check on the PR. `make pr-prep` is the one command that catches this locally.

If repeated forgetting is a problem, opt into the strict pre-commit hook:

```bash
git config core.hooksPath .githooks   # enable
git config --unset core.hooksPath     # disable
```

`.githooks/pre-commit` refuses commits when the index is stale and prints the exact fix. It is opt-in (not configured by default) because it adds friction every commit, not just PR-final commits.

```bash
# Compile specs
cargo build --release --manifest-path tools/spec-spine/spec-compiler/Cargo.toml
./tools/spec-spine/spec-compiler/target/release/spec-compiler compile

# Query registry
cargo build --release --manifest-path tools/spec-spine/registry-consumer/Cargo.toml
./tools/spec-spine/registry-consumer/target/release/registry-consumer list
./tools/spec-spine/registry-consumer/target/release/registry-consumer show <feature-id>

# Lint specs
cargo build --release --manifest-path tools/spec-spine/spec-lint/Cargo.toml

# Codebase index
cargo build --release --manifest-path tools/spec-spine/codebase-indexer/Cargo.toml
./tools/spec-spine/codebase-indexer/target/release/codebase-indexer compile  # emit index.json
./tools/spec-spine/codebase-indexer/target/release/codebase-indexer check    # staleness check
./tools/oap/oap-code-index-enrich/target/release/oap-code-index-enrich render   # emit CODEBASE-INDEX.md (Cut D W-07b moved this out of codebase-indexer)

# Compile policies
cargo build --release --manifest-path tools/oap/policy-compiler/Cargo.toml

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

## Policy Rules

```policy
id: CONST-001-destructive-ops
description: "Block destructive file/git operations without explicit confirmation"
mode: enforce
scope: global
gate: destructive_operation
```

```policy
id: CONST-002-secrets-scanner
description: "Prevent committing API keys, tokens, private keys, .env files"
mode: enforce
scope: global
gate: secrets_scanner
```

```policy
id: CONST-003-tool-allowlist
description: "Warn when Tier3 (unclassified/dangerous) tools are invoked without approval"
mode: warn
scope: global
gate: tool_allowlist
```

```policy
id: CONST-004-diff-size
description: "Warn when a single patch exceeds 500 lines"
mode: warn
scope: global
gate: diff_size_limiter
```

```policy
id: CONST-005-spec-code-coherence
description: "Refuse instructions that engineer drift between spec and code; halt and surface (spec 131)"
mode: enforce
scope: global
gate: spec_code_coherence
```
