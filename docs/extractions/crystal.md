---
source: crystal
source_path: ~/Dev2/stagecraft-ing/crystal
status: extracted
---

## Summary

Crystal is a deprecated Electron desktop application (now replaced by "Nimbalyst") for managing multiple AI code assistant sessions (Claude Code and OpenAI Codex) against a single project directory using git worktrees. Built with Electron + React 19 + TypeScript + Tailwind + Zustand + better-sqlite3, it provides session lifecycle management, parallel AI coding sessions, git worktree isolation, panel-based UI with terminal/diff/editor/logs panels, MCP permission bridging, context compaction for session resumption, commit mode management (structured/checkpoint/disabled), and a pluggable CLI tool registry for adding new AI agents. The codebase is ~182 source files, well-structured with a modular main process (services/, ipc/, utils/) and React frontend (components/, hooks/, stores/).

## Extractions

### [Architecture Pattern]: CLI Tool Registry and Factory

- **What**: A singleton registry pattern (`CliToolRegistry`) with capability declarations, availability caching (5min TTL), and a factory (`CliManagerFactory`) for creating CLI tool managers. Each tool declares capabilities (supportsResume, supportsPermissions, supportsGitIntegration, etc.), config requirements (env vars, executables), and supported output formats. The factory auto-registers built-in tools and provides `discoverTools()` for system scanning. This is a clean abstraction for supporting multiple AI coding agents (Claude, Codex, and future Aider/Continue/Cursor).
- **Where in source**: `main/src/services/cliToolRegistry.ts`, `main/src/services/cliManagerFactory.ts`, `main/src/services/panels/cli/AbstractCliManager.ts`
- **Integration target in OAP**: Could inform a similar agent/tool registry pattern in OAP's spec system or desktop app for managing multiple AI tool backends
- **Action**: outline-spec
- **Priority**: P1

### [Architecture Pattern]: Abstract CLI Manager Base Class

- **What**: `AbstractCliManager` is an abstract EventEmitter-based base class (~994 lines) that handles: PTY process spawning via node-pty, process lifecycle (spawn/kill/killAll), availability checking with caching, shell PATH resolution, output parsing pipeline, and session/panel coordination. Subclasses override: `getCliToolName()`, `testCliAvailability()`, `buildCommandArgs()`, `getCliExecutablePath()`, `parseCliOutput()`, `initializeCliEnvironment()`, `cleanupCliResources()`, `getCliNotAvailableMessage()`. This provides a robust template for integrating any CLI-based AI tool.
- **Where in source**: `main/src/services/panels/cli/AbstractCliManager.ts`, implementations in `main/src/services/panels/claude/claudeCodeManager.ts` and `main/src/services/panels/codex/codexManager.ts`
- **Integration target in OAP**: `apps/desktop` if OAP ever needs to manage CLI AI tool processes
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Panel Event Bus System

- **What**: A typed inter-panel event system with `PanelEventBus` (EventEmitter-based singleton), `PanelEvent` types (terminal:command_executed, files:changed, diff:refreshed, editor:file_saved, process:started/ended, git:operation_*), and `PanelCapabilities` declarations per panel type defining what events each can emit/consume. Panels subscribe with typed callbacks, events auto-exclude the source panel, and history is kept (last 100 events). This enables loose coupling between panels -- e.g., a terminal detecting file changes triggers diff panel refresh.
- **Where in source**: `main/src/services/panelEventBus.ts`, `shared/types/panels.ts` (PANEL_CAPABILITIES constant)
- **Integration target in OAP**: `apps/desktop` -- the event bus pattern for inter-panel communication is directly applicable to OAP's Tauri desktop app panel system
- **Action**: outline-spec
- **Priority**: P1

### [Architecture Pattern]: Panel Manager with Lazy Initialization

