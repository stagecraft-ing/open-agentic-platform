---
source: claude-code-sub-agents
source_path: ~/Dev2/stagecraft-ing/claude-code-sub-agents
status: extracted
---

## Summary

A curated collection of 33 specialized AI sub-agents for Claude Code, organized into 7 categories (development, data-ai, infrastructure, quality-testing, security, specialization, business), plus a meta-agent ("agent-organizer") that analyzes tasks and assembles optimal agent teams. Each agent is a markdown file with YAML frontmatter (name, description, tools, model) and a detailed system prompt defining role, expertise, interaction patterns, and decision frameworks. The project includes a comprehensive CLAUDE.md that doubles as an "Agent Dispatch Protocol" defining when to delegate vs. handle directly, and a follow-up complexity assessment framework.

## Extractions

### [Agent/Skill Definitions]: Agent Organizer (Meta-Orchestrator)
- **What**: A master orchestrator agent that analyzes project requirements, detects technology stacks, assembles optimal agent teams (typically 3 agents for focused tasks, more for complex multi-domain work), defines phased workflows with dependency management, and provides clear delegation justifications. Uses a structured output format with team composition, workflow phases, and success criteria. Runs on `haiku` model for cost efficiency since it only plans, not implements.
- **Where in source**: `agents/agent-organizer.md`
- **Integration target in OAP**: `packages/agents/orchestration/` -- adapt as OAP's spec-aware task decomposition agent
- **Action**: outline-spec
- **Priority**: P0

### [Agent/Skill Definitions]: Development Agents (14 agents)
- **What**: Specialized agents covering:
  - **Frontend**: frontend-developer (React components, responsive layouts), react-pro (hooks, performance), nextjs-pro (SSR/SSG), ui-designer (visual design), ux-designer (interaction design)
  - **Backend**: backend-architect (system design, API design, microservices), full-stack-developer (cross-stack), golang-pro, python-pro, typescript-pro
  - **Platform**: mobile-developer (React Native, Flutter), electron-pro (desktop apps)
  - **Specialized**: dx-optimizer (developer experience tooling), legacy-modernizer (migration strategies)
  Each has detailed system prompts with technology-specific best practices, decision frameworks, and interaction patterns with other agents.
- **Where in source**: `agents/development/*.md`
- **Integration target in OAP**: `packages/agents/development/` -- cherry-pick agents relevant to OAP's Rust/TypeScript/Tauri stack
- **Action**: outline-spec
- **Priority**: P1

### [Agent/Skill Definitions]: Quality & Testing Agents (5 agents)
- **What**: code-reviewer (multi-dimensional review with scoring), test-automator (test strategy, coverage analysis, TDD), qa-expert (comprehensive QA), debugger (systematic debugging), architect-review (architecture evaluation). Each defines interaction patterns with development agents.
- **Where in source**: `agents/quality-testing/*.md`
- **Integration target in OAP**: `packages/agents/quality/` -- adapt as spec conformance review agents
- **Action**: outline-spec
- **Priority**: P1

### [Agent/Skill Definitions]: Infrastructure Agents (5 agents)
- **What**: cloud-architect (AWS/GCP/Azure design), deployment-engineer (CI/CD, IaC), performance-engineer (profiling, optimization), incident-responder (triage, mitigation), devops-incident-responder (infrastructure incidents). Each includes technology-specific runbooks and decision trees.
- **Where in source**: `agents/infrastructure/*.md`
- **Integration target in OAP**: `packages/agents/infrastructure/` -- adapt deployment-engineer for OAP's release workflows
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: Data & AI Agents (8 agents)
- **What**: data-engineer (ETL, pipelines), data-scientist (modeling, analysis), ml-engineer (training, deployment), ai-engineer (LLM integration), prompt-engineer (prompt design), database-optimizer (query optimization), postgres-pro (PostgreSQL specialist), graphql-architect (schema design). Each includes domain-specific best practices and tool recommendations.
- **Where in source**: `agents/data-ai/*.md`
- **Integration target in OAP**: `packages/agents/data-ai/` -- prompt-engineer and ai-engineer could inform OAP's own prompt governance
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: Security Auditor Agent
- **What**: Comprehensive security agent covering OWASP Top 10, dependency scanning, secret detection, authentication/authorization review, network security, and compliance. Includes severity classification framework and remediation prioritization.
- **Where in source**: `agents/security/security-auditor.md`
- **Integration target in OAP**: `packages/agents/security/` -- adapt as OAP's security conformance agent
- **Action**: outline-spec
- **Priority**: P1

