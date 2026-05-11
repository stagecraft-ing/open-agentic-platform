---
source: claudepal
source_path: ~/Dev2/stagecraft-ing/claudepal
status: extracted
---

## Summary

Claudepal is a Tauri v2 desktop application that wraps Claude CLI and other AI providers (Gemini, OpenAI, Bedrock) in a rich multi-session workspace. It features a React 19/TypeScript/Tailwind v4 frontend communicating via Socket.IO with a Node.js sidecar server backed by SQLite. The app includes background agents with Git worktree isolation, a governed 9-stage software factory pipeline (Pronghorn), context compaction, a memory MCP server, scheduling, analytics/cost tracking, a plugin/skills system, an axiomregent Rust MCP governance engine (snapshot/diff, featuregraph, xray analysis), a VS Code sidebar extension, a remote-control CLI, and a durable append-only stream server. The project is substantial (~635 non-git files) and represents a production-grade AI desktop shell with governance features that directly overlap with OAP's mission.

## Extractions

### Architecture Pattern: Tauri + Node.js Sidecar over Socket.IO

- **What**: Tauri spawns a Node.js sidecar as a child process. The sidecar writes its port to stdout (`CLAUDEPAL_SERVER_PORT=NNNN`). The Tauri Rust backend reads this, stores it in state, and emits a `server:ready` event to the React frontend. The frontend connects via Socket.IO WebSocket (no polling fallback). This decouples the UI from the server and allows the server to be independently tested or remotely accessed.
- **Where in source**: `src-tauri/src/sidecar.rs`, `server/src/index.ts`, `src-tauri/src/lib.rs` (setup block)
- **Integration target in OAP**: `apps/desktop/src-tauri/` -- OAP already has a Tauri app. This pattern is identical but the sidecar management and Socket.IO bridge pattern could replace or augment OAP's current IPC approach if a server sidecar is desired.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Pattern: State Management Onion (useState -> Zustand -> TanStack Query)

- **What**: Three-tier state architecture: component-local `useState`, cross-component UI state in Zustand v5 (with enforced selector-only access via ast-grep lint rule that blocks destructuring), and persistent/server data via TanStack Query. The `getState()` pattern is enforced in `lib/` modules to prevent React coupling in business logic.
- **Where in source**: `AGENTS.md` (Architecture Patterns section), `.ast-grep/rules/zustand/no-destructure.yml`, `.ast-grep/rules/architecture/no-store-in-lib.yml`
- **Integration target in OAP**: `apps/desktop/src/stores/` -- OAP already uses Zustand. The ast-grep enforcement rules and the architectural boundary discipline are directly portable.
- **Action**: integrate-now
- **Priority**: P1

### Directly Portable Code: ast-grep Architecture Rules

- **What**: Three ast-grep YAML rules that enforce: (1) no Zustand store destructuring (prevents render cascades), (2) no React hooks in `lib/` directory (enforces clean separation), (3) no store subscriptions in `lib/` (must use `getState()` for non-reactive access). These are lint-time enforcement of architecture boundaries.
- **Where in source**: `.ast-grep/rules/zustand/no-destructure.yml`, `.ast-grep/rules/architecture/hooks-in-hooks-dir.yml`, `.ast-grep/rules/architecture/no-store-in-lib.yml`
- **Integration target in OAP**: `apps/desktop/.ast-grep/rules/` (new directory)
- **Action**: integrate-now
- **Priority**: P0

### Directly Portable Code: Secret Censoring Module

- **What**: Comprehensive regex-based secret scrubbing for CLI output. Covers: Anthropic API keys (`sk-ant-*`), OpenAI keys, GitHub tokens (PAT/fine-grained), AWS access/secret keys, Bearer tokens, Basic auth in URLs, PEM private key blocks, and generic high-entropy env var assignments. Returns sanitized string. Zero dependencies.
- **Where in source**: `server/src/cli/censor.ts`
- **Integration target in OAP**: `apps/desktop/src/lib/censor.ts` or a shared `packages/` utility. Useful wherever CLI output is surfaced to the UI or logs.
- **Action**: integrate-now
- **Priority**: P0

### Directly Portable Code: Claude CLI Stream-JSON Parser

