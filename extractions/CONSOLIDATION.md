# Extraction Consolidation — Master Integration Priority List

## Statistics
- Total projects analyzed: 17
- Total extraction items (raw): 189
- After deduplication: 62
- P0 items: 14
- P1 items: 28
- P2 items: 20

## P0 — Integrate Now / Outline Spec Immediately

### 1. Parallel Sub-Agent Orchestration for Code Review
- **What**: Spawn N specialized sub-agents (security, performance, architecture, testing, cross-platform, documentation) in parallel via the Task tool, each analyzing the same diff or codebase from a different angle, then consolidate findings into a unified severity-tiered report. Includes adaptive agent selection (skip irrelevant agents based on change type), cross-pattern consolidation with alternative-hypothesis thinking, and structured scoring.
- **Sources**: developer-claude-code-commands (code-review.md), skills (branch-reviewer), agents (comprehensive-review), claude-code (PR review toolkit), claude-code-sub-agents (quality & testing agents)
- **Integration target in OAP**: `.claude/commands/code-review.md` and `packages/agents/review/`
- **Action**: integrate-now
- **Dedup notes**: Merged code-review.md (6 parallel agents), branch-reviewer (7 parallel agents), comprehensive-review (5-phase parallel), PR review toolkit (6 specialized agents), and quality-testing agents (5 agents). All implement the same core pattern: parallel specialized review with consolidation. The developer-cc-commands and skills versions are the most mature.

### 2. Multi-Provider Agent Registry and Abstraction
- **What**: A unified provider abstraction layer that normalizes multiple LLM backends (Claude Code SDK, Anthropic API, OpenAI, Gemini, Codex, Bedrock) through a common interface. Each provider implements the same spawn/query/abort/stream contract. Output is normalized to a single event format so the UI only handles one protocol.
- **Sources**: claude-code-by-agents (ProviderRegistry), claudecodeui (multi-provider agent abstraction), claudepal (multi-provider normalization shim), crystal (CLI tool registry), ruflo (multi-LLM provider system), equilateral-agents (BYOL provider)
- **Integration target in OAP**: `apps/desktop/src/lib/providers/` or `packages/agents/providers/`
- **Action**: outline-spec
- **Dedup notes**: Six projects independently built multi-provider abstractions. claude-code-by-agents has the cleanest TypeScript interface; claudepal has the best normalization-to-single-format approach; crystal has the best capability declaration pattern. Consolidate the best of each.

### 3. Multi-Agent Orchestration with File-Based Artifact Passing
- **What**: Orchestrator decomposes tasks into steps, assigns each to a specialized agent, agents write results to filesystem artifacts (e.g., `/tmp/research_*.md`), subsequent agents read those files as input. Dependencies between steps are tracked. Achieves ~90% token reduction vs. passing full content through conversation context. Trigger phrases control agent effort depth ("Quick check:", "Investigate:", "Deep dive:").
- **Sources**: claude-code-by-agents (file-based inter-agent communication), skills (deep-researcher, filesystem artifact passing), product-manager-cc-commands (research.md parallel subagent), agents (full-stack-feature orchestrator), claudepal (orchestrator with handoff protocol)
- **Integration target in OAP**: `packages/agents/orchestration/` and spec for multi-agent workflow patterns
- **Action**: outline-spec
- **Dedup notes**: Merged the research.md parallel subagent pattern, deep-researcher skill, claude-code-by-agents file-based communication, full-stack-feature orchestrator, and claudepal handoff protocol. All share the core insight: write intermediate results to files to avoid token bloat.

### 4. Agent Organizer / Meta-Orchestrator (Task Decomposition)
- **What**: A master orchestrator agent that analyzes project requirements, detects technology stacks, assembles optimal agent teams (typically 3 agents for focused tasks, more for complex multi-domain work), defines phased workflows with dependency management, and provides clear delegation justifications. Runs on cheapest model (Haiku) since it only plans. Includes agent dispatch protocol with triage criteria for when to delegate vs. handle directly.
- **Sources**: claude-code-sub-agents (agent-organizer + agent dispatch protocol), claudepal (complexity scoring for auto-orchestration), agents (agent teams)
- **Integration target in OAP**: `packages/agents/orchestration/` and `.claude/agents/`
- **Action**: outline-spec
- **Dedup notes**: Merged agent-organizer meta-agent, complexity scoring heuristic (prompt length, verb count, multi-step connectors), and agent teams preset compositions. The dispatch protocol (when to delegate vs. handle directly) is unique to claude-code-sub-agents.

### 5. Claude Code SDK Integration Pattern
- **What**: Working integration with `@anthropic-ai/claude-code` SDK's `query()` function, including session resumption, permission bypass mode, working directory setting, custom executable paths, AbortController support, OAuth token management, and structured message protocol (SDKSystemMessage, SDKUserMessage, SDKAssistantMessage, SDKResultMessage with cost/usage tracking).
- **Sources**: claude-code-by-agents (executeClaudeCommand), claude-code-single-binary (SDK message protocol + TypeScript interface), claudepal (CLI stream-JSON parser + spawner)
- **Integration target in OAP**: `apps/desktop/src/lib/claude-code-bridge/`
- **Action**: integrate-now
- **Dedup notes**: Merged SDK integration pattern, message protocol types, CLI stream parser, and multi-platform CLI spawner. claudepal's parser and spawner are the most battle-tested; claude-code-single-binary has the cleanest type definitions.

