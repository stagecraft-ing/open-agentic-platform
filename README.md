# open-agentic-platform

> **Status:** pre-alpha, stealth. Single-developer repository. No public releases, no external contributors.

**The governed operating system for AI-native software delivery.**

Three layers, one contract system:

- **OPC** (`apps/desktop/`) — local Tauri v2 + React cockpit where humans and agents ask, plan, approve, execute, inspect, and rewind work
- **Platform** (`platform/`) — organisational control plane for identity, policy, approvals, deployments, audit
- **Spec Spine** (`specs/`) — canonical contract system that turns intent into traceable, machine-verifiable truth

## Architecture

```mermaid
flowchart LR
    subgraph OPC[OPC -- Local Cockpit]
        O1[Local workspaces]
        O2[Desktop UX]
        O3[Git and GitHub context]
        O4[Semantic and structural analysis]
        O5[Snapshots and checkpoints]
        O6[Human + agent collaboration]
    end

    subgraph SPINE[Shared Spec Spine]
        S1[Spec definitions]
        S2[Canonical feature registry]
        S3[Changesets and traceability]
        S4[Approval and policy metadata]
        S5[Compiled governance truth]
    end

    subgraph PLATFORM[Platform -- Organisational Control Plane]
        P1[Identity and access]
        P2[Policy engine]
        P3[Approval centre]
        P4[Deployment orchestration]
        P5[Activity timeline and audit]
        P6[Multi-repo feature graph]
    end

    OPC <--> SPINE
    PLATFORM <--> SPINE
    OPC <--> PLATFORM
```

OPC is where work is experienced. Platform is where work is governed. The Spec Spine is what keeps both sides honest.

## Core workflow

1. **Ask** — a human or agent expresses intent
2. **Plan** — intent expands into specs, plans, tasks, dependencies, risk boundaries
3. **Approve** — humans review proposed work, risk posture, policy checks, feature impact
4. **Execute** — agents perform bounded work against governed tools and workspaces
5. **Inspect** — structure, semantics, git context, runtime state, history, drift, results in one place
6. **Rewind** — work can be compared, restored, replayed, or rolled back with a full state trail

## Spec-first foundation

Human truth is **markdown** (with optional YAML frontmatter); machine registries are **compiler-emitted JSON** only. Authoritative specs live at repo-root `specs/NNN-kebab-case/`. The constitutional baseline is [`000-bootstrap-spec-system`](specs/000-bootstrap-spec-system/spec.md); status lifecycle is `draft | approved | superseded | retired`.

Current spec corpus: 107 specs (`000`–`106`). Query the compiled registry:

```bash
./tools/registry-consumer/target/release/registry-consumer list
./tools/registry-consumer/target/release/registry-consumer status-report
```

## Factory delivery engine

The Factory (`factory/`, `crates/factory-engine/`, `crates/factory-contracts/`) is a two-phase AI pipeline:

1. **Phase 1** (s0–s5) — six sequential stages extract requirements, design services, model data, specify APIs and UI, producing a frozen Build Spec
2. **Phase 2** (s6a–s6g) — dynamic scaffold fan-out generates code per-entity, per-operation, per-page with post-step verification and retry

Four pluggable adapters: `aim-vue-node`, `next-prisma`, `rust-axum`, `encore-react`.

## Claude-native development

This repository ships with first-class [Claude Code](https://docs.anthropic.com/en/docs/claude-code) integration in `.claude/`:

- **Agents** — `architect`, `explorer`, `implementer`, `reviewer`, `encore-expert`
- **Commands** — `/init`, `/commit`, `/code-review`, `/review-branch`, `/implement-plan`, `/research`, `/validate-and-fix`, `/cleanup`, `/factory-sync`, `/refactor-claude-md`
- **Rules** — orchestrator behavioral rules (step ordering, file-based artifact passing, checkpoint discipline, governed artifact reads)

Combined with `CLAUDE.md` (project conventions) and `AGENTS.md` (session init protocol), this is the environment the platform is built in.

## Getting started

```bash
make setup     # install deps, build tools, compile spec registry + codebase index
make dev       # start OPC desktop (Vite + Tauri, hot-reload)
make ci        # parallel local validation (~5 min warm) — daily dev loop (spec 135)
make ci-strict # full parity mirror (~90 min) — pre-merge / parity-investigation
```

Full setup guide, prerequisites, and platform-service instructions: **[DEVELOPERS.md](DEVELOPERS.md)**.

Repository layout and conventions: **[CLAUDE.md](CLAUDE.md)**.
