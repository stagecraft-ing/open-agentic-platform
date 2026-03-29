---
source: agents
source_path: ~/Dev2/stagecraft-ing/agents
status: extracted
---

## Summary

A Claude Code plugin marketplace ("claude-code-workflows") containing 72 focused single-purpose plugins, 112 specialized AI agent definitions, 146 agent skills, 16 workflow orchestrators, and 79 development tools. Everything is markdown-based prompt engineering -- agent system prompts, skill definitions with progressive disclosure, and orchestration commands that coordinate multi-agent workflows. The project contains zero executable code besides a single Python YouTube transcript extractor tool. The value for OAP lies in architectural patterns, workflow orchestration ideas, governance/spec parallels in the Conductor plugin, and specific agent/skill definitions that could inform OAP's agent framework and desktop app features.

## Extractions

### [Architecture Pattern]: Plugin Marketplace Structure
- **What**: A `marketplace.json` catalog with 72 plugin entries, each pointing to a source directory containing agents/, commands/, skills/ subdirs. Each plugin is self-contained with its own `plugin.json`. Plugins are composable -- users install only what they need, and orchestrator plugins coordinate agents from focused plugins. Average 3.4 components per plugin.
- **Where in source**: `.claude-plugin/marketplace.json`, `plugins/*/. claude-plugin/plugin.json`
- **Integration target in OAP**: Informs the registry-consumer and spec system architecture. The marketplace.json structure parallels OAP's spec registry concept. Could inform how OAP's plugin/extension system is designed in the desktop app.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Three-Tier Model Strategy
- **What**: Strategic model assignment across agents: Tier 1 (Opus) for critical architecture/security/review (42 agents), Tier 2 (inherit/user-chosen) for complex tasks (42 agents), Tier 3 (Sonnet) for support tasks (51 agents), Tier 4 (Haiku) for fast operational tasks (18 agents). Orchestration chains combine tiers: `Planning (Opus) -> Execution (Sonnet) -> Review (Opus)`.
- **Where in source**: `docs/agents.md`, README, individual agent frontmatter `model:` fields
- **Integration target in OAP**: `crates/agent` -- when OAP's agent framework supports model routing, this tiering strategy is a proven reference. Also relevant for the desktop app's agent configuration UI.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Progressive Disclosure for Skills
- **What**: Three-tier knowledge architecture: (1) Metadata/frontmatter always loaded (~name, description, activation trigger), (2) Instructions loaded when skill activated, (3) Resources/references loaded on demand. Each skill is a SKILL.md with YAML frontmatter + markdown body, optionally with a `references/` subdirectory for deep-dive content. This keeps token usage minimal while providing deep expertise when needed.
- **Where in source**: `docs/agent-skills.md`, `docs/architecture.md`, every `plugins/*/skills/*/SKILL.md`
- **Integration target in OAP**: Directly relevant to how OAP structures knowledge for its agents. Could inform a "skill registry" concept within the spec spine, or the way agent capabilities are loaded in the desktop app's MCP integrations.
- **Action**: outline-spec
- **Priority**: P1

### [Governance/Spec Idea]: Conductor - Context-Driven Development
- **What**: A structured workflow system that treats project context as a first-class managed artifact alongside code. Enforces `Context -> Spec & Plan -> Implement` workflow. Creates a `conductor/` directory with: `product.md` (vision/goals), `tech-stack.md` (technology decisions), `workflow.md` (TDD/commit/review policies), `tracks.md` (work registry), and per-track `spec.md` + `plan.md` with phased task breakdowns. Supports greenfield detection, resumable setup state via `setup_state.json`, and track lifecycle (create/implement/revert/archive).
- **Where in source**: `plugins/conductor/` (README, commands/setup.md, commands/new-track.md, commands/implement.md, commands/revert.md, commands/manage.md, commands/status.md, skills/, templates/)
- **Integration target in OAP**: This is essentially a lightweight spec-driven development system that strongly parallels OAP's spec spine concept. The `product.md`/`tech-stack.md`/`workflow.md` pattern maps to OAP's spec hierarchy. The track system (spec.md + plan.md -> phased implementation) is a simplified version of OAP's feature lifecycle. Key differences: Conductor is imperative/interactive, OAP's spec system is declarative/compiled.
- **Action**: outline-spec
- **Priority**: P1