### 6. Orchestrator Behavioral Rules Preamble
- **What**: A reusable governance preamble for any multi-step agent workflow: (1) Execute steps in order, (2) Write output files between steps -- do NOT rely on context window memory, (3) Stop at checkpoints and wait for user approval, (4) Halt on failure -- do not silently continue, (5) Use only local agents, (6) Never enter plan mode autonomously. Consistently used across multiple orchestrator commands.
- **Sources**: agents (full-stack-feature, full-review, tdd-cycle behavioral rules)
- **Integration target in OAP**: `CLAUDE.md` or `.claude/rules/` as standard preamble for all multi-step workflows
- **Action**: integrate-now
- **Dedup notes**: Single source (agents project) but used consistently across 4+ orchestrator commands there. Validated pattern.

### 7. Validate-and-Fix Pipeline
- **What**: A systematic quality-gate pipeline: (1) discover available validation commands from project config, (2) run all checks in parallel, (3) categorize findings by severity, (4) execute fixes in phased order (safe quick wins first, then functionality, then critical with user confirmation), (5) verify after each phase. Includes git-stash rollback capability and dependency mapping between issues.
- **Sources**: developer-claude-code-commands (validate-and-fix.md), claudepal (/check command)
- **Integration target in OAP**: `.claude/commands/validate-and-fix.md`
- **Action**: integrate-now
- **Dedup notes**: Merged validate-and-fix pipeline with /check command. Both implement discover-run-categorize-fix-verify loops; validate-and-fix is more comprehensive with rollback support.

### 8. Featuregraph Scanner + Feature Registry (features.yaml)
- **What**: A YAML-based master feature registry where each entry has: id, title, governance status, implementation status, spec path, dependencies, and file list. Source files carry `// Feature: FEATURE_ID` attribution headers. A Rust scanner walks the filesystem, parses headers, builds a feature-to-file graph, and detects violations (dangling IDs, missing spec files, spec path mismatches, duplicate IDs). Includes preflight checker with safety tiers (Tier1=autonomous, Tier2=gated, Tier3=forbidden).
- **Sources**: claudepal (features.yaml + featuregraph scanner + preflight checker)
- **Integration target in OAP**: `crates/spec-compiler/` or new `crates/featuregraph/`
- **Action**: integrate-now
- **Dedup notes**: Unique to claudepal. This IS the conformance-lint concept already implemented in Rust.

### 9. ast-grep Architecture Enforcement Rules
- **What**: Three ast-grep YAML rules enforcing: (1) no Zustand store destructuring (prevents render cascades), (2) no React hooks in `lib/` directory (enforces clean separation), (3) no store subscriptions in `lib/` (must use `getState()` for non-reactive access). Lint-time enforcement of architecture boundaries.
- **Sources**: claudepal (ast-grep rules)
- **Integration target in OAP**: `apps/desktop/.ast-grep/rules/`
- **Action**: integrate-now
- **Dedup notes**: Unique to claudepal. Directly portable.

### 10. Secret Censoring Module
- **What**: Comprehensive regex-based secret scrubbing for CLI output. Covers: Anthropic API keys, OpenAI keys, GitHub tokens, AWS keys, Bearer tokens, Basic auth in URLs, PEM private key blocks, and generic high-entropy env var assignments. Zero dependencies.
- **Sources**: claudepal (censor.ts), ruflo (WASM kernel secret scanning)
- **Integration target in OAP**: `apps/desktop/src/lib/censor.ts` or shared utility
- **Action**: integrate-now
- **Dedup notes**: Merged claudepal's TypeScript censor module with ruflo's WASM-based secret scanning. claudepal's is simpler and immediately portable; ruflo's has more patterns (8 regex) in Rust/WASM.

### 11. Session Init Protocol (/claudepal-init)
- **What**: Deterministic cold-start session initialization: Step 0 loads memory files. Step 1 batch-reads AGENTS.md, tasks, README, architecture docs, features.yaml, git log/diff in parallel. Step 2 emits structured `## initialized: project` block. The protocol is self-extending -- items added to AGENTS.md "New Sessions" section are automatically picked up.
- **Sources**: claudepal (/claudepal-init + AGENTS.md as self-extending init source)
- **Integration target in OAP**: `.claude/commands/init.md` and `AGENTS.md`
- **Action**: integrate-now
- **Dedup notes**: Unique to claudepal. Most sophisticated init protocol across all 17 projects.

### 12. Shell PATH Resolution for Packaged Desktop Apps
- **What**: Cross-platform shell PATH detection for packaged Electron/Tauri apps: login shell sourcing, nvm/yarn/npm global bin directories, Windows PowerShell + cmd.exe, Homebrew, user-configurable paths, caching with manual clear, and graceful fallback chain. Critical for finding CLI tools (claude, git, etc.) in packaged apps.
- **Sources**: crystal (shellPath.ts + shellDetector.ts), claudepal (CLI spawner with multi-platform resolution)
- **Integration target in OAP**: `apps/desktop/src/lib/shellPath.ts`
- **Action**: integrate-now
- **Dedup notes**: Merged crystal's comprehensive PATH detection with claudepal's CLI resolution. Both solve the same problem (packaged app can't find user's CLI tools). Crystal's is more comprehensive; claudepal's includes the stdin prompt technique for Windows command-line length limits.

