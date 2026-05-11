---
source: claudecodeui
source_path: ~/Dev2/stagecraft-ing/claudecodeui
status: extracted
---

## Summary

ClaudeCodeUI (aka CloudCLI UI, v1.25.2, GPL-3.0) is a web-based UI for Claude Code CLI that wraps the `@anthropic-ai/claude-agent-sdk` in an Express/WebSocket server with a React/TypeScript/Tailwind frontend. It provides multi-provider support (Claude, Cursor, Gemini CLI, OpenAI Codex), a real-time chat interface with tool rendering, permission approval workflows, an integrated terminal (xterm.js + node-pty), file tree browser, CodeMirror code editor, git panel (branches/changes/history), task management (TaskMaster integration), PRD editor, plugin system (git-installable with manifest validation), project creation wizard (with GitHub clone support), voice input (Whisper transcription), web push notifications (VAPID/service worker), onboarding flow, i18n (6 languages), and a SQLite-backed auth/credentials/settings system. The architecture is a Node.js Express server with WebSocket streaming to a Vite-built React SPA, designed for both local use and platform deployment behind a proxy.

## Extractions

### [UI Components]: Tool Renderer System (config-driven tool display)

- **What**: A declarative, config-driven tool rendering system where each tool (Bash, Read, Edit, Write, Grep, Glob, TodoWrite, Task, AskUserQuestion, etc.) has a `ToolDisplayConfig` entry specifying input display type (`one-line` | `collapsible` | `hidden`), result display behavior (`hidden` | `hideOnSuccess` | `collapsible`), content renderers (diff, markdown, file-list, todo-list, task, question-answer, text), color schemes, and action handlers. A `ToolRenderer` component routes to `OneLineDisplay` or `CollapsibleDisplay` based on config. This replaces hard-coded if/else chains with a registry pattern.
- **Where in source**: `src/components/chat/tools/configs/toolConfigs.ts`, `src/components/chat/tools/ToolRenderer.tsx`, `src/components/chat/tools/components/`
- **Integration target in OAP**: `apps/desktop/src/components/ToolWidgets.tsx` / `ToolWidgets.new.tsx` -- OAP already has tool widgets but they use a different pattern. The config-driven registry approach is cleaner and more extensible.
- **Action**: outline-spec
- **Priority**: P1

### [UI Components]: Permission Request Banner with Allow/Remember/Deny

- **What**: A `PermissionRequestsBanner` component that renders pending tool permission requests from the Claude SDK's `canUseTool` hook. Each request shows the tool name, input details (expandable), and three action buttons: "Allow once", "Allow & remember" (persists a permission entry like `Bash(git commit:*)` to local settings), and "Deny". Supports batch approval for matching permission entries. Includes a `permissionPanelRegistry` for tool-specific custom panels (e.g., `AskUserQuestionPanel` gets a multi-step form with keyboard shortcuts).
- **Where in source**: `src/components/chat/view/subcomponents/PermissionRequestsBanner.tsx`, `src/components/chat/tools/configs/permissionPanelRegistry.ts`, `src/components/chat/utils/chatPermissions.ts`
- **Integration target in OAP**: `apps/desktop/src/components/ClaudeCodeSession.tsx` -- OAP handles permissions differently via the SDK. The "Allow & remember" pattern with `Bash(command:*)` wildcards and the custom panel registry are valuable additions.
- **Action**: outline-spec
- **Priority**: P1

### [UI Components]: AskUserQuestion Interactive Panel

- **What**: A polished multi-step question/answer panel for the Claude SDK's `AskUserQuestion` tool. Features: multi-question wizard with step progress dots, single/multi-select options with keyboard shortcuts (1-9 for options, 0 for "Other", Enter to advance, Esc to skip), animated mount transitions, gradient accent styling, and a compact footer with navigation. Fully keyboard-navigable.
- **Where in source**: `src/components/chat/tools/components/InteractiveRenderers/AskUserQuestionPanel.tsx`
- **Integration target in OAP**: New component in `apps/desktop/src/components/` -- OAP doesn't have a dedicated interactive question panel for the SDK's AskUserQuestion tool.
- **Action**: integrate-now
- **Priority**: P0

