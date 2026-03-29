---
source: claude-code-by-agents
source_path: ~/Dev2/stagecraft-ing/claude-code-by-agents
status: extracted
---

## Summary

A full-stack multi-agent workspace ("Agentrooms") built by baryhuang, providing a web UI and backend for orchestrating multiple Claude Code instances. Features a React/Vite frontend, Hono backend (running on both Deno and Node.js via a runtime abstraction), multi-provider support (Claude Code SDK, Anthropic API, OpenAI), agent routing via @mentions, orchestration with file-based inter-agent communication, Claude OAuth integration, conversation history management (reading `.claude/projects/` JSONL files), and cross-platform desktop apps via Electron and native SwiftUI (macOS). Also includes AWS Lambda deployment support and Swagger API docs.

## Extractions

### [Architecture Patterns]: Multi-Provider Agent Registry
- **What**: A `ProviderRegistry` class that abstracts over multiple LLM providers (Claude Code SDK, Anthropic API direct, OpenAI) with a common `AgentProvider` interface. Each agent is registered with a provider ID, and the registry routes requests to the correct provider. The `AgentProvider` interface defines `executeChat()` returning `AsyncGenerator<ProviderResponse>` with types for text, image, tool_use, error, and done.
- **Where in source**: `backend/providers/registry.ts`, `backend/providers/types.ts`, `backend/providers/anthropic.ts`, `backend/providers/claude-code.ts`, `backend/providers/openai.ts`
- **Integration target in OAP**: `packages/agents/providers/` -- adapt as OAP's provider abstraction layer for multi-LLM agent execution
- **Action**: outline-spec
- **Priority**: P0

### [Architecture Patterns]: Runtime Abstraction Layer (Deno/Node)
- **What**: A clean `Runtime` interface abstracting file I/O, env access, process execution, platform detection, HTTP serving, and static file middleware. Two implementations: `NodeRuntime` and `DenoRuntime`. Allows the same Hono application to run on either runtime without changes.
- **Where in source**: `backend/runtime/types.ts`, `backend/runtime/node.ts`, `backend/runtime/deno.ts`
- **Integration target in OAP**: Reference pattern for cross-runtime backend design if OAP ever needs Deno support
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Multi-Agent Orchestration via File-Based Communication
- **What**: Orchestrator agent creates execution plans with steps, each assigned to a specific agent. Agents write results to temp files (e.g., `/tmp/step1_results.txt`), and subsequent agents read those files as input. Dependencies between steps are tracked. The orchestrator uses Anthropic API tool_use with a structured `orchestrate_execution` tool schema.
- **Where in source**: `backend/handlers/chat.ts` (executeOrchestratorWorkflow), `backend/handlers/multiAgentChat.ts`
- **Integration target in OAP**: `packages/agents/orchestration/` -- adapt as spec-driven multi-agent orchestration with artifact-based communication
- **Action**: outline-spec
- **Priority**: P0

### [Directly Portable Code]: NDJSON Streaming Response Pattern
- **What**: Complete streaming pattern using ReadableStream with NDJSON encoding, connection acknowledgment messages, flush markers to prevent proxy buffering, timeout handling on individual reads, and proper CORS headers for streaming. Headers include `X-Accel-Buffering: no` for Nginx and `X-Proxy-Buffering: no` for other proxies.
- **Where in source**: `backend/handlers/chat.ts` (handleChatRequest response construction)
- **Integration target in OAP**: `apps/desktop/` or future web API -- use as reference for streaming API responses
- **Action**: integrate-now
- **Priority**: P1

### [Directly Portable Code]: Claude Code SDK Integration Pattern
- **What**: Working integration with `@anthropic-ai/claude-code` SDK's `query()` function, including session resumption (`resume: sessionId`), permission bypass mode, working directory setting, custom executable paths, AbortController support, and OAuth token environment management. Shows how to handle the SDK's streaming message types (system, assistant, user, result).
- **Where in source**: `backend/handlers/chat.ts` (executeClaudeCommand), `backend/providers/claude-code.ts`
- **Integration target in OAP**: `packages/agents/claude-code-bridge/` -- direct reference for OAP's Claude Code SDK integration
- **Action**: integrate-now
- **Priority**: P0

### [Directly Portable Code]: Claude OAuth Authentication Flow
- **What**: Complete OAuth flow for Claude subscriptions: credential file writing (`~/.claude/credentials.json`), preload script injection for Electron, token refresh, and environment variable management. Allows using Claude subscription auth instead of API keys.
- **Where in source**: `backend/auth/claude-auth-utils.ts`, `auth/claude-oauth-service.ts`, `backend/auth/preload-script.cjs`, `frontend/src/hooks/useClaudeAuth.ts`
- **Integration target in OAP**: `packages/auth/` -- reference for OAP's authentication layer if subscription-based auth is needed
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components/Features]: Agent Hub UI with Chat Streaming
- **What**: React/TypeScript UI with: agent selector sidebar, per-agent chat sessions, streaming message display with tool use visualization, orchestration step visualization (ExecutionPlan component), permission dialogs, dark/light theme toggle, conversation history browser, settings modal for agent configuration (name, description, working directory, API endpoint), @mention parsing, and enter behavior configuration.
- **Where in source**: `frontend/src/components/` (entire tree), `frontend/src/hooks/` (streaming/, chat/)
- **Integration target in OAP**: `apps/desktop/` -- borrow UI patterns for OAP's Tauri desktop agent interaction views
- **Action**: outline-spec
- **Priority**: P1

