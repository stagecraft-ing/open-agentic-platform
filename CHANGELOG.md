# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-05

Initial open source release.

### Spec System (specs 000-006)
- Constitutional bootstrap: markdown-only authored truth, compiler-emitted JSON machine truth
- Spec compiler producing deterministic `registry.json` with content hashing
- Registry consumer with list, show, status, and governance gate commands
- Conformance linter with W-xxx workflow warnings
- Feature lifecycle semantics (draft / active / superseded / retired)

### Registry Consumer (specs 007-031)
- 25 specs delivering structured JSON output, compact display, sorting, filtering, field shape invariants, error contracts, help/usage, and contract governance

### Agent and Orchestrator Framework (specs 035-044, 052)
- Multi-agent orchestration with DAG validation, artifact-based context passing, effort classification
- Checkpoint and approval gates with timeout escalation
- Post-step verification with retry (compile, test, lint)
- SQLite state persistence with crash-resume and SSE event replay
- Safety tier governance for agent execution

### Desktop App — OPC (specs 032, 041, 050, 058-060, 064-066)
- Tauri v2 + React desktop cockpit
- Inspect surface with xray structural analysis
- Git panel, session memory, notification system
- Factory pipeline visualization with DAG, artifact inspector, gate dialogs

### Platform Services (specs 047, 072, 077)
- Stagecraft (Encore.ts): auth, admin, monitoring, Slack, GitHub webhooks, Factory lifecycle API
- deployd-api-rs (axum + hiqlite): Kubernetes deployment orchestration
- Rauthy OIDC identity provider
- Multi-cloud K8s portability with Terraform + Helm

### Claude Code Integration (specs 067-071)
- Tool Definition Registry with permission gates
- Permission Runtime and Settings Layering (5-tier merge)
- Lifecycle Hook Runtime
- Prompt Assembly and Cache Boundaries
- Skill and Command Factory

### Axiomregent Unification (spec 073)
- Unified MCP agent absorbing gitctx, blockoli, stackwalk
- GitHub API tools, semantic search, checkpoint subsystem
- Distributed locking, event system, tree-sitter integration

### Xray Analysis (spec 032)
- Complexity scoring, incremental scanning, call graphs
- Dependency extraction, semantic embeddings, structural fingerprinting
- Policy engine and context budget optimizer

### Factory Delivery Engine (specs 074-078)
- Rust contract types: BuildSpec, AdapterManifest, PipelineState, Verification
- Two-phase workflow engine: process stages (s0-s5) and scaffold fan-out (s6a-s6g)
- Four adapters: aim-vue-node, next-prisma, rust-axum, encore-react
- Native Rust verification harness (replaced Python)
- `factory-run` CLI for end-to-end pipeline execution with real agent dispatch
- Orchestrator integration: gate evaluation, verify+retry loop
- Desktop pipeline visualization panel

### Infrastructure
- 11 GitHub Actions CI workflows (desktop, stagecraft, deployd-api, tools, spec conformance)
- Cross-platform release automation (macOS, Linux, Windows)
- Root Makefile for streamlined local development
- Claude Code agents, commands, and rules as first-class development infrastructure
