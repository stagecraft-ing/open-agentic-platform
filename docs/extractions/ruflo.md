---
source: ruflo
source_path: ~/Dev2/stagecraft-ing/ruflo
status: extracted
---

## Summary

Ruflo (formerly "Claude Flow") is a TypeScript/Rust enterprise AI agent orchestration platform (v3.5, ~2,461 source files). It provides multi-agent swarm coordination with 60+ specialized agents, 259 MCP tools, a full MCP server implementation (stdio/HTTP/WebSocket), a governance "guidance" control plane with Rust WASM kernel, vector memory with HNSW search, hook-based lifecycle management, issue claiming/handoff, neural pattern learning (SONA), multi-LLM provider routing, and a plugin system with domain-specific modules (code intelligence, test intelligence, financial risk, etc.). The project is structured as a monorepo with v2 (legacy), v3 (current) with `@claude-flow/*` scoped packages, and a `.claude-plugin` for Claude Code integration. The entire codebase is MIT-licensed.

## Extractions

### Architecture Pattern: Guidance Control Plane (Governance System)

- **What**: A complete governance/policy system that compiles CLAUDE.md files into policy bundles containing a "constitution" (always-loaded core rules) and task-scoped "shards" (retrieved by intent/domain). Includes a Rust WASM kernel for deterministic policy enforcement, enforcement gates (destructive ops, secrets scanning, tool allowlist, diff size), a coherence scheduler (drift detection with privilege levels: full/restricted/read-only/suspended), economic governor (budget tracking for tokens/tool-calls/storage/time/cost), run ledger with evaluators, and cryptographic proof chains (HMAC-SHA256 signed, hash-chained envelopes for audit trails).
- **Where in source**: `v3/@claude-flow/guidance/src/` -- compiler.ts, gates.ts, retriever.ts, coherence.ts, ledger.ts, proof.ts, memory-gate.ts, conformance-kit.ts, wasm-kernel.ts; WASM kernel at `v3/@claude-flow/guidance/wasm-kernel/src/` (Rust: lib.rs, gates.rs, proof.rs, scoring.rs)
- **Integration target in OAP**: This maps directly to OAP's spec-compiler and conformance-lint concepts. The compiler parses markdown governance files into structured policy bundles -- exactly what OAP's spec-compiler does. The WASM kernel approach (Rust compiled to WASM with JS fallback) matches OAP's Rust crate pattern. The proof chain / audit trail concept is valuable for OAP's governed delivery model. The coherence scheduler's privilege-level system (degrading agent capabilities on drift) is a novel governance pattern OAP should adopt.
- **Action**: outline-spec
- **Priority**: P0

### Portable Rust Crate: guidance-kernel WASM

- **What**: A Rust crate (guidance-kernel) that compiles to WASM providing: SHA-256 content hashing, HMAC-SHA256 signing, hash chain verification, secret pattern scanning (8 regex patterns for API keys, tokens, private keys, etc.), destructive command detection (12 patterns), and shard scoring/ranking. Zero dependencies on filesystem or network -- pure deterministic functions. 92KB WASM output.
- **Where in source**: `v3/@claude-flow/guidance/wasm-kernel/` -- Cargo.toml, src/lib.rs, src/gates.rs, src/proof.rs, src/scoring.rs
- **Integration target in OAP**: `crates/` -- could be adapted into OAP's spec-compiler or a new `crates/conformance-kernel` crate. The secret scanning and destructive command detection are directly useful for OAP's conformance-lint. The proof chain verification could underpin OAP's audit/governance layer.
- **Action**: integrate-now
- **Priority**: P0

### Architecture Pattern: MCP Server Implementation

- **What**: A full MCP (Model Context Protocol) server with: tool registry (O(1) lookup, category/tag indexing, batch registration, execution metrics), session management, connection pooling, resource registry, prompt registry, task manager, rate limiter, OAuth support, sampling manager, and multi-transport support (stdio, HTTP, WebSocket, in-process). Complete type definitions for MCP 2025-11-25 spec including resources, prompts, sampling, roots, logging, completion, pagination, progress notifications, and cancellation.
- **Where in source**: `v3/@claude-flow/mcp/src/` -- server.ts, tool-registry.ts, types.ts, session-manager.ts, connection-pool.ts, resource-registry.ts, prompt-registry.ts, task-manager.ts, rate-limiter.ts, oauth.ts, sampling.ts, schema-validator.ts, transport/
- **Integration target in OAP**: OAP already has MCP integrations (gitctx-mcp). The comprehensive type definitions (types.ts) are valuable as a reference for ensuring OAP's MCP implementations are spec-complete. The tool registry pattern with category/tag indexing and execution metrics is useful for OAP's registry-consumer.
- **Action**: outline-spec
- **Priority**: P1

