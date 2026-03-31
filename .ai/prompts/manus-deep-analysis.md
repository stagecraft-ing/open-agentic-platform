# Manus Deep Analysis — Open Agentic Platform

## Repository

https://github.com/stagecraft-ing/open-agentic-platform

## Your Mission

Perform a comprehensive analysis of this repository to inform the next major development phase: building the **Platform control plane** — the organisational layer handling identity, policy enforcement, approval workflows, deployment governance, and audit. Today the project has a mature spec system (67 specs), a Tauri+React desktop app (OPC), 12 Rust crates, 20 TypeScript packages, and 3 CLI tools — but no running platform services yet. Your analysis should bridge the gap between what exists and what needs to be built.

---

## Analysis Areas (in priority order)

### 1. Architecture & Dependency Map

- Map the full dependency graph: Rust crates → Tauri commands → TypeScript packages → React components. Identify which modules are actually wired end-to-end vs scaffolded but disconnected.
- For each Rust crate in `crates/`, determine: is it used by the desktop app? By CLI tools? By nothing? Pay special attention to `policy-kernel` (compiles to WASM), `orchestrator`, `agent`, and `run`.
- Map the 20 `@opc/*` TypeScript packages in `packages/` — which are imported by `apps/desktop/`? Which are standalone libraries? Are there circular dependencies?
- Identify the integration seams where a future platform service would need to connect (e.g., where does the desktop app currently make decisions that should come from a central authority?).

### 2. Governance & Policy Infrastructure Assessment

The project's thesis is "governed AI execution." Assess how real the governance is today:

- Read `specs/047-governance-control-plane/spec.md` carefully. This is the P0 platform spec. Assess: how much of it is implemented in `crates/policy-kernel/` and the TypeScript packages?
- Trace the actual enforcement path: when the desktop app runs an agent, what governance checks actually execute? Is there a real policy evaluation, or is it UI-only?
- Examine `crates/axiomregent/`, `crates/agent/`, `crates/run/` — do these enforce safety tiers, permissions, or checkpoints at runtime? Or are they stubs?
- Look at `packages/permission-system/`, `packages/hookify-rule-engine/`, `packages/verification-profiles/` — are these functional with tests, or scaffolding?
- Read `specs/036-safety-tier-governance/spec.md` and `specs/035-agent-governed-execution/spec.md` — compare what's specced vs what the Rust code actually does.

### 3. Spec Spine Health & Gaps

- Read the YAML frontmatter of all 67 specs. Build a matrix: spec ID, title, status (draft/active), priority (P0/P1/P2), kind (platform/desktop/tooling), and whether implementation artifacts exist.
- Identify specs marked `active` that have no corresponding code, and code that has no corresponding spec.
- Look for contradictions between specs — e.g., does spec 044 (multi-agent orchestration) conflict with spec 043 (agent organizer)? Does spec 049 (permission system) overlap with spec 047 (governance control plane)?
- Which specs would need to be implemented first to unblock the platform layer? Build a dependency graph of specs 042-066.

### 4. Code Quality & Technical Debt

- Assess test coverage: for the Rust crates, do they have meaningful tests or just `#[test] fn it_works()`? For the TypeScript packages, check vitest test files — are they comprehensive or minimal?
- Look for dead code: crates or packages that are defined but never imported anywhere.
- Check the Tauri command surface (`apps/desktop/src-tauri/src/commands/`) — are there commands that are registered but never called from the frontend?
- Assess error handling patterns: does the codebase use `Result<T, E>` consistently in Rust? Are TypeScript errors typed or `any`?
- Review the `pnpm-workspace.yaml` and root `package.json` — is the monorepo configured correctly? Are there version inconsistencies?

### 5. Platform Readiness Assessment

Given that the next phase is building platform services, assess:

- **What can be extracted into a service today?** Which TypeScript packages or Rust crates have clean enough interfaces to become microservices or API endpoints without major refactoring?
- **What's missing entirely?** There's no auth, no API gateway, no database schema, no event streaming, no multi-tenant support. Prioritize: what should be built first to enable everything else?
- **Where are the hard decisions?** E.g., should the policy kernel (Rust/WASM) run server-side, client-side, or both? Should the platform be a monolith or microservices? REST or gRPC? What's the data model for organizations, teams, and policies?
- **Competitive landscape**: How does this compare to other "governed AI" or "AI DevOps" platforms? (e.g., Langchain's LangSmith, CrewAI, AutoGen, Fixie, Humanloop, Braintrust, etc.) What's unique here and what's table-stakes that's missing?

### 6. CI/CD & Release Infrastructure

- Analyze `.github/workflows/` — what's automated? What's missing? (e.g., no test CI, no package publishing, no integration tests)
- The `release-desktop.yml` builds platform installers — is it robust? Does it handle signing, notarization, auto-update?
- What CI would a platform service need? Draft a recommendation.

### 7. Developer Experience & Onboarding

- Clone the repo (or analyze the setup). How hard is it to get running? What are the prerequisites?
- Is the `CLAUDE.md` / `AGENTS.md` / `.claude/` setup coherent and useful, or is it over-engineered for the current state?
- Are there README gaps? Can a new contributor understand the architecture from the docs alone?

---

## Output Format

Structure your analysis as a report with these sections:

1. **Executive Summary** (1 page) — The 5 most important findings and recommendations
2. **Architecture Map** — Visual or structured representation of the dependency graph
3. **Governance Reality Check** — What's enforced vs what's aspirational
4. **Spec Health Matrix** — Table of all specs with status, implementation state, and gaps
5. **Platform Roadmap Recommendations** — Prioritized list of what to build first, with justification
6. **Technical Debt Inventory** — Categorized by severity (blocking / should-fix / cosmetic)
7. **Competitive Positioning** — Where this project fits in the AI tooling landscape
8. **Quick Wins** — 5-10 things that could be done in a day each to improve the project significantly

---

## Important Context

- The project uses a "spec spine" methodology: every feature starts as a spec in `specs/NNN-slug/spec.md` with YAML frontmatter, gets compiled into a machine-readable registry (`build/spec-registry/registry.json`), and is verified against the implementation.
- Specs 000-041 are foundational (spec system + desktop app features). Specs 042-066 are the next generation covering platform capabilities.
- The Rust `crates/policy-kernel/` is designed to compile to both native and `wasm32` — this is a key architectural decision for the platform (policy evaluation at edge vs server).
- The project is AGPL-licensed — this affects platform deployment strategy (SaaS implications).
- Recent velocity has been ~1 feature/week, each delivered in 4-8 phase commits. The development is primarily AI-assisted (Claude Code, Cursor).
- There are 20 `@opc/*` TypeScript packages under `packages/` managed by pnpm workspaces, each mapped 1:1 to a spec.

## What Makes This Analysis Valuable

We're at an inflection point. The desktop cockpit and spec system are mature. The next step is the platform layer — but we need to know:
1. Is the foundation solid enough to build on, or are there structural issues to fix first?
2. What's the right architecture for platform services given what exists?
3. Where are we fooling ourselves — specs that claim completeness but have hollow implementations?
4. What should the first platform service be, and what's the minimal viable slice?

---

## Pre-Gathered Context (verified 2026-03-31)

The following data has been gathered from the live codebase. Use it to skip discovery and go straight to deeper analysis, cross-referencing, and recommendations. Verify anything that seems surprising before building conclusions on it.

### A. Implementation Depth Summary

| Area | Files | ~Lines | Assessment |
|------|-------|--------|------------|
| Desktop App (React) | 137 TSX/TS | 43,635 | Fully implemented — 72 UI components, routing, context-based state management |
| Desktop App (Tauri) | 30+ Rust | 14,868 | Fully implemented — rich command set, checkpoint system, sidecar management |
| Rust Crates (12) | 100+ | ~42,000 | 80% complete — titor, axiomregent, gitctx, orchestrator are production-ready; run is ~30% |
| TS Packages (20) | 331 | ~48,000 | 85% complete — most packages substantive; mcp-client intentionally thin |
| CLI Tools (4) | 10+ | ~2,100 | Fully implemented — spec-compiler, registry-consumer, spec-lint, policy-compiler |
| **TOTAL** | **600+** | **~150,000+** | **Substantial, production-oriented codebase** |

### B. Rust Crate Assessment

| Crate | Lines | Status | Notes |
|-------|-------|--------|-------|
| titor | 12,639 | Fully implemented | Checkpointing: snapshots, Merkle trees, compression, parallel processing |
| axiomregent | 5,630 | Fully implemented | MCP router: agent tools, feature tools, workspace management, run tools |
| gitctx | 6,780 | Fully implemented | GitHub MCP server: auth, caching, GitHub API client, XML formatting. Standalone sidecar binary |
| orchestrator | 4,361 | Fully implemented | Multi-agent: manifest parsing, DAG validation, SQLite state, SSE events, HTTP layer |
| agent | 2,900 | Fully implemented | Complexity scoring, dispatch, executor, plan building, validator, safety checks |
| featuregraph | 1,330 | Fully implemented | Registry scanning, preflight checks, tool discovery |
| xray | 1,473 | Fully implemented | Canonical JSON, digest computation, language detection, LOC analysis, Merkle traversal |
| asterisk | 1,195 | Fully implemented | Code indexing: call graphs, tree-sitter parsing, block analysis |
| policy-kernel | 1,115 | Fully implemented | Policy evaluation: coherence scheduler, proof chains, destructive op gates, secrets scanning. Compiles to WASM |
| blockoli | 1,318 | Partial | Vector store/embeddings foundation only |
| stackwalk | 1,195 | Partial | Core modules present but less developed |
| run | 454 | Partial (~30%) | Task runner skeleton: config, registry, runner stubs |

### C. TypeScript Package Assessment

**Fully implemented (3000+ lines):** provider-registry (3,438 — Anthropic/OpenAI/Gemini/Bedrock adapters), yaml-standards-schema (3,514), notification-orchestrator (3,716), conductor-track (3,039 — state machine, storage, plan implementation)

**Well implemented (1500–3000 lines):** file-mention (2,591), git-panel (2,541), hookify-rule-engine (2,509), session-memory (2,739), permission-system (1,684), verification-profiles (2,291), coherence-scoring (1,764), ui (1,767 — 20 shadcn-style components), multi-model-chaining (1,715 — streaming, cost aggregation)

**Intentionally minimal:** mcp-client (68 lines — thin Tauri invoke wrapper), types (package.json only), claude-code-bridge (680 lines — bridge IPC), agent-frontmatter (1,086 lines)

### D. Dependency Wiring Map

```
┌─ apps/desktop (TypeScript + Rust)
│  ├─ TS deps: @opc/claude-code-bridge, @opc/mcp-client, @opc/types, @opc/ui
│  │  (Only 4 of 20 @opc/* packages are direct desktop dependencies)
│  └─ Rust deps: titor, xray, featuregraph, agent, run, blockoli, asterisk, stackwalk, orchestrator
│     (9 of 12 crates linked into desktop)
│
├─ crates/ inter-dependencies
│  ├─ axiomregent → featuregraph, xray, agent, run, policy-kernel [sidecar binary]
│  ├─ blockoli → asterisk
│  └─ orchestrator (standalone)
│
├─ tools/
│  ├─ spec-compiler, registry-consumer, spec-lint → no crate deps (only shared/frontmatter)
│  └─ policy-compiler → policy-kernel (ONLY tool that uses a crate)
│
├─ packages/ inter-dependencies
│  ├─ multi-model-chaining → provider-registry → {claude-code-bridge, permission-system}
│  └─ All other 17 packages have NO internal @opc/* deps
│
└─ Standalone/sidecar (not linked into desktop binary):
   ├─ gitctx [sidecar binary, MCP server]
   ├─ axiomregent [sidecar binary, MCP router]
   └─ policy-kernel (only used by policy-compiler tool)
```

**Key finding:** 16 of 20 TypeScript packages are NOT directly imported by the desktop app or by other packages. They exist as standalone feature modules — either awaiting integration or designed as future service components.

### E. Spec Health Matrix (all 67 specs)

| ID | Title | Status | Kind | Execution Artifacts |
|---|---|---|---|---|
| 000 | Bootstrap spec system | active | constitutional-bootstrap | No |
| 001 | Spec compiler MVP | active | tooling | No |
| 002 | Registry consumer MVP | draft | platform | No |
| 003 | Feature lifecycle & status semantics | draft | platform | No |
| 004 | Spec-to-execution bridge | draft | platform | Yes |
| 005 | Verification and reconciliation | draft | platform | Yes |
| 006 | Conformance lint | draft | platform | No |
| 007 | Registry consumer status reporting UX | active | platform-delivery | Yes |
| 008 | Registry consumer status-report JSON output | active | platform-delivery | Yes |
| 009 | Registry consumer status-report nonzero filtering | active | platform-delivery | Yes |
| 010 | Registry consumer status-report JSON contract tests | active | platform-delivery | Yes |
| 011 | Registry consumer status-report status filter | active | platform-delivery | Yes |
| 012 | Registry consumer list JSON output | active | platform-delivery | Yes |
| 013 | Registry consumer show JSON contract | active | platform-delivery | Yes |
| 014 | Registry consumer show compact JSON | active | platform-delivery | Yes |
| 015 | Registry consumer list compact JSON | active | platform-delivery | Yes |
| 016 | Registry consumer status-report compact JSON | active | platform-delivery | Yes |
| 017 | Registry consumer shared JSON serialization helper | active | platform-delivery | Yes |
| 018 | Registry consumer list/show JSON contract tests | active | platform-delivery | Yes |
| 019 | Registry consumer README examples contract | active | platform-delivery | Yes |
| 020 | Registry consumer error contract MVP | active | platform-delivery | Yes |
| 021 | Registry consumer field-shape invariants contract | active | platform-delivery | Yes |
| 022 | Registry consumer help/usage output contract | active | platform-delivery | Yes |
| 023 | Registry consumer flag-conflict argument-validation | active | platform-delivery | Yes |
| 024 | Registry consumer version/banner contract | active | platform-delivery | Yes |
| 025 | Registry consumer default-path contract | active | platform-delivery | Yes |
| 026 | Registry consumer allow-invalid contract | active | platform-delivery | Yes |
| 027 | Registry consumer sorting-order contract | active | platform-delivery | Yes |
| 028 | Registry consumer stderr/stdout channel discipline | active | platform-delivery | Yes |
| 029 | Registry consumer contract-governance gate | active | platform-delivery | Yes |
| 030 | Registry consumer internal output/exit refactor | active | platform-delivery | Yes |
| 031 | Registry consumer list ids-only contract | active | platform-delivery | Yes |
| 032 | OPC inspect + governance wiring | active | product-consolidation | Yes |
| 033 | axiomregent sidecar activation | active | platform | Yes |
| 034 | featuregraph scanner reads compiled registry | active | platform | Yes |
| 035 | agent execution through governed dispatch | active | platform | Yes |
| 036 | safety tier governance | active | platform | No |
| 037 | cross-platform axiomregent binaries | active | platform | No |
| 038 | titor Tauri command wiring | active | platform | No |
| 039 | Feature ID reconciliation (codeAliases) | active | platform | Yes |
| 040 | Blockoli semantic search wiring | active | product | No |
| 041 | Checkpoint / Restore UI panel | active | product | No |
| 042 | Multi-Provider Agent Registry | draft | platform | No |
| 043 | agent organizer and meta-orchestrator | active | platform | Yes |
| 044 | multi-agent orchestration with artifacts | draft | platform | Yes |
| 045 | Claude Code SDK bridge | active | platform | No |
| 046 | Context Compaction for Session Resumption | active | product | No |
| 047 | governance control plane (policy compiler) | active | platform | Yes |
| 048 | Declarative Hook Rule Engine | draft | platform | Yes |
| 049 | Permission Request System | draft | platform | Yes |
| 050 | Config-Driven Tool Rendering System | active | platform | Yes |
| 051 | Background Agents with Worktree Isolation | draft | platform | Yes |
| 052 | State Persistence for Resumable Workflows | draft | platform | Yes |
| 053 | Verification Profiles | draft | platform | No |
| 054 | Agent Frontmatter Schema | draft | platform | No |
| 055 | YAML Standards Schema | draft | platform | No |
| 056 | Session Memory / Project-Object Persistence | active | platform | No |
| 057 | Notification Orchestrator | draft | platform | No |
| 058 | @-Mention Autocomplete for Files and Agents | active | ui | No |
| 059 | Comprehensive Git Panel | draft | ui | No |
| 060 | Typed Inter-Panel Event Bus | active | platform | No |
| 061 | Conductor Track Lifecycle | draft | platform | No |
| 062 | Multi-Model Chaining | active | platform | No |
| 063 | Coherence Scoring with Privilege Degradation | active | platform | No |
| 064 | WebSocket Session Reconnection | active | desktop | No |
| 065 | Encrypted Keychain and Credential Storage | active | desktop | No |
| 066 | VS Code Extension (Sidebar Webview) | active | platform | No |

**Totals:** 39 active / 28 draft. 44 have execution artifacts / 23 do not.

**Pattern:** Specs 007–031 (registry consumer series) are 100% active with execution artifacts — 25 specs for a single CLI tool. Specs 042–066 are the next-gen platform capabilities, mostly draft or recently activated.

### F. Governance Enforcement — Reality Check

**What is REAL and enforced at runtime:**

1. **Safety Tier Classification (036)** — `crates/agent/src/safety.rs` classifies all 21 router tools into Tier1/Tier2/Tier3. Tests validate completeness.
2. **Permission Flag Checking (035)** — `crates/axiomregent/src/router/permissions.rs` implements `check_grants()` and `check_tool_permission()` with three flags: `enable_file_read`, `enable_file_write`, `enable_network`. Tier ceiling enforcement via `max_tier`. Unit tests confirm blocking logic.
3. **Policy Kernel Gates (047)** — `crates/policy-kernel/src/lib.rs` implements four gates: secrets scanner, destructive op guard, tool allowlist, diff size limiter. Tests pass.

**What is IMPLEMENTED but UNUSED at runtime:**

4. **Coherence Scheduler (047)** — Full privilege degradation logic (Full → Restricted → ReadOnly → Suspended) in `crates/policy-kernel/src/coherence.rs`. Rolling-window scoring. **Never instantiated** — no code path calls `CoherenceScheduler::new()`.
5. **Proof Chain / Audit Trail (047)** — Cryptographic proof records in `crates/policy-kernel/src/proof_chain.rs`. Data structures and tests exist. **Never called at runtime.**
6. **Policy Bundles** — `build/policy-bundles/policy-bundle.json` contains an **empty constitution and empty shards**. No actual policies are authored anywhere.

**What is DISCONNECTED:**

7. **Permission System package** — `packages/permission-system/` has a functional evaluator with tests but is NOT used by axiomregent or agent execution.
8. **Hookify Rule Engine** — `packages/hookify-rule-engine/` implements priority-ordered rule evaluation. NOT connected to governance enforcement.

**The actual runtime enforcement flow:**
```
Agent Execution (agents.rs)
  → planGoverned() checks if axiomregent sidecar is available
  → IF available: pass permission grants via OPC_GOVERNANCE_GRANTS env var
    → router::preflight_tool_permission() checks tier rank + permission flags
    → Tool dispatched or PermissionDenied
  → IF unavailable: fallback to --dangerously-skip-permissions (governance bypass)
```

**Verdict:** Governance is **REAL for tier + permission flags** but **ASPIRATIONAL for policy gates, coherence degradation, and audit trails**. The policy kernel has working code that is never invoked because no policies are authored. The fallback to unrestricted mode when axiomregent is unavailable means governance is opt-in, not enforced.

### G. CI/CD Workflows (5 total in .github/workflows/)

1. **spec-conformance.yml** — Builds/tests spec-compiler, registry-consumer, spec-lint on push to main + PRs
2. **build-axiomregent.yml** — Builds axiomregent sidecar for macOS/Linux/Windows (manual + push trigger)
3. **build-gitctx-mcp.yml** — Same pattern for gitctx-mcp sidecar
4. **release-desktop.yml** — Full Tauri build for all platforms, bundles sidecars, creates draft GH Release (tag trigger)
5. **release-tools.yml** — Builds CLI tools for all platforms, attaches to same GH Release

**What's missing from CI:** No test runner for TypeScript packages. No integration tests. No package publishing (npm). No code coverage. No linting/formatting enforcement. No dependency vulnerability scanning.

### H. What's Missing Entirely for the Platform Vision

These are capabilities described in the README's "future-state capability map" with **zero implementation**:

- **Auth / Identity service** — no IAM, no org/team model, no SSO
- **API gateway / backend service** — no server, no REST/gRPC endpoints, no database schema
- **Remote workspace registry** — all workspaces are local-only
- **Deployment orchestration** — no CI/CD integration, no environment promotion
- **Approval centre** — no human-in-the-loop approval workflow service
- **Centralized audit trail** — local session history only
- **Multi-repo feature graph** — featuregraph is single-repo
- **Shared agent catalogue** — local agent management only
- **Reusable workflow registries** — not started
- **Multi-tenant support** — no tenancy model
- **Event streaming** — no pub/sub or event bus beyond local panel events

### I. Observations for Your Analysis

1. **Registry consumer spec concentration** — 25 of 67 specs (37%) service a single CLI tool. This is thorough contract work but heavily weighted.
2. **16 of 20 TS packages are disconnected** — They exist as feature implementations but aren't wired into the desktop app or each other. They may be designed as future service modules, or they may be dead code.
3. **AGPL license + SaaS gap** — The project is AGPL-licensed but has no server component. If the platform becomes a hosted service, AGPL has significant implications for deployment strategy.
4. **The `run` crate is the thinnest** — At ~30% complete, task execution is central to the vision but least developed in the runtime layer.
5. **No database anywhere** — SQLite is used transiently by orchestrator for state, but there's no persistent data model for the platform. This is the single biggest infrastructure gap.
6. **Sidecar architecture is clever but fragile** — axiomregent and gitctx run as sidecar processes managed by the desktop app. This works for local use but doesn't translate to a platform service without rearchitecting.