- **What**: `PanelManager` manages panel lifecycle (create, delete, update, setActive) with SQLite persistence, in-memory caching, mutex-protected operations, and lazy process initialization (panels don't start processes until first viewed). Panels have metadata (position, permanent flag, createdAt/lastActiveAt), state (isActive, hasBeenViewed, customState), and types (terminal, claude, codex, diff, editor, logs, dashboard, setup-tasks). The `CreatePanelRequest` pattern with auto-generated titles and position ordering is clean.
- **Where in source**: `main/src/services/panelManager.ts`, `shared/types/panels.ts`
- **Integration target in OAP**: `apps/desktop` panel management system
- **Action**: outline-spec
- **Priority**: P1

### [Architecture Pattern]: Git Worktree Session Isolation

- **What**: `WorktreeManager` (~931 lines) manages git worktree lifecycle for parallel AI sessions: create worktree from branch or new branch, handle empty repos (auto initial commit), rebase from main, squash-and-rebase to main, cleanup on session delete, branch management, commit history tracking with stats (additions/deletions/filesChanged). Uses mutex locks for concurrent safety. The worktree folder can be configured per-project (default: `worktrees/` subdir or absolute path).
- **Where in source**: `main/src/services/worktreeManager.ts`
- **Integration target in OAP**: OAP already has gitctx-mcp; this worktree manager pattern could enhance it with session-scoped worktree lifecycle management
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Context Compaction for Session Resumption

- **What**: `ProgrammaticCompactor` generates structured session summaries for AI conversation continuation. It analyzes: prompt markers with completion status, file modifications extracted from tool_use messages (Edit/Write/MultiEdit), TodoWrite task status, git diff statistics, and interruption detection. Produces a `<session_context>` block with per-call summaries (user prompt, assistant response, files modified, duration) plus aggregate file modifications, task status, and git status. This is critical for efficient conversation resumption without replaying full history.
- **Where in source**: `main/src/utils/contextCompactor.ts`
- **Integration target in OAP**: Highly relevant for OAP's session management -- any multi-turn AI workflow needs context compaction
- **Action**: outline-spec
- **Priority**: P0

### [Architecture Pattern]: Async Mutex with Named Locks

- **What**: A simple but effective async mutex implementation with named resource locks, configurable timeouts (default 30s), `withLock()` convenience function, and a global singleton. Used throughout the codebase to prevent race conditions in panel creation, git operations, session updates, etc.
- **Where in source**: `main/src/utils/mutex.ts`
- **Integration target in OAP**: Useful utility for any concurrent operations in the desktop app
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Shell PATH Resolution for Packaged Desktop Apps

- **What**: Comprehensive cross-platform shell PATH detection (`getShellPath()`) that handles: packaged Electron apps losing user PATH, login shell sourcing (zsh/bash config files), nvm/yarn/npm global bin directories, Windows PowerShell + cmd.exe, Linux-specific paths (/snap/bin, ~/.local/bin), user-configurable additional paths, caching with manual clear, and graceful fallback chain. `findExecutableInPath()` searches with Windows extension handling. `ShellDetector` detects default shell via $SHELL, macOS Directory Services, /etc/passwd, and common shell paths.
- **Where in source**: `main/src/utils/shellPath.ts`, `main/src/utils/shellDetector.ts`
- **Integration target in OAP**: Critical for `apps/desktop` (Tauri) -- same PATH resolution problems exist. OAP should port this logic for finding CLI tools (claude, git, etc.) in packaged apps
- **Action**: integrate-now
- **Priority**: P0

### [Architecture Pattern]: Command Executor with Enhanced PATH

- **What**: Drop-in `execSync`/`execAsync` replacements that automatically inject the enhanced shell PATH into every subprocess, with logging (command + preview of result), error handling, and a `silent` flag to suppress verbose logs. Used throughout the codebase instead of raw child_process.
- **Where in source**: `main/src/utils/commandExecutor.ts`
- **Integration target in OAP**: `apps/desktop` for consistent subprocess execution with proper PATH
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Git Plumbing Commands for Performance

- **What**: Optimized git status checking using low-level plumbing commands instead of porcelain `git status`. `fastCheckWorkingDirectory()` uses: `git update-index --refresh`, `git diff-files --quiet` (unstaged), `git diff-index --cached --quiet HEAD` (staged), `git ls-files --others --exclude-standard` (untracked), `git diff --diff-filter=U` (conflicts). Also `fastGetAheadBehind()` via `git rev-list --left-right --count`, and `fastGetDiffStats()` via `git diff --numstat`. Each checks for directory existence first (handles deleted worktrees gracefully).
- **Where in source**: `main/src/services/gitPlumbingCommands.ts`
- **Integration target in OAP**: `tools/gitctx-mcp` -- these optimized git operations could improve gitctx-mcp performance
- **Action**: integrate-now
- **Priority**: P1

### [Architecture Pattern]: Git File Watcher with Smart Debouncing

- **What**: `GitFileWatcher` uses native `fs.watch` (recursive) to monitor worktree directories, filters out non-git-relevant changes (.git/, node_modules/, editor temp files, .DS_Store), debounces rapid changes (1.5s), and uses git plumbing commands to verify if refresh is actually needed before emitting events. This avoids polling and reduces unnecessary git status recalculations.
- **Where in source**: `main/src/services/gitFileWatcher.ts`
- **Integration target in OAP**: `tools/gitctx-mcp` or `apps/desktop` for efficient git status monitoring
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Commit Mode System (Structured/Checkpoint/Disabled)

- **What**: Three commit modes for AI sessions: (1) **Checkpoint** -- auto-commits after each prompt completion with configurable prefix ("checkpoint: "), using `--no-verify` to skip hooks. (2) **Structured** -- enhances the AI prompt with conventional commit instructions so the AI itself creates proper commits. (3) **Disabled** -- no auto-commit. Includes pre-commit hook warning detection (husky, changesets), session finalization with squash, and a `CommitManager` service with event emission.
- **Where in source**: `main/src/services/commitManager.ts`, `shared/types.ts` (CommitModeSettings, DEFAULT_STRUCTURED_PROMPT_TEMPLATE)
- **Integration target in OAP**: Spec system or desktop app session management -- the commit mode concept is directly useful for governed AI workflows
- **Action**: outline-spec
- **Priority**: P1

### [Architecture Pattern]: MCP Permission Bridge

- **What**: An MCP server (`crystal-permissions`) that bridges Claude Code's permission system to a desktop UI approval flow. The bridge runs as a subprocess communicating with the main Electron process via Unix domain socket IPC. When Claude Code requests tool permission, the bridge forwards it to the main process, which shows a UI dialog, and returns the user's allow/deny decision. Uses `@modelcontextprotocol/sdk` Server with `StdioServerTransport`.
- **Where in source**: `main/src/services/mcpPermissionBridge.ts`, `main/src/services/mcpPermissionServer.ts`, `main/src/services/permissionManager.ts`, `main/src/services/permissionIpcServer.ts`
- **Integration target in OAP**: Directly relevant to OAP's MCP integrations -- pattern for desktop-mediated tool permission approval
- **Action**: outline-spec
- **Priority**: P1

### [UI Component]: Design Token System (CSS Custom Properties)

- **What**: A comprehensive CSS custom property token system with: primitive colors (gray scale, blue, green, amber, red), brand colors (lilac/purple for light theme), semantic tokens (bg-primary/secondary/tertiary, surface-*, text-*, border-*, interactive-*, status-*), context-aware text tokens (text-on-primary, text-on-interactive, etc.), component-specific tokens (button, card, input, modal, navigation/sidebar), scrollbar colors, and full terminal color tokens (16 ANSI colors). Supports dark theme (default) and light theme via `:root.light` class. Uses warm off-whites and lilac accent for light theme.
- **Where in source**: `frontend/src/styles/tokens/colors.css`, `frontend/src/styles/tokens/typography.css`, `frontend/src/styles/tokens/spacing.css`, `frontend/src/styles/tokens/effects.css`
- **Integration target in OAP**: `apps/desktop` -- OAP's Tailwind-based desktop app could adopt this token architecture for theming
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Component]: Message Transformer Architecture