### 13. Governance Control Plane (Policy Compiler)
- **What**: Compiles CLAUDE.md files into structured policy bundles containing a "constitution" (always-loaded core rules) and task-scoped "shards" (retrieved by intent/domain). Includes a Rust WASM kernel for deterministic policy enforcement, enforcement gates (destructive ops, secrets scanning, tool allowlist, diff size), coherence scheduler with privilege degradation (full/restricted/read-only/suspended based on drift), and cryptographic proof chains for audit trails.
- **Sources**: ruflo (guidance control plane + WASM kernel + policy compiler + retriever + coherence + proof chains)
- **Integration target in OAP**: `crates/spec-compiler/` or new `crates/conformance-kernel/`
- **Action**: outline-spec
- **Dedup notes**: Unique to ruflo but maps directly to OAP's spec-compiler concept. The constitution/shard split, intent-based retrieval, coherence scoring with privilege degradation, and WASM kernel are all novel.

### 14. Context Compaction for Session Resumption
- **What**: When token usage exceeds threshold (75% of context window), older messages are replaced with structured summaries. Analyzes prompt markers with completion status, file modifications, TodoWrite task status, git diff stats, and interruption detection. Produces `<session_context>` blocks for efficient conversation continuation.
- **Sources**: crystal (ProgrammaticCompactor), claudepal (context compaction system)
- **Integration target in OAP**: `apps/desktop/src/lib/compaction.ts`
- **Action**: outline-spec
- **Dedup notes**: Merged crystal's detailed compaction (per-call summaries, aggregate file modifications, task status) with claudepal's model-aware context windows and compaction logging. Both solve the same critical problem of session continuation.

## P1 — Outline Spec Soon

### 15. Agent Frontmatter Schema and Definition Format
- **What**: Standard YAML frontmatter for agent definitions: name (kebab-case), description (auto-invocation trigger), tools (allowed list), model (haiku/sonnet/opus), category, color, displayName. Description doubles as trigger condition. Skill definitions use name + description frontmatter with markdown body instructions. Contributing guidelines define quality criteria ("Would I pay $5/month?").
- **Sources**: claude-code-sub-agents (agent frontmatter schema + category taxonomy), skills (skill manifest format), developer-claude-code-commands (create-subagent framework), agents (progressive disclosure for skills)
- **Integration target in OAP**: `specs/` as S-SKILL-001 and S-AGENT-001 spec definitions
- **Action**: outline-spec
- **Dedup notes**: Merged four overlapping schema definitions. claude-code-sub-agents has the cleanest agent schema; skills has the cleanest skill schema; developer-cc-commands adds quality criteria; agents adds progressive disclosure (metadata always loaded, instructions on activation, resources on demand).

### 16. Hookify -- Declarative Hook Rule Engine
- **What**: Intercepts Claude Code events (PreToolUse, PostToolUse, UserPromptSubmit, Stop) using markdown-defined rules with YAML frontmatter. Rules specify matchers (tool names, patterns), conditions (field contains/matches), and actions (block/warn/modify). Includes hooks.json format for Claude Code plugin integration with pre/post tool-use hooks that modify bash commands, track file edits, and inject guidance.
- **Sources**: claude-code (Hookify rule engine + security guidance hook), ruflo (hook-based lifecycle system + hooks.json), claudepal (plugin lifecycle hooks)
- **Integration target in OAP**: `packages/governance/hooks/` and `.claude-plugin/hooks/`
- **Action**: outline-spec
- **Dedup notes**: Merged Hookify rule engine, security guidance hook (9 patterns), ruflo's 19-event hook system, and claudepal's 6-event plugin hooks. Core pattern is the same: intercept agent events, evaluate rules, take action (block/warn/modify).

### 17. Permission Request System with Memory and Wildcards
- **What**: SDK canUseTool hook with layered permission model: interactive tools always prompt, bypass mode auto-allows, disallowed tools denied, allowed tools (including wildcards like `Bash(git commit:*)`) auto-approved, everything else triggers UI permission request. Decisions can include `rememberEntry` to dynamically expand allow list. UI shows Allow once / Allow & remember / Deny.
- **Sources**: claudecodeui (canUseTool hook + permission memory + wildcard patterns + PermissionRequestsBanner), crystal (MCP permission bridge)
- **Integration target in OAP**: `apps/desktop/src/components/ClaudeCodeSession.tsx`
- **Action**: outline-spec
- **Dedup notes**: Merged claudecodeui's in-SDK permission handling with crystal's MCP permission bridge. Both solve desktop-mediated tool permission approval with persistence.

### 18. Tool Renderer System (Config-Driven Display)
- **What**: Declarative, config-driven tool rendering where each tool has a `ToolDisplayConfig` entry specifying input display type, result display behavior, content renderers, and color schemes. Replaces hard-coded if/else chains with a registry pattern. Includes subagent container with nested tool history and collapsible thinking traces with elapsed time.
- **Sources**: claudecodeui (ToolRenderer + SubagentContainer + AskUserQuestion panel), deepreasoning (collapsible thinking trace)
- **Integration target in OAP**: `apps/desktop/src/components/ToolWidgets.tsx`
- **Action**: outline-spec
- **Dedup notes**: Merged claudecodeui's config-driven renderer, subagent container, AskUserQuestion panel, and deepreasoning's thinking trace. All address the same need: rich, structured display of AI tool execution.