### Architecture Pattern: Hook-based Lifecycle System

- **What**: A hooks system with 19 event types (pre/post tool-use, pre/post edit, pre/post command, pre/post task, session start/end, agent spawn/terminate, pattern learned/consolidated, etc.) and priority levels (Critical=1000, High=100, Normal=50, Low=10, Background=1). Includes workers, daemons, statusline rendering, LLM routing, and bridge connections. The `.claude-plugin/hooks/hooks.json` shows practical pre/post tool-use hooks that modify bash commands, track file edits, and inject guidance on compact.
- **Where in source**: `v3/@claude-flow/hooks/src/` -- types.ts, index.ts, workers/, daemons/, bridge/, cli/, executor/, mcp/, statusline/; `.claude-plugin/hooks/hooks.json`
- **Integration target in OAP**: OAP's desktop app could use a similar hook system for pre/post operations. The hooks.json format for Claude Code plugin integration is directly applicable to OAP's Claude Code integration story.
- **Action**: outline-spec
- **Priority**: P1

### Architecture Pattern: Issue Claiming and Handoff (ADR-016)

- **What**: A complete issue claiming system with: claim/release/handoff lifecycle, 7 status types (active, paused, handoff-pending, review-requested, blocked, stealable, completed), work stealing with contest windows, load balancing and swarm rebalancing, 17 MCP tools, full event sourcing. Supports human-to-agent and agent-to-agent handoffs.
- **Where in source**: `v3/@claude-flow/claims/src/` -- domain/, application/, infrastructure/, api/mcp-tools.js
- **Integration target in OAP**: Relevant for OAP's multi-agent collaboration story. The claiming/handoff pattern could be adapted for the desktop app's task management or for governing which agents can work on which spec slices.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Pattern: Coherence Scoring and Privilege Degradation

- **What**: A coherence scheduler that computes an overall coherence score (0-1) from three components: violation rate, rework frequency, and intent drift. Maps scores to privilege levels: full (>0.7), restricted (0.5-0.7), read-only (0.3-0.5), suspended (<0.3). Includes an economic governor tracking budgets (tokens, tool calls, storage, time, cost) with alert thresholds.
- **Where in source**: `v3/@claude-flow/guidance/src/coherence.ts`
- **Integration target in OAP**: Directly applicable to OAP's governed delivery model. When an agent drifts from spec, OAP could progressively restrict capabilities rather than hard-blocking. The economic governor pattern is useful for cost management in OAP's multi-agent scenarios.
- **Action**: outline-spec
- **Priority**: P1

### Architecture Pattern: Memory Write Gating

- **What**: Authority-scoped memory writes with: role hierarchy (queen > coordinator > worker > observer), namespace-based access control, rate limiting, TTL/decay/confidence scoring, lineage tracking (provenance), and contradiction detection between memory entries. Write decisions include authority checks, rate checks, and overwrite permission checks.
- **Where in source**: `v3/@claude-flow/guidance/src/memory-gate.ts`
- **Integration target in OAP**: Useful pattern for OAP's registry operations where different agents/users should have different write authorities. The contradiction detection pattern is valuable for spec governance (detecting conflicting spec entries).
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Pattern: Policy Compiler (Markdown to Policy Bundle)

