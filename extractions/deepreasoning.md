---
source: deepreasoning
source_path: ~/Dev2/stagecraft-ing/deepreasoning
status: extracted
---

## Summary

DeepReasoning is a Rust+Next.js application that chains DeepSeek R1's Chain-of-Thought reasoning with Anthropic Claude's response generation into a single unified API and chat interface. The Rust backend (axum) proxies requests to both providers sequentially -- first obtaining R1's reasoning trace, then feeding that as `<thinking>` context to Claude for the final answer. It supports both streaming (SSE) and non-streaming modes, BYOK (Bring Your Own Keys) via request headers, per-request token usage tracking with cost calculation, and a verbose mode that surfaces raw upstream responses. The Next.js frontend is a full chat UI with sidebar chat history, collapsible thinking traces, markdown rendering with syntax highlighting, localStorage persistence, PostHog analytics, and a settings panel for API tokens and per-provider config overrides.

## Extractions

### Architecture Patterns: Multi-model chaining with reasoning injection

- **What**: The core pattern of calling Model A for reasoning, wrapping output in `<thinking>` tags, injecting it as an assistant message, then calling Model B for the final response. This is a generic "reasoning + generation" pipeline applicable to any two-model orchestration.
- **Where in source**: `src/handlers.rs` (both `chat` and `chat_stream` functions)
- **Integration target in OAP**: `crates/agent` or a new `crates/model-chain` crate -- the pattern could be generalized into a spec-governed model-chaining primitive where the spec defines which models play which roles.
- **Action**: capture-as-idea
- **Priority**: P1

### Architecture Patterns: SSE streaming with multi-phase content

- **What**: The streaming handler emits SSE events across two sequential model calls in a single response stream. It sends `start` -> thinking content deltas -> thinking close -> Claude content deltas -> `usage` -> `done`. The client-side parses these phases to render thinking vs. response separately. Uses `tokio::sync::mpsc` channel piped through `ReceiverStream` for backpressure.
- **Where in source**: `src/handlers.rs:chat_stream`, `frontend/components/chat.tsx:handleSubmit`
- **Integration target in OAP**: `apps/desktop` (Tauri app) for any future chat/agent streaming UI; `crates/agent` for streaming orchestration patterns.
- **Action**: capture-as-idea
- **Priority**: P1

### Architecture Patterns: BYOK (Bring Your Own Keys) via request headers

- **What**: API tokens are passed per-request via `X-DeepSeek-API-Token` and `X-Anthropic-API-Token` headers rather than stored server-side. The server never persists credentials. This is a clean zero-trust pattern for proxy services.
- **Where in source**: `src/handlers.rs:extract_api_tokens`
- **Integration target in OAP**: Any future MCP proxy or gateway service that mediates between the desktop app and external LLM APIs.
- **Action**: capture-as-idea
- **Priority**: P2

### Directly Portable Code: Rust Anthropic API client with streaming

- **What**: A complete, well-documented Rust client for the Anthropic Messages API supporting both streaming (SSE event parsing) and non-streaming modes. Handles `message_start`, `content_block_delta`, `message_delta`, `message_stop`, and `ping` event types. Includes proper request building with protected fields, custom header merging, and model-aware max_tokens defaults.
- **Where in source**: `src/clients/anthropic.rs` (~350 lines)
- **Integration target in OAP**: `crates/agent` or `crates/asterisk` -- if OAP needs a native Rust Anthropic client rather than going through MCP, this is a solid starting point. However, OAP likely uses the Claude Agent SDK, so this may be redundant.
- **Action**: outline-spec
- **Priority**: P2

### Directly Portable Code: Rust DeepSeek API client with streaming

- **What**: A complete Rust client for DeepSeek's chat completion API, handling the `reasoning_content` field specific to DeepSeek R1. Includes SSE stream parsing, custom config merging, and proper error propagation.
- **Where in source**: `src/clients/deepseek.rs` (~300 lines)
- **Integration target in OAP**: `crates/agent` -- if OAP ever integrates DeepSeek or other reasoning models, this client + its `reasoning_content` handling pattern would be directly useful.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Patterns: Token usage tracking and cost calculation

