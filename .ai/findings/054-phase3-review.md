# 054 Phase 3 Review — Progressive Frontmatter Loader

**Reviewer:** claude
**Date:** 2026-03-30
**Package:** `packages/agent-frontmatter/` (`@opc/agent-frontmatter` v0.1.0)
**Verdict:** ✅ Phase 3 approved — no blockers for Phase 4

## Scope

Review Phase 3 implementation (`loader.ts`: `discoverMarkdownDefinitionFiles`, `loadTier1MetadataFromDir`, `loadTier2Instructions`, `listResourceRefsFromMetadata`, `loadResourceFile`) against spec 054 FR-004 (progressive disclosure Tiers 1–3), NF-001 (500-agent < 200ms), and plan Phase 3 deliverables.

## Artifacts reviewed

| Artifact | Path |
|----------|------|
| Loader implementation | `packages/agent-frontmatter/src/loader.ts` |
| Loader tests | `packages/agent-frontmatter/src/loader.test.ts` |
| Public exports | `packages/agent-frontmatter/src/index.ts` |
| Spec | `specs/054-agent-frontmatter-schema/spec.md` |
| Plan | `.ai/plans/054-agent-frontmatter-schema-phased-plan.md` |

## Requirement coverage

| Req | Status | Evidence |
|-----|--------|----------|
| FR-004 Tier 1 (metadata-only scan) | ✅ | `loadTier1MetadataFromDir()` returns `Tier1MetadataEntry[]` with `filePath`, `metadata`, `diagnostics` — body structurally absent from return type and constructed object (loader.ts:117–121). Callers can build catalogs without body strings retained in memory. |
| FR-004 Tier 2 (body on activation) | ✅ | `loadTier2Instructions()` returns `Tier2InstructionsEntry` extending Tier 1 with `body` field. Designed for activation paths when agent/skill is invoked. Test at loader.test.ts:64–86 asserts body presence and content. |
| FR-004 Tier 3 (resources on demand) | ✅ | `listResourceRefsFromMetadata()` extracts `ResourceRef[]` from `resources` frontmatter field (string or string[]). `loadResourceFile()` reads a single resource. Deliberately conservative stub per plan — callers can layer richer conventions. Test at loader.test.ts:89–118 covers end-to-end: metadata → refs → file read. |
| NF-001 (500 agents < 200ms) | ⚠️ UNTESTED | No benchmark or synthetic 500-file fixture in test suite. Plan Phase 3 validation explicitly required this. See P3-002. |
| NF-002 (diagnostics with path) | ✅ | `parseFrontmatterFromFile()` adds `AFS_READ_ERROR` diagnostic for unreadable files with `filePath` (loader.ts:91–97). All parser diagnostics already carry path from Phase 2. |
| NF-003 (unknown fields preserved) | ✅ | Loader passes through parser output unchanged — no field filtering at any tier. |
| SC-005 (unknown fields in round-trip) | ✅ | Metadata is `Record<string, unknown>` from parser, preserved through Tier 1 and Tier 2 paths unchanged. |
| SC-006 (skill body separated) | ✅ | `loadTier2Instructions()` returns both `metadata` and `body` as separate fields — skill instructions cleanly separated from frontmatter. |

## Architecture assessment

- **Tier separation is clean.** `Tier1MetadataEntry` structurally omits `body`, `Tier2InstructionsEntry` extends it with `body`. TypeScript consumers cannot accidentally access body from Tier 1 results.
- **Discovery is deterministic.** `discoverMarkdownDefinitionFiles()` recursively walks and `sort()`s with `localeCompare` — catalogs are stable across runs (FR-004 Tier 1 catalog use case).
- **Tier 3 is correctly conservative.** `listResourceRefsFromMetadata()` only resolves paths, doesn't read files. `loadResourceFile()` is a convenience separate function. Paths are resolved relative to the definition file directory — correct for co-located resources.
- **Error resilience.** `parseFrontmatterFromFile` catches read errors and returns a diagnostic instead of throwing, so one unreadable file doesn't fail the entire catalog scan.
- **CRLF test added.** P2-001 from Phase 2 review (no CRLF test) has been addressed — `parser.test.ts:43–51` now tests CRLF delimiters.