### [Governance/Spec Idea]: Conductor Track Lifecycle
- **What**: Work units ("tracks") follow a lifecycle: `Pending -> In Progress -> Complete -> Archived`. Each track has: type (feature/bug/chore/refactor), spec.md (requirements with acceptance criteria), plan.md (phased tasks with `[ ]`/`[~]`/`[x]` markers), metadata.json (progress state). Implementation follows TDD with phase checkpoints requiring human approval. Git-aware revert by logical unit (track/phase/task) using `git revert` (never `reset --hard`). Semantic commit format: `[track-id] task: description`.
- **Where in source**: `plugins/conductor/commands/new-track.md`, `plugins/conductor/commands/implement.md`, `plugins/conductor/commands/revert.md`, `plugins/conductor/templates/`
- **Integration target in OAP**: Directly informs the feature lifecycle spec (003) and execution bridge (004). The phase checkpoint pattern (halt-and-confirm before next phase) is a governance pattern OAP could adopt. The revert-by-logical-unit concept is novel and worth capturing.
- **Action**: outline-spec
- **Priority**: P1

### [Agent/Skill Definition]: Agent Teams - Multi-Agent Orchestration
- **What**: A plugin for Claude Code's experimental Agent Teams feature. Spawns parallel agent teams using presets (review, debug, feature, fullstack, research, security, migration). Key concepts: file ownership boundaries (one file per agent), task decomposition with dependency graphs (blockedBy/blocks), team-lead orchestrating team-reviewer/team-debugger/team-implementer agents, structured communication protocols, hypothesis-driven debugging (3 competing hypotheses investigated in parallel).
- **Where in source**: `plugins/agent-teams/` (README, agents/, commands/, skills/)
- **Integration target in OAP**: `crates/agent` and desktop app agent orchestration UI. The file-ownership-based conflict avoidance pattern is directly applicable. The preset team compositions (review=3 reviewers, debug=3 hypothesis investigators, etc.) could become built-in orchestration modes.
- **Action**: outline-spec
- **Priority**: P1

### [Workflow Orchestration]: Full-Stack Feature Orchestrator
- **What**: A 9-step phased orchestration workflow: (1) Interactive requirements gathering, (2) Database design, (3) Backend + frontend architecture, [CHECKPOINT], (4) DB implementation, (5) Backend implementation, (6) Frontend implementation, (7) Testing + security + performance review in parallel, [CHECKPOINT], (8) Deployment config, (9) Documentation. Uses state.json for resumability. Each step writes to `.full-stack-feature/` directory. Critical behavioral rules enforce sequential execution, file-based context passing between steps, mandatory checkpoints, and halt-on-failure.
- **Where in source**: `plugins/full-stack-orchestration/commands/full-stack-feature.md`
- **Integration target in OAP**: The phased orchestration pattern with checkpoints, state persistence, and file-based context passing is a reusable architecture pattern for OAP's agent workflows. The "critical behavioral rules" preamble (execute in order, write output files, stop at checkpoints, halt on failure, use only local agents) is a governance pattern.
- **Action**: capture-as-idea
- **Priority**: P2