- **What**: Per-request token usage aggregation across multiple providers with configurable per-model pricing (input/output/cache-read/cache-write per million tokens). Calculates combined cost and returns it in the API response. Pricing is TOML-configured.
- **Where in source**: `src/handlers.rs` (calculate_deepseek_cost, calculate_anthropic_cost), `src/config.rs` (PricingConfig), `config.toml`
- **Integration target in OAP**: `crates/agent` or `apps/desktop` -- usage/cost tracking is valuable for any agent orchestration system, especially one that chains multiple model calls.
- **Action**: outline-spec
- **Priority**: P1

### Directly Portable Code: Axum error handling pattern with typed API errors

- **What**: A clean `ApiError` enum using `thiserror` with `IntoResponse` implementation that maps each variant to appropriate HTTP status codes and consistent JSON error response format (`{error: {message, type, param, code}}`). Includes SSE type aliases.
- **Where in source**: `src/error.rs` (~170 lines)
- **Integration target in OAP**: Any Rust axum service in OAP (e.g., spec-compiler if it gains an HTTP mode, or future API services). The pattern is standard but well-executed.
- **Action**: capture-as-idea
- **Priority**: P2

### UI Components/Features: Collapsible thinking trace in chat UI

- **What**: Chat messages from the assistant can include a `thinking` field. The UI renders this as a collapsible section with a timer showing "Thought for X seconds/minutes". During streaming, the thinking section shows a spinner and elapsed time. After completion, it shows final duration. Uses Radix Collapsible primitive.
- **Where in source**: `frontend/components/chat.tsx` (MessageContent component, thinkingStartTime/elapsedTime state)
- **Integration target in OAP**: `apps/desktop` -- if the desktop app ever surfaces agent reasoning traces (which is likely for a governed AI system), this UI pattern is directly applicable.
- **Action**: outline-spec
- **Priority**: P1

### UI Components/Features: Virtualized chat message list

- **What**: Uses `@tanstack/react-virtual` for efficient rendering of long chat histories. Includes RAF-based auto-scroll, scroll-position-aware auto-scroll toggling, dynamic row sizing, and optimized re-render prevention for streaming content updates.
- **Where in source**: `frontend/components/chat.tsx` (rowVirtualizer, handleScroll, scrollToBottom)
- **Integration target in OAP**: `apps/desktop` or `packages/ui` -- virtualized lists are essential for any chat UI with long conversations.
- **Action**: capture-as-idea
- **Priority**: P2

### UI Components/Features: Markdown rendering with syntax highlighting

- **What**: Full markdown rendering pipeline: `react-markdown` + `remark-gfm` + `rehype-raw` + `rehype-sanitize` + `react-syntax-highlighter` (Prism/oneDark). Memoized code block renderer that distinguishes inline code from code blocks. Includes copy-to-clipboard button on code blocks.
- **Where in source**: `frontend/components/chat.tsx` (renderers useMemo), `frontend/components/ui/copy-button.tsx`, `frontend/app/globals.css` (prose styles)
- **Integration target in OAP**: `packages/ui` or `apps/desktop` -- markdown rendering with code highlighting is fundamental for any AI-facing UI.
- **Action**: capture-as-idea
- **Priority**: P2

### UI Components/Features: Chat history sidebar with localStorage persistence

- **What**: Sidebar with stored chat list, new chat creation, chat deletion with confirmation dialog, clear-all with confirmation dialog. All persisted to localStorage. Auto-generates chat titles from first message.
- **Where in source**: `frontend/components/chat.tsx` (chats state, createNewChat, deleteChat, clearAllChats, sidebar JSX)
- **Integration target in OAP**: `apps/desktop` -- basic chat persistence pattern, though OAP's Tauri app would likely use a proper database.
- **Action**: capture-as-idea
- **Priority**: P2

### UI Components/Features: Markdown editor with code block autocompletion

- **What**: A dialog-based markdown editor component that auto-completes backtick pairs and triple-backtick code blocks with proper cursor positioning. Supports shift+enter to submit.
- **Where in source**: `frontend/components/ui/markdown-editor.tsx`
- **Integration target in OAP**: `packages/ui` -- useful for any prompt composition UI.
- **Action**: capture-as-idea
- **Priority**: P2

### UI Components/Features: Settings panel with per-provider config

- **What**: Sheet-based settings panel with auto-save (debounced to localStorage), reset-to-defaults, and key-value pair editors for custom headers and body params per provider. Uses react-hook-form with watch-based auto-save.
- **Where in source**: `frontend/components/settings.tsx`
- **Integration target in OAP**: `apps/desktop` -- the pattern of per-provider configuration with custom headers/body overrides could apply to MCP server configuration in the desktop app.
- **Action**: capture-as-idea
- **Priority**: P2