### [Agent/Skill Definitions]: Documentation Agents (2 agents)
- **What**: api-documenter (OpenAPI specs, endpoint documentation, SDK docs) and documentation-expert (technical writing, architecture docs, user guides). Both define interaction patterns with development agents.
- **Where in source**: `agents/specialization/*.md`
- **Integration target in OAP**: `packages/agents/docs/` -- adapt for auto-generating OAP spec documentation
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: Product Manager Agent
- **What**: Business-focused agent for requirement analysis, user story creation, feature prioritization, stakeholder communication, and roadmap planning. Bridges technical and business concerns.
- **Where in source**: `agents/business/product-manager.md`
- **Integration target in OAP**: Reference only -- OAP's spec spine already handles feature lifecycle
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Agent Dispatch Protocol
- **What**: A comprehensive dispatch protocol (in CLAUDE.md) defining: triage criteria for when to delegate vs. handle directly, mandatory delegation triggers (code gen, refactoring, debugging, analysis, feature work, testing, docs, strategy), a follow-up complexity assessment framework with decision tree (simple/moderate/complex), and clear NEVER/ALWAYS rules. Includes Mermaid diagrams for workflow and decision trees.
- **Where in source**: `CLAUDE.md`
- **Integration target in OAP**: `packages/agents/dispatch/` -- directly inform OAP's agent dispatch protocol in the governance layer
- **Action**: integrate-now
- **Priority**: P0

### [Spec/Governance]: Full-Stack Development Guidelines
- **What**: Opinionated development guidelines (also in CLAUDE.md) covering: planning/staging with `IMPLEMENTATION_PLAN.md`, test-first implementation flow, "when stuck" protocol (max 3 attempts), architecture standards (composition over inheritance, explicit data flow, TDD), code quality gates (every commit must pass lint/type/test), error handling with correlation IDs, decision framework (testability > readability > consistency > simplicity > reversibility), and definition of done.
- **Where in source**: `CLAUDE.md`
- **Integration target in OAP**: `docs/governance/development-guidelines.md` -- adapt as OAP's development standards, merge with spec-driven governance
- **Action**: outline-spec
- **Priority**: P1

### [Architecture Patterns]: Agent Frontmatter Schema
- **What**: Standard YAML frontmatter format for agent definitions: `name` (kebab-case identifier), `description` (auto-invocation trigger text), `tools` (allowed tool list), `model` (haiku/sonnet/opus). The description field doubles as the trigger condition for automatic agent selection.
- **Where in source**: All `agents/**/*.md` files
- **Integration target in OAP**: `packages/agents/schema/` -- adopt as OAP's agent definition schema
- **Action**: integrate-now
- **Priority**: P0

### [Architecture Patterns]: Agent Category Taxonomy
- **What**: Seven-category taxonomy for organizing agents: development (14), data-ai (8), infrastructure (5), quality-testing (5), security (1), specialization (2), business (1). Each category maps to a software delivery lifecycle phase.
- **Where in source**: `agents/` directory structure, `README.md`
- **Integration target in OAP**: `packages/agents/` -- use as OAP's agent classification system
- **Action**: outline-spec
- **Priority**: P1