- **What**: A `MessageTransformer` interface that normalizes different AI agent output formats into a `UnifiedMessage` structure with typed `MessageSegment`s (text, tool_call, tool_result, system_info, thinking, diff, error). Separate implementations for Claude (`ClaudeMessageTransformer`) and Codex (`CodexMessageTransformer`). Tool calls include hierarchical support (sub-agents with parentToolId/childToolCalls).
- **Where in source**: `frontend/src/components/panels/ai/transformers/MessageTransformer.ts`, `ClaudeMessageTransformer.ts`, `CodexMessageTransformer.ts`
- **Integration target in OAP**: `apps/desktop` -- if OAP displays AI agent outputs, this normalization pattern is essential
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: AI-Powered Session Naming

- **What**: `WorktreeNameGenerator` uses Claude Haiku to generate 2-4 word session names from user prompts (e.g., "Fix user authentication bug" -> "Fix Auth Bug"). Falls back to simple word extraction if no API key. Separates session names (with spaces) from worktree names (kebab-case). Ensures uniqueness by checking both DB and filesystem.
- **Where in source**: `main/src/services/worktreeNameGenerator.ts`
- **Integration target in OAP**: Nice UX pattern for any session/task naming in OAP
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Prompt Enhancement for Commit Modes

- **What**: `PromptEnhancer` appends structured commit instructions to AI prompts when sessions are configured for structured commit mode. Uses configurable templates defaulting to Conventional Commits format with instructions for the AI to create proper git commits.
- **Where in source**: `main/src/utils/promptEnhancer.ts`, `shared/types.ts` (DEFAULT_STRUCTURED_PROMPT_TEMPLATE)
- **Integration target in OAP**: Spec system -- prompt enhancement/injection is a governance mechanism
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Tool Output Formatter