### Spec/Governance Ideas: Model pricing as configuration

- **What**: Per-model pricing (input/output/cache per million tokens) defined in TOML config with Rust structs. Allows operators to update pricing without code changes. Includes cache-hit vs. cache-miss differentiation for DeepSeek.
- **Where in source**: `config.toml`, `src/config.rs`
- **Integration target in OAP**: Could inform a "cost governance" spec feature in `specs/` -- defining acceptable cost bounds per agent action, with pricing tables as configuration.
- **Action**: capture-as-idea
- **Priority**: P2

### Build/CI/Packaging: Multi-stage Docker build for Rust

- **What**: Two-stage Dockerfile: `rust:latest` builder with OpenSSL dev libs, then `debian:bookworm-slim` runtime with just `libssl3` and `ca-certificates`. Env vars for host/port override. Docker Compose with loopback-only port binding and named network.
- **Where in source**: `Dockerfile`, `docker-compose.yml`
- **Integration target in OAP**: Reference pattern for any future containerized OAP services. OAP already has CI workflows but may benefit from the slim runtime stage pattern.
- **Action**: capture-as-idea
- **Priority**: P2

### MCP/Tool Integrations: PostHog analytics integration pattern

- **What**: A PostHog provider with development-mode mock (logs to console instead of sending events), production-only initialization, and a `usePostHog` hook that returns a mock in dev. Feature flags hook (`useFeatureFlags`) with typed flag definitions.
- **Where in source**: `frontend/providers/posthog.tsx`, `frontend/hooks/use-feature-flags.ts`
- **Integration target in OAP**: `apps/desktop` -- if the desktop app needs analytics or feature flags, this is a clean pattern. The dev-mode mock pattern is especially useful.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Patterns: Request validation with system prompt deduplication

- **What**: The API accepts system prompts either as a top-level `system` field or as a system-role message in the `messages` array, but not both. Validation logic prevents duplication and normalizes the prompt position.
- **Where in source**: `src/models/request.rs` (validate_system_prompt, get_messages_with_system, get_system_prompt)
- **Integration target in OAP**: `crates/agent` -- system prompt handling is a common concern in agent frameworks.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Patterns: Per-request provider config overrides

- **What**: Each API request can include `deepseek_config` and `anthropic_config` with custom headers and body params that get merged into the upstream request. Protected fields (stream, messages, system) cannot be overridden. This allows callers to tune temperature, model, max_tokens etc. per-request.
- **Where in source**: `src/models/request.rs` (ApiConfig), `src/clients/deepseek.rs:build_request`, `src/clients/anthropic.rs:build_request`
- **Integration target in OAP**: `crates/agent` -- the pattern of allowing per-request config overrides while protecting critical fields is useful for any multi-model orchestration system.
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

- **frontend/components/ui/\*.tsx** (button, card, dialog, sheet, select, toast, toaster, collapsible, label, form, textarea, input): Standard shadcn/ui components generated via CLI. No custom logic worth extracting -- OAP's `packages/ui` would use its own shadcn installation.
- **frontend/app/page.tsx**: Marketing landing page with hero section, feature cards, FAQ. Asterisk-branded content with no reusable patterns beyond what's already common.
- **frontend/app/docs/page.tsx**: API documentation page with hardcoded cURL examples and response samples. Content is deepreasoning-specific.
- **frontend/app/globals.css**: Dark theme CSS variables, custom scrollbar styles, background patterns. Standard Tailwind dark mode setup -- nothing novel.
- **frontend/tailwind.config.ts, tsconfig.json, eslint.config.mjs, postcss.config.mjs, components.json, next.config.ts**: Standard Next.js/Tailwind/shadcn configuration. No value for OAP.
- **frontend/lib/utils.ts**: Standard `cn()` utility (clsx + tailwind-merge). Already exists in any shadcn project.
- **Cargo.lock, package-lock.json, bun.lockb**: Lock files, no extraction value.
- **CONTRIBUTING.md**: Standard contribution guidelines, nothing OAP-specific.
- **LICENSE.md**: MIT license from Asterisk. OAP has its own license.
- **.gitignore, .gitattributes**: Standard Rust/Node ignore patterns.
- **README.md**: Project documentation for deepreasoning. Content is project-specific.
- **frontend/public/\***: Brand assets (logos, icons, benchmark screenshots). Not applicable to OAP.

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