### [Ideas Only]: Contributing Guidelines for Agent Development
- **What**: Standards for adding new agents: naming conventions (lowercase, hyphen-separated), standard format requirements, description writing for auto-invocation triggers, specialized behavior definition, integration testing patterns, and quality standards (domain expertise, clear boundaries, integration ready, consistent voice).
- **Where in source**: `CONTRIBUTING.md`
- **Integration target in OAP**: `docs/contributing/agents.md` -- adapt as OAP's agent contribution guide
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

- `LICENSE` -- MIT license, standard
- `.gitignore` -- standard ignores
- `_images/` -- demo screenshots and GIFs, documentation visuals only
- `.DS_Store` -- macOS artifact
- `.git/` -- repository metadata
- `agents/development/electorn-pro.md` -- typo in filename ("electorn" vs "electron"), content duplicates standard Electron patterns already well-known

## Safe-to-delete confirmation
- [x] All valuable content extracted or documented above

## Cross-project unique findings

The following items are unique to each individual project and might be missed in a group analysis:

### Unique to claude-code (Project 1)
- **Hookify rule engine**: The only project with a programmatic Python rule engine for intercepting Claude Code events. The other projects define agents but not runtime hooks with conditional logic.
- **DevContainer network sandboxing**: The `init-firewall.sh` with iptables/ipset firewall is unique -- no other project implements network-level security sandboxing.
- **GitHub issue automation suite**: The TypeScript scripts for duplicate detection, auto-closing, lifecycle management, and Claude-powered triage are unique -- a complete issue ops system.
- **Marketplace manifest schema**: The `.claude-plugin/marketplace.json` bundling format for distributing plugin collections is unique to this project.
- **CHANGELOG intelligence**: 180KB of release notes covering ~90 releases provides deep intelligence on Claude Code SDK capabilities and edge cases that no other source captures.

### Unique to claude-code-by-agents (Project 2)
- **Multi-provider abstraction**: The only project that abstracts over Claude Code SDK, Anthropic API, and OpenAI in a unified provider interface. The other two projects are Claude-only.
- **Full web UI with streaming**: The only project with a complete React web interface for agent interaction, including real-time NDJSON streaming, permission dialogs, and tool use visualization.
- **Native macOS SwiftUI app**: A full SwiftUI implementation exists alongside the web UI -- the only project with a native macOS client.
- **Conversation history reader**: The only project that reads and parses Claude Code's `.claude/projects/` JSONL history files, with timestamp restoration and deduplication.
- **Runtime abstraction (Deno/Node)**: The clean Runtime interface allowing the same Hono app to run on both Deno and Node is unique to this project.
- **OAuth authentication flow**: The only project implementing Claude subscription OAuth (credential file writing, preload script injection, token management).
- **File-based inter-agent communication**: The orchestrator pattern where agents write results to temp files and subsequent agents read them is unique -- simpler than message passing.
- **AWS Lambda deployment support**: SAM templates and Lambda handler for serverless deployment are unique to this project.

### Unique to claude-code-sub-agents (Project 3)
- **Agent Dispatch Protocol**: The CLAUDE.md dispatch protocol with triage criteria, mandatory delegation triggers, and follow-up complexity assessment is unique -- a complete meta-governance layer for when to use agents.
- **33-agent taxonomy**: The comprehensive categorized collection of 33 specialized agents is the largest and most organized of the three projects. The other two have 6-10 agents each.
- **Agent Organizer meta-agent**: The only project with a dedicated orchestrator agent that assembles teams of agents based on project analysis, rather than routing to individual agents.
- **Cross-agent interaction patterns**: Each agent explicitly defines how it interacts with other specific agents (e.g., "collaborate with code-reviewer for quality gates, delegate to test-automator for coverage"). No other project has this level of inter-agent relationship definition.
- **Full-stack development guidelines**: The CLAUDE.md doubles as a comprehensive development methodology document (planning stages, test-first, when-stuck protocol, decision frameworks) that goes beyond agent definitions.
- **Model-aware cost optimization**: The agent-organizer runs on `haiku` (cheapest model) since it only plans, while implementation agents use `sonnet`. This cost-conscious model selection pattern is unique.
