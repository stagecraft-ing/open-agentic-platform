---
id: "042-multi-provider-agent-registry"
title: "Multi-Provider Agent Registry"
feature_branch: "042-multi-provider-agent-registry"
status: draft
implementation: pending
kind: platform
created: "2026-03-29"
authors:
  - "open-agentic-platform"
language: en
summary: >
  Unified abstraction layer normalizing multiple LLM backends (Claude Code SDK,
  Anthropic API, OpenAI, Gemini, Bedrock) through a common ProviderRegistry.
  Each provider implements spawn/query/abort/stream. All output normalizes to a
  single event format, enabling backend-agnostic agent orchestration.
code_aliases:
  - PROVIDER_REGISTRY
---

# Feature Specification: Multi-Provider Agent Registry

## Deferred

This spec is intentionally deferred until the governed execution path (specs 089-099) stabilizes. Multi-provider expansion adds governance surface area — every additional backend is a new bypass surface — which conflicts with the convergence plan's focus on tightening a single governed path first.

When revisited, this spec needs rewriting to:
- Target Rust (not TypeScript) — the agent execution stack is `crates/agent/`, `crates/orchestrator/`
- Require governance coverage per provider — each adapter must integrate with `PolicyEvaluator` and `ToolCallContext`
- Align with the `ToolDef` trait (`crates/tool-registry/src/types.rs`) and the unified frontmatter schema (spec 054)

## Purpose

The platform currently couples agent execution to a single LLM backend (Claude Code CLI). Multiple consolidation sources — claude-code-by-agents (ProviderRegistry), claudecodeui, claudepal, crystal (CLI tool registry), ruflo, equilateral-agents (BYOL provider) — each implement their own provider abstraction with incompatible interfaces, event formats, and lifecycle management. There is no shared contract for spawning, querying, aborting, or streaming agent interactions across backends.

This feature introduces a single ProviderRegistry that normalizes all LLM backends behind a common interface, so that agent orchestration code never depends on a specific provider's wire format or lifecycle model.

## Scope

### In scope

- **Provider interface contract**: A `Provider` interface with four operations — `spawn`, `query`, `abort`, `stream` — that every backend must implement.
- **ProviderRegistry**: A singleton registry that manages provider registration, lookup by id, and capability discovery.
- **Normalized event format**: A single `AgentEvent` union type that all providers emit regardless of their native wire format.
- **Built-in provider implementations**: Adapters for Claude Code SDK, Anthropic API (Messages), OpenAI Chat Completions, Google Gemini, and AWS Bedrock.
- **Provider capability declaration**: Each provider advertises supported capabilities (streaming, tool use, vision, extended thinking) so the orchestrator can route appropriately.
- **Configuration**: Per-provider config (API keys, endpoints, model selection, rate limits) loaded from a unified config structure.

### Out of scope

- **Agent orchestration logic**: This feature provides the provider layer; multi-agent coordination, planning, and delegation are separate concerns.
- **Prompt routing / model selection**: Intelligent routing of prompts to the best provider is a higher-level feature built on top of the registry.
- **Cost tracking / billing**: Usage metering per provider is a follow-on concern.
- **Provider-specific advanced features**: Features unique to one backend (e.g., Anthropic batch API, OpenAI assistants API) are not exposed through the common interface; they remain accessible via provider-specific escape hatches.
- **UI changes**: No desktop or web UI changes in this feature.

## Requirements

### Functional