- **What**: Comprehensive formatter (~1200+ lines frontend, ~800+ lines backend) that converts Claude Code's JSON streaming output into formatted terminal output with ANSI colors. Handles: assistant text, tool calls (Read, Edit, Write, Bash, etc.) with syntax-highlighted previews, tool results with error detection, thinking blocks, system messages, and session results. Filters base64 data from output. Frontend and backend versions exist for different contexts.
- **Where in source**: `frontend/src/utils/toolFormatter.ts`, `main/src/utils/toolFormatter.ts`
- **Integration target in OAP**: `apps/desktop` if OAP shows AI agent output in terminal-like views
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI]: Electron Builder Configuration (macOS/Linux/Windows)

- **What**: Complete electron-builder config for: macOS universal binary (DMG + ZIP, code signing, notarization, hardened runtime, JAR cleanup for claude-code vendor), Linux (deb + AppImage), Windows (NSIS installer). asar packaging with selective unpacking for native modules. Canary build system with version injection. GitHub Actions workflows for build (canary on push to main) and release (on version tags). After-sign hook removes unsigned JAR files from claude-code vendor directory.
- **Where in source**: `package.json` (build section), `.github/workflows/build.yml`, `.github/workflows/release.yml`, `build/afterSign.js`, `scripts/`
- **Integration target in OAP**: OAP uses Tauri not Electron, but the CI patterns (matrix builds, caching, parallel frontend+main builds, canary system) are transferable
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Task Queue with SimpleQueue Fallback

- **What**: `TaskQueue` wraps Bull (Redis-backed) with an in-memory `SimpleQueue` fallback for Electron environments without Redis. Manages three queues: session-creation (concurrency 5, 1 on Linux due to PTY limits), session-input (concurrency 10), and session-continue (concurrency 10). Session creation orchestrates: project validation, AI name generation, worktree creation, build script execution, panel creation with polling, and AI process startup. Multi-session creation auto-creates folders.
- **Where in source**: `main/src/services/taskQueue.ts`, `main/src/services/simpleTaskQueue.ts`
- **Integration target in OAP**: The SimpleQueue pattern (in-memory task queue with concurrency control) could be useful for OAP's desktop app task orchestration
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: SQLite Database with Migration System