### 19. Background Agents with Git Worktree Isolation
- **What**: Up to N concurrent background agents, each in an isolated git worktree with its own branch. Agents run with skip-permissions, have inactivity timeouts, and emit lifecycle events. On completion, user previews full diff and chooses to merge. Worktree lifecycle includes create, rebase from main, squash-and-rebase to main, cleanup.
- **Sources**: claudepal (background agents with worktree isolation), crystal (WorktreeManager + session isolation)
- **Integration target in OAP**: `apps/desktop/` agent execution system
- **Action**: outline-spec
- **Dedup notes**: Merged claudepal's 4-agent concurrent system with crystal's comprehensive WorktreeManager (931 lines). Both use git worktrees for session isolation with diff preview and optional merge.

### 20. State Persistence for Resumable Workflows
- **What**: Orchestrator commands use state files (state.json or SQLite) to track workflow progress, enabling resume after interruption. Pattern: create state at start, update current_step/completed_steps after each step, check for existing state on startup and offer resume. Combined with checkpoint/approval gates.
- **Sources**: agents (state.json pattern across conductor, full-stack, review, TDD), claudepal (durable append-only stream server with SSE replay), crystal (session persistence)
- **Integration target in OAP**: `crates/agent/` workflow engine
- **Action**: outline-spec
- **Dedup notes**: Merged simple state.json files from agents with claudepal's durable stream server and crystal's session persistence. All solve workflow resumability; claudepal's append-only stream with SSE replay is the most sophisticated.

### 21. Verification Profiles and Post-Session Gates
- **What**: YAML-based verification configuration with profiles (pr, release), each composing skills. Each skill has: determinism level, safety tier, and steps (command, timeout, read_only, network policy). Combined with post-session security verification gates where registered verifiers (security, conformance, lint) must pass before changes are marked as delivered.
- **Sources**: claudepal (verification.yaml + safety tier classification), asterisk-mcp-server (post-session security gate as governance primitive), developer-claude-code-commands (validate-and-fix severity categories)
- **Integration target in OAP**: `specs/verification.yaml` and governance layer
- **Action**: outline-spec
- **Dedup notes**: Merged verification profiles with safety tier classification and post-session gate concept. claudepal's verification.yaml is the most structured; asterisk-mcp-server adds the gate-as-governance-primitive concept.

### 22. Four-Agent Pipeline (Plan/Explore/Implement/Review)
- **What**: Four specialized agents: Architect (plans, decomposes tasks, read-only), Explorer (codebase analysis, context gathering), Implementer (focused code changes from plans, minimal diffs), Guardian (post-change review for bugs, security, performance). Orchestrator routes complex prompts through pipeline based on complexity scoring.
- **Sources**: claudepal (four agents + complexity scoring), claude-code-sub-agents (development + quality agents)
- **Integration target in OAP**: `.claude/agents/`
- **Action**: integrate-now
- **Dedup notes**: Merged claudepal's four-agent pattern with claude-code-sub-agents' broader taxonomy. The plan/explore/implement/review pipeline is the common pattern.

### 23. YAML Standards Schema with Three-Tier Hierarchy
- **What**: Machine-readable coding standards in YAML with: id, category, priority, rules (ALWAYS/NEVER/USE/PREFER/AVOID), anti-patterns, examples, context, tags. Three-tier override: official -> community -> project-local. Standards loaded with caching, tag filtering, and agent-type mapping.
- **Sources**: equilateral-agents (YAML standards schema + three-tier hierarchy + standards contributor pipeline)
- **Integration target in OAP**: `specs/` governance system as a new spec kind
- **Action**: outline-spec
- **Dedup notes**: Unique to equilateral-agents. The auto-generation of standards from execution findings (standards contributor) and the three-tier override pattern are both novel.

### 24. Session Memory / Project-Object Persistence
- **What**: Persistent context across AI coding sessions. Stores decisions, patterns, corrections, and notes. Harvesting rules detect signals from conversation ("let's go with" = decision, "actually/no" = correction). Includes memory MCP server with importance levels (ephemeral to permanent), expiry, and project scoping.
- **Sources**: equilateral-agents (project-object session memory), claudepal (memory MCP server with importance levels)
- **Integration target in OAP**: `packages/mcp-servers/memory/` or `crates/gitctx/`
- **Action**: outline-spec
- **Dedup notes**: Merged equilateral's harvesting rules with claudepal's MCP-based memory with importance levels. Both solve session-to-session knowledge persistence.

### 25. Commit Message Governance
- **What**: Enforces conventional commit prefixes (feat/fix/refactor/docs/test/chore), impact-focused titles (lead with problem solved, not technique used), 72-char line limit, issue linking. Explicitly bans Co-Authored-By, marketing taglines.
- **Sources**: skills (commit-helper), developer-claude-code-commands (commit.md), crystal (commit mode system)
- **Integration target in OAP**: `.claude/commands/commit.md` and CLAUDE.md
- **Action**: integrate-now
- **Dedup notes**: Merged three overlapping commit governance patterns. skills and developer-cc-commands are nearly identical (same author). Crystal adds commit modes (structured/checkpoint/disabled).