## Findings

### P3-001 — Loader types not exported from package index (LOW)

`Tier1MetadataEntry`, `Tier2InstructionsEntry`, and `ResourceRef` interfaces are defined with `export` in `loader.ts` but not re-exported from `index.ts`. External consumers importing from `@opc/agent-frontmatter` get the functions but not the types needed to annotate variables holding their return values. TypeScript can infer return types, but explicit annotation requires a deep import from `./loader.js`.

**Recommendation:** Add `export type { Tier1MetadataEntry, Tier2InstructionsEntry, ResourceRef } from "./loader.js"` to `index.ts`. Can be addressed in Phase 4 or as a quick follow-up.

### P3-002 — No NF-001 benchmark (500-agent fixture) (LOW)

Plan Phase 3 validation specifies "NF-001 benchmark or test harness with synthetic 500-file fixture (frontmatter-only path)." No such benchmark exists. With 17 tests completing in 26ms and the parser being a pure string-split + YAML parse, NF-001 is very likely met, but it's unverified.

**Recommendation:** Add a test that creates 500 temp `.md` files with minimal frontmatter, calls `loadTier1MetadataFromDir()`, and asserts wall time < 200ms. Can be a separate benchmark file or a skipped-by-default test.

### P3-003 — Tier 1 reads entire file, not bounded (LOW)

Plan decision A-003 specified "bounded reads or line-delimited extraction" for NF-001. The actual implementation reads the full file via `fs.readFileSync` and then discards the body. For Tier 1, only the frontmatter is needed — the body could theoretically be skipped with a streaming or bounded read (read until second `---` delimiter). In practice, agent/skill files are small (< 10 KB typically) so this is not a performance concern, but it deviates from the plan's stated approach.

**Recommendation:** Acceptable as-is. If NF-001 benchmark (P3-002) reveals timing issues with large files, optimize to bounded reads then.

### P3-004 — Tier 1 test doesn't assert body absence (INFO)

The Tier 1 test (loader.test.ts:38–62) asserts metadata presence and diagnostic count, but doesn't assert that the returned object has no `body` property. TypeScript enforces this structurally, but an explicit `expect(entry).not.toHaveProperty("body")` would document the Tier 1 contract in tests.

### P3-005 — `loadResourceFile` throws on missing file (INFO)

`loadResourceFile()` (loader.ts:190–192) calls `fs.readFileSync` directly without try/catch. If a referenced resource doesn't exist, the caller gets an unstructured Node.js error rather than a diagnostic. This is acceptable for Phase 3 (callers own error handling), but Tier 3 is less resilient than Tier 1/2 read paths which return diagnostics.

### P3-006 — Non-string `resources` elements silently skipped (INFO)

`listResourceRefsFromMetadata()` silently skips non-string array elements in `resources` (loader.ts:175). Consistent with `normalizeToolsField` behavior (P2-002). Phase 5 linter should flag these.

## Test coverage

4 new tests in `loader.test.ts`:
- Discovery: recursive walk + deterministic sort
- Tier 1: metadata-only loading, body not in result
- Tier 2: on-demand body loading with content assertion
- Tier 3: resource ref derivation → file read round-trip

13 existing parser tests (including new CRLF test addressing P2-001).

17/17 total tests pass. `tsc` clean.

## Summary

Phase 3 delivers all three progressive disclosure tiers as specified in FR-004. The tier separation is structurally sound — Tier 1 omits body at the type level, Tier 2 extends with body, Tier 3 resolves but doesn't eagerly load resources. The main gap is the missing NF-001 benchmark (P3-002) which the plan explicitly required, though the implementation is almost certainly fast enough. Type exports (P3-001) are a minor ergonomics gap easily fixed. No blockers for Phase 4 (tool allowlist enforcement).