- **FR-001**: A `ProviderRegistry` singleton allows registering and retrieving `Provider` instances by string id (e.g., `"anthropic"`, `"openai"`, `"gemini"`, `"bedrock"`, `"claude-code-sdk"`).
- **FR-002**: Each `Provider` implements `spawn()` to create a new agent session, `query()` for single-turn request/response, `abort()` to cancel an in-flight operation, and `stream()` to receive incremental output.
- **FR-003**: All providers emit events conforming to the `AgentEvent` union type. Provider adapters translate native formats (Anthropic SSE, OpenAI SSE, Bedrock response stream) into `AgentEvent` before delivery.
- **FR-004**: `ProviderRegistry.list()` returns all registered providers with their declared capabilities.
- **FR-005**: Provider registration fails with a descriptive error if a provider with the same id is already registered (no silent overwrite).
- **FR-006**: When a provider's API key or endpoint is missing or invalid, `spawn()` and `query()` return a structured `ProviderError` rather than throwing unhandled exceptions.
- **FR-007**: `stream()` returns an `AsyncIterable<AgentEvent>` that the consumer can break out of, which triggers cleanup via `abort()`.

### Non-functional

- **NF-001**: Adding a new provider requires implementing a single interface — no changes to registry core or event types.
- **NF-002**: Event normalization adds < 5ms p99 overhead per event over raw provider SDK latency.
- **NF-003**: The registry is thread-safe (or task-safe in async contexts) for concurrent provider access.

## Architecture

### Interface definitions

```typescript
/** Identifies a registered provider. */
type ProviderId = string;

/** Capabilities a provider may advertise. */
interface ProviderCapabilities {
  streaming: boolean;
  toolUse: boolean;
  vision: boolean;
  extendedThinking: boolean;
  maxContextTokens: number;
}

/** Configuration supplied when registering a provider. */
interface ProviderConfig {
  id: ProviderId;
  apiKey?: string;
  baseUrl?: string;
  defaultModel: string;
  rateLimitRpm?: number;
  timeoutMs?: number;
  extra?: Record<string, unknown>;
}

/** A session created by spawn(). */
interface AgentSession {
  sessionId: string;
  providerId: ProviderId;
  model: string;
  createdAt: number;
}

/** Message role for normalized events. */
type Role = "user" | "assistant" | "system" | "tool";

/** Normalized event emitted by all providers. */
type AgentEvent =
  | { type: "text_delta"; delta: string }
  | { type: "text_complete"; text: string }
  | { type: "tool_use_start"; toolCallId: string; toolName: string }
  | { type: "tool_use_delta"; toolCallId: string; delta: string }
  | { type: "tool_use_complete"; toolCallId: string; input: unknown }
  | { type: "tool_result"; toolCallId: string; output: unknown; isError: boolean }
  | { type: "thinking_delta"; delta: string }
  | { type: "thinking_complete"; text: string }
  | { type: "message_start"; role: Role; model: string }
  | { type: "message_complete"; stopReason: string; usage: TokenUsage }
  | { type: "error"; code: string; message: string; retryable: boolean };

interface TokenUsage {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens?: number;
  cacheWriteTokens?: number;
}

/** Query parameters for single-turn and streaming calls. */
interface QueryParams {
  model?: string;
  messages: Array<{ role: Role; content: string | ContentBlock[] }>;
  tools?: ToolDefinition[];
  maxTokens?: number;
  temperature?: number;
  systemPrompt?: string;
  signal?: AbortSignal;
}

interface ContentBlock {
  type: "text" | "image" | "tool_use" | "tool_result";
  [key: string]: unknown;
}

interface ToolDefinition {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
}

/** The interface every provider must implement. */
interface Provider {
  readonly id: ProviderId;
  readonly capabilities: ProviderCapabilities;

  spawn(config?: Partial<ProviderConfig>): Promise<AgentSession>;
  query(session: AgentSession, params: QueryParams): Promise<AgentEvent[]>;
  stream(session: AgentSession, params: QueryParams): AsyncIterable<AgentEvent>;
  abort(session: AgentSession): Promise<void>;
}

/** Singleton registry managing all providers. */
interface ProviderRegistry {
  register(provider: Provider): void;
  get(id: ProviderId): Provider;
  has(id: ProviderId): boolean;
  list(): Array<{ id: ProviderId; capabilities: ProviderCapabilities }>;
  unregister(id: ProviderId): boolean;
}
```

### Provider adapter structure