### [UI Components]: Subagent Container (nested tool history)

- **What**: Renders Claude SDK subagent/Task tool executions with: a purple left-border visual marker, collapsible prompt display, real-time "Currently: [toolName]" indicator with pulse animation, completion status with tool count, expandable tool history showing each child tool's name and key input (file path, command, etc.), and final result display. Provides visual hierarchy for nested agent executions.
- **Where in source**: `src/components/chat/tools/components/SubagentContainer.tsx`
- **Integration target in OAP**: `apps/desktop/src/components/ClaudeCodeSession.tsx` -- OAP shows subagent output but lacks this structured visual container.
- **Action**: outline-spec
- **Priority**: P1

### [UI Components]: Token Usage Pie Chart

- **What**: A minimal SVG-based circular progress indicator showing token usage percentage with color-coded thresholds (blue <50%, orange 50-75%, red >75%). Shows exact token counts on hover. Pure component, no dependencies.
- **Where in source**: `src/components/chat/view/subcomponents/TokenUsagePie.tsx`
- **Integration target in OAP**: `apps/desktop/src/components/TokenCounter.tsx` -- OAP has a token counter but this visual representation is more intuitive.
- **Action**: integrate-now
- **Priority**: P2

### [UI Components]: Diff Viewer (compact, VS Code-style)

- **What**: A compact inline diff viewer showing added/removed lines with red/green highlighting, line number gutters, file path header with clickable navigation, and badge labels ("Edit", "New", "Patch"). Used for Edit/Write/ApplyPatch tool displays.
- **Where in source**: `src/components/chat/tools/components/ToolDiffViewer.tsx`
- **Integration target in OAP**: `apps/desktop/src/components/ClaudeFileEditor.tsx` -- could complement existing diff display.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Notification Orchestrator (multi-channel, deduped)

- **What**: A server-side notification system with: event creation (`createNotificationEvent` with provider, sessionId, kind, code, meta, severity, dedupeKey), preference-gated delivery (`shouldSendPush` checks per-event-type preferences), 20-second deduplication window, Web Push delivery via VAPID keys (auto-generated and stored in SQLite), subscription cleanup on 410/404, and convenience wrappers (`notifyRunStopped`, `notifyRunFailed`). Frontend has `useWebPush` hook for subscribe/unsubscribe, service worker handles push display and notification click navigation.
- **Where in source**: `server/services/notification-orchestrator.js`, `server/services/vapid-keys.js`, `src/hooks/useWebPush.ts`, `public/sw.js`, `src/components/settings/view/tabs/NotificationsSettingsTab.tsx`
- **Integration target in OAP**: `apps/desktop/` -- Tauri has its own notification system, but the orchestrator pattern (deduplication, preference gates, event codes) and the settings UI are useful. The notification event schema could be adopted for Tauri native notifications.
- **Action**: outline-spec
- **Priority**: P1

### [Architecture Patterns]: Multi-Provider Agent Abstraction

- **What**: A unified architecture for running multiple AI coding agents (Claude via SDK, Cursor via CLI, Gemini via CLI, OpenAI Codex via SDK) through a common WebSocket protocol. Each provider has a dedicated module (`claude-sdk.js`, `cursor-cli.js`, `gemini-cli.js`, `openai-codex.js`) implementing the same interface: `spawn/query(command, options, ws)`, `abort(sessionId)`, `isActive(sessionId)`, `getActiveSessions()`. All normalize their output to compatible WebSocket message types (`session-created`, `claude-response`/provider-specific, `claude-complete`, `token-budget`). The frontend uses a `provider` discriminator to route messages.
- **Where in source**: `server/claude-sdk.js`, `server/cursor-cli.js`, `server/gemini-cli.js`, `server/openai-codex.js`, `shared/modelConstants.js`
- **Integration target in OAP**: `apps/desktop/src/components/ClaudeCodeSession.tsx` -- OAP currently only supports Claude. The multi-provider abstraction pattern could inform future provider support. The `modelConstants.js` model registry is a useful reference.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: WebSocket Session Reconnection