- **What**: NDJSON line parser for Claude CLI's `--output-format stream-json` output. Handles all event types: `system`, `user`, `assistant`, `stream_event` (with `content_block_start/delta/stop`, `message_start/delta/stop`), and `result` (with usage stats, cost, duration). Includes a buffered `createLineParser` that handles chunk boundaries correctly.
- **Where in source**: `server/src/cli/parser.ts`
- **Integration target in OAP**: `apps/desktop/src/lib/cli-parser.ts` -- OAP already interacts with Claude CLI; this parser is battle-tested and handles all edge cases.
- **Action**: integrate-now
- **Priority**: P0

### Directly Portable Code: Claude CLI Spawner with Multi-Platform Resolution

- **What**: Spawns Claude CLI with proper flags (`--output-format stream-json`, `--verbose`, `--include-partial-messages`). Handles: (1) CLI path resolution across platforms (probes `which`, npm global bin dirs, Homebrew, NVM, Windows AppData), (2) prompt via stdin to avoid Windows command-line length limits, (3) active process tracking with session-keyed kill, (4) effort level injection via system prompt, (5) orchestration prompt injection, (6) Linux bubblewrap sandbox (PID/UTS namespace isolation).
- **Where in source**: `server/src/cli/spawner.ts`
- **Integration target in OAP**: `apps/desktop/src/lib/cli-spawner.ts` -- the multi-platform CLI resolution and stdin prompt technique are especially valuable.
- **Action**: integrate-now
- **Priority**: P1

### Architecture Pattern: Multi-Provider Normalization Shim

- **What**: A provider abstraction that normalizes Gemini SSE and OpenAI chat completions streaming output into Claude's `stream-json` event format. Claude passes through natively. Gemini/OpenAI fetch their respective APIs and emit synthetic `stream_event` objects matching the same interface. Includes Bedrock support (passthrough with AWS credentials) and Kiro (routes through Bedrock).
- **Where in source**: `server/src/providers/shim.ts`
- **Integration target in OAP**: `apps/desktop/src/lib/providers/` -- if OAP wants multi-provider support, this normalization pattern means the entire UI only needs to handle one event format.
- **Action**: outline-spec
- **Priority**: P2

### Architecture Pattern: Durable Append-Only Stream Server

- **What**: Separate HTTP server (port 4437) providing append-only, offset-resumable session logs. Three endpoints: POST to append, GET for SSE tail from offset N, DELETE to purge. In-memory store with live SSE fan-out to subscribers. Also exposes an in-process `EventEmitter` for the orchestrator to subscribe. Enables crash recovery and remote replay.
- **Where in source**: `server/src/streams/server.ts`, `server/src/streams/store.ts`, `server/src/streams/schema.ts`
- **Integration target in OAP**: `apps/desktop/` -- the concept of a durable session log with SSE replay is architecturally valuable for session persistence and crash recovery.
- **Action**: outline-spec
- **Priority**: P2

### Agent/Skill Definition: Session Init Protocol (/claudepal-init)

- **What**: A deterministic cold-start session initialization skill. Step 0: loads all memory files from MEMORY.md index. Step 1 (batch A): parallel reads of AGENTS.md, tasks, README, architecture docs, features.yaml, git log, git diff, task directories. Step 1 (batch B): reads active task files and recently completed tasks. Step 2: emits a structured `## initialized: project` block with type, key files, structure, memory summary, and focus area. The protocol is self-extending -- any items added to the AGENTS.md "New Sessions" section are automatically picked up.
- **Where in source**: `.claude/commands/claudepal-init.md`
- **Integration target in OAP**: `.claude/commands/` -- this is the most sophisticated init protocol seen in any extraction. The batched parallel read pattern, memory loading step, and structured output schema should become OAP's standard init command.
- **Action**: integrate-now
- **Priority**: P0

### Agent/Skill Definition: /check Command

- **What**: Post-work validation command: (1) checks all session work against architecture docs, (2) removes leftover console.log and dead-end code, (3) runs `npm run check:all`, (4) suggests commit message.
- **Where in source**: `.claude/commands/check.md`
- **Integration target in OAP**: `.claude/commands/check.md`
- **Action**: integrate-now
- **Priority**: P1

### Agent/Skill Definition: /cleanup with Delegated Agent