### [UI Components/Features]: Stream Parser and Message Type System
- **What**: Modular stream parsing architecture with `useStreamParser`, `useToolHandling`, and `useMessageProcessor` hooks. Defines a rich message type system (ChatMessage, SystemMessage, ToolMessage, ToolResultMessage, OrchestrationMessage) with type guards. Handles SDK message conversion, permission error detection, and tool use caching for result correlation.
- **Where in source**: `frontend/src/hooks/streaming/`, `frontend/src/types.ts`, `frontend/src/utils/messageTypes.ts`, `frontend/src/utils/toolUtils.ts`
- **Integration target in OAP**: `apps/desktop/src/` -- adapt message type system for OAP's desktop app
- **Action**: outline-spec
- **Priority**: P1

### [Directly Portable Code]: Conversation History Loader
- **What**: Utilities for reading Claude Code's `.claude/projects/` JSONL history files: path encoding (project path to directory name), conversation file parsing, timestamp restoration for continued sessions, message deduplication via ID-set subsetting, and conversation grouping. Handles edge cases like symlinks, dangerous characters, and empty files.
- **Where in source**: `backend/history/` (conversationLoader.ts, parser.ts, grouping.ts, timestampRestore.ts, pathUtils.ts)
- **Integration target in OAP**: `packages/history/` or `tools/gitctx-mcp/` -- could enhance gitctx-mcp with conversation history awareness
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components/Features]: Native macOS SwiftUI App
- **What**: Complete macOS native app in SwiftUI with: agent hub view, chat view with streaming, settings view, API service with URLSession streaming, history service, agent/project models. Uses Combine for reactive state management. Separate Xcode project targeting macOS.
- **Where in source**: `ClaudeAgentHub/` (entire tree)
- **Integration target in OAP**: Reference only -- OAP uses Tauri, not native SwiftUI
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI/Packaging]: Electron Multi-Platform Build
- **What**: Electron-based desktop app build with electron-builder targeting macOS (universal), Windows (x64), and Linux (x64/ARM64). CI workflow builds all platforms, records demo videos with Playwright, and creates GitHub releases with artifacts.
- **Where in source**: `package.json`, `.github/workflows/release.yml`, `scripts/build-windows.sh`
- **Integration target in OAP**: Reference for OAP's Tauri release workflow patterns
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Agent @Mention Routing
- **What**: Message parsing that detects `@agent-name` mentions. Single mentions route directly to the agent's HTTP endpoint. Multiple mentions trigger orchestrator mode. Clean separation between direct execution (no overhead) and coordinated execution.
- **Where in source**: `frontend/src/config/agents.ts` (parseAgentMention), `backend/handlers/chat.ts` (shouldUseOrchestrator)
- **Integration target in OAP**: `apps/desktop/` -- adopt @mention routing for OAP's agent interaction model
- **Action**: outline-spec
- **Priority**: P1

### [MCP/Tool Integrations]: Swagger/OpenAPI Documentation
- **What**: Full OpenAPI 3.0 specification for all API endpoints (health, projects, histories, conversations, chat, multi-agent-chat, abort, agent-projects, agent-histories, agent-conversations) with Swagger UI served at `/api-docs`.
- **Where in source**: `backend/swagger/config.ts`, `backend/app.ts` (swagger annotations)
- **Integration target in OAP**: Reference pattern for API documentation if OAP exposes REST APIs
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Image Handling and Screenshot Capture
- **What**: `ImageHandler` class managing screenshot capture (platform-specific shell commands), base64 conversion, temp file management with cleanup, and MIME type detection. Integrated into the multi-agent flow for visual analysis (e.g., UX agent analyzes screenshots from implementation agent).
- **Where in source**: `backend/utils/imageHandling.ts`, `backend/handlers/multiAgentChat.ts` (handleScreenCapture)
- **Integration target in OAP**: `packages/agents/visual/` -- pattern for visual feedback loops in agent workflows
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI/Packaging]: Makefile-Based Quality Gates
- **What**: Makefile with `check`, `format`, `test`, `lint`, `build-backend`, `build-frontend` targets. Lefthook pre-commit hooks for format/lint/typecheck. Vitest for testing.
- **Where in source**: `Makefile`, `.lefthook.yml`, `backend/vitest.config.ts`
- **Integration target in OAP**: Reference for CI patterns
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

- `.env` / `.env.example` -- contains placeholder API keys, OAP has its own env pattern
- `openmemory.sqlite` / `frontend/openmemory.sqlite` / `backend/openmemory.sqlite` -- SQLite databases, likely test artifacts
- `database.js` -- root-level database file, appears unused/orphaned
- `ClaudeAgentHub.xcodeproj/` -- Xcode project config, not applicable to Tauri
- `ClaudeAgentHub/Assets.xcassets/` -- iOS/macOS asset catalogs
- `backend/lambda.ts` / `backend/template.yml` / `backend/samconfig.toml` -- AWS SAM deployment config; OAP doesn't target Lambda
- `backend/deno.json` / `backend/deno.lock` -- Deno-specific config files
- `frontend/playwright.config.ts` -- Playwright config for demo recording
- `frontend/src/components/chat/ThemeToggle.tsx` -- simple dark/light toggle, trivial to implement
- `frontend/src/components/auth/AuthButton.tsx` -- Electron-specific auth button
- `frontend/src/contexts/EnterBehaviorContext.tsx` -- enter key behavior preference, trivial UX choice
- `scripts/build-windows.sh` -- Windows-specific electron build script
- `.tagpr` -- tag-based release automation config
- `CHANGELOG.md` -- project changelog, historical only
- `LICENSE` -- MIT license file

## Safe-to-delete confirmation
- [x] All valuable content extracted or documented above