- **What**: When a client reconnects (page refresh) while the SDK is still running, the server's `reconnectSessionWriter` swaps the new raw WebSocket into the existing session's writer object, allowing in-flight SDK streams to continue delivering to the reconnected client. The `WebSocketWriter` wrapper class has an `updateWebSocket(newRawWs)` method. Frontend detects reconnection via `hasConnectedRef` and emits a `websocket-reconnected` event so components can catch up on missed messages.
- **Where in source**: `server/claude-sdk.js` (`reconnectSessionWriter`), `src/contexts/WebSocketContext.tsx`
- **Integration target in OAP**: `apps/desktop/` -- relevant if OAP ever supports long-running sessions that survive UI restarts.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Plugin System (git-installable, sandboxed)

- **What**: A plugin architecture where plugins are git repos cloned to `~/.claude-code-ui/plugins/`, validated against a `manifest.json` schema (required: name, displayName, entry; optional: server, permissions, type, slot), with `npm install --production --ignore-scripts` for deps. Plugin servers run as restricted subprocesses (minimal env, no host secrets) that report `{ready: true, port: N}` on stdout. Host proxies requests to plugin servers via `/api/plugins/:name/rpc/*`. Plugins can be enabled/disabled, updated via `git pull --ff-only`, and uninstalled. Path traversal prevention, duplicate name detection, credential stripping from repo URLs.
- **Where in source**: `server/utils/plugin-loader.js`, `server/utils/plugin-process-manager.js`, `server/routes/plugins.js`, `src/contexts/PluginsContext.tsx`, `src/components/plugins/`
- **Integration target in OAP**: Could inform a plugin/extension system for the desktop app. The manifest schema, sandboxed subprocess model, and proxy pattern are well-designed.
- **Action**: outline-spec
- **Priority**: P2

### [UI Components]: Slash Command System with Fuzzy Search

- **What**: A slash command system that fetches available commands (built-in + custom from project `.claude/commands/`) from the server, provides fuzzy search via Fuse.js with configurable weights, tracks per-project command usage history in localStorage for frequency-based sorting, and renders a command menu with keyboard navigation (arrow keys, Tab/Enter to select, Esc to dismiss). Detects `/` at start-of-line or after whitespace (ignoring code blocks), debounces query filtering.
- **Where in source**: `src/components/chat/hooks/useSlashCommands.ts`, `src/components/chat/view/subcomponents/CommandMenu.tsx`
- **Integration target in OAP**: `apps/desktop/src/components/SlashCommandPicker.tsx` -- OAP already has a slash command picker. The fuzzy search via Fuse.js and usage-frequency sorting could enhance it.
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components]: File Mention System (@-mention files)

- **What**: An `@`-mention autocomplete for files in the chat composer. Fetches project file tree on project selection, flattens to a searchable list, detects `@` followed by non-space characters, shows dropdown with fuzzy-filtered file paths (max 10), keyboard navigable, renders selected mentions with highlighted background in the input overlay. Tracks active mentions and provides a regex for splitting input text around mentions.
- **Where in source**: `src/components/chat/hooks/useFileMentions.tsx`
- **Integration target in OAP**: `apps/desktop/src/components/` -- OAP's chat input could benefit from file mention support if it doesn't already have it.
- **Action**: outline-spec
- **Priority**: P1

### [UI Components]: Voice Input (Whisper Transcription)

- **What**: A microphone button component that records audio via `getUserMedia`/`MediaRecorder`, sends the blob to a server-side Whisper transcription endpoint, and inserts the transcript into the chat input. Supports multiple Whisper modes (default, enhanced), shows recording/transcribing/processing states, handles secure context requirements, debounces mobile double-taps.
- **Where in source**: `src/components/mic-button/`, `src/components/mic-button/data/whisper.ts`
- **Integration target in OAP**: `apps/desktop/` -- voice input is not present in OAP. The mic button controller pattern could be reused with a Tauri-native recording approach.
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components]: Onboarding Wizard (Git + Agent Connections)

