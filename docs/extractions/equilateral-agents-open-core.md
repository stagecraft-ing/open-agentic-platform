---
source: equilateral-agents-open-core
source_path: ~/Dev2/stagecraft-ing/equilateral-agents-open-core
status: extracted
date: 2026-03-29
---

## Summary

Equilateral Agents Open Core is a Node.js-based multi-agent orchestration framework (v3.1.0, MIT) providing 22 specialized AI agents organized into development, quality, security, and infrastructure packs. The framework centers on a "self-learning" loop: agents execute workflows, record outcomes to file-based memory, patterns are harvested into YAML standards, and those standards are injected into future sessions. Key concepts include a three-tier standards hierarchy (.standards/, .standards-community/, .standards-local/), a "Pain to Pattern" methodology for turning incidents into enforceable rules, the "GlideCoding" methodology for AI-assisted development with architectural governance, a project-object session memory skill, and protocol layers for MCP/A2A/WebSocket agent communication. The project is heavily documentation-oriented and commercially motivated (upsell to MindMeld/commercial tiers).

## Extractions

### [Spec/Governance]: YAML Standards Schema and Three-Tier Hierarchy

- **What**: A well-defined YAML schema for machine-readable coding standards with fields: `id`, `category`, `priority` (10/20/30), `rules[]` (action: ALWAYS|NEVER|USE|PREFER|AVOID + rule text), `anti_patterns[]`, `examples{}`, `context`, `tags[]`, `updated`. Standards load from three directories in override order: official -> community -> local. The StandardsLoader class handles caching, tag-based filtering, category filtering, action filtering, and agent-type-to-tag mapping.
- **Where in source**: `equilateral-core/StandardsLoader.js`, `.standards-local-template/**/*.yaml` (6 exemplary standards: error-first-design, database-query-patterns, auth-and-access-control, credential-scanning, input-validation-security, integration-tests-no-mocks)
- **Integration target in OAP**: `specs/` governance system. The YAML schema maps naturally to OAP's spec spine concept. Standards could become a new spec kind (e.g., "coding-standard") that the spec-compiler validates and conformance-lint enforces. The three-tier override pattern (official > community > project-local) is a governance model OAP could adopt for spec distribution.
- **Action**: outline-spec
- **Priority**: P1

### [Spec/Governance]: Pain-to-Pattern Methodology

- **What**: A structured process for converting production incidents into enforceable standards: (1) capture incident with timeline, (2) calculate cost (time/money/trust), (3) extract anti-pattern and correct pattern, (4) create standard with "What Happened, The Cost, The Rule" format, (5) enforce via CLAUDE.md/agents/CI, (6) measure prevented incidents and ROI. Includes incident templates, pattern templates, and graduation criteria (3+ months incident-free -> promote to community).
- **Where in source**: `docs/guides/PAIN_TO_PATTERN.md`, `docs/guides/KNOWLEDGE_HARVEST.md`
- **Integration target in OAP**: `docs/methodology/` or as a new spec describing OAP's own standards-from-incidents workflow. The "incident -> standard -> enforcement -> measurement" flywheel is directly applicable to how OAP specs should evolve.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Agent Orchestrator with Sequential Workflow Execution

- **What**: EventEmitter-based orchestrator that registers agents, executes named workflows as sequential agent task chains, persists workflow history to `.equilateral/workflow-history.json`, and supports background (non-blocking) workflow execution via Promise-based handles with status/result/cancel methods. Clean separation: orchestrator coordinates, agents execute.
- **Where in source**: `equilateral-core/AgentOrchestrator.js`
- **Integration target in OAP**: `crates/agent/` or `crates/run/`. The pattern of named workflows with sequential agent execution, workflow history persistence, and background execution handles maps to OAP's agent-governed-execution model (spec 035). The Rust agent crate could implement a similar orchestrator trait.
- **Action**: capture-as-idea
- **Priority**: P1

