# 055 YAML Standards Schema — Pre-Implementation Readiness Review

**Reviewer:** claude
**Date:** 2026-03-31
**Base commit:** 32e817a
**Verdict:** Spec is well-defined with clear schema examples and phased implementation plan. Dependencies (054, 053) are feature-complete and provide reusable patterns. Six phases scoped below with 9 findings (3 LOW, 6 INFO). No blockers.

## Dependency Readiness

| Dep | Package | Status | What 055 uses |
|-----|---------|--------|---------------|
| 054 — Agent Frontmatter Schema | `@opc/agent-frontmatter` | ✅ feature-complete | Pattern: YAML parsing via `yaml` package, diagnostic codes (`AFS_*`), kebab-case validation, `additionalProperties: true` for forward compat (NF-003). JSON Schema Draft 7 in `schemas/`. Agents may reference applicable standards via tags. |
| 053 — Verification Profiles | `@opc/verification-profiles` | ✅ feature-complete | Pattern: plain YAML file parsing (not markdown frontmatter), three-tier precedence (local > community > official), `parseDocument()` for line-number errors, diagnostic code convention (`VP_*`). Verification skills can check compliance with active standards. |
| 035 — Agent Governed Execution | `crates/agent/` | ✅ active (spec) | Integration point: standards inform governed execution constraints for code generation. Standards resolver output feeds into agent system prompts. |

## Spec Assessment

### Strengths

- **Complete schema example** — The error-handling-001.yaml example in the Architecture section is production-quality and exercises all features: rules with 3 different verbs, anti_patterns with corrections, examples with good/bad/explanation, context, tags.
- **Clear three-tier model** — Override resolution is well-specified with directory locations, precedence rules, and an explicit example showing resolution for a conflicting `id`.
- **Contributor pipeline scoped sensibly** — The auto-generation pipeline (Phase 4) outputs `status: candidate` files requiring human review, avoiding over-automation.
- **JSON Schema validation** — FR validation + JSON Schema output (NF-002) enables both programmatic and editor-based validation.

### Architecture Decision: TypeScript Package

055 should be a new TypeScript package (`@opc/yaml-standards-schema` or `@opc/coding-standards`) in `packages/` because:

1. **Pattern alignment** — The dependency chain is TypeScript: 054 (`@opc/agent-frontmatter`), 053 (`@opc/verification-profiles`), both in `packages/`.
2. **YAML parsing** — Both 054 and 053 use the `yaml` npm package with `parseDocument()` for source position tracking (NF-002 line-number errors).
3. **Integration target** — Standards resolve into agent system prompts (Phase 6), which are managed by the TypeScript agent layer.
4. **Prior art** — The equilateral-agents `StandardsLoader.js` was JavaScript; the OAP implementation follows the same domain in TypeScript.

### Architecture Decision: Plain YAML files (not markdown frontmatter)

The spec shows standards as plain `.yaml` files (e.g., `standards/official/error-handling-001.yaml`), not markdown with YAML frontmatter. This matches 053's approach (verification skill/profile files are plain YAML) and diverges from 054 (agent definition `.md` files with frontmatter). Follow the spec — use plain `.yaml`.

## Phase-by-Phase Implementation Scope

### Phase 1 — Schema Definition & JSON Schema

**Deliverables:**
- Package scaffold: `packages/yaml-standards-schema/` (or `packages/coding-standards/`)
- `src/types.ts` — `CodingStandard`, `StandardRule`, `RuleVerb` (`ALWAYS`|`NEVER`|`USE`|`PREFER`|`AVOID`), `AntiPattern`, `StandardExample`, `StandardPriority` (`critical`|`high`|`medium`|`low`), `StandardStatus` (`active`|`candidate`), parse result types, diagnostic types
- `src/schema.ts` — `validateStandardObject()` with `CS_*`-prefixed diagnostic codes for all validation failures (missing `id`, invalid priority, empty rules, malformed anti_patterns, etc.)
- `src/parser.ts` — `parseStandardFile(content, filePath)` using `yaml.parseDocument()` for line-number errors (NF-002)
- `schemas/coding-standard.schema.json` — JSON Schema Draft 7 for the standards YAML format (NF-002)
- Tests: valid/invalid YAML, all five verb types (SC-005), error messages with file path + line number (NF-002), unknown fields preserved (NF-003)

**Satisfies:** FR-001 (required fields), FR-002 (rule verbs), FR-003 (optional fields), NF-002 (JSON Schema), NF-003 (extensibility), SC-001 (validation), SC-005 (all verbs)

### Phase 2 — Three-Tier Loader & Resolver

**Deliverables:**
- `src/loader.ts` — `loadStandardsFromDir(dirPath)` discovers and parses `*.yaml`/`*.yml` files from a directory, `loadAllTiers(projectRoot, communityPath?)` loads from `standards/official/`, `standards/community/`, `standards/local/`
- `src/resolver.ts` — `resolveStandards(tiers, filter?)` merges standards across tiers with later-wins precedence for same `id`, filters by `status: active` (excludes `candidate`), supports category/tag filtering (FR-008)
- `src/defaults.ts` — bundled official standards (can be stubs in Phase 2, content in Phase 3)
- Tests: single-tier load, multi-tier override (SC-002), candidate exclusion (SC-003), category/tag filtering (FR-008), empty directory handling, duplicate `id` resolution, `.yml` fallback

**Satisfies:** FR-004 (three-tier override), FR-005 (well-known directories), FR-008 (resolver with filter), SC-002 (override correctness), SC-003 (candidate exclusion)

### Phase 3 — Official Standards Library

