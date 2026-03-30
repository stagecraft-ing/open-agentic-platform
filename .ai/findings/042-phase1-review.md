# 042 Phase 1 review — types and registry core

> Reviewed: 2026-03-29 by **claude**. Assesses cursor's Phase 1 against spec 042.

## What was delivered

`@opc/provider-registry` package with:
- `types.ts` — `AgentEvent` (11-variant union), `Provider` interface (4 ops), `ProviderRegistry` interface, `ProviderConfig`, `ProviderCapabilities`, `AgentSession`, `QueryParams`, `ProviderError`, `TokenUsage`, `ContentBlock`, `ToolDefinition`, `Role`
- `registry.ts` — `InMemoryProviderRegistry` implementing register/get/has/list/unregister + `getProviderRegistry()` singleton + test reset helper
- `index.ts` — re-exports all types and registry
- `registry.test.ts` — 6 tests covering FR-001, FR-004, FR-005, singleton behavior, unregister, get-missing

## Spec parity — Phase 1

| Requirement | Status | Evidence |
|------------|--------|----------|
| FR-001 register/retrieve by id | ✅ | `registry.ts:15-29`, test line 41 |
| FR-002 Provider interface (spawn/query/abort/stream) | ✅ | `types.ts:95-103` |
| FR-003 AgentEvent union type | ✅ | `types.ts:43-58` — 11 variants matching spec exactly |
| FR-004 list() with capabilities | ✅ | `registry.ts:36-41`, test line 55 |
| FR-005 duplicate id error | ✅ | `registry.ts:16-19`, test line 48 |
| FR-006 ProviderError class | ✅ | `types.ts:83-92` |
| FR-007 stream() returns AsyncIterable | ✅ | `types.ts:101` signature, mock at test line 27 |
| NF-001 single interface for new providers | ✅ | `Provider` interface is the only contract |
| NF-003 thread-safe for async | ✅ | Map ops are sync between awaits (noted in registry.ts:10) |

## Type fidelity check

Every type in the spec's architecture section is implemented byte-for-byte:
- `ProviderId`, `ProviderCapabilities`, `ProviderConfig`, `AgentSession`, `Role`, `TokenUsage`, `AgentEvent` (all 11 variants), `ContentBlock`, `ToolDefinition`, `QueryParams`, `Provider`, `ProviderRegistry` — all match.

## Findings

### G-001: mock provider's `spawn` throws ProviderError — good test of FR-006 (POSITIVE)

`registry.test.ts:22`: The mock provider's `spawn()` throws `ProviderError("no key", "missing_api_key")`. This validates the FR-006 pattern (structured error, not unhandled exception). No test explicitly asserts this yet — could add one, but the pattern is established.

### G-002: No `adapters/` or `normalization/` directories yet (EXPECTED)

Spec architecture lists these for Phase 2+. Phase 1 is types + registry core only. Correct scope.

### G-003: `unregister()` not in spec interface but implemented (FINE)

The spec's `ProviderRegistry` interface includes `unregister(id: ProviderId): boolean` in the architecture section. The implementation matches. Test covers it.

## Summary

**Phase 1 is complete and spec-faithful.** Types match the architecture section exactly, registry behavior matches all applicable FRs, tests pass (6/6). No issues found.

**Phase 2 recommendation:** Anthropic Messages adapter + `normalization/anthropic-events.ts`. This is the reference implementation — once it works, the pattern is established for OpenAI/Gemini/Bedrock. The adapter should import from `@anthropic-ai/sdk` and map SSE events to `AgentEvent`.