- **What**: A two-step onboarding flow: Step 1 collects git name/email and persists to the database. Step 2 shows agent connection status cards for each provider (Claude, Cursor, Gemini, Codex) with real-time auth status checking and login modals. Tracks onboarding completion in the user record.
- **Where in source**: `src/components/onboarding/`
- **Integration target in OAP**: `apps/desktop/src/components/StartupIntro.tsx` -- OAP has a startup intro but the structured onboarding with provider connection verification is more thorough.
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components]: Project Creation Wizard (with GitHub Clone)

- **What**: A three-step wizard: (1) workspace type selection (existing folder, new folder, GitHub clone), (2) configuration (folder browser or GitHub URL + auth token), (3) review + create/clone. Supports stored GitHub tokens with auto-selection, inline token entry, clone progress streaming. Uses a folder browser modal for local path selection.
- **Where in source**: `src/components/project-creation-wizard/`
- **Integration target in OAP**: `apps/desktop/src/components/ProjectList.tsx` -- OAP's project creation could benefit from the clone-from-GitHub workflow.
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components]: Quick Settings Panel (draggable, categorized toggles)

- **What**: A slide-in settings panel with categorized toggle sections: Appearance (dark mode, language), Tool Display (auto-expand, raw parameters, tool results), View Options (show timestamps, compact mode), Input Settings (ctrl+enter to send), and Whisper voice mode selection. Uses a drag handle for resize.
- **Where in source**: `src/components/quick-settings-panel/`
- **Integration target in OAP**: `apps/desktop/` -- OAP has Settings but no quick-access panel. The categorized toggle pattern is useful for frequently-changed preferences.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Claude SDK canUseTool Hook with Permission Memory

- **What**: The `canUseTool` callback in `claude-sdk.js` implements a layered permission model: (1) interactive tools like `AskUserQuestion` always prompt, (2) `bypassPermissions` mode auto-allows, (3) disallowed tools are denied, (4) allowed tools (including wildcard patterns like `Bash(git commit:*)`) are auto-approved, (5) everything else triggers a WebSocket permission request to the frontend with a timeout. Decisions can include `rememberEntry` to dynamically add to the allow list. The `matchesToolPermission` function handles exact tool names and `Bash(prefix:*)` glob patterns.
- **Where in source**: `server/claude-sdk.js` (`mapCliOptionsToSDK`, `canUseTool` callback, `matchesToolPermission`, `waitForToolApproval`)
- **Integration target in OAP**: `apps/desktop/src/components/ClaudeCodeSession.tsx` -- OAP uses the SDK but the permission memory/wildcard pattern system could enhance its permission handling.
- **Action**: outline-spec
- **Priority**: P0

### [Architecture Patterns]: Auth System (JWT + API Keys + Platform Mode)

- **What**: A multi-mode auth system: (1) OSS mode uses bcrypt-hashed passwords with JWT tokens (7-day expiry, auto-refresh at half-life via `X-Refreshed-Token` header), (2) Platform mode bypasses auth for managed deployments, (3) External API keys (`ck_` prefixed, 32 random bytes) for programmatic access. WebSocket auth extracts token from query param. JWT secret auto-generated per installation and stored in SQLite `app_config`.
- **Where in source**: `server/middleware/auth.js`, `server/database/db.js` (`userDb`, `apiKeysDb`, `appConfigDb`), `server/routes/auth.js`
- **Integration target in OAP**: OAP doesn't currently have user auth (it's a desktop app). The API key system and the platform-mode pattern could inform future multi-user or remote-access features.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: MCP Server Config Detection and Management