### [Workflow Orchestration]: Comprehensive Code Review
- **What**: 5-phase parallel review workflow: Phase 1 (Code Quality + Architecture in parallel), Phase 2 (Security + Performance in parallel), [CHECKPOINT], Phase 3 (Testing + Documentation in parallel), Phase 4 (Best Practices + CI/CD in parallel), Phase 5 (Consolidated report). Output is a prioritized findings report organized by severity (P0-P3). Same critical behavioral rules pattern as full-stack orchestrator.
- **Where in source**: `plugins/comprehensive-review/commands/full-review.md`
- **Integration target in OAP**: Could inform a "review" command in OAP's desktop app or agent framework. The parallel agent dispatch + consolidation pattern is reusable.
- **Action**: capture-as-idea
- **Priority**: P2

### [Workflow Orchestration]: TDD Cycle Orchestrator
- **What**: 12-step TDD workflow with strict red-green-refactor discipline: Phases for test specification, RED (write failing tests + verify failure), GREEN (minimal implementation + verify success), REFACTOR (code + test refactoring), integration testing, and final review. Includes incremental mode (one test at a time) and validation checklists for each phase.
- **Where in source**: `plugins/tdd-workflows/commands/tdd-cycle.md`
- **Integration target in OAP**: The TDD enforcement pattern could inform spec verification workflows. The anti-patterns list and validation checklists are useful governance references.
- **Action**: capture-as-idea
- **Priority**: P2

### [Workflow Orchestration]: C4 Architecture Documentation
- **What**: Bottom-up C4 model documentation generator: (1) Discover all subdirectories, (2) Process each bottom-up with c4-code agent (Haiku), (3) Synthesize into components with c4-component agent (Sonnet), (4) Map to containers with c4-container agent (Sonnet) including OpenAPI specs, (5) Create system context with c4-context agent (Sonnet) including personas and user journeys. Produces Mermaid diagrams at all levels.
- **Where in source**: `plugins/c4-architecture/commands/c4-architecture.md`, `plugins/c4-architecture/agents/c4-code.md` through `c4-context.md`
- **Integration target in OAP**: Could become an OAP tool/command. The bottom-up analysis pattern (cheapest model for leaf nodes, progressively smarter models for synthesis) is an efficient multi-model orchestration pattern. The C4 code agent's paradigm-aware diagram selection (OOP -> classDiagram, FP -> flowchart) is well-thought-out.
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definition]: Agent Improvement Workflow
- **What**: Systematic agent optimization through: (1) Performance analysis with baseline metrics (success rate, corrections, token usage), (2) Failure mode classification (instruction misunderstanding, output format errors, context loss, tool misuse), (3) Prompt engineering improvements (chain-of-thought, few-shot optimization, constitutional AI self-correction), (4) A/B testing with statistical significance, (5) Staged rollout (5% -> 20% -> 50% -> 100%). Includes rollback triggers (success rate drops >10%, cost increases >20%).
- **Where in source**: `plugins/agent-orchestration/commands/improve-agent.md`
- **Integration target in OAP**: Directly relevant to OAP's agent framework quality assurance. The failure mode taxonomy and improvement cycle methodology could inform a spec for agent evaluation.
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definition]: AI Engineer Agent - RAG/LLM Expertise
- **What**: Comprehensive system prompt for an AI/LLM application development expert covering: RAG architectures (hybrid search, HyDE, GraphRAG, self-RAG), vector databases (Pinecone, Qdrant, Weaviate, Chroma, pgvector), embedding models (voyage-3-large, text-embedding-3-large), agent frameworks (LangGraph, LlamaIndex, CrewAI, Claude Agent SDK), prompt engineering patterns, AI safety, and multimodal integration. The associated RAG implementation skill includes production-ready code examples.
- **Where in source**: `plugins/llm-application-dev/agents/ai-engineer.md`, `plugins/llm-application-dev/skills/rag-implementation/SKILL.md`
- **Integration target in OAP**: Reference material for OAP's MCP integrations and any RAG/knowledge-base features in the desktop app. The RAG skill's code examples (LangGraph StateGraph, hybrid search with RRF, HyDE, parent document retrieval) are production-quality.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Orchestrator Behavioral Rules Preamble
- **What**: A reusable preamble pattern used across multiple orchestrator commands that enforces disciplined execution: (1) Execute steps in order, (2) Write output files between steps -- do NOT rely on context window memory, (3) Stop at checkpoints and wait for user approval, (4) Halt on failure -- do not silently continue, (5) Use only local agents, (6) Never enter plan mode autonomously. This pattern appears in full-stack-feature, full-review, tdd-cycle, and others.
- **Where in source**: `plugins/full-stack-orchestration/commands/full-stack-feature.md`, `plugins/comprehensive-review/commands/full-review.md`, `plugins/tdd-workflows/commands/tdd-cycle.md`
- **Integration target in OAP**: This is a governance pattern for reliable agent orchestration. Should be adopted as a standard preamble for any multi-step agent workflow in OAP. The "write output files between steps" rule is particularly important for long-running workflows.
- **Action**: integrate-now
- **Priority**: P0