```
packages/provider-registry/
  src/
    index.ts                    — ProviderRegistry implementation + public API
    types.ts                    — AgentEvent, Provider, ProviderConfig, etc.
    registry.ts                 — ProviderRegistry class
    adapters/
      anthropic.ts              — Anthropic Messages API adapter
      openai.ts                 — OpenAI Chat Completions adapter
      gemini.ts                 — Google Gemini adapter
      bedrock.ts                — AWS Bedrock adapter
      claude-code-sdk.ts        — Claude Code SDK (subprocess) adapter
    normalization/
      anthropic-events.ts       — Anthropic SSE -> AgentEvent mapper
      openai-events.ts          — OpenAI SSE -> AgentEvent mapper
      gemini-events.ts          — Gemini response -> AgentEvent mapper
      bedrock-events.ts         — Bedrock stream -> AgentEvent mapper
      claude-code-events.ts     — Claude Code output -> AgentEvent mapper
```

### Event normalization flow

```
Provider SDK native event
  |
  v
Adapter-specific normalizer (e.g., anthropic-events.ts)
  |
  v
AgentEvent (unified type)
  |
  v
Consumer (orchestrator, UI, logger)
```

### Registration flow

```
Application startup
  |
  +---> Load provider configs from settings / env
  |
  +---> For each configured provider:
  |       |
  |       v
  |     Instantiate adapter with ProviderConfig
  |       |
  |       v
  |     registry.register(adapter)
  |
  +---> Orchestration layer calls registry.get("anthropic").stream(...)
```

## Implementation approach

1. **Phase 1 — types and registry core**: Define `AgentEvent`, `Provider`, `ProviderRegistry` interfaces and implement the registry class with register/get/list/unregister.
2. **Phase 2 — Anthropic adapter**: Implement the Anthropic Messages API adapter and event normalizer as the reference implementation, since the platform already uses Anthropic extensively.
3. **Phase 3 — Claude Code SDK adapter**: Wrap the existing Claude Code subprocess execution in the provider interface, normalizing its output events.
4. **Phase 4 — OpenAI adapter**: Add OpenAI Chat Completions adapter with SSE normalization.
5. **Phase 5 — Gemini and Bedrock adapters**: Add remaining provider adapters.
6. **Phase 6 — integration**: Wire the registry into the existing agent execution paths so governed dispatch (Feature 035) can route through any registered provider.

## Success criteria

- **SC-001**: `ProviderRegistry` can register and retrieve at least two different provider adapters (Anthropic + OpenAI).
- **SC-002**: A streaming query through the Anthropic adapter emits well-formed `AgentEvent` objects that a consumer can iterate with `for await`.
- **SC-003**: The same consumer code works unchanged when swapped to the OpenAI adapter — only the provider id changes.
- **SC-004**: `abort()` on an in-flight streaming session terminates the stream and cleans up resources without throwing.
- **SC-005**: Registering a duplicate provider id returns a descriptive error.
- **SC-006**: A provider with a missing API key returns a `ProviderError` from `spawn()` rather than an unhandled exception.

## Dependencies

| Spec | Relationship |
|------|-------------|
| 035-agent-governed-execution | Governed dispatch routes tool calls; this feature provides the provider layer underneath |
| 036-safety-tier-governance | Safety tiers apply to tool calls dispatched through providers |
| 033-axiomregent-activation | axiomregent sidecar may mediate provider access for governance |

## Risk

- **R-001**: Provider SDK APIs change frequently (especially OpenAI and Gemini). Mitigation: adapters isolate SDK-specific code; version-pin SDKs and test against known versions.
- **R-002**: Normalizing all providers to a single event format may lose provider-specific metadata. Mitigation: `AgentEvent` includes the most commonly needed fields; provider-specific data can be attached via the `extra` field on `ProviderConfig` or through escape-hatch APIs.
- **R-003**: Claude Code SDK adapter wraps a subprocess, which has fundamentally different lifecycle semantics than HTTP API calls. Mitigation: the `spawn`/`abort` interface is designed to accommodate long-lived sessions; the adapter manages subprocess lifecycle internally.