- **What**: Spawns a `cleanup-analyzer` agent (separate context) that runs knip, jscpd, and check:all, investigates each finding in the codebase, categorizes them (safe-to-remove, needs-review, keep-as-intentional), and returns a structured report. The main agent then offers to create a task document. Notable: the categorization logic (always keep shadcn, Radix deps, barrel exports; only flag high-confidence removals) prevents false positives.
- **Where in source**: `.claude/commands/cleanup.md`, `.claude/agents/cleanup-analyzer.md`
- **Integration target in OAP**: `.claude/commands/cleanup.md`, `.claude/agents/cleanup-analyzer.md`
- **Action**: integrate-now
- **Priority**: P1

### Agent/Skill Definition: Four Specialized Claude Code Agents

- **What**: Four markdown-defined agents installed to `~/.claude/agents/`: (1) Architect -- plans and decomposes 3+ step tasks, read-only, creates TodoWrite plans; (2) Explorer -- read-only codebase analysis and context gathering; (3) Implementer -- makes focused code changes from Architect plans, minimal diffs; (4) Guardian -- post-change review for bugs, security, performance. The orchestrator automatically routes complex prompts through Architect -> Implementer -> Guardian based on complexity scoring.
- **Where in source**: `server/src/agents/manager.ts` (syncAgentFiles function), `.claude/agents/plan-checker.md`, `.claude/agents/docs-reviewer.md`, `.claude/agents/userguide-reviewer.md`
- **Integration target in OAP**: `.claude/agents/` -- the four-agent pattern (plan, explore, implement, review) and the additional review agents (plan-checker, docs-reviewer, userguide-reviewer) are all directly useful.
- **Action**: integrate-now
- **Priority**: P1

### Architecture Pattern: Complexity Scoring for Auto-Orchestration

- **What**: Heuristic scoring of user prompts (0-N score) to decide whether to invoke the multi-agent orchestration pipeline. Factors: prompt length (>300, >600 chars), imperative verb count (3+, 5+), multi-step connectors ("and then", "after that", etc.), and multiple file path mentions. Score >= 2 triggers architect -> implementer -> guardian pipeline.
- **Where in source**: `server/src/sessions/complexity.ts`
- **Integration target in OAP**: `apps/desktop/src/lib/complexity.ts` -- simple, effective, zero-dependency complexity scoring.
- **Action**: integrate-now
- **Priority**: P1

### Architecture Pattern: Context Compaction System

- **What**: When a session's total token count exceeds 75% of the model's context window, older messages are replaced with a Claude-generated summary. Keeps the 5 most recent messages intact. Model-specific context windows are tracked (200K for Claude, 1M for Gemini, 128K for GPT-4o). Compaction is logged in a `compaction_log` table for audit.
- **Where in source**: `server/src/sessions/compaction.ts`
- **Integration target in OAP**: `apps/desktop/src/lib/compaction.ts` or spec for session management feature.
- **Action**: outline-spec
- **Priority**: P2

### Architecture Pattern: Background Agents with Git Worktree Isolation

- **What**: Up to 4 concurrent background agents, each running in an isolated Git worktree (`~/.claudepal/worktrees/<agentId>`). Agents get their own branch (`claudepal-agent-<prefix>`), run with `--dangerously-skip-permissions`, have a 15-minute inactivity timeout, and emit lifecycle events. On completion, the user can preview the full diff and choose to merge. The agent lifecycle is tracked in SQLite and broadcast via Socket.IO events.
- **Where in source**: `server/src/agents/manager.ts`
- **Integration target in OAP**: `apps/desktop/` -- OAP could benefit from background agent execution with worktree isolation. The pattern of git worktree per agent + diff preview + optional merge is production-ready.
- **Action**: outline-spec
- **Priority**: P1

### Architecture Pattern: Orchestrator with Handoff Protocol

- **What**: Two-phase orchestration: (1) Orchestrator subscriber watches the stream event bus for `agent:done` control chunks, packages context from the completed agent's stream, checks if human approval is required, and writes `agent:start` or `agent:blocked` control chunks back to the stream. (2) Sidecar listener acts on control chunks -- spawns agents immediately or holds them pending approval. Includes auto-memory on successful turn completion.
- **Where in source**: `server/src/orchestrator/subscriber.ts`, `server/src/orchestrator/sidecar-listener.ts`
- **Integration target in OAP**: Architecture reference for multi-agent orchestration.
- **Action**: outline-spec
- **Priority**: P2

