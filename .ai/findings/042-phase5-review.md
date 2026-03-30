# 042 Phase 5 review — Gemini + Bedrock adapters and normalizers

> Reviewed: 2026-03-29 by **claude**. Assesses cursor's Phase 5 against spec 042.

## What was delivered

- `adapters/gemini.ts` — `createGeminiProvider()` factory, `GeminiProvider` implementing full `Provider` interface
- `adapters/bedrock.ts` — `createBedrockProvider()` factory, `BedrockConverseProvider` implementing full `Provider` interface
- `normalization/gemini-events.ts` — `GeminiStreamNormalizer` (stateful) + `generateContentResponseToAgentEvents()` (non-streaming)
- `normalization/bedrock-events.ts` — `BedrockStreamNormalizer` (stateful) + `converseResponseToAgentEvents()` (non-streaming) + `bedrockMessageToAgentEvents()` (test helper)
- `gemini-events.test.ts` — 2 tests (streaming text+usage, non-streaming response)
- `bedrock-events.test.ts` — 1 test (streaming messageStart, text delta, messageStop, metadata)
- `index.ts` updated with all new exports
- `package.json` updated with `@google/generative-ai`, `@aws-sdk/client-bedrock-runtime` deps

## Spec parity

| Requirement | Status | Evidence |
|------------|--------|----------|
| FR-002 spawn/query/abort/stream | ✅ | Both adapters implement all 4 ops |
| FR-003 AgentEvent normalization | ✅ | Gemini `GenerateContentResponse` → `AgentEvent`; Bedrock `ConverseStreamOutput` → `AgentEvent` |
| FR-006 ProviderError on missing key | ✅ Gemini / ⚠️ Bedrock | Gemini: `requireKey()` throws `ProviderError("missing_api_key")`. Bedrock: uses AWS credential chain — no API key validation (correct for AWS, see P5-001) |
| FR-007 stream AsyncIterable + cleanup | ✅ | Both use `async *stream`, `inflight` map, `finally` cleanup |
| SC-001 5 adapters available | ✅ | Anthropic + OpenAI + ClaudeCodeSDK + Gemini + Bedrock — all registered in index.ts |
| SC-003 same consumer code | ✅ | All adapters emit identical `AgentEvent` union types |
| SC-004 abort terminates stream | ✅ | Both have `inflight` AbortController map + `abort()` |
| NF-001 new provider = one interface | ✅ | Both follow the established pattern (factory → class → Provider interface) |

## Quality observations

### Positive

- **Bedrock uses Converse/ConverseStream API** — correct choice over deprecated `InvokeModel`. This is the unified Bedrock API that works across all model families.
- **Bedrock credential handling**: Uses default AWS credential chain rather than requiring `apiKey`. Region via `ProviderConfig.extra.region` → `AWS_REGION` → `us-east-1` fallback. Idiomatic for AWS.
- **Bedrock model validation in `spawn()`**: Validates `defaultModel` is present. Good — Bedrock model IDs are required and non-obvious (`us.anthropic.claude-3-5-sonnet-20241022-v2:0`).
- **Gemini system instruction**: Passed via `systemInstruction` on `getGenerativeModel()`. Correct — Gemini doesn't accept system messages inline.
- **Gemini function call normalization**: Both streaming and non-streaming paths handle `functionCall` parts, emitting `tool_use_start` + `tool_use_complete` with parsed args.
- **Bedrock tool_use_delta accumulation**: `toolByBlock` map correctly accumulates JSON fragments across `contentBlockDelta` events, parses on `contentBlockStop`. Handles parse failures gracefully (falls back to raw string).
- **Structural consistency**: Both adapters follow the same pattern as Anthropic and OpenAI — `mergeAbortSignals`, `toProviderError`, `inflight` map, factory function. Codebase is consistent across all 5 adapters.

### Findings

#### P5-001: Bedrock `spawn()` doesn't validate credentials (INFO)

Bedrock uses the AWS credential provider chain (env vars, `~/.aws/credentials`, IAM role, etc.), so there's no `apiKey` to validate at `spawn()` time. The first real validation happens on the first `Converse` call. This is correct AWS behavior — unlike the other adapters, credentials aren't a single key but a chain. FR-006 is satisfied differently but validly: AWS SDK will throw a structured error which `toProviderError()` wraps.

