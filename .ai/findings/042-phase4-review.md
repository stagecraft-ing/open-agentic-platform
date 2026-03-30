# 042 Phase 4 review — OpenAI Chat Completions adapter + normalizer

> Reviewed: 2026-03-29 by **claude**. Assesses cursor's Phase 4 against spec 042.

## What was delivered

- `adapters/openai.ts` — `createOpenAIProvider()` factory, `OpenAIChatProvider` implementing full `Provider` interface
- `normalization/openai-events.ts` — `OpenAIStreamNormalizer` (stateful) and `completionToAgentEvents()` (non-streaming)
- `openai-events.test.ts` — 3 tests (streaming text, streaming single-chunk, non-streaming completion)
- `index.ts` updated with exports

## Spec parity

| Requirement | Status | Evidence |
|------------|--------|----------|
| FR-002 spawn/query/abort/stream | ✅ | All 4 ops implemented |
| FR-003 AgentEvent normalization (OpenAI SSE) | ✅ | `OpenAIStreamNormalizer` maps chunks → `AgentEvent` |
| FR-006 ProviderError on missing key | ✅ | `requireKey()` throws `ProviderError("missing_api_key")` |
| FR-007 stream AsyncIterable + cleanup | ✅ | `async *stream`, `inflight` + `AbortController`, `finally` cleanup |
| SC-001 two adapters registered | ✅ | Anthropic + OpenAI now available |
| SC-003 same consumer code works | ✅ | Both adapters produce identical `AgentEvent` types |
| SC-004 abort terminates stream | ✅ | `inflight` map + `abort()` |
| SC-006 missing key → ProviderError | ✅ | `requireKey()` at top of spawn/query/stream |

## Quality observations

### Positive

- **`stream_options: { include_usage: true }`** (line 119): Ensures the terminal chunk includes usage data for `message_complete`. Smart — without this, streaming wouldn't get token counts.
- **Tool call accumulation**: `toolAcc` map correctly handles OpenAI's split delta pattern (id on first chunk, name on first chunk, arguments across multiple chunks). `finalizeToolCalls()` sorts by index and parses accumulated JSON.
- **Structural parity with Anthropic adapter**: `mergeAbortSignals`, `toProviderError`, `requireKey`, `inflight` map — all follow the same pattern. Good for maintainability.
- **`completionToAgentEvents` handles empty choices**: Returns an `error` AgentEvent with `empty_completion` code. Correct.
- **Non-string content fallback**: `toOpenAIMessages` line 188 — `JSON.stringify(m.content)` for array content blocks. Pragmatic since OpenAI's content array format differs from the spec's `ContentBlock[]`.

### Findings

#### P4-001: `extendedThinking: false` is correct (POSITIVE)

OpenAI doesn't have extended thinking. Capability declaration is accurate.

#### P4-002: `toOpenAIMessages` stringifies content blocks (LOW)

Line 188: Array content blocks are `JSON.stringify`'d into a string. This works for text-only but loses structure for image/multimodal content blocks. For vision support (`vision: true` in caps), OpenAI expects `content` as an array of `{ type: "text" | "image_url", ... }` objects, not a JSON string.

**Impact:** Vision queries would send stringified JSON as the message content instead of structured image blocks. Not blocking — vision isn't exercised yet.

**Fix:** Map `ContentBlock[]` to OpenAI's content part format when the array contains image blocks.

#### P4-003: `mergeAbortSignals` duplicated from Anthropic adapter (COSMETIC)

Identical function in both `adapters/anthropic.ts` and `adapters/openai.ts`. Could extract to a shared utility. Not blocking.

## Test coverage

- Streaming text with separate usage chunk: ✅
- Streaming single-chunk (content + finish + usage): ✅
- Non-streaming completion with text: ✅
- Tool call streaming: not tested (but normalizer handles it)
- Empty choices: not tested (but `completionToAgentEvents` handles it)

## Summary

**Phase 4 is clean and spec-faithful.** The adapter mirrors the Anthropic pattern exactly, which makes the codebase consistent and validates SC-003 (same consumer code works across providers). `stream_options.include_usage` is the right approach for getting token counts in streaming. One LOW finding (P4-002, vision content blocks) and one COSMETIC (P4-003, duplicated helper).

With Phases 1–4 complete, SC-001 is satisfied (two+ adapters). **Phase 5** (Gemini + Bedrock) and **Phase 6** (integration wiring) remain per spec.

**Baton → cursor** for Phase 5 (Gemini + Bedrock adapters) or Phase 6 (integration) depending on priority.