### MCP Integration: Memory MCP Server

- **What**: stdio JSON-RPC 2.0 MCP server exposing 3 tools: `memory_write` (save a learning with content, category, importance level, optional project scope), `memory_read` (read memories filtered by project), `memory_prune` (remove expired entries). Importance levels: ephemeral, low, normal, high, permanent. Backed by SQLite. Register in `~/.claude/mcp.json` for Claude to read/write its own memories during sessions.
- **Where in source**: `server/src/memory/mcp-server.ts`, `server/src/memory-mcp-entry.ts`
- **Integration target in OAP**: Could become an OAP MCP server package, or the concept could be integrated into the existing gitctx-mcp or a new memory-mcp tool.
- **Action**: outline-spec
- **Priority**: P1

### MCP Integration: Axiomregent MCP Client (Content-Length Framed JSON-RPC)

- **What**: TypeScript MCP client that spawns a Rust binary and communicates over stdio with Content-Length framed JSON-RPC 2.0 (LSP wire format). Handles: auto-start/restart, initialization handshake, 30-second request timeouts, tool calls with typed results, event emission for connect/disconnect lifecycle. The binary resolution searches: env var, `~/.claudepal/axiomregent`, dev paths.
- **Where in source**: `server/src/axiomregent/client.ts`
- **Integration target in OAP**: Reference implementation for how OAP's desktop app should communicate with MCP server binaries. The Content-Length framing, lazy start, and auto-restart pattern are production-ready.
- **Action**: capture-as-idea
- **Priority**: P2

### Directly Portable Code: Encrypted Keychain (AES-256-GCM)

- **What**: Software keychain using AES-256-GCM with a machine-derived key (scrypt of `hostname:username`). Stores encrypted API keys in the SQLite settings table. Not a true OS keychain but encrypts at rest and ties keys to the current machine user. Three functions: `keychainSet`, `keychainGet`, `keychainDelete`.
- **Where in source**: `server/src/auth/keychain.ts`
- **Integration target in OAP**: `apps/desktop/src/lib/keychain.ts` -- useful for storing provider API keys without native keychain dependencies.
- **Action**: capture-as-idea
- **Priority**: P2

### Directly Portable Code: Plugin System with Lifecycle Hooks

- **What**: Plugin loader that scans `~/.claudepal/plugins/` for `plugin.json` manifests. Supports 5 types: command, agent, hook, skill, mcp. Hooks provide: `beforePrompt`, `afterResponse`, `beforeToolUse`, `afterToolUse`, `onSessionStart`, `onSessionEnd`. Plugins are registered in SQLite, can be enabled/disabled, and are hot-reloadable via `reloadPlugins()`.
- **Where in source**: `server/src/plugins/loader.ts`
- **Integration target in OAP**: Architecture reference for a plugin system. The hook lifecycle (before/after prompt, before/after tool use, session start/end) is a good model.
- **Action**: capture-as-idea
- **Priority**: P2

### Directly Portable Code: Skills System (Trigger-Based Context Injection)

- **What**: Skills are trigger-matched context injections appended to system prompts. Triggers include: file extension, keyword, regex, cwd_contains. Built-in skills: react-typescript, rust, python, sql-database, git. Users can create custom skills stored in `~/.claudepal/skills/*.json` and in SQLite. The `buildSkillsInjection()` function matches all enabled skills against the prompt and CWD, returning combined injection text.
- **Where in source**: `server/src/plugins/skills.ts`
- **Integration target in OAP**: `apps/desktop/src/lib/skills.ts` -- OAP already has a skills concept. This trigger-based matching pattern and the built-in skill definitions are directly useful.
- **Action**: integrate-now
- **Priority**: P1

### Directly Portable Code: Remote Control Server + CLI

- **What**: HTTP REST API (port 38000-39000, random) with token auth (`X-Control-Token`). Endpoints: list/create/delete sessions, list/send messages. Token and port written to `~/.claudepal/control.token` and `~/.claudepal/control.port`. Accompanying `claudepal-ctl` CLI tool reads these files and provides a command-line interface for remote session management. Localhost-only with reject of non-localhost connections.
- **Where in source**: `server/src/control/server.ts`, `server/src/control/token.ts`, `packages/claudepal-ctl/src/cli.js`
- **Integration target in OAP**: Architecture reference for remote control of the desktop app. The lockfile-based port discovery pattern is elegant.
- **Action**: capture-as-idea
- **Priority**: P2