### [Architecture Pattern]: State Persistence for Resumable Workflows
- **What**: Orchestrator commands use `state.json` files to track workflow progress, enabling resume after interruption. Pattern: create state file at start with `status: "in_progress"`, update `current_step` and `completed_steps` after each step, check for existing state on startup and offer resume. Used consistently across conductor, full-stack-orchestration, comprehensive-review, and tdd-workflows.
- **Where in source**: `plugins/conductor/commands/setup.md`, `plugins/full-stack-orchestration/commands/full-stack-feature.md`
- **Integration target in OAP**: `crates/agent` workflow engine. OAP's agent workflows should support checkpointing and resumption natively.
- **Action**: outline-spec
- **Priority**: P1

### [Spec/Governance Idea]: Conductor Templates
- **What**: Standardized markdown templates for project artifacts: product.md (vision/goals), tech-stack.md (with placeholders), workflow.md (TDD lifecycle with `{{TEST_COMMAND}}` placeholders), track-spec.md, track-plan.md, and 8 language-specific code style guides (TypeScript, Python, Go, Rust, C#, Dart, HTML/CSS, general).
- **Where in source**: `plugins/conductor/templates/`
- **Integration target in OAP**: The template system could inform OAP's spec scaffolding. The workflow.md template with its task lifecycle diagram (Select -> Mark [~] -> RED -> GREEN -> REFACTOR -> Coverage -> Commit -> Mark [x] -> Commit Plan) is a well-designed process artifact.
- **Action**: capture-as-idea
- **Priority**: P2

### [MCP/Tool Integration]: Context Save/Restore
- **What**: Context management commands for saving and restoring project context across sessions. Save captures: project metadata, architectural decisions, dependency graphs, semantic tags. Restore supports modes: full, incremental, diff. Includes token budget management for context rehydration and semantic relevance scoring for prioritizing what to restore.
- **Where in source**: `plugins/context-management/commands/context-save.md`, `plugins/context-management/commands/context-restore.md`
- **Integration target in OAP**: `crates/gitctx` already handles git context. The context save/restore concept could extend gitctx with project-level context persistence, or inform a desktop app feature for session continuity.
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definition]: Threat Modeling Expert
- **What**: STRIDE threat analysis methodology: (1) Define scope and trust boundaries, (2) Create data flow diagrams, (3) Identify assets and entry points, (4) Apply STRIDE to each component, (5) Build attack trees, (6) Score and prioritize, (7) Design mitigations, (8) Document residual risks. Associated skills: stride-analysis-patterns, attack-tree-construction, security-requirement-extraction, threat-mitigation-mapping.
- **Where in source**: `plugins/security-scanning/agents/threat-modeling-expert.md`, `plugins/security-scanning/skills/`
- **Integration target in OAP**: Reference for OAP's security review capabilities. The STRIDE methodology workflow could be a template for security-focused spec verification.
- **Action**: capture-as-idea
- **Priority**: P2