### [Architecture Patterns]: Background Worker Orchestration with Worker Threads

- **What**: Extended orchestrator using Node.js Worker Threads for true background execution with: real-time progress events, step-complete events, workflow-complete events, auto-generated worker scripts, persisted results to disk, configurable max workers, and SSH tunnel support for private database access.
- **Where in source**: `equilateral-core/BackgroundAgentOrchestrator.js`
- **Integration target in OAP**: `crates/run/` for background task execution. The progress-event pattern (progress, step_complete, workflow_complete, error) is a clean protocol that could inform OAP's Tauri command wiring (spec 038) for long-running operations.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Agent Communication Bus with Priority Queues

- **What**: Full pub/sub message bus for inter-agent communication with: 5-level priority queues (CRITICAL through BACKGROUND), delivery guarantees (fire-and-forget, at-least-once, exactly-once), capability-based routing, load balancing strategies (round-robin, random, least-loaded, all), agent health monitoring via heartbeats, message TTL, exponential backoff retry, and message audit trail.
- **Where in source**: `equilateral-core/protocols/AgentCommunicationBus.js`
- **Integration target in OAP**: `crates/agent/` for multi-agent coordination. The priority queue + capability routing + delivery guarantees pattern is a well-designed message bus architecture. Could inform how OAP agents communicate, especially for the agent-governed-execution spec (035).
- **Action**: capture-as-idea
- **Priority**: P2

### [MCP/Tool Integrations]: Minimal MCP Server Implementation

- **What**: Lightweight MCP server implementing JSON-RPC 2.0 with STDIO and HTTP transports. Supports `initialize` (capability negotiation), `tools/list`, `tools/call`, and `ping` methods. Clean tool registration API with name, description, inputSchema, and handler function. Approximately 300 lines of focused code.
- **Where in source**: `equilateral-core/protocols/MinimalMCPServer.js`
- **Integration target in OAP**: Reference for `crates/asterisk/` or any new MCP server tooling. OAP already has MCP infrastructure but this is a clean, minimal reference implementation showing the bare essentials of an MCP server. The tool registration pattern (`registerTool(name, {description, inputSchema, handler})`) is particularly clean.
- **Action**: capture-as-idea
- **Priority**: P2

### [MCP/Tool Integrations]: Protocol Compatibility Layer (MCP + A2A + WebSocket)