- **What**: `DatabaseService` (~2700+ lines) wraps better-sqlite3 with: transaction helper with auto-rollback, dual migration system (TypeScript + SQL files), comprehensive CRUD for projects/sessions/outputs/conversations/diffs/panels/folders/preferences, JSON column handling for panel state/metadata, session archiving (soft delete), prompt markers with completion tracking, and performance-optimized queries with prepared statements. Tables include: projects, sessions, session_outputs, conversation_messages, execution_diffs, prompt_markers, tool_panels, folders, project_run_commands, claude_panel_settings (legacy), ui_state, app_opens, user_preferences.
- **Where in source**: `main/src/database/database.ts`, `main/src/database/models.ts`
- **Integration target in OAP**: OAP uses SQLite via the spec compiler; the migration pattern and schema design for session tracking could inform OAP's data model
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Zustand Store with Performance Optimizations

- **What**: Session store uses aggressive output trimming (300 stdout items, 100 JSON messages max), batched git status updates with 50ms debounce window, periodic cleanup of inactive session data (every 30s + on session switch + on visibility change), and careful array cloning (only clone when mutation found). Panel store uses `immer` middleware for safe immutable updates.
- **Where in source**: `frontend/src/stores/sessionStore.ts`, `frontend/src/stores/panelStore.ts`
- **Integration target in OAP**: `apps/desktop` -- these V8/React performance patterns are directly applicable
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Logger with File Rotation and Queue

- **What**: Production-grade `Logger` class with: file-based logging to `~/.crystal/logs/`, 10MB rotation with 5-file retention, async write queue to prevent blocking, EPIPE error handling (common in packaged apps), captures original console methods before any overrides, verbose/info/warn/error levels tied to config.
- **Where in source**: `main/src/utils/logger.ts`
- **Integration target in OAP**: `apps/desktop` -- Tauri app needs similar file logging for debugging packaged builds
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Git Status Manager with Smart Polling

- **What**: `GitStatusManager` (~872 lines) with: visibility-aware polling (only polls active/visible sessions), 5-second cache TTL, 2-second debounce for rapid changes, concurrent operation limiting (max 3 parallel git operations), cancellation support via AbortController, event throttling (100ms) to prevent UI flooding, staggered initial load (200ms between sessions), and integration with GitFileWatcher for change-driven refresh instead of pure polling.
- **Where in source**: `main/src/services/gitStatusManager.ts`
- **Integration target in OAP**: Performance patterns for any polling-based status system in the desktop app
- **Action**: capture-as-idea
- **Priority**: P2

### [Spec/Governance]: CLAUDE.md and AGENTS.md Patterns

- **What**: Well-structured project guidance files: `CLAUDE.md` provides comprehensive context (architecture, commands, coding standards, file structure, troubleshooting, API endpoints) for AI coding assistants. `AGENTS.md` provides concise machine-actionable rules (build commands, coding style, testing guidelines, commit conventions, security tips). Both demonstrate effective AI-assisted development documentation patterns.
- **Where in source**: `CLAUDE.md`, `AGENTS.md`
- **Integration target in OAP**: OAP already has CLAUDE.md files; the AGENTS.md pattern (structured, brief, actionable) is a good complement
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Pattern]: Shell Escape Utilities

- **What**: `escapeShellArg()` for cross-platform shell argument escaping (Unix single-quote wrapping, Windows double-quote + backslash escaping), `buildGitCommitCommand()` for safe multi-line commit messages, and `buildSafeCommand()` for general command construction. Prevents command injection.
- **Where in source**: `main/src/utils/shellEscape.ts`
- **Integration target in OAP**: `tools/gitctx-mcp` and `apps/desktop` -- security-critical utility
- **Action**: integrate-now
- **Priority**: P1

### [Architecture Pattern]: Analytics with Privacy Controls

- **What**: `AnalyticsManager` wrapping PostHog with: opt-in consent (disabled by default), anonymous distinct ID (random UUID stored in config, not hardware-derived), session ID hashing for privacy, numeric value categorization (ranges instead of exact values), duration bucketing, separate minimal client for basic tracking even when opted out, and batch flushing (20 events or 10 seconds).
- **Where in source**: `main/src/services/analyticsManager.ts`, `frontend/src/components/AnalyticsConsentDialog.tsx`
- **Integration target in OAP**: Privacy-respecting analytics pattern for the desktop app
- **Action**: capture-as-idea
- **Priority**: P2