**Deliverables:**
- `standards/official/*.yaml` — initial set of official standards covering categories from the spec and equilateral-agents source: error-handling, naming, testing, security, architecture
- Each standard exercises the full schema: rules with various verbs, anti_patterns, examples, context, tags
- Tests: all official standards pass schema validation, round-trip parse/serialize preserves unknown fields

**Satisfies:** FR-001–003 (proven by real content), NF-003 (forward compat verified)

**Finding R-001 (INFO):** The equilateral-agents source in `docs/extractions/equilateral-agents-open-core.md` contains 6 exemplary standards from `.standards-local-template/`: error-first-design, database-query-patterns, auth-and-access-control, credential-scanning, input-validation-security, integration-tests-no-mocks. These can seed the official library but must be adapted to the 055 schema (the source uses numeric priority `10/20/30` rather than `critical/high/medium/low`).

### Phase 4 — Contributor Pipeline

**Deliverables:**
- `src/generator.ts` — `generateCandidateStandard(findings)` accepts structured execution findings (lint results, review comments, test failures) and produces a candidate standard YAML with `status: candidate`, derived `id`, `category`, draft `rules`, and `anti_patterns`
- `src/aggregator.ts` — `aggregateFindings(findings)` groups findings by category, counts frequency, identifies patterns
- Tests: finding → candidate generation, frequency threshold, valid YAML output, candidate status marking (FR-007)

**Satisfies:** FR-006 (contributor pipeline), FR-007 (candidate status), SC-004 (syntactically valid candidate from findings)

**Finding R-002 (LOW):** The spec's FR-006 says the pipeline "accepts execution findings (structured data from linters, test runners, code review tools)" but doesn't define the input schema for findings. The implementer must define a `Finding` type. Recommendation: align with 053's `SkillResult`/`StepResult` types since verification profile execution is the most likely source of structured findings.

**Finding R-003 (LOW):** The spec says the pipeline "generates candidate standard YAML files" but doesn't specify where candidates are written. The Architecture section shows `standards/candidates/` — the implementer should use this directory and ensure generated files don't overwrite existing candidates with the same `id`.

### Phase 5 — Candidate Review Workflow

**Deliverables:**
- `src/reviewer.ts` — `listCandidates(projectRoot)` lists pending candidates, `promoteCandidate(id, tier, projectRoot)` moves a candidate to `status: active` in the target tier, `rejectCandidate(id, projectRoot)` marks as rejected or removes
- Tests: list/promote/reject workflow, promotion to correct tier directory, status field update, reject cleans up

**Satisfies:** FR-007 (review before promotion)

**Finding R-004 (LOW):** The spec doesn't define a `status: rejected` value — only `candidate` and `active`. The implementer should decide: (a) delete rejected candidates, (b) add `status: rejected` as a third status, or (c) move rejected candidates to a separate directory. Recommendation: delete rejected candidates (simplest, avoids schema expansion).

### Phase 6 — Integration

**Deliverables:**
- `src/injector.ts` — `resolveStandardsForPrompt(context)` resolves active standards relevant to the current task (by category/tags) and formats them for injection into agent system prompts
- Integration with agent system: wire standards resolution into the agent prompt construction path (likely via `@opc/agent-frontmatter` or agent runtime)
- Tests: prompt formatting, tag-based filtering, context-appropriate standard selection, performance (NF-001: < 100ms for 200 standards)

**Satisfies:** SC-006 (performance), NF-001 (resolution speed)

**Finding R-005 (INFO):** The spec's Phase 6 says "wire the standards resolver into agent system prompts" but doesn't specify the integration surface. The likely integration point is whatever constructs agent system prompts in the desktop app (`apps/desktop/`) or the Claude Code bridge (`packages/claude-code-bridge/`). The implementer should identify the prompt construction path before Phase 6.

## Cross-Cutting Findings

**Finding R-006 (INFO):** The spec uses `standards/` as the top-level directory, which doesn't exist yet in the repo. This is a new directory at repo root, parallel to `specs/`, `tools/`, `packages/`. The implementer should create this directory structure.

**Finding R-007 (INFO):** The spec mentions "community standards (from shared config)" in FR-005 but doesn't define how the community path is configured. The `standards/community/` directory location may need to be configurable (e.g., via a config file or environment variable) for teams that share standards across repos. For Phase 2, a hardcoded path is acceptable; configurability can be deferred.

**Finding R-008 (INFO):** FR-004 mentions "extend a standard from a broader tier rather than fully replacing it" but the Architecture section only shows full replacement (local version wins). Extend semantics (merging rules from multiple tiers) are more complex and not demonstrated. Recommendation: implement full replacement only in Phase 2; if extend semantics are needed, add in a follow-up.

**Finding R-009 (INFO):** NF-001 requires resolution of 200 standards in < 100ms. Given that 053's verification profile resolution is file-system-based (readdir + parse), the implementer should consider caching parsed standards in memory after first load. The `Map`-based pattern from 053's `loadSkillLibrary` is suitable.

## Summary

| Item | Status |
|------|--------|
| Spec clarity | ✅ Well-defined with complete examples |
| Dependencies | ✅ All feature-complete (054, 053) |
| Architecture fit | ✅ TypeScript package, plain YAML, three-tier |
| Phase scoping | ✅ 6 phases follow established pattern |
| Blockers | None |
| Findings | 3 LOW, 6 INFO |

**Recommendation:** Ready for Phase 1 implementation. The implementer should scaffold `packages/yaml-standards-schema/` (or similar name) following the 053 package pattern (ESM, `tsc` build, vitest, `yaml` dependency). Key decisions for Phase 1: package name, `CS_*` diagnostic code prefix, whether to use `coding-standard.schema.json` or `standard.schema.json` for the JSON Schema file.