### Directly Portable Code: VS Code Extension (Sidebar Webview)

- **What**: VS Code extension that embeds a claudepal panel as a sidebar webview. Uses the control server's REST API (port/token from lockfiles). Shows session list, allows creating sessions, sending messages, and viewing responses. Auto-connects on activation. All HTML/CSS/JS inline in a single `_getHtml()` method using VS Code's CSS variables for native theming.
- **Where in source**: `packages/vscode-extension/src/extension.ts`
- **Integration target in OAP**: Reference for building an OAP VS Code extension. The pattern of using a lockfile-based REST API for communication (rather than Socket.IO directly) makes the extension lightweight.
- **Action**: capture-as-idea
- **Priority**: P2

### Spec/Governance: Feature Registry (features.yaml)

- **What**: A YAML-based master feature registry with 35+ features. Each entry has: id, title, governance status, implementation status, spec path, group, dependencies, aliases, implementation notes, and file list. The featuregraph scanner enforces `// Feature: FEATURE_ID` attribution in source files and validates against this registry. This is the authoritative source for what exists, what's deferred, and what depends on what.
- **Where in source**: `spec/features.yaml`
- **Integration target in OAP**: `specs/` -- OAP has a spec spine. This features.yaml pattern with per-file Feature attribution is exactly what OAP's conformance lint could enforce.
- **Action**: integrate-now
- **Priority**: P0

### Spec/Governance: Featuregraph Scanner + Preflight Checker (Rust)

- **What**: Rust crate that: (1) parses `spec/features.yaml` into a feature graph, (2) walks the filesystem scanning source file headers for `// Feature: X` and `// Spec: spec/path.md` directives, (3) builds a graph of features -> impl files -> test files, (4) detects violations: DANGLING_FEATURE_ID, MISSING_SPEC_FILE, SPEC_PATH_MISMATCH, INVALID_HEADER_FORMAT, DUPLICATE_FEATURE_ID. The preflight checker validates proposed changes against the graph and assigns safety tiers (Tier1=autonomous, Tier2=gated, Tier3=forbidden). Content-addressed graph fingerprint via SHA256.
- **Where in source**: `crates/featuregraph/src/scanner.rs`, `crates/featuregraph/src/preflight.rs`, `crates/featuregraph/src/graph.rs`
- **Integration target in OAP**: `crates/spec-compiler/` or a new `crates/featuregraph/` -- this IS OAP's conformance lint concept, fully implemented in Rust. The header parsing, violation detection, and preflight checking are directly portable to OAP's spec compiler.
- **Action**: integrate-now
- **Priority**: P0

### Spec/Governance: Safety Tier Classification for Agent Tools

- **What**: Three-tier safety classification for MCP tools: Tier1 (safe to auto-execute: read-only checks), Tier2 (requires human approval: file writes, snapshots), Tier3 (forbidden: unknown tools). A plan's overall tier is the maximum tier of any tool call in it. This determines whether an agent changeset can be auto-applied or needs human review.
- **Where in source**: `crates/agent/src/safety.rs`
- **Integration target in OAP**: `crates/` or spec -- the tiered safety model for tool execution is directly applicable to OAP's governance model.
- **Action**: integrate-now
- **Priority**: P1

### Spec/Governance: Verification Profiles (verification.yaml)

- **What**: YAML-based verification configuration with toolchain checks, profiles (pr, release), and individual skills. Each skill has: description, determinism level (D0=fully deterministic, D1=may vary), safety tier, and steps (command, timeout, read_only mode, network policy). Profiles compose skills. The `pr` profile runs format + typecheck; `release` runs everything including Rust.
- **Where in source**: `spec/verification.yaml`
- **Integration target in OAP**: `specs/` -- this verification.yaml format could standardize how OAP defines quality gates for the spec compiler or CI.
- **Action**: integrate-now
- **Priority**: P1

### Spec/Governance: Unified Development Workflow (spec -> tasks -> changes -> verification)