### 26. Notification Orchestrator (Multi-Channel, Deduplicated)
- **What**: Server-side notification system with event creation, preference-gated delivery, 20-second deduplication window, and multi-channel delivery (web push, native). Event schema with provider, session, kind, severity, dedupeKey.
- **Sources**: claudecodeui (notification orchestrator with VAPID/web push)
- **Integration target in OAP**: `apps/desktop/` Tauri native notifications
- **Action**: outline-spec
- **Dedup notes**: Unique to claudecodeui. The event schema and deduplication patterns are applicable to Tauri native notifications.

### 27. File Mention System (@-mention files in chat)
- **What**: @-mention autocomplete for files in chat composer. Fetches project file tree, flattens to searchable list, fuzzy filters on @ trigger, keyboard navigable dropdown.
- **Sources**: claudecodeui (useFileMentions), claude-code-by-agents (agent @mention routing)
- **Integration target in OAP**: `apps/desktop/src/components/FloatingPromptInput.tsx`
- **Action**: outline-spec
- **Dedup notes**: Merged file mentions with agent @mention routing. Both use @-trigger detection with dropdown selection.

### 28. Git Panel (Branches, Changes, History, AI Commit Messages)
- **What**: Comprehensive git panel with staged/unstaged files, branch management, commit history with expandable diffs, and AI-generated commit messages. Plus optimized git plumbing commands for fast status checking.
- **Sources**: claudecodeui (git panel), crystal (git plumbing commands + git status manager)
- **Integration target in OAP**: `apps/desktop/src/components/GitContextPanel.tsx`
- **Action**: outline-spec
- **Dedup notes**: Merged claudecodeui's full commit workflow with crystal's optimized plumbing commands. Both enhance git integration.

### 29. CLI Tool Registry with Capability Declarations
- **What**: Singleton registry where each AI tool (Claude, Codex, Cursor, etc.) declares capabilities (supportsResume, supportsPermissions, etc.), config requirements, and output formats. Factory creates tool managers with availability caching.
- **Sources**: crystal (CliToolRegistry + CliManagerFactory + AbstractCliManager)
- **Integration target in OAP**: `apps/desktop/` if OAP supports multiple AI tool backends
- **Action**: outline-spec
- **Dedup notes**: Unique to crystal but overlaps with multi-provider abstractions above. This focuses on CLI tool discovery/capability rather than API-level abstraction.

### 30. Panel Event Bus System
- **What**: Typed inter-panel event system with capability declarations per panel type, event history, auto-exclusion of source panel. Panels subscribe to typed events (terminal:command_executed, files:changed, git:operation_*) for loose coupling.
- **Sources**: crystal (PanelEventBus + PanelCapabilities)
- **Integration target in OAP**: `apps/desktop/` panel communication
- **Action**: outline-spec
- **Dedup notes**: Unique to crystal. The typed event bus with capability declarations is clean.

### 31. CLAUDE.md Refactorer (Modularization)
- **What**: Systematic approach to breaking large CLAUDE.md files into modular pieces: extract cross-cutting patterns to `docs/[NAME].md`, create `.claude/rules/[name].md` with glob-based path-scoped rules and @imports, update main CLAUDE.md with brief references. Reports size reduction metrics.
- **Sources**: skills (claude-md-refactorer)
- **Integration target in OAP**: `.claude/commands/refactor-claude-md.md`
- **Action**: integrate-now
- **Dedup notes**: Unique to skills. Directly applicable as OAP's CLAUDE.md grows.

### 32. Conductor Track Lifecycle (Spec-Driven Work Units)
- **What**: Work units ("tracks") follow lifecycle: Pending -> In Progress -> Complete -> Archived. Each has spec.md (requirements + acceptance criteria), plan.md (phased tasks with checkbox markers), metadata.json. Implementation follows TDD with phase checkpoints requiring human approval. Git-aware revert by logical unit.
- **Sources**: agents (conductor track lifecycle + templates), skills (plan-implementer lifecycle states), developer-claude-code-commands (plan-to-implementation progress tracking)
- **Integration target in OAP**: Feature lifecycle spec (003) and execution bridge (004)
- **Action**: outline-spec
- **Dedup notes**: Merged conductor's track system with plan-implementer's lifecycle states and plan-to-implementation progress tracking. All implement spec -> plan -> phased implementation with progress tracking.

### 33. DevContainer with Network Sandboxing
- **What**: Complete devcontainer with Dockerfile (Node 20, zsh, git-delta, Claude Code), and iptables/ipset firewall allowing only GitHub, npm, Anthropic API, Sentry, VS Code Marketplace traffic. DNS resolution and IP aggregation for firewall rules.
- **Sources**: claude-code (devcontainer + init-firewall.sh)
- **Integration target in OAP**: `.devcontainer/`
- **Action**: integrate-now
- **Dedup notes**: Unique to claude-code. The network sandboxing is the key differentiator.