### [Tool]: YouTube Design Extractor
- **What**: 809-line Python script that extracts transcripts + keyframes from YouTube videos and produces structured markdown reference documents. Features: transcript extraction via youtube-transcript-api, keyframe extraction via ffmpeg, OCR (tesseract/easyocr), color palette extraction (colorthief), scene detection. Outputs markdown suitable for agent consumption.
- **Where in source**: `tools/yt-design-extractor.py`, `tools/requirements.txt`
- **Integration target in OAP**: Not directly relevant. Could be a standalone tool reference if OAP ever needs video content extraction.
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definition]: Architect Review Agent
- **What**: Comprehensive system prompt for a master software architect covering: Clean/Hexagonal Architecture, microservices, event-driven architecture, DDD, SOLID principles, cloud-native patterns, security architecture (zero trust, OAuth2), performance/scalability patterns, data architecture (polyglot persistence, CQRS, event sourcing), and quality attributes assessment. 8-step response approach from context analysis through implementation guidance.
- **Where in source**: `plugins/comprehensive-review/agents/architect-review.md`
- **Integration target in OAP**: Reference for architecture review capabilities within OAP's agent framework.
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components/Features]: Design System Skills
- **What**: 9 UI design skills covering: design tokens/component architecture/theming, WCAG 2.1/2.2 accessibility with ARIA, responsive design with container queries, iOS HIG, Material Design 3, React Native cross-platform, web components with Shadow DOM, micro-interactions/animations, and visual design foundations (typography/color/spacing). Each skill has 2-3 reference documents with detailed patterns.
- **Where in source**: `plugins/ui-design/skills/`
- **Integration target in OAP**: Reference material for OAP's desktop app (React/TypeScript/Tailwind). The design-system-patterns and accessibility-compliance skills have directly applicable content.
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI]: GitHub Templates and Community Structure
- **What**: Issue templates (bug_report.yml, feature_request.yml, moderation_report.yml, new_subagent.yml), CONTRIBUTING.md with clear quality standards, CODE_OF_CONDUCT.md, FUNDING.yml. The templates use YAML-based GitHub issue forms.
- **Where in source**: `.github/`
- **Integration target in OAP**: OAP already has CI workflows. The issue templates could be adapted if needed. Low value.
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

- **Individual language agent prompts** (python-pro, typescript-pro, rust-pro, golang-pro, etc.) -- These are generic system prompts listing language capabilities. No novel content beyond what any LLM already knows. ~40 agents in this category.
- **SEO/marketing/sales/HR/legal agent prompts** -- Domain-irrelevant to OAP. ~15 agents.
- **Game development plugins** (Unity, Minecraft, Godot) -- Irrelevant domain.
- **Blockchain/Web3 plugins** -- Irrelevant domain.
- **Quantitative trading plugins** -- Irrelevant domain.
- **Payment processing plugins** -- Irrelevant domain (Stripe/PayPal integration details).
- **Shell scripting, Julia, embedded systems plugins** -- Generic language knowledge, no OAP relevance.
- **Most "reference" subdirectories in skills** -- These contain well-known patterns documented in standard references (e.g., REST best practices, GraphQL schema design, breakpoint strategies). No novel content.
- **The Makefile** -- Only serves the YouTube extractor tool.
- **LICENSE (MIT)** -- Standard.
- **docs/plugins.md, docs/agents.md** -- Catalog/reference listings, no novel content beyond what's in the plugins themselves.

## Safe-to-delete confirmation
- [x] All valuable content extracted or documented above
- The only executable code is `tools/yt-design-extractor.py` (809-line Python script) which is documented above
- All 72 plugins have been reviewed; the ~20 most valuable ones are individually called out
- The remaining ~52 plugins contain domain-specific agent prompts and skills that are generic knowledge repackaged as markdown prompts -- no novel code or architecture
- The key architectural patterns (progressive disclosure, orchestrator behavioral rules, state persistence, Conductor workflow, agent teams, C4 bottom-up analysis) are all documented above