- **What**: A four-system workflow: `spec/features.yaml` (design + backlog) -> `tasks-todo/` (sprint unit) -> `changes/` (audit trail with meta, implementation plan, walkthrough) -> `spec/verification.yaml` (quality gates). Decision table: new feature design goes in spec, day-to-day work in tasks-todo, large refactors get a changeset in changes/, shipped features update features.yaml. Every changeset links to a task; the `implementation: deferred` field in features.yaml IS the authoritative backlog.
- **Where in source**: `AGENTS.md` (Unified Development Workflow section), `changes/README.md`
- **Integration target in OAP**: Process/governance model -- this workflow is more formalized than OAP's current approach and could inform OAP's task management spec.
- **Action**: outline-spec
- **Priority**: P1

### Architecture Pattern: Snapshot + Diff for Session Governance

- **What**: Before each AI session: axiomregent takes a content-addressed filesystem snapshot. After the session: diffs against the baseline to surface exactly what changed. The snapshot store uses SQLite for metadata + filesystem blob store with optional zstd compression. Manifests are sorted, content-addressed, and validated. This creates a full audit trail of what every AI session changed.
- **Where in source**: `server/src/axiomregent/session-hooks.ts`, `crates/axiomregent/src/snapshot/store.rs`
- **Integration target in OAP**: `crates/` -- OAP's gitctx-mcp already does some of this. The snapshot store's content-addressing, compression, and validation patterns are production-grade.
- **Action**: outline-spec
- **Priority**: P1

### Architecture Pattern: Xray Repository Analysis

- **What**: Rust crate that performs comprehensive repository analysis: file traversal (respecting .gitignore), language detection, LOC counting, digest computation, module file identification, and canonical JSON output. Produces an `XrayIndex` with stats, languages, top directories, and file details. Atomic write to output file.
- **Where in source**: `crates/xray/src/`
- **Integration target in OAP**: `crates/` or gitctx-mcp -- could enhance OAP's context generation with repo analysis.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Pattern: Factory Pipeline (9-Stage Governed Software Delivery)

- **What**: A complete pipeline manager for the "Pronghorn" software development methodology. 9 stages (Init, Requirements, Client Service, Database, API, Designer, Codebase Init, Coding, Security) with 3 human approval gates. Each stage has: preflight checks, agent spec assembly, upstream artifact injection, governed session dispatch, artifact detection, and failure mode detection (legacy prefixes, DML in DDL, duplicate files, missing artifacts). Manifest writes go through axiomregent's governed write path.
- **Where in source**: `server/src/factory/` (manager.ts, types.ts, config.ts, manifest.ts, scanner.ts, agents.ts, dispatcher.ts, gate.ts, failure.ts, ingest.ts)
- **Integration target in OAP**: This IS the governed software delivery pipeline that OAP aspires to build. The stage/gate/artifact/manifest model is directly relevant. However, it's deeply coupled to the Pronghorn methodology -- OAP would need to generalize it.
- **Action**: outline-spec
- **Priority**: P1

### Directly Portable Code: SQLite Schema for AI Desktop App

- **What**: Complete SQLite schema covering: sessions, messages (with token accounting and compaction), tool calls, background agents, usage analytics, crash recovery snapshots, plugins, settings (key-value), compaction log, schedules (cron + event triggers), memory entries (with importance levels and expiry), orchestrator cursors, and message branches. Includes WAL mode, foreign keys, and idempotent migrations.
- **Where in source**: `server/src/db/schema.ts`
- **Integration target in OAP**: `apps/desktop/` -- this schema represents a mature data model for an AI desktop app. Particularly valuable: the message branching tables, compaction log, and memory entries with importance levels.
- **Action**: outline-spec
- **Priority**: P1

### Build/CI: Tauri Cross-Platform Build Workflow

- **What**: GitHub Actions workflow building for macOS (arm64 + x86_64), Ubuntu, and Windows. Uses: tauri-apps/tauri-action, Rust cache, Node.js LTS with npm cache. Uploads platform-specific artifacts (DMG, MSI, NSIS, AppImage, DEB).
- **Where in source**: `.github/workflows/build.yml`
- **Integration target in OAP**: `.github/workflows/` -- OAP already has CI for desktop. This workflow is a clean reference for cross-platform Tauri builds.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Pattern: i18n with RTL Support