- **What**: A unified system for detecting, listing, adding, and removing MCP servers from Claude's `~/.claude.json` config. Supports user-scope and local/project-scope servers. The detector (`mcp-detector.js`) reads config files directly for fast status checks. Routes use the Claude CLI (`claude mcp add/remove/list/get`) for mutations. The `loadMcpConfig` function in `claude-sdk.js` merges global and project-specific MCP servers before passing to the SDK.
- **Where in source**: `server/utils/mcp-detector.js`, `server/routes/mcp.js`, `server/claude-sdk.js` (`loadMcpConfig`)
- **Integration target in OAP**: `apps/desktop/src/components/MCPManager.tsx` -- OAP already has MCP management. The config-file-direct-read approach for detection is faster than spawning CLI processes.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Session Name Override System

- **What**: A `session_names` table that stores custom display names for sessions, keyed by `(session_id, provider)`. Batch lookup via `getNames(sessionIds, provider)` returns a Map. Applied after fetching sessions via `applyCustomSessionNames(sessions, provider)`. Enables user-friendly session naming independent of the auto-generated CLI summary.
- **Where in source**: `server/database/db.js` (`sessionNamesDb`), `server/database/init.sql`
- **Integration target in OAP**: `apps/desktop/src/components/SessionList.tsx` -- if OAP doesn't already support session renaming.
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI]: PWA Service Worker with Push Notifications

- **What**: A service worker that caches static assets, handles push notification display (with session-aware notification tags for replacement), and navigates to the relevant session on notification click. Combined with VAPID key generation, push subscription management, and a notification preferences UI.
- **Where in source**: `public/sw.js`, `public/manifest.json`
- **Integration target in OAP**: Not directly applicable (Tauri desktop app), but the notification event schema and preference model could inform Tauri's native notification integration.
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components]: Git Panel (Branches, Changes, History, Diff, Commit)

- **What**: A comprehensive git panel with three views: Changes (staged/unstaged files with status icons, select all/none, commit composer with AI-generated messages via Claude/Cursor), Branches (list, create, switch, delete), History (commit list with expandable diffs, revert support). Includes a diff viewer for per-file changes and a commit composer with provider selection for AI message generation.
- **Where in source**: `src/components/git-panel/`
- **Integration target in OAP**: `apps/desktop/src/components/GitContextPanel.tsx` -- OAP has git context but the full commit workflow (staging, AI commit messages, branch management) is more comprehensive.
- **Action**: outline-spec
- **Priority**: P1

### [UI Components]: Task Board (Kanban/List/Grid with TaskMaster)

- **What**: A task management panel integrating with the TaskMaster MCP server. Features: Kanban board with drag-implied status columns (pending, in-progress, review, done, blocked, deferred, cancelled), list view, grid view, task detail modals, create task modal, task filters (status, priority, search), sort controls, "next task" banner, PRD file listing per project. Detects TaskMaster MCP server configuration status.
- **Where in source**: `src/components/task-master/`
- **Integration target in OAP**: Could inform a task/work-item panel in OAP. The Kanban board component and TaskMaster integration pattern are useful references.
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components]: PRD Editor (Markdown with Save/Download)

- **What**: A markdown-based PRD (Product Requirements Document) editor with: file save to project directory, download as .md, overwrite confirmation, auto-`.prd.md` extension, keyboard shortcuts (Cmd+S to save, Esc to close), registry of existing PRDs per project to prevent accidental overwrites, and integration with TaskMaster for task generation from PRDs.
- **Where in source**: `src/components/prd-editor/`
- **Integration target in OAP**: OAP has a MarkdownEditor but not a dedicated PRD workflow. The save-to-project-directory and overwrite-protection patterns could enhance spec editing in the governance panel.
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components]: Integrated Terminal (xterm.js + node-pty)