- **What**: A protocol translation layer that wraps MCP, Google's A2A (Agent-to-Agent), and WebSocket protocols into a unified interface. Converts between internal message format and each protocol's wire format. Includes agent card generation (A2A), protocol-specific transport management, and a compatibility matrix.
- **Where in source**: `equilateral-core/protocols/ProtocolCompatibilityLayer.js`, `equilateral-core/protocols/README.md`
- **Integration target in OAP**: `crates/asterisk/` or a new protocol-bridge crate. The A2A protocol support (Google's agent-to-agent spec) is forward-looking. The unified `send({protocol, to, message})` API pattern is clean, though the implementation is largely stub/skeleton code.
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: Project-Object Session Memory Skill

- **What**: A skill definition for persistent session memory across AI coding sessions. Stores context at `~/.project-object/{project-name}/context.md` with four sections: Decisions, Patterns, Corrections, Notes. Defines harvesting rules (what to capture from conversation: decisions by "let's go with" signals, corrections by "actually/no" signals, etc.), merge protocol (dedup, contradiction resolution), staleness detection (30+ days), and cross-platform sync targets (Cursor, Codex, Windsurf, Continue). Also includes standards injection protocol: load YAML standards, map actions to [REQUIRE]/[AVOID]/[PREFER] directives, cap at 30 rules.
- **Where in source**: `.agents/skills/project-object/SKILL.md`, `.agents/skills/project-object/scripts/`
- **Integration target in OAP**: `packages/ui/` (Tauri desktop app) as a session memory feature, or `crates/gitctx/` as persistent project context. The harvesting rules (signal detection for decisions, patterns, corrections) are directly applicable to OAP's agent system. The cross-platform sync table is a useful reference for multi-tool support. The standards injection protocol (YAML -> [REQUIRE]/[AVOID]/[PREFER]) could enhance OAP's conformance-lint.
- **Action**: outline-spec
- **Priority**: P1

### [Agent/Skill Definitions]: Claude Code Slash Commands

- **What**: 10 Claude Code slash commands defined as markdown files: ea-security-review, ea-code-quality, ea-deploy-feature, ea-test-workflow, ea-infrastructure-check, ea-memory, ea-list, ea-hipaa-compliance, ea-gdpr-check, ea-full-stack-dev. Each defines a multi-step workflow with agent registration, orchestrator startup, workflow execution, and result reporting patterns.
- **Where in source**: `.claude/commands/*.md`
- **Integration target in OAP**: `docs/` as reference for OAP's own Claude Code command design patterns. OAP already has developer and product-manager command sets. These show a different pattern: workflow-oriented commands that orchestrate multiple agents.
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: Agent Classifier (Task Routing)

- **What**: Keyword-based task classification system that scores agents against task descriptions using four weighted factors: capability matching (40%), knowledge base relevance (30%), agent status/availability (20%), and task history success rate (10%). Returns recommended agent, alternatives, and confidence score.
- **Where in source**: `equilateral-core/infrastructure/AgentClassifier.js`
- **Integration target in OAP**: `crates/agent/` for intelligent task routing. The weighted scoring model (capabilities + knowledge + status + history) is a reasonable baseline for agent selection that could inform OAP's axiomregent (spec 033) or agent-governed-execution (spec 035).
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: File-Based Agent Memory with Pattern Recognition

- **What**: Per-agent execution memory stored as JSON files in `.agent-memory/{agentId}/memory.json`. Tracks last 100 executions with task type, duration, success/failure. Computes success rate, average duration, common patterns, failure patterns, improvement trends (first 10 vs last 10 executions), and suggests optimal workflows with confidence scores. Atomic writes via temp-file-then-rename. Export/import for migration.
- **Where in source**: `equilateral-core/SimpleAgentMemory.js`
- **Integration target in OAP**: `crates/agent/` or `crates/stackwalk/`. The pattern of tracking execution outcomes, computing success rates, and detecting improvement trends over time is valuable for OAP's agent system. The atomic file write pattern (write to .tmp, rename) is a good practice already used in OAP.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Standards Contributor (Pattern-to-Standard Pipeline)

- **What**: After workflow execution, analyzes findings to detect pattern-worthy results (enough history + interesting findings). Generates YAML standards automatically from findings with: category classification, priority calculation from severity, rule generation from known pattern types, anti-pattern extraction, example code generation, tag assignment. Supports YAML-aware merging of new rules into existing standard files (dedup rules, merge tags, update priority).
- **Where in source**: `equilateral-core/StandardsContributor.js`
- **Integration target in OAP**: `crates/agent/` or `tools/spec-compiler/`. The concept of automatically generating governance artifacts (specs/standards) from execution findings is powerful for OAP. The YAML merge logic (dedup rules by text, merge tags, take higher priority) is directly useful.
- **Action**: capture-as-idea
- **Priority**: P1

### [Architecture Patterns]: BaseAgent Class Design

- **What**: EventEmitter-based base class with: automatic orchestrator wiring, opt-in memory system, opt-in standards loading (with tag-based filtering per agent type), AI enhancement via pluggable LLM provider, task validation, completion/error reporting through events, and a clean `executeTaskWithMemory()` wrapper that records success/failure automatically.
- **Where in source**: `equilateral-core/BaseAgent.js`
- **Integration target in OAP**: `crates/agent/` as reference for OAP's agent trait design. The pattern of opt-in capabilities (memory, standards, AI) configured via constructor options is clean. The event-based reporting (taskComplete, taskError through orchestrator) is a well-proven pattern.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: BYOL (Bring Your Own LLM) Provider Pattern

- **What**: LLM provider abstraction supporting OpenAI, Anthropic, Ollama, Azure, and AWS Bedrock through a unified `complete(prompt, options)` API. Lazy initialization to avoid requiring unused SDKs. Shared singleton instance. Environment variable auto-detection for API keys. Agent-context formatting helpers with task-type-specific system prompts.
- **Where in source**: `equilateral-core/LLMProvider.js`, `equilateral-core/providers/BedrockProvider.js`
- **Integration target in OAP**: Reference only. OAP's architecture delegates LLM interaction to Claude Code itself rather than embedding LLM providers. However, the BYOL pattern could inform future MCP tool server designs that need model access.
- **Action**: capture-as-idea
- **Priority**: P2

### [Spec/Governance]: CLAUDE.md as Governance Document Template

- **What**: A comprehensive template for `.claude/CLAUDE.md` that serves as the AI assistant's governance handbook. Includes: mandatory pre-change workflow (check standards -> design errors first -> implement -> validate), critical alerts system with severity levels, banned patterns section, trigger words for extra caution (security, performance, infrastructure, compliance terms), background execution patterns, knowledge harvest instructions, community contribution graduation path, and standards directory structure template.
- **Where in source**: `.claude/CLAUDE.md`
- **Integration target in OAP**: OAP's own `.claude/CLAUDE.md` or as a reference doc. The "trigger words" concept (when you see these terms, check standards) and the "critical alerts" format (What Happened / The Cost / The Rule) are patterns OAP could adopt in its own governance model.
- **Action**: capture-as-idea
- **Priority**: P2

### [Spec/Governance]: Example YAML Standards (6 Templates)

- **What**: Six production-quality YAML standard templates covering: error-first-design (architecture), database-query-patterns (performance), auth-and-access-control (security), credential-scanning (security), input-validation-security (security), integration-tests-no-mocks (testing). Each includes rules with action verbs, anti-patterns, before/after code examples, and a `context` field explaining the real incident that motivated the standard with quantified costs.
- **Where in source**: `.standards-local-template/**/*.yaml`
- **Integration target in OAP**: Could seed an OAP standards library or serve as reference for OAP's conformance-lint rule definitions. The `context` field pattern (real incident + cost) is particularly valuable for explaining why a rule exists.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Knowledge Harvest Automation

- **What**: Automated script that scans agent memory files, identifies error patterns (3+ occurrences), finds successful optimizations (85%+ success rate), detects improvement/decline trends, calculates knowledge gaps (agents with <85% success rate), and generates a prioritized YAML report with recommendations for new standards. Includes Levenshtein-style similarity clustering for grouping related errors.
- **Where in source**: `scripts/harvest-knowledge.js`, `docs/guides/KNOWLEDGE_HARVEST.md`
- **Integration target in OAP**: `crates/agent/` or `crates/xray/`. The pattern analysis algorithms (error clustering, trend detection, gap identification) could inform OAP's agent self-improvement loop. The weekly-harvest-to-standards pipeline is a governance pattern OAP could automate.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Agent Orchestration Guardrails Case Study

- **What**: Post-mortem documenting how Claude Code "freelanced" outside agent-orchestrated workflows, bypassing proven patterns and reintroducing bugs that agents had already caught. Introduces FI/FDI (Fully Independent vs Fully Dependent Implementation) framework for deciding when AI should work alone vs within orchestration. Key finding: agent orchestration is not overhead but necessary constraint on AI autonomy.
- **Where in source**: `case-studies/AGENT_ORCHESTRATION_GUARDRAILS.md`
- **Integration target in OAP**: Directly relevant to OAP's agent-governed-execution (spec 035) and safety-tier-governance (spec 036). The FI/FDI framework could inform OAP's safety tiers. The case study provides real evidence for why governed execution matters.
- **Action**: capture-as-idea
- **Priority**: P1

### [Build/CI/Packaging]: GitHub Actions Workflow for Agent-Based Code Review

- **What**: GitHub Actions workflow that runs on PRs: installs equilateral-agents, configures LLM provider from secrets, executes agent workflows (code-review, security-scan, deployment-check, quality-gate), posts results as PR comments with evidence-based messaging, and creates GitHub check runs with pass/fail conclusions.
- **Where in source**: `.github/workflows/equilateral-agents.yml`
- **Integration target in OAP**: `.github/workflows/` as reference for CI-integrated agent execution. OAP already has CI workflows; this shows a pattern for running governance agents as PR checks.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Database Adapter Interface

- **What**: Abstract database adapter class defining a clean interface for workflow/task/agent-state/event persistence. Methods: createWorkflow, updateWorkflowStatus, getWorkflow, createTask, updateTaskStatus, saveAgentState, getAgentState, logEvent, beginTransaction, commitTransaction, rollbackTransaction, healthCheck, cleanupOldRecords. SQLite implementation provided.
- **Where in source**: `equilateral-core/database/DatabaseAdapter.js`, `equilateral-core/database/SQLiteAdapter.js`
- **Integration target in OAP**: `crates/agent/` if OAP needs workflow persistence. The interface design is clean but OAP likely prefers Rust-native approaches (e.g., rusqlite).
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Agent Factory (Code Generation Agent)

- **What**: Meta-agent that generates other agents, Lambda handlers, React components with Storybook stories, TypeScript types from DB schemas, API endpoint definitions, and test suites (unit/integration/E2E with Playwright). Follows template-based code generation with standards compliance.
- **Where in source**: `equilateral-core/infrastructure/AgentFactoryAgent.js`
- **Integration target in OAP**: Conceptual reference only. The idea of a "factory agent" that generates other agents or code artifacts following governance standards is interesting but the implementation is too JS/React-specific for OAP's Rust-native stack.
- **Action**: capture-as-idea
- **Priority**: P2

### [Spec/Governance]: GlideCoding Methodology Reference

- **What**: The README references GlideCoding as the overarching methodology: AI-assisted development with architectural governance. Three domains: glidecoding.com (methodology), glidecoding.org (67 YAML standards, 808+ rules), glidecoding.ai (platform). Equilateral Agents is positioned as the open-core engine behind this methodology.
- **Where in source**: `README.md` (bottom section)
- **Integration target in OAP**: The glidecoding.org standards library (67 YAML, 808+ rules) could be evaluated as a source for OAP's conformance-lint rules. The methodology alignment (AI-assisted development + architectural governance) is exactly OAP's thesis.
- **Action**: capture-as-idea
- **Priority**: P1

## No-value items

| Item | Reason skipped |
|------|---------------|
| `agent-packs/development/CodeGeneratorAgent.js` | Template-based JS code gen, too framework-specific (React/Flowbite/Lambda) |
| `agent-packs/development/TestAgent.js` | Playwright element remapping, very HappyHippo-specific UI testing |
| `agent-packs/development/UIUXSpecialistAgent.js` | HappyHippo/Flux Systems brand-specific design system definitions |
| `agent-packs/infrastructure/DeploymentAgent.js` | AWS SDK v2 deployment stubs, OAP is not AWS-centric |
| `agent-packs/infrastructure/ResourceOptimizationAgent.js` | AWS cost optimization stubs |
| `agent-packs/infrastructure/ConfigurationManagementAgent.js` | AWS multi-account config with hardcoded patterns |
| `agent-packs/infrastructure/MonitoringOrchestrationAgent.js` | SaaS multi-tenancy monitoring, requires missing dependencies |
| `agent-packs/quality/BackendAuditorAgent.js` | Lambda/API-specific audit patterns |
| `agent-packs/quality/FrontendAuditorAgent.js` | React/Flowbite-specific frontend audit patterns |
| `agent-packs/quality/TemplateValidationAgent.js` | IaC template validation, too domain-specific |
| `agent-packs/security/SecurityReviewerAgent.js` | Depends on missing AgentConfiguration/ModelConfiguration modules |
| `agent-packs/security/ComplianceCheckAgent.js` | Stub compliance rules, not production-ready |
| `equilateral-core/providers/BedrockProvider.js` | AWS Bedrock-specific LLM provider, OAP delegates to Claude Code |
| `equilateral-core/database/SQLiteAdapter.js` | Node.js SQLite adapter, OAP uses Rust-native |
| `equilateral-core/infrastructure/AgentMemoryManager.js` | File-based agent memory with tasks/knowledge, overlaps with SimpleAgentMemory |
| `equilateral-core/infrastructure/AgentFactoryAgent.js` | JS/React code generation, too framework-specific |
| `examples/*.js` | Demo scripts, no novel patterns beyond what's in core |
| `demo-*.js`, `test-*.js` | Test/demo scripts |
| `scripts/add-pathscanner-to-constructors.js` | Internal migration script |
| `scripts/update-all-agents.js` | Internal migration script |
| `scripts/verify-pathscanner-rollout.js` | Internal verification script |
| `docs/development/*.md` | Internal development notes (agent rollout, path scanning fixes) |
| `docs/releases/*.md` | Release notes for equilateral-agents versions |
| `docs/community/*.md` | Community submission guides for equilateral ecosystem |
| `docs/BYOL-*.md` | BYOL setup docs, OAP delegates LLM to Claude Code |
| `docs/BACKGROUND_EXECUTION.md` | API docs for background execution, captured in architecture pattern above |
| `docs/SUPPORTED_PATTERNS.md` | Agent pattern catalog, captured in architecture patterns above |
| `docs/AGENT_INVENTORY.md` | Agent listing, captured in skill definitions above |
| `docs/PLUGIN_USAGE.md` | Plugin installation guide |
| `docs/AI-INTEGRATION.md` | AI provider setup guide |
| `case-studies/HONEYDOLIST_CASE_STUDY.md` | Marketing case study for HoneyDoList.vip |
| `.claude/skills/equilateral-agents/SKILL.md` | Claude Code skill definition, captured above |
| `.claude/skills/equilateral-agents/reference.md` | Reference doc for skill |
| `.claude-plugin/marketplace.json` | Plugin marketplace metadata |
| `.github/workflows/release.yml`, `npm-publish.yml`, `test.yml` | Standard CI workflows |
| `.github/ISSUE_TEMPLATE/*`, `pull_request_template.md` | GitHub templates |
| `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md` | Standard OSS governance docs |
| `CHANGELOG.md` | Version history |
| `LICENSE` | MIT license |
| `.npmignore`, `.gitignore`, `.gitmodules*` | Build/packaging config |
| `package.json` | Node.js package config (deps: aws-sdk, axios, commander, discord.js, dotenv, js-yaml, playwright) |
| `equilateral-core/PathScanningHelper.js` | File scanning utility, standard directory walker |
| `workflows/README.md` | Workflow documentation, patterns captured above |

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
- [x] YAML standards schema and three-tier hierarchy documented (P1)
- [x] Pain-to-Pattern methodology documented (P2)
- [x] Agent orchestration patterns documented (P1)
- [x] MCP server reference documented (P2)
- [x] Session memory skill documented (P1)
- [x] Communication bus architecture documented (P2)
- [x] Standards contributor pipeline documented (P1)
- [x] Agent classifier routing documented (P2)
- [x] GlideCoding methodology reference documented (P1)
- [x] FI/FDI guardrails case study documented (P1)
- [x] All 22 agents reviewed, domain-specific ones skipped with reason
- [x] All workflows, scripts, docs, examples reviewed
- [x] No code worth directly porting (all JS, OAP is Rust-native)