### [Idea]: Nimbalyst Migration/Launch Pattern

- **What**: Clean pattern for app deprecation and successor launching. IPC handler checks if successor app exists at known path, spawns it with workspace path and filter arguments as detached process. Frontend shows install dialog when successor not found.
- **Where in source**: `main/src/ipc/nimbalyst.ts`, `frontend/src/components/NimbalystInstallDialog.tsx`
- **Integration target in OAP**: Pattern for app lifecycle transitions
- **Action**: capture-as-idea
- **Priority**: P2

### [Idea]: Auto-Updater for Desktop App

- **What**: electron-updater integration with: manual download trigger (no auto-download), auto-install on quit, progress tracking with IPC events, canary build support, test mode with local update server.
- **Where in source**: `main/src/autoUpdater.ts`, `main/src/ipc/updater.ts`, `frontend/src/components/UpdateDialog.tsx`
- **Integration target in OAP**: Tauri has its own updater, but the UX patterns (check/download/install flow, canary builds) are relevant
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

- **frontend/src/components/DiscordPopup.tsx**: Crystal-specific community link popup. No value.
- **frontend/src/components/StravuConnection.tsx, StravuFileSearch.tsx, StravuStatusIndicator.tsx**: Stravu SaaS integration. Not relevant to OAP.
- **main/src/services/stravuAuthManager.ts, stravuNotebookService.ts, stravuMcpService.ts**: Stravu backend integration. MCP service is stubbed out (returns mock data). No value.
- **frontend/src/components/icons/NimbalystIcon.tsx**: Brand-specific icon. No value.
- **frontend/src/components/TokenTest.tsx**: Dev-only design token test page. No value.
- **main/src/test-updater.ts**: Local update server for testing. Too Electron-specific.
- **screenshots/**: Product screenshots. No value.
- **CONTRIBUTING.md, CHANGELOG.md, LICENSE**: Standard project files. No value.
- **docs/ANALYTICS_UI_EVENTS_INTEGRATION.md, FEATURE_USAGE_TRACKING_INTEGRATION.md**: Analytics implementation guides specific to Crystal's PostHog integration.
- **docs/troubleshooting/**: Crystal-specific troubleshooting. No value.
- **docs/DATABASE_DOCUMENTATION.md, SESSION_OUTPUT_SYSTEM.md, STATE_MANAGEMENT.md, TIMESTAMP_HANDLING.md**: Crystal-internal documentation. Patterns already captured above.
- **docs/TOOL_PANEL_SYSTEM.md**: Crystal panel system docs. Patterns already captured above.
- **com.stravu.crystal.yml**: Flatpak manifest. Electron-specific.
- **check-license-compatibility.sh, scripts/generate-notices.js, scripts/test-license-generation.js**: License compliance tooling. Standard OSS practice.
- **frontend/src/components/demos/ExtendedThinkingDemo.tsx**: Demo component. No value.
- **playwright.*.config.ts, tests/**: E2E test infrastructure. Electron-specific tests.
- **main/src/polyfills/readablestream.ts**: Electron-specific polyfill for ReadableStream.
- **Backup files (*.backup)**: Stale code copies. No value.
- **shared/types/aiPanelConfig.ts, cliPanels.ts**: Thin type files. Patterns already captured in panels.ts extraction.
- **frontend/src/services/analyticsService.ts, panelApi.ts**: Thin API wrappers. No unique patterns.
- **frontend/src/utils/cn.ts**: Standard clsx/tailwind-merge utility. No value.
- **frontend/src/utils/debounce.ts, performanceUtils.ts, dashboardCache.ts**: Standard utility functions. No unique patterns.
- **frontend/src/utils/sanitizer.ts, console.ts, gitStatusLogger.ts**: Crystal-specific utilities.
- **frontend/src/contexts/**: Standard React context providers (Session, ContextMenu, Theme). No unique patterns.
- **frontend/src/hooks/useResizable.ts, useResizablePanel.ts**: Standard resize hooks. No unique patterns.

## Safe-to-delete confirmation
- [x] All valuable content extracted or documented above