### 34. Multi-Model Chaining and Cost Tracking
- **What**: Pattern of calling Model A for reasoning, wrapping output in `<thinking>` tags, injecting as assistant message, then calling Model B for final response. Per-request token usage aggregation across providers with configurable per-model pricing (input/output/cache per million tokens).
- **Sources**: deepreasoning (multi-model chaining + SSE streaming + cost calculation)
- **Integration target in OAP**: `crates/agent/` or `apps/desktop/` agent cost tracking
- **Action**: outline-spec
- **Dedup notes**: Unique architecture from deepreasoning. The cost tracking pattern overlaps with ruflo's economic governor and claude-code-single-binary's SDKResultMessage cost fields.

### 35. Unified Development Workflow (Spec -> Tasks -> Changes -> Verification)
- **What**: Four-system workflow: features.yaml (design + backlog) -> tasks-todo/ (sprint unit) -> changes/ (audit trail) -> verification.yaml (quality gates). Decision table: new feature design in spec, daily work in tasks, large refactors get changesets, shipped features update features.yaml.
- **Sources**: claudepal (unified development workflow + changes/ audit trail)
- **Integration target in OAP**: Process/governance model
- **Action**: outline-spec
- **Dedup notes**: Unique to claudepal. More formalized than OAP's current approach.

### 36. Cleanup with Delegated Agent
- **What**: Spawns a separate cleanup-analyzer agent that runs knip, jscpd, and check:all, investigates each finding, categorizes them (safe-to-remove, needs-review, keep-as-intentional), and returns structured report. Prevents false positives by always keeping shadcn, Radix deps, barrel exports.
- **Sources**: claudepal (/cleanup command + cleanup-analyzer agent)
- **Integration target in OAP**: `.claude/commands/cleanup.md` and `.claude/agents/cleanup-analyzer.md`
- **Action**: integrate-now
- **Dedup notes**: Unique to claudepal. The false-positive prevention logic is the key differentiator.

### 37. MCP Server Implementation Reference (Full Spec)
- **What**: Complete MCP server with tool registry (O(1) lookup, category/tag indexing, batch registration, execution metrics), session management, multi-transport (stdio, HTTP, WebSocket, in-process), and comprehensive type definitions for MCP 2025-11-25 spec.
- **Sources**: ruflo (full MCP server), equilateral-agents (minimal MCP server), claude-code (MCP integration reference)
- **Integration target in OAP**: Reference for OAP MCP servers
- **Action**: outline-spec
- **Dedup notes**: Merged three MCP implementations of varying completeness. ruflo's is the most feature-complete; equilateral's is the cleanest minimal reference.

### 38. Shell Escape and Safe Execution Utilities
- **What**: Cross-platform shell argument escaping (Unix single-quote wrapping, Windows double-quote + backslash), safe git commit command building for multi-line messages, and general safe command construction to prevent injection.
- **Sources**: crystal (shellEscape.ts), ruflo (safe-executor with command allowlists + path validation)
- **Integration target in OAP**: `tools/gitctx-mcp` and `apps/desktop/`
- **Action**: integrate-now
- **Dedup notes**: Merged crystal's shell escape with ruflo's broader safe executor. Both prevent command injection; ruflo adds allowlists and blocked patterns.

### 39. Coherence Scoring and Privilege Degradation
- **What**: Computes coherence score (0-1) from violation rate, rework frequency, and intent drift. Maps to privilege levels: full (>0.7), restricted (0.5-0.7), read-only (0.3-0.5), suspended (<0.3). When agents drift from spec, progressively restrict capabilities rather than hard-blocking.
- **Sources**: ruflo (coherence scheduler)
- **Integration target in OAP**: Governance/spec-compiler for progressive enforcement
- **Action**: outline-spec
- **Dedup notes**: Unique to ruflo. Novel governance pattern -- degradation instead of binary pass/fail.

### 40. Proof Chain / Audit Trail
- **What**: Cryptographically signed, hash-chained proof system where each agent action gets a ProofEnvelope with content hash, previous-envelope linkage, tool call hashes, guidance hash, memory lineage, and HMAC-SHA256 signature. Both TypeScript and Rust WASM implementations.
- **Sources**: ruflo (proof chain in TS + Rust WASM)
- **Integration target in OAP**: Governance audit trail
- **Action**: outline-spec
- **Dedup notes**: Unique to ruflo. The tamper-evident record of what happened, what rules applied, and what memory was accessed.

### 41. Conformance Kit (Agent Cell Acceptance Test)
- **What**: Canonical acceptance test proving governance control plane works end-to-end. Implements a "Memory Clerk" agent cell: reads entries, runs inference, proposes writes, injects coherence drop, verifies system blocks remaining writes, emits signed proof envelope, returns replayable trace.
- **Sources**: ruflo (conformance-kit.ts)
- **Integration target in OAP**: spec-compiler test strategy
- **Action**: outline-spec
- **Dedup notes**: Unique to ruflo. The inject-fault-verify-response testing approach.