- **What**: A full terminal emulator using xterm.js with WebGL renderer, fit addon, clipboard addon, web links addon, connected to server-side node-pty via a dedicated WebSocket. Supports project-scoped shell sessions, session-aware initialization, connection overlay, keyboard shortcuts panel, and process exit detection via ANSI parsing.
- **Where in source**: `src/components/shell/`, `src/components/standalone-shell/`
- **Integration target in OAP**: OAP doesn't have an integrated terminal. However, Tauri has different constraints (no node-pty). The xterm.js frontend setup and shell connection protocol could inform a future terminal panel using Tauri's shell API.
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components]: Code Editor (CodeMirror with Sidebar)

- **What**: A CodeMirror-based code editor with: language detection, dark/light theme, minimap, file sidebar with tree navigation, binary file detection, markdown preview mode, keyboard shortcuts (Cmd+S save, Cmd+B toggle sidebar), loading states, and footer with cursor position/language info.
- **Where in source**: `src/components/code-editor/`
- **Integration target in OAP**: `apps/desktop/src/components/ClaudeFileEditor.tsx` -- OAP has a file editor. The minimap and sidebar file navigation could enhance it.
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components]: File Tree (with Upload, Search, Context Menu, Image Viewer)

- **What**: A file tree browser with: recursive directory loading, file type icons (language-specific), grid/list view modes, search/filter, file upload via drag-and-drop (react-dropzone), context menu (open, rename, delete, copy path), detailed columns (size, modified date), image viewer for image files, binary file detection. Lazy-loads directory contents on expand.
- **Where in source**: `src/components/file-tree/`
- **Integration target in OAP**: `apps/desktop/src/components/FilePicker.tsx` -- OAP has a file picker but this is a more full-featured file browser.
- **Action**: capture-as-idea
- **Priority**: P2

### [Spec/Governance]: Credential Storage Pattern

- **What**: A generic credentials table (`user_credentials`) storing typed credentials (github_token, gitlab_token, etc.) with name, type, value, description, and active toggle. Accessed through a type-filtered API. Backward-compatible wrapper for legacy GitHub-specific token storage.
- **Where in source**: `server/database/db.js` (`credentialsDb`), `server/database/init.sql`, `server/routes/settings.js`
- **Integration target in OAP**: Could inform how OAP stores provider API keys and tokens if it ever needs persistent credential management beyond environment variables.
- **Action**: capture-as-idea
- **Priority**: P2

### [Ideas Only]: i18n Infrastructure

- **What**: Full internationalization via i18next with 6 languages (en, de, ja, ko, ru, zh-CN), namespace-scoped translation files (auth, chat, codeEditor, common, settings, sidebar, tasks), browser language detection, and a compact language selector component.
- **Where in source**: `src/i18n/`, `src/shared/view/ui/LanguageSelector.tsx`
- **Integration target in OAP**: OAP doesn't have i18n. The namespace-scoped translation file organization is a good pattern if OAP ever adds multi-language support.
- **Action**: capture-as-idea
- **Priority**: P2

### [Ideas Only]: Chat Composer Patterns

- **What**: The chat composer combines slash commands, file mentions, image attachments (paste/drag), thinking mode selection (default, extended thinking), provider/model selection, and voice input into a unified input experience. The `useChatComposerState` hook manages all these concerns. Image attachments are sent as base64 to the server, which saves them to temp files and appends paths to the prompt.
- **Where in source**: `src/components/chat/hooks/useChatComposerState.ts`, `src/components/chat/view/subcomponents/ChatComposer.tsx`
- **Integration target in OAP**: `apps/desktop/src/components/FloatingPromptInput.tsx` -- OAP's prompt input could adopt the image attachment and thinking mode patterns.
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

