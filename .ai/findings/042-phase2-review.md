# 042 Phase 2 review — Anthropic adapter + normalizer

> Reviewed: 2026-03-29 by **claude**. Assesses cursor's Phase 2 against spec 042.

## What was delivered

- `adapters/anthropic.ts` — `createAnthropicProvider()` factory returning `AnthropicMessagesProvider` implementing full `Provider` interface
- `normalization/anthropic-events.ts` — `AnthropicStreamNormalizer` (stateful, one-per-request) and `messageToAgentEvents()` (non-streaming)
- `anthropic-events.test.ts` — end-to-end stream normalization test (message_start → text_delta → message_complete)
- `index.ts` updated with exports

## Spec parity

| Requirement | Status | Evidence |
|------------|--------|----------|
| FR-002 spawn/query/abort/stream | ✅ | All 4 ops implemented in `AnthropicMessagesProvider` |
| FR-003 AgentEvent normalization | ✅ | `AnthropicStreamNormalizer.push()` maps all Anthropic SSE types to `AgentEvent` |
| FR-006 ProviderError on missing key | ✅ | `requireKey()` throws `ProviderError("missing_api_key")` |
| FR-007 stream returns AsyncIterable | ✅ | `stream()` is `async *` generator with cleanup in `finally` |
| SC-002 streaming emits well-formed AgentEvents | ✅ | Test confirms message_start → text_delta → message_complete |
| SC-004 abort terminates stream | ✅ | `inflight` Map + `AbortController`, `abort()` calls `.abort()` |
| SC-006 missing key → ProviderError | ✅ | `requireKey()` at top of spawn/query/stream |

## Quality observations

### Positive

- **`mergeAbortSignals`** (line 141-154): Clean helper that combines the provider's internal AC with the caller's `params.signal`. Correctly handles already-aborted signals.
- **Stateful normalizer**: `toolInputByIndex` / `toolMetaByIndex` maps correctly accumulate JSON deltas and emit `tool_use_complete` with parsed input on `content_block_stop`. Edge case: empty input buffer → `{}` (line 82). Good.
- **`thinking_delta` / `thinking_complete`** support: Both streaming and non-streaming paths handle `thinking` content blocks, matching the `extendedThinking: true` capability.
- **Cache token mapping**: `cache_read_input_tokens` and `cache_creation_input_tokens` correctly mapped to `cacheReadTokens` / `cacheWriteTokens`.
- **Error wrapping**: `toProviderError()` preserves `ProviderError` instances, wraps generic `Error` as retryable, wraps unknown as non-retryable.

### Findings

#### P2-001: `message_delta` input_tokens may be 0 (LOW)

`anthropic-events.ts:94`: `u.input_tokens ?? 0` — the `message_delta` usage object typically only has `output_tokens` (input tokens come from `message_start`). Setting `inputTokens: 0` in `lastUsage` means `message_complete` will report 0 input tokens. The `message_start` event's input tokens are not captured.

**Impact:** `message_complete.usage.inputTokens` will be 0 for streaming. Non-streaming `messageToAgentEvents` correctly gets input tokens from `message.usage`.

**Fix:** Capture input tokens from `message_start.message.usage` and merge with `message_delta.usage` before emitting `message_complete`.

#### P2-002: `content_block_stop` for text blocks doesn't emit `text_complete` (LOW)

When a text block ends (`content_block_stop`), the normalizer only emits `tool_use_complete` for tool blocks (line 73). There's no `text_complete` event emitted. Consumers relying on `text_complete` for streaming won't receive it.

**Impact:** Consumers that need the full assembled text from streaming must concatenate `text_delta` events themselves. The non-streaming path does emit `text_complete` via `contentBlockToEvents`.

**Fix:** Track accumulated text per index (like tools) and emit `text_complete` on `content_block_stop` for text blocks.

#### P2-003: `toMessageParams` system message handling (FINE)

`anthropic.ts:168`: System-role messages are silently skipped in `toMessageParams` — correct because Anthropic takes `system` as a separate param, which is wired via `params.systemPrompt`. If a caller passes system messages in the `messages` array AND sets `systemPrompt`, the array ones are dropped. Acceptable — documented by the `continue` statement.

#### P2-004: No `text_complete` for non-streaming thinking blocks (COSMETIC)

`contentBlockToEvents` line 168 emits `thinking_complete` for thinking blocks but doesn't emit `thinking_delta` first. This is consistent with non-streaming semantics (complete block, no delta). Fine.

## Test coverage

- Stream normalization: ✅ (1 test, covers text path end-to-end)
- Non-streaming `messageToAgentEvents`: not directly tested (but simple mapping, low risk)
- Tool use stream: not tested (normalizer handles it, but no test exercises `tool_use_start` → `tool_use_delta` → `tool_use_complete`)
- Missing API key: not tested (but pattern established in Phase 1 mock)

## Summary

**Phase 2 is solid.** The adapter correctly implements all 4 `Provider` operations, wires `@anthropic-ai/sdk` streaming and non-streaming paths, and normalizes to `AgentEvent`. Two findings worth addressing: P2-001 (input tokens lost in streaming) and P2-002 (no `text_complete` in streaming). Both are LOW — the adapter works correctly for the primary text streaming use case.

**Phase 3 recommendation:** Claude Code SDK adapter (`adapters/claude-code-sdk.ts` + `normalization/claude-code-events.ts`). This adapter wraps the existing `@opc/claude-code-bridge` package — `BridgeEvent` → `AgentEvent` mapping. Reuses `queryClaudeCode()` from the bridge.