### 42. Branch Review Checklist (Cross-Platform + Comprehensive)
- **What**: Exhaustive read-only branch review with structured checklist covering: database changes, security, performance, cross-platform (file paths, keyboard shortcuts, env vars, line endings, permissions, path length limits), dependencies, logging, type safety, bugs, cleanup.
- **Sources**: developer-claude-code-commands (review-branch.md)
- **Integration target in OAP**: `.claude/commands/review-branch.md`
- **Action**: integrate-now
- **Dedup notes**: Overlaps with parallel code review (#1) but this is a single-agent sequential checklist rather than multi-agent parallel. Complementary.

## P2 — Capture as Ideas

### 43. Plugin Marketplace Structure
- **What**: marketplace.json catalog with categorized plugins, each with manifest. Self-contained plugins with agents/commands/skills subdirs. Git-installable with manifest validation and sandboxed subprocess execution.
- **Sources**: agents (marketplace.json), claude-code (marketplace manifest), claudecodeui (git-installable plugin system), claudepal (plugin system with lifecycle hooks)
- **Integration target in OAP**: `packages/registry/` or desktop app extension system

### 44. Three-Tier Model Strategy
- **What**: Strategic model assignment: Opus for critical architecture/security/review, Sonnet for complex tasks, Haiku for fast operational tasks. Orchestration chains combine tiers.
- **Sources**: agents (three-tier model strategy), ruflo (3-tier WASM/Haiku/Sonnet routing), deepreasoning (multi-model chaining)
- **Integration target in OAP**: Agent framework model routing

### 45. Context Save/Restore and Conversation History
- **What**: Context management for saving/restoring project context across sessions. Includes reading Claude Code's .claude/projects/ JSONL history files, timestamp restoration, and deduplication.
- **Sources**: agents (context save/restore), claude-code-by-agents (conversation history loader)
- **Integration target in OAP**: `crates/gitctx/` context persistence

### 46. Encrypted Keychain and Credential Storage
- **What**: Software keychain using AES-256-GCM with machine-derived key. Generic credentials table with typed entries and active toggle.
- **Sources**: claudepal (encrypted keychain), claudecodeui (credential storage pattern)
- **Integration target in OAP**: `apps/desktop/` credential management

### 47. WebSocket Session Reconnection
- **What**: Hot-swap WebSocket connections for in-flight SDK streams. Server swaps new raw WS into existing session's writer object on reconnect.
- **Sources**: claudecodeui (WebSocketWriter.updateWebSocket)
- **Integration target in OAP**: `apps/desktop/` if using WebSocket for long-running sessions

### 48. i18n Infrastructure
- **What**: i18next with namespace-scoped translations, RTL support via CSS logical properties, Tauri tray menu i18n.
- **Sources**: claudecodeui (6 languages), claudepal (RTL + tray i18n)
- **Integration target in OAP**: `apps/desktop/` if multi-language support needed

### 49. Quick Pane (Global Floating AI Chat)
- **What**: Separate Tauri window with global keyboard shortcut (Alt+Space). Floating chat accessible from anywhere on desktop.
- **Sources**: claudepal (quick_pane.rs)
- **Integration target in OAP**: `apps/desktop/` UX feature

### 50. NDJSON Streaming Response Pattern
- **What**: Complete streaming pattern with NDJSON encoding, connection acknowledgment, flush markers for proxy buffering prevention, timeout handling, proper CORS headers.
- **Sources**: claude-code-by-agents (NDJSON streaming), claudepal (CLI stream-JSON parser)
- **Integration target in OAP**: `apps/desktop/` streaming API

### 51. Bun Single-Binary Compilation Pipeline
- **What**: Multi-target build system compiling Node.js CLI to standalone executables. 15 platform/arch/libc combinations.
- **Sources**: claude-code-single-binary (build pipeline)
- **Integration target in OAP**: Reference for CLI tool distribution

### 52. Pain-to-Pattern Methodology
- **What**: Structured process: capture incident -> calculate cost -> extract pattern -> create standard -> enforce -> measure prevented incidents. Graduation criteria for promoting standards.
- **Sources**: equilateral-agents (Pain-to-Pattern + Knowledge Harvest)
- **Integration target in OAP**: Governance methodology

### 53. Agent Communication Bus with Priority Queues
- **What**: Full pub/sub message bus with 5-level priority queues, delivery guarantees, capability routing, health monitoring, and audit trail.
- **Sources**: equilateral-agents (AgentCommunicationBus)
- **Integration target in OAP**: `crates/agent/` multi-agent coordination

### 54. FI/FDI Guardrails Framework
- **What**: Framework for deciding when AI should work independently vs. within orchestration constraints. Post-mortem evidence that unconstrained AI reintroduces bugs agents already caught.
- **Sources**: equilateral-agents (agent orchestration guardrails case study)
- **Integration target in OAP**: Governance documentation

### 55. SQLite Schema for AI Desktop App
- **What**: Complete schema covering sessions, messages (with token accounting), tool calls, background agents, usage analytics, crash recovery, plugins, memory entries with importance levels, and message branches.
- **Sources**: claudepal (schema.ts), crystal (DatabaseService)
- **Integration target in OAP**: `apps/desktop/` data model reference

### 56. AI-Powered Session Naming
- **What**: Use Haiku to generate 2-4 word session names from prompts. Fallback to word extraction. Uniqueness checking against DB and filesystem.
- **Sources**: crystal (WorktreeNameGenerator)
- **Integration target in OAP**: Session management UX

### 57. Security Scanning MCP Tool Pattern
- **What**: MCP tools that proxy code to external security APIs, parse vulnerability reports, return markdown-formatted results with structured error handling.
- **Sources**: asterisk-mcp-server (scan_snippet, scan_codebase, verify)
- **Integration target in OAP**: `packages/mcp-servers/` reference

### 58. Markdown-Formatted Tool Responses Convention
- **What**: All tool responses (including errors) return well-structured markdown. Error taxonomy: connection, timeout, auth, rate-limit, generic with specific remediation guidance.
- **Sources**: asterisk-mcp-server (markdown responses), claudecodeui (tool renderer)
- **Integration target in OAP**: MCP tool response convention

### 59. VS Code Extension (Sidebar Webview)
- **What**: VS Code extension embedding an AI panel as sidebar webview using lockfile-based REST API for lightweight communication.
- **Sources**: claudepal (vscode-extension)
- **Integration target in OAP**: Future OAP VS Code integration

### 60. Slash Command System with Fuzzy Search
- **What**: Fetch available commands (built-in + custom), fuzzy search via Fuse.js, per-project usage history for frequency-based sorting, keyboard-navigable menu.
- **Sources**: claudecodeui (useSlashCommands + CommandMenu)
- **Integration target in OAP**: `apps/desktop/` slash command enhancement

### 61. Design Token System (CSS Custom Properties)
- **What**: Comprehensive token system with primitives, semantic tokens, component-specific tokens, dark/light themes, terminal ANSI colors.
- **Sources**: crystal (CSS tokens), claudepal (OKLCH themes)
- **Integration target in OAP**: `apps/desktop/` theming

### 62. Scheduling System (Cron + Event Triggers)
- **What**: Task scheduling with time-based (daily/weekly) and event-based triggers. Session binding, enable/disable, last_run tracking.
- **Sources**: claudepal (schedule manager)
- **Integration target in OAP**: Future automation feature

## Cross-cutting themes

Patterns that appeared across 3+ projects, representing validated approaches worth prioritizing:

### 1. Parallel Sub-Agent Orchestration (7 projects)
agents, skills, developer-cc-commands, product-manager-cc-commands, claude-code, claude-code-sub-agents, claudepal. The most validated pattern across all extractions. Every project that attempted multi-agent workflows converged on the same approach: spawn parallel specialists, collect results, consolidate.

### 2. Multi-Provider LLM Abstraction (6 projects)
claude-code-by-agents, claudecodeui, claudepal, crystal, ruflo, equilateral-agents. Every UI-oriented project built provider abstractions. Strong signal that OAP will need this eventually.

### 3. File-Based Artifact Passing Between Agents (5 projects)
skills, product-manager-cc-commands, claude-code-by-agents, agents, claudepal. Writing intermediate results to files instead of passing through conversation context is the dominant pattern for token-efficient multi-agent workflows.

### 4. Git Worktree Session Isolation (3 projects)
claudepal, crystal, agents. Independent invention of the same pattern: one worktree per AI session, diff preview, optional merge. Production-validated.

### 5. Governance/Policy Compilation from Markdown (3 projects)
ruflo, agents (conductor), equilateral-agents. Parsing markdown governance documents into structured policy bundles. Directly maps to OAP's spec-compiler.

### 6. Progressive Skill/Knowledge Loading (3 projects)
agents, skills, equilateral-agents. Three-tier loading: metadata always, instructions on activation, deep resources on demand. Minimizes token usage.

### 7. Context Compaction for Long Sessions (3 projects)
crystal, claudepal, ruflo. All three independently built compaction systems for sessions approaching context limits.

### 8. Commit Message Governance (3 projects)
skills, developer-cc-commands, crystal. Conventional commits with impact-focused messaging. Identical pattern across all three.

### 9. Secret Censoring in CLI Output (3 projects)
claudepal, ruflo, asterisk-mcp-server. Regex-based scrubbing of API keys, tokens, and credentials from output surfaced to users or logs.

### 10. Shell PATH Resolution for Packaged Apps (3 projects)
crystal, claudepal, claude-code-single-binary. All three solved the problem of packaged desktop apps losing user PATH independently.

## Safe-to-delete status

| # | Project | Safe to Delete | Notes |
|---|---------|---------------|-------|
| 1 | agents | Yes | All patterns extracted; pure markdown prompt engineering |
| 2 | asterisk-mcp-server | Yes | ~650 lines Python; all patterns extracted |
| 3 | claude-code | Yes | Official Anthropic community repo; patterns extracted |
| 4 | claude-code-by-agents | Yes | All patterns extracted; architecture reference captured |
| 5 | claude-code-single-binary | Yes | Build tooling only; all patterns extracted |
| 6 | claude-code-sub-agents | Yes | All 33 agents and dispatch protocol extracted |
| 7 | claudecodeui | Yes | All UI patterns and architecture extracted |
| 8 | claudepal | Yes | Highest-value source; all Rust crates, patterns, and code extracted |
| 9 | crystal | Yes | All architecture patterns and utilities extracted |
| 10 | deepreasoning | Yes | All patterns extracted; Rust client code captured |
| 11 | developer-claude-code-commands | Yes | All 14 commands extracted |
| 12 | equilateral-agents-open-core | Yes | All patterns and governance methodology extracted |
| 13 | gitctx | Yes | Already byte-identical to OAP's crates/gitctx/ |
| 14 | product-manager-claude-code-commands | Yes | All 19 commands extracted |
| 15 | research | Yes | Empty Zola blog skeleton; zero content |
| 16 | ruflo | Yes | All governance, MCP, and WASM kernel patterns extracted |
| 17 | skills | Yes | All 25 skills extracted |