- **README.md / README.*.md**: Marketing/documentation content, not portable. Skipped.
- **CHANGELOG.md / CONTRIBUTING.md**: Project lifecycle documents specific to this repo. Skipped.
- **package-lock.json**: Dependency lock file, not portable. Skipped.
- **.release-it.json / release.sh / commitlint.config.js**: Release tooling specific to npm publishing. OAP has its own CI. Skipped.
- **.github/workflows/discord-release.yml**: Discord webhook notification on release. OAP has its own CI. Skipped.
- **.github/ISSUE_TEMPLATE/**: Issue templates, not portable. Skipped.
- **public/icons/**, **public/logo*.png**, **public/favicon.***: Brand assets, not portable (different project identity). Skipped.
- **public/api-docs.html**, **public/clear-cache.html**: Utility pages for the web app. Not applicable to Tauri desktop. Skipped.
- **scripts/fix-node-pty.js**: node-pty build fix for npm installs. Not applicable (OAP doesn't use node-pty). Skipped.
- **server/load-env.js**, **server/cli.js**: Server startup/CLI entry points specific to Express. OAP uses Tauri. Skipped.
- **server/constants/config.js**: Simple IS_PLATFORM flag. Too trivial to extract. Skipped.
- **server/projects.js**: Claude CLI project discovery (reads `~/.claude/projects/`). OAP has its own project management. Skipped.
- **server/gemini-response-handler.js**: NDJSON parser for Gemini CLI. Too provider-specific. Skipped.
- **server/utils/commandParser.js**, **server/utils/frontmatter.js**, **server/utils/gitConfig.js**: Small utility modules. Not novel enough to extract. Skipped.
- **server/routes/auth.js**, **server/routes/cli-auth.js**, **server/routes/user.js**: Web auth routes. OAP is a desktop app. Skipped.
- **server/routes/commands.js**: Slash command file listing from project directory. Simple file listing. Skipped.
- **server/routes/taskmaster.js**: TaskMaster MCP proxy routes. Too tightly coupled to this server's architecture. Skipped.
- **server/routes/codex.js**, **server/routes/cursor.js**, **server/routes/gemini.js**: Provider-specific route files. Not portable as-is. The pattern is captured in the multi-provider extraction above. Skipped.
- **server/routes/mcp-utils.js**: MCP server utility routes. Covered by the MCP extraction above. Skipped.
- **src/contexts/AuthContext.jsx**, **src/contexts/ThemeContext.jsx**, **src/contexts/TasksSettingsContext.jsx**, **src/contexts/TaskMasterContext.ts**: Framework plumbing. Not novel. Skipped.
- **src/hooks/useDeviceSettings.ts**, **src/hooks/useLocalStorage.jsx**, **src/hooks/useSessionProtection.ts**, **src/hooks/useUiPreferences.ts**, **src/hooks/useVersionCheck.ts**: Small utility hooks. Not novel. Skipped.
- **src/lib/utils.js**: `cn()` Tailwind merge utility. OAP already has this. Skipped.
- **src/utils/clipboard.ts**, **src/utils/dateUtils.ts**, **src/utils/api.js**: Small utility files. Not novel. Skipped.
- **src/shared/view/ui/**: Basic UI primitives (Button, Badge, Input, ScrollArea, Tooltip, PillBar). OAP has its own component library. Skipped.
- **src/components/llm-logo-provider/**: Provider logo SVG components. Brand-specific. Skipped.
- **src/components/provider-auth/**: CLI auth terminal modal. Specific to web-based CLI auth flow. Skipped.
- **src/components/version-upgrade/**: Version upgrade modal. OAP has its own update mechanism. Skipped.
- **src/components/app/MobileNav.tsx**: Mobile navigation. OAP is desktop-only. Skipped.
- **src/types/**, **tsconfig.json**, **eslint.config.js**, **postcss.config.js**, **tailwind.config.js**, **vite.config.js**: Config files. OAP has its own. Skipped.
- **src/index.css**, **src/main.jsx**, **src/App.tsx**: App shell. Not portable. Skipped.
- **src/components/sidebar/**: Session sidebar with project/session tree. OAP has its own sidebar. Skipped.
- **src/components/main-content/**: Layout shell, tab switcher, mobile menu. OAP has its own layout. Skipped.
- **src/components/settings/**: Full settings panel. Pattern captured in other extractions; the actual UI is too web-app-specific. Skipped.
- **src/components/auth/**: Login/setup forms. Desktop app doesn't need web auth UI. Skipped.

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
