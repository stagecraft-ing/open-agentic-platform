# open-agentic-platform

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024_edition-orange.svg)](https://www.rust-lang.org/)
[![Specs](https://img.shields.io/badge/Specs-57_active-green.svg)](specs/)

**The governed operating system for AI-native software delivery.**

> [Getting Started](DEVELOPERS.md) | [Architecture](docs/ARCHITECTURE.md) | [Specs](specs/000-bootstrap-spec-system/spec.md)

---

## TLDR: Agents
Authoritative architecture rules: human truth is **markdown** (with optional YAML **frontmatter inside** `.md` files); machine registries are **compiler-emitted JSON** only. See the constitutional bootstrap spec:

- [`specs/000-bootstrap-spec-system/spec.md`](specs/000-bootstrap-spec-system/spec.md)
- [`.specify/contract.md`](.specify/contract.md)

The **spec compiler MVP** (implements Feature 000’s contracts) is specified in [`specs/001-spec-compiler-mvp/spec.md`](specs/001-spec-compiler-mvp/spec.md). Build and run from the repo root:

```bash
cargo build --release --manifest-path tools/spec-compiler/Cargo.toml
./tools/spec-compiler/target/release/spec-compiler compile
```

Outputs: `build/spec-registry/registry.json` and `build-meta.json`. Details: [`tools/spec-compiler/README.md`](tools/spec-compiler/README.md).

The **registry consumer** (Feature 002) reads `build/spec-registry/registry.json` after a successful compile:

```bash
cargo build --release --manifest-path tools/registry-consumer/Cargo.toml
./tools/registry-consumer/target/release/registry-consumer list
./tools/registry-consumer/target/release/registry-consumer show 000-bootstrap-spec-system
```

Details: [`tools/registry-consumer/README.md`](tools/registry-consumer/README.md). Lifecycle semantics for feature **`status`** (draft / active / superseded / retired): [`specs/003-feature-lifecycle-mvp/spec.md`](specs/003-feature-lifecycle-mvp/spec.md).

**Execution protocol** (spec → plan → tasks → changeset / verification, task lifecycle vs feature lifecycle): [`specs/004-spec-to-execution-bridge-mvp/spec.md`](specs/004-spec-to-execution-bridge-mvp/spec.md).

**Verification & reconciliation** (evidence, changeset states, drift, `execution/verification.md`): [`specs/005-verification-reconciliation-mvp/spec.md`](specs/005-verification-reconciliation-mvp/spec.md).

**Conformance lint** (optional **`W-xxx`** workflow warnings; non-blocking vs **`spec-compiler`**): [`specs/006-conformance-lint-mvp/spec.md`](specs/006-conformance-lint-mvp/spec.md), tool: [`tools/spec-lint/README.md`](tools/spec-lint/README.md).

**OPC desktop** (inspect / governance / git context — operator notes): [`apps/desktop/README.md`](apps/desktop/README.md).

**Registry-consumer governance** (controlled extension mode, change-class rubric, release gate checklist, contract baseline): [`docs/registry-consumer-contract-governance.md`](docs/registry-consumer-contract-governance.md).

---

## Open Agentic Platform

**The governed operating system for AI-native software delivery.**

Open Agentic Platform (OAP) is the future-state fusion of two systems:

- **OPC** - the local cockpit where humans and agents ask, plan, approve, execute, inspect, and rewind work
- **Platform** - the organisational control plane that governs identity, policy, environments, approvals, deployments, and audit

What binds them is a third layer:

- **The Spec Spine** - the canonical contract system that turns intent into traceable, machine-verifiable truth

OAP is built on a simple belief:

> the best AI engineering environment is not just fast - it is fast, governed, reviewable, and replayable.

---

## Why this exists

Today, teams are forced to choose between two incomplete worlds:

- **local AI tools** that feel powerful but become chaotic at scale
- **enterprise platforms** that promise control but sit too far away from the work itself

Open Agentic Platform is designed to close that gap.

It gives developers an AI-native cockpit for real engineering work, while giving organisations a control plane for policy, visibility, reuse, and trust.

The result is a system where:

- local speed does not come at the cost of governance
- agent execution is bounded, inspectable, and auditable
- specs are not static documents; they become operational infrastructure
- every meaningful change can be traced from idea to execution to approval to deployment

---

## The pitch in one paragraph

Open Agentic Platform is a full-stack system for governed agentic software delivery. It combines a rich local developer cockpit with an organisational platform layer, connected through a shared spec and registry spine. Engineers get high-agency local workflows with AI; organisations get policy enforcement, approvals, workspace control, deployment orchestration, and auditability. Instead of bolting governance onto AI after the fact, OAP makes governance part of the execution model from the beginning.

---

## What the finished system becomes

In its intended end state, Open Agentic Platform enables teams to:

- express work as intent through chat, commands, workflows, and agent coordination
- expand that intent into specs, plans, tasks, dependencies, and risk boundaries
- review execution paths before work happens
- run governed agent workflows inside local or remote workspaces
- inspect git, code structure, semantic meaning, feature relationships, runtime traces, and execution history in one place
- rewind, diff, branch, restore, and replay work with checkpoint-backed confidence
- manage many repos, agents, workspaces, and environments as a single governed portfolio
- promote trusted work from local exploration to organisational execution without losing context

This is not just chat plus tools.
It is a **closed-loop system for software change**.

---

## OPC <-> Platform symbiosis

```mermaid
flowchart LR
    subgraph OPC[OPC - Local Cockpit]
        A1[Ask]
        A2[Plan]
        A3[Approve]
        A4[Execute]
        A5[Inspect]
        A6[Rewind]

        O1[Local workspaces]
        O2[Desktop UX]
        O3[Git and GitHub context]
        O4[Semantic and structural analysis]
        O5[Snapshots and checkpoints]
        O6[Human plus agent collaboration]
    end

    subgraph SPINE[Shared Spec Spine]
        S1[Spec definitions]
        S2[Canonical feature registry]
        S3[Changesets and traceability]
        S4[Approval and policy metadata]
        S5[Compiled governance truth]
    end

    subgraph PLATFORM[Platform - Organisational Control Plane]
        P1[Remote workspace registry]
        P2[Shared agent registry]
        P3[Policy engine]
        P4[Approval centre]
        P5[Identity and access]
        P6[Activity timeline and audit]
        P7[Multi-repo feature graph]
        P8[Deployment orchestration]
    end

    OPC <--> SPINE
    PLATFORM <--> SPINE
    OPC <--> PLATFORM
```

### The mental model

- **OPC** is where work is experienced
- **Platform** is where work is governed
- **The Spec Spine** is what keeps both sides honest

OPC without Platform is a powerful but isolated tool.
Platform without OPC is a control plane disconnected from real engineering.
Together, they create a system where local execution and organisational governance reinforce each other.

---

## The core workflow

### 1. Ask
A human or agent expresses intent.

### 2. Plan
The system expands intent into structured specs, plans, dependencies, affected features, and execution boundaries.

### 3. Approve
Humans review the proposed work, risk posture, policy checks, and feature impact.

### 4. Execute
Agents perform bounded work against governed tools and workspaces.

### 5. Inspect
The system exposes visibility into structure, semantics, git context, runtime state, history, drift, and results.

### 6. Rewind
Any work can be compared, restored, replayed, or rolled back with a full trail of state transitions.

---

## What lives where

### OPC ('apps/desktop') owns the local loop

- AI-native desktop workflow
- workspace execution and local orchestration
- repository inspection and semantic discovery
- git and GitHub context exploration
- snapshots, diffs, checkpoints, and rewind
- fast human feedback and agent collaboration

### Platform owns the organisational loop

- identity, access, and org policy
- shared workspace and environment management
- deployment orchestration and promotion
- approvals, reviews, and audit trails
- shared agent catalogue and controls
- cross-repo reasoning and portfolio visibility

### The Spec Spine binds both sides

- feature definitions
- execution contracts
- changesets and traceability links
- approval records
- governance metadata
- canonical compiled machine truth

---

## Why this is different

### AI work becomes reviewable
Plans, changesets, policy checks, feature impact, and execution results are first-class artifacts rather than hidden model behavior.

### Specs become infrastructure
The spec layer is not documentation sitting off to the side. It becomes the operational contract between humans, agents, cockpit, and platform.

### Rewind is built in
Checkpointing, snapshots, and auditable state transitions make rollback and replay part of the system design, not an afterthought.

### Local and organisational workflows finally connect
The same work can move from a developer cockpit to a governed platform without losing structure, context, or trust.

### It is built for real multi-repo organisations
The future platform is designed for many codebases, many environments, many agents, and many approval boundaries.

---

## Who this is for

Open Agentic Platform is for teams that want AI leverage without losing operational discipline.

Especially:

- engineers who want deep local agency with AI
- platform teams who need standards, control, and auditability
- organisations adopting agentic workflows in serious production settings
- product and infrastructure groups working across multiple repositories and environments
- teams that care about trust, not just demos

---

## Future-state capability map

When complete, the platform will present a unified experience for:

- conversational intent capture
- structured planning and task decomposition
- bounded agent execution
- policy-aware approval flows
- semantic search and structural analysis
- git and GitHub context exploration
- feature graph and governance drift analysis
- snapshotting, checkpointing, and rewind
- shared remote workspaces and environments
- reusable agents, skills, and workflow registries
- activity timelines, audit trails, and release promotion

---

## Why people should care

Open Agentic Platform is not trying to be another thin wrapper around an LLM.
It is an attempt to define what a serious AI-native engineering system should look like when:

- developers need speed
- organisations need control
- AI needs boundaries
- work needs to stay understandable over time

If successful, OAP gives teams a path from individual AI-assisted productivity to governed, organisation-scale agentic delivery.

That is the real idea: **make AI-native engineering operationally trustworthy.**

---

## Claude-native development

This repository ships with first-class [Claude Code](https://docs.anthropic.com/en/docs/claude-code) integration. The `.claude/` directory contains development infrastructure that contributors can use immediately:

- **Agents** (`.claude/agents/`) — architect, explorer, implementer, reviewer: task-focused sub-agents for parallel work
- **Commands** (`.claude/commands/`) — `/init`, `/commit`, `/code-review`, `/validate-and-fix`, `/research`, `/implement-plan`, `/cleanup`
- **Rules** (`.claude/rules/`) — Orchestrator behavioral rules enforcing step ordering, file-based artifact passing, and checkpoint discipline

Combined with `CLAUDE.md` (project conventions) and `AGENTS.md` (agent protocol), this means every contributor gets AI-assisted development workflows out of the box. This is not optional tooling — it is how the platform was built.

---

## Factory delivery engine

The Factory (`factory/`, `crates/factory-engine/`, `crates/factory-contracts/`) is a two-phase AI pipeline that transforms business requirements into production code:

1. **Phase 1** (s0-s5): Six sequential stages extract requirements, design services, model data, specify APIs and UI, producing a frozen Build Spec
2. **Phase 2** (s6a-s6g): Dynamic scaffold fan-out generates code per-entity, per-operation, per-page with post-step verification

Four pluggable adapters: `aim-vue-node`, `next-prisma`, `rust-axum`, `encore-react`.

Run a pipeline:
```bash
cargo build --release --manifest-path crates/factory-engine/Cargo.toml
cargo run --manifest-path crates/factory-engine/Cargo.toml --bin factory-run -- \
  --adapter next-prisma --project /tmp/my-app \
  --business-docs requirements.md
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full pipeline architecture.
