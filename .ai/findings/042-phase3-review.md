# 042 Phase 3 review — Claude Code SDK adapter + normalizer

> **Recorded:** 2026-03-29 — spec-backed verification against `specs/042-multi-provider-agent-registry/spec.md` (Phase 3: Claude Code SDK adapter).

## What was delivered

- `packages/provider-registry/src/adapters/claude-code-sdk.ts` — `createClaudeCodeSdkProvider()` / `ClaudeCodeSdkProvider`: `spawn`, `query`, `stream`, `abort`; wraps `queryClaudeCode()` from `@opc/claude-code-bridge`.
- `packages/provider-registry/src/normalization/claude-code-events.ts` — `ClaudeCodeBridgeNormalizer`, `bridgeEventToAgentEvents()` — `BridgeEvent` → `AgentEvent`.
- `packages/provider-registry/src/normalization/claude-code-events.test.ts` — normalizer coverage (assistant text, tool use, permission-request).
- `packages/provider-registry/src/ambient-claude-code-sdk.d.ts` — optional SDK module shim for `tsc` when workspace-linked bridge sources pull in `sdk-adapter.ts`.
- `index.ts` / `package.json` — exports and `workspace:*` dependency on `@opc/claude-code-bridge`.

## Spec parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| FR-002 `spawn` / `query` / `abort` / `stream` | ✅ | All four implemented; `query` and `stream` both drive `queryClaudeCode()` with shared `toBridgeOptions()`. |
| FR-003 `AgentEvent` normalization | ✅ | `mapBridgeEvent` maps assistant text, tool blocks, `session-complete`, bridge `error`, `permission-request` → `AgentEvent`. |
| FR-006 structured `ProviderError` (missing/invalid config) | ⚠️ | No explicit API-key gate on `spawn`/`query`/`stream` (unlike Anthropic). Invalid/missing **cwd** throws `ProviderError` (`missing_cwd`) when `process.cwd` absent. Runtime failures from the bridge/SDK are wrapped via `toProviderError()`. |
| FR-007 `stream()` `AsyncIterable` + cleanup on break | ✅ | `async *stream`; `inflight` + `AbortController`; `forwardAbort` links `params.signal`; `finally` clears `inflight`; `abort()` aborts controller. |
| NF-002 normalization overhead | ✅ | Per-event mapping is O(1) synchronous work on already-materialized bridge events. |

## Success criteria (partial — full SC set is multi-phase)

| ID | Status | Notes |
|----|--------|--------|
| SC-002 (streaming → well-formed `AgentEvent`) | ✅ | Normalizer tests + same pipeline as `stream()`. |
| SC-004 (`abort` terminates stream) | ✅ | `inflight` map + `abort()` → `AbortController.abort()`. |
| SC-006 (missing API key → `ProviderError` from `spawn`) | ⚠️ | Not modeled for Claude Code: OAuth/CLI paths may not require `ProviderConfig.apiKey`. **By design gap vs Anthropic** unless product requires an explicit “no credentials” check at spawn. |

## Quality observations

### Positive

- **Resume wiring**: `bridgeSessionIds` updated from bridge `start` before `sessionId` is passed on subsequent `toBridgeOptions()` calls in the same process.
- **Extra passthrough**: `canUseTool`, `permissionMode`, `allowedTools`, `disallowedTools`, `oauthToken` forwarded via `ProviderConfig.extra`, matching `BridgeQueryOptions`.
- **`queryParamsToPrompt`**: Skips `system` role lines; uses `systemPrompt` on the bridge options separately — consistent with Anthropic adapter’s split of system vs messages.
- **Permission UX**: `permission-request` → retriable `error` with `permission_request` code so orchestrators can distinguish from fatal bridge errors.

### Findings

#### P3-001: FR-006 / SC-006 vs optional credentials (LOW / product)

Claude Code may run with OAuth or environment-based auth. The adapter does not throw `ProviderError` from `spawn()` when `apiKey` is absent. If governance requires parity with “missing key” for every provider, add an explicit policy check or document Claude Code as exempt in spec 042.

#### P3-002: No `tool_use_delta` for Claude Code path (INFO)

The bridge delivers complete `tool_use` blocks. The normalizer emits `tool_use_start` + `tool_use_complete` only. Consumers expecting incremental `tool_use_delta` for this provider will not see them — consistent with non-SSE completion semantics.

#### P3-003: `query()` and `stream()` are behaviorally similar (INFO)

Both consume the same async generator from `queryClaudeCode()`. “Streaming” here means async iteration over bridge events, not sub-message token deltas from the Claude Code SDK. Capability `streaming: true` remains accurate for `AsyncIterable<AgentEvent>`.

## Tests

- `claude-code-events.test.ts`: 3 tests (system+assistant+session-complete, tool_use, permission-request).
- Full `pnpm test` in `@opc/provider-registry`: 10/10 passing (including Phase 1 registry + Phase 2 anthropic tests).

## Summary

**Phase 3 meets the core 042 contract** for the Claude Code SDK path: full `Provider` surface, `BridgeEvent` → `AgentEvent` mapping, abort/cleanup, and session resume. Minor gaps are **credential parity (FR-006 / SC-006)** vs key-based providers and **incremental tool deltas** (not applicable to current bridge shape).

**Recommended next slice:** Phase 4 — OpenAI Chat Completions adapter + SSE normalization (`adapters/openai.ts`, `normalization/openai-events.ts`) per spec implementation approach.

**Baton → cursor** for Phase 4 implementation; optional **claude** pass after Phase 4 landing.