- **What**: i18next + react-i18next with locale files in `/locales/` (en.json, fr.json, ar.json). CSS uses logical properties (`text-start` not `text-left`) for RTL support. Tray menu strings are sent from the frontend for i18n support (`rebuild_tray_menu` Tauri command).
- **Where in source**: `locales/*.json`, `src/i18n/`, `src-tauri/src/tray.rs`
- **Integration target in OAP**: `apps/desktop/src/` -- OAP's desktop app could adopt the same i18n pattern. The Tauri tray i18n approach (frontend sends localized strings to Rust) is clever.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Pattern: Scheduling System (Cron + Event Triggers)

- **What**: Task scheduling with two trigger types: time-based (daily HH:MM, weekly DAY HH:MM) and event-based (fired on internal events like session end). 60-second tick interval. Each schedule can optionally bind to a session. Stores in SQLite with enable/disable, last_run_at tracking.
- **Where in source**: `server/src/schedule/manager.ts`
- **Integration target in OAP**: Reference for if OAP adds task scheduling.
- **Action**: capture-as-idea
- **Priority**: P2

### UI Pattern: Quick Pane (Global Floating AI Chat)

- **What**: A separate Tauri window registered with a global keyboard shortcut (Alt+Space). Shows/hides a floating chat panel accessible from anywhere on the desktop, independent of the main window.
- **Where in source**: `src-tauri/src/commands/quick_pane.rs`, `src/components/quick-pane/QuickPaneApp.tsx`
- **Integration target in OAP**: `apps/desktop/` -- a quick-access floating panel is a compelling UX feature for a desktop AI tool.
- **Action**: capture-as-idea
- **Priority**: P2

### Spec/Governance: AGENTS.md as Self-Extending Init Protocol Source

- **What**: The AGENTS.md file serves dual purpose: (1) human documentation of architecture patterns and rules, (2) machine-readable protocol that the init command dynamically parses. Any item added to the "New Sessions" section is automatically picked up by `/claudepal-init`. This means the initialization protocol extends itself as new documentation is added -- no need to update the init command separately.
- **Where in source**: `AGENTS.md`
- **Integration target in OAP**: `AGENTS.md` -- OAP's AGENTS.md should adopt this self-extending pattern where the init command reads from it dynamically.
- **Action**: integrate-now
- **Priority**: P0

## No-value items

- **UI component library (src/components/ui/)**: Standard shadcn/ui v4 components (button, dialog, input, etc.). OAP already has these. No unique customizations worth extracting.
- **Icon/image assets**: Project-specific branding (Icon.svg, app icons). Not applicable to OAP.
- **Cargo.lock / package-lock.json**: Dependency lockfiles. Not transferable.
- **locales/ar.json, fr.json**: Translation files with claudepal-specific strings. Would need complete rewrite for OAP.
- **changes/ directory (000-024)**: Claudepal-specific changeset history. Internal project management, not portable.
- **tasks-done/ directory**: Completed task documents specific to claudepal development.
- **docs/developer/**: Developer documentation for claudepal patterns. Useful as reference but too project-specific to port directly.
- **docs/userguide/**: User guide for claudepal. Not applicable.
- **server/src/files/manager.ts**: File listing/reading handlers. Standard file I/O, no unique value.
- **server/src/settings/manager.ts**: Key-value settings CRUD. Trivial implementation.
- **server/src/rules/manager.ts**: CRUD for ~/.claude/rules/ files. Simple file management.
- **src/data/themes.ts**: OKLCH color theme definitions. Project-specific aesthetic choices.
- **src/components/titlebar/**: Platform-specific window control components (macOS, Windows, Linux). Standard Tauri titlebar implementations.
- **prettier.config.js, eslint.config.js**: Standard linting config. OAP has its own.
- **Cargo.toml (workspace)**: Workspace configuration specific to claudepal crate structure.
- **axiomregent.tasks.yaml**: Task runner configuration specific to claudepal's axiomregent skills.
- **sgconfig.yml**: SourceGraph config. Not relevant.
- **vite.config.ts, vitest.config.ts, tsconfig.json**: Standard build config. OAP has its own.
- **detached-session.html, quick-pane.html**: HTML entry points for secondary Tauri windows. Trivial.
- **scripts/complete-task.js, prepare-release.js**: Claudepal-specific task/release scripts.
- **server/scripts/build-sidecar.mjs**: Build script for the Node.js sidecar. Specific to claudepal's build setup.

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