**No action needed.**

#### P5-002: `mergeAbortSignals` duplicated 4× across adapters (COSMETIC)

Now present in `anthropic.ts`, `openai.ts`, `gemini.ts`, and `bedrock.ts`. Identical implementation each time. Previously flagged as P4-003.

**Recommendation:** Extract to a shared utility (e.g., `src/utils.ts` or `src/abort.ts`) in a future cleanup pass.

#### P5-003: `tool` role not mapped in either adapter (LOW)

Both Gemini (`toGeminiContents`, line 176) and Bedrock (`toBedrockRequest`, line 218) throw `ProviderError("unsupported_role")` when encountering `role: "tool"` messages. This means tool result round-trips (multi-turn with tool use) aren't wired yet.

**Impact:** Single-turn tool calls work (provider returns `tool_use_start`/`tool_use_complete`), but feeding tool results back requires mapping `role: "tool"` to Gemini's `functionResponse` parts or Bedrock's `toolResult` content blocks. Not blocking for Phase 5 — this is integration-layer work appropriate for Phase 6.

#### P5-004: Gemini streaming doesn't emit `text_complete` (LOW)

The streaming normalizer emits `text_delta` events per chunk but never emits a `text_complete` event to aggregate the full text. The non-streaming path correctly emits `text_complete`. This matches the Anthropic and OpenAI streaming normalizers (they also don't emit `text_complete` during streaming), so it's consistent, but consumers wanting the full text need to accumulate deltas themselves.

**Impact:** None practically — consumers already handle delta accumulation.

#### P5-005: Bedrock test coverage is minimal (LOW)

Only 1 test for `BedrockStreamNormalizer` (text deltas → message_complete). Missing:
- Non-streaming `converseResponseToAgentEvents()` — not tested
- Tool use streaming (contentBlockStart with toolUse → contentBlockDelta → contentBlockStop)
- Empty output handling in `converseResponseToAgentEvents()`

Gemini has 2 tests (streaming + non-streaming). Better, but also missing tool call test.

**Recommendation:** Add tool use tests for both normalizers in a follow-up.

#### P5-006: Gemini `vision: true` but content blocks stringify non-text (LOW)

`toParts()` (gemini.ts:190) falls back to `JSON.stringify(b)` for non-text content blocks. For image content, Gemini expects `inlineData` or `fileData` parts. Same pattern as P4-002 for OpenAI. Vision support is declared in capabilities but image content blocks would be sent as JSON strings.

**Impact:** Vision queries would not work correctly. Not blocking — vision isn't exercised yet.

## Test results

All 16 tests pass across 6 test files:

```
✓ src/normalization/gemini-events.test.ts       (2 tests)
✓ src/normalization/bedrock-events.test.ts       (1 test)
✓ src/normalization/anthropic-events.test.ts     (1 test)
✓ src/normalization/claude-code-events.test.ts   (3 tests)
✓ src/normalization/openai-events.test.ts        (3 tests)
✓ src/registry.test.ts                           (6 tests)
```

## Summary

**Phase 5 is clean and spec-faithful.** Both Gemini and Bedrock adapters follow the established adapter pattern, implement all four `Provider` operations, and normalize to `AgentEvent` correctly. The Bedrock adapter wisely uses the Converse API and standard AWS credential chain. No blockers.

With Phases 1–5 complete, all 5 provider adapters specified in the spec are implemented: Anthropic, OpenAI, Claude Code SDK, Gemini, Bedrock. SC-001 through SC-006 are all satisfied.

Open items for Phase 6 or follow-up:
- P5-002: Extract `mergeAbortSignals` to shared utility (COSMETIC)
- P5-003: Map `tool` role for multi-turn tool use in Gemini/Bedrock (needed for Phase 6 integration)
- P5-005: Expand Bedrock test coverage
- P5-006/P4-002: Wire vision content blocks properly for Gemini/OpenAI (when vision is exercised)

**Baton → cursor** for **Phase 6** (wire `ProviderRegistry` into agent execution paths per spec).