- **What**: Parses CLAUDE.md and CLAUDE.local.md files into structured policy bundles using regex-based rule extraction. Detects rule IDs (R001, RULE-001), risk classes (critical/high/medium/low/info), domain tags (@security, @testing), tool class tags ([edit], [bash]), intent tags (#bug-fix, #feature), scope patterns (scope:src/**), and verifier annotations (verify:tests-pass). Splits rules into always-loaded "constitution" and task-scoped "shards" with embeddings for semantic retrieval.
- **Where in source**: `v3/@claude-flow/guidance/src/compiler.ts`
- **Integration target in OAP**: The spec-compiler in OAP performs a similar function (compiling spec markdown into structured output). The specific parsing patterns (rule IDs, risk classes, domain tags, scope patterns, verifier annotations) could inform OAP's spec syntax. The constitution/shard split pattern (always-loaded core + task-specific retrieval) is directly applicable to OAP's spec retrieval strategy.
- **Action**: outline-spec
- **Priority**: P0

### Architecture Pattern: Task Intent Classification + Shard Retrieval

- **What**: Classifies tasks into intents (bug-fix, feature, refactor, security, performance, testing, docs, deployment, architecture, debug, general) using weighted regex patterns. Then retrieves relevant policy shards by semantic similarity + hard filters (risk class, repo scope). Constitution is always included. Contradictions resolved by priority.
- **Where in source**: `v3/@claude-flow/guidance/src/retriever.ts`
- **Integration target in OAP**: Directly applicable to how OAP retrieves relevant spec slices for a given task. The intent classification could drive which spec sections are surfaced to agents.
- **Action**: outline-spec
- **Priority**: P1

### Architecture Pattern: Conformance Kit (Agent Cell Pattern)

- **What**: A canonical acceptance test framework proving the entire governance control plane works end-to-end. Implements a "Memory Clerk" agent cell: reads 20 memory entries, runs 1 model inference, proposes 5 memory writes, injects a coherence drop at write #3, verifies system switches to read-only and blocks remaining writes, emits a signed proof envelope, returns a complete replayable trace. Includes SimulatedRuntime that wires all governance components together.
- **Where in source**: `v3/@claude-flow/guidance/src/conformance-kit.ts`
- **Integration target in OAP**: Directly informs how OAP's conformance-lint should test its governance system. The agent cell pattern (self-contained unit of work with traced execution) is a strong architecture pattern. The conformance test approach (inject fault, verify system response) should be adopted for OAP's spec-compiler testing.
- **Action**: outline-spec
- **Priority**: P1

### Security Module: Input Validation and Safe Execution

- **What**: Zod-based validation schemas for security-critical inputs (safe strings, identifiers, filenames, emails, passwords, UUIDs, URLs, semver). Safe command executor using execFile (not shell exec) with command allowlists, argument validation, blocked patterns (shell metacharacters), timeout controls, and dangerous command blacklist. Path validation preventing traversal attacks. Token/credential generation utilities.
- **Where in source**: `v3/@claude-flow/security/src/` -- input-validator.ts, safe-executor.ts, path-validator.ts, token-generator.ts, credential-generator.ts, CVE-REMEDIATION.ts
- **Integration target in OAP**: The input validation patterns are directly useful for any OAP component accepting user input. The safe executor pattern is relevant for OAP's desktop app's command execution. The Zod validation schemas could be shared across OAP packages.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Pattern: Multi-LLM Provider System

- **What**: Abstraction over multiple LLM providers (Anthropic, OpenAI, Google, Cohere, Ollama, RuVector) with: load balancing (round-robin, latency, cost-based), automatic failover, request caching, cost optimization routing, circuit breaker protection, and health monitoring.
- **Where in source**: `v3/@claude-flow/providers/src/` -- anthropic-provider.ts, openai-provider.ts, google-provider.ts, cohere-provider.ts, ollama-provider.ts, provider-manager.ts
- **Integration target in OAP**: Useful reference if OAP ever needs multi-provider LLM routing. The 3-tier model routing concept (WASM for simple transforms at <1ms/$0, Haiku for simple tasks at ~500ms, Sonnet/Opus for complex reasoning) from CLAUDE.md is an interesting cost optimization pattern.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Pattern: Swarm Topologies

- **What**: Multiple swarm topology types (mesh, hierarchical, centralized, hybrid) with: topology state management, node roles (queen/worker/coordinator/peer), edge weights with latency tracking, partitioning strategies, federation hub for cross-swarm coordination, consensus protocols (Raft, Byzantine, Gossip), and attention-based coordination.
- **Where in source**: `v3/@claude-flow/swarm/src/` -- types.ts, topology-manager.ts, federation-hub.ts, consensus/, unified-coordinator.ts, queen-coordinator.ts
- **Integration target in OAP**: The topology concepts are interesting for OAP's multi-agent orchestration. The hierarchical topology with anti-drift (from CLAUDE.md) is the most practically useful pattern.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Pattern: Embeddings Service

- **What**: Multi-provider embedding service (OpenAI, Transformers.js/ONNX, mock) with: persistent SQLite cache, document chunking with overlap, multiple normalization methods (L2/L1/minmax/zscore), hyperbolic embeddings (Poincare ball), and a pure-TS hash-based fallback (RVF). Also includes HNSW index for vector similarity search.
- **Where in source**: `v3/@claude-flow/embeddings/src/`, `v3/@claude-flow/memory/src/hnsw-index.ts`, `v3/@claude-flow/memory/src/hnsw-lite.ts`
- **Integration target in OAP**: Could be useful if OAP needs semantic search over specs or code. The chunking and normalization utilities are generally reusable.
- **Action**: capture-as-idea
- **Priority**: P2

### Plugin Architecture: Domain-Specific Plugins

- **What**: 16 domain-specific plugins including: code-intelligence (GNN-based architecture analysis, semantic code search, refactoring impact prediction), test-intelligence (predictive test selection, flaky test detection, coverage gaps), prime-radiant (mathematical AI interpretability with sheaf cohomology, spectral analysis, causal inference), perf-optimizer, financial-risk, healthcare-clinical, legal-contracts, quantum-optimizer, hyperbolic-reasoning, and neural-coordination.
- **Where in source**: `v3/plugins/` -- code-intelligence/, test-intelligence/, prime-radiant/, perf-optimizer/, financial-risk/, etc.
- **Integration target in OAP**: The code-intelligence and test-intelligence plugins are the most relevant. Architecture drift detection, refactoring impact prediction, and predictive test selection could enhance OAP's quality engineering capabilities.
- **Action**: capture-as-idea
- **Priority**: P2

### Agent Definitions: 60+ Agent Templates

- **What**: Markdown-based agent definitions organized by category (core: coder/tester/reviewer/researcher/planner; specialized: security-architect, performance-engineer, memory-specialist; swarm: hierarchical-coordinator, mesh-coordinator; consensus: byzantine-coordinator, raft-manager). Each agent definition includes role description, responsibilities, guidelines, and code patterns.
- **Where in source**: `.claude/agents/` (120+ entries including skills), `.agents/skills/` (90+ agent definitions like agent-coder, agent-tester, agent-queen-coordinator, etc.)
- **Integration target in OAP**: The agent role taxonomy and template pattern is useful for OAP's agent orchestration. The core 5 roles (coder, tester, reviewer, researcher, planner) map well to OAP's delivery pipeline.
- **Action**: capture-as-idea
- **Priority**: P2

### Build/CI: Claude Code Plugin Format

- **What**: A `.claude-plugin/` directory with plugin.json (manifest with MCP server configurations, metadata, capabilities), hooks/hooks.json (pre/post tool-use hooks), and documentation. The plugin.json format declares MCP servers that the plugin provides and hooks into Claude Code's tool lifecycle.
- **Where in source**: `.claude-plugin/plugin.json`, `.claude-plugin/hooks/hooks.json`
- **Integration target in OAP**: Directly relevant to how OAP packages itself as a Claude Code plugin. The hooks.json format for injecting pre/post tool-use behavior is the exact mechanism OAP needs for spec enforcement in Claude Code.
- **Action**: integrate-now
- **Priority**: P1

### Architecture Pattern: Proof Chain / Audit Trail

- **What**: A cryptographically signed, hash-chained proof system where each "run event" (agent action) gets a ProofEnvelope containing: content hash, previous-envelope linkage (like a blockchain), tool call hashes, guidance hash, memory lineage, and HMAC-SHA256 signature. Supports verification of the entire chain. Both TypeScript and Rust WASM implementations.
- **Where in source**: `v3/@claude-flow/guidance/src/proof.ts`, `v3/@claude-flow/guidance/wasm-kernel/src/proof.rs`
- **Integration target in OAP**: Directly applicable to OAP's governance audit trail. Every spec-governed action could produce a proof envelope, creating a tamper-evident record of what happened, what rules applied, and what memory was accessed.
- **Action**: outline-spec
- **Priority**: P1

### Architecture Pattern: DDD Bounded Contexts

- **What**: Domain-Driven Design structure with bounded contexts: agent-lifecycle (domain entities), coordination (application services), task-execution (domain + application), memory (domain + infrastructure), infrastructure (MCP + plugins). Each context has clear domain/application/infrastructure layers.
- **Where in source**: `v3/src/` -- agent-lifecycle/, coordination/, task-execution/, memory/, infrastructure/, shared/
- **Integration target in OAP**: OAP already follows a similar pattern. The specific bounded context boundaries (agent-lifecycle, coordination, task-execution, memory) are a useful reference.
- **Action**: capture-as-idea
- **Priority**: P2

### Idea: 3-Tier Model Routing

- **What**: From CLAUDE.md -- a cost optimization strategy: Tier 1 (WASM agent booster, <1ms, $0) handles simple transforms like var-to-const, add-types; Tier 2 (Haiku, ~500ms, $0.0002) handles simple tasks with <30% complexity; Tier 3 (Sonnet/Opus, 2-5s, $0.003-0.015) handles complex reasoning >30% complexity. The system auto-detects which tier to use.
- **Where in source**: `CLAUDE.md` (ADR-026 section)
- **Integration target in OAP**: Interesting cost optimization for OAP's multi-agent delivery -- route simple spec conformance checks to cheap/fast tiers, reserve expensive models for complex reasoning.
- **Action**: capture-as-idea
- **Priority**: P2

### Idea: Dual-Mode Collaboration (Claude + Codex)

- **What**: A protocol for running Claude Code and OpenAI Codex workers in parallel with shared memory coordination. Each platform has complementary strengths. Workers share state via a common memory namespace and collaborate on tasks.
- **Where in source**: `CLAUDE.md` (Dual-Mode Collaboration section), `v3/@claude-flow/codex/`
- **Integration target in OAP**: The multi-model collaboration concept is relevant if OAP supports heterogeneous agent backends. The shared memory namespace pattern for cross-agent coordination is generally applicable.
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

- **v2/ directory**: Legacy version with a different architecture (pre-DDD). All valuable patterns have been evolved into v3. Includes 300+ files of adapters, hooks, hive-mind, consciousness-symphony, etc. that are superseded by v3 equivalents.
- **ruflo/ directory**: Brand assets (images), legacy bin wrappers, and docs that duplicate v3 content.
- **agents/ (root)**: Empty or minimal -- real agent definitions are in `.claude/agents/` and `.agents/skills/`.
- **tests/docker-regression/**: Docker-specific regression tests not applicable to OAP.
- **v3/plugins/gastown-bridge/**: Domain-specific WASM modules for formula parsing (chemistry/cooking domain) -- not relevant to OAP.
- **v3/plugins/healthcare-clinical/, financial-risk/, legal-contracts/**: Domain-specific plugins not applicable to a developer platform.
- **v3/plugins/quantum-optimizer/, hyperbolic-reasoning/**: Experimental/research-oriented plugins with no practical applicability to OAP.
- **v3/@claude-flow/aidefence/**: AI defense module -- OAP handles this differently.
- **v3/@claude-flow/browser/**: Browser automation with Docker -- out of scope for OAP.
- **v3/@claude-flow/deployment/**: Deployment orchestration -- OAP has its own CI/CD story.
- **v3/implementation/**: Planning documents, migration notes, research -- process artifacts not code.
- **v3/docs/**: Documentation of ruflo internals -- process artifacts.
- **v3/.agentic-flow/**: Third-party integration configuration.
- **scripts/**: Install/cleanup scripts specific to ruflo's deployment.
- **v3/@claude-flow/neural/**: SONA learning modes, RL algorithms, trajectory learning -- interesting but too specialized and coupled to ruflo's learning infrastructure.
- **v3/@claude-flow/performance/**: Performance benchmarking specific to ruflo.
- **v3/@claude-flow/integration/**: Integration glue code specific to ruflo's package wiring.
- **CHANGELOG.md, SECURITY.md, LICENSE**: Standard project files, MIT license noted.

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
