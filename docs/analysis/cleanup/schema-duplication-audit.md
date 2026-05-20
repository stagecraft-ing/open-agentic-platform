# Schema Duplication Audit

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** `find`, `md5`, `python3 json.load`, `git grep`. Read-only.

## All authored schemas

19 files total. Excludes generated content (none found inside the audit window).

| current path | target path (post-I4) | format | $id | title | top-props | bytes |
|---|---|---|---|---|---|---|
| `crates/agent/src/schemas/verification.schema.json` | `standards/schemas/agent/verification.schema.json` | json | `axiomregent://spec/verification.schema.json` | VerificationConfig | 5 | 4 270 |
| `crates/agent/src/schemas/verify-result.schema.json` | `standards/schemas/agent/verify-result.schema.json` | json | `axiomregent://spec/verify-result.schema.json` | VerifyResult | 12 | 3 124 |
| `crates/factory-contracts/schemas/adapter-manifest.schema.yaml` | `standards/schemas/factory/adapter-manifest.schema.yaml` | yaml | — (no `$id`) | — (no `title`) | 13 | 18 308 |
| `crates/factory-contracts/schemas/build-spec.schema.yaml` | `standards/schemas/factory/build-spec.schema.yaml` | yaml | — | — | 16 | 28 237 |
| `crates/factory-contracts/schemas/pipeline-state.schema.yaml` | `standards/schemas/factory/pipeline-state.schema.yaml` | yaml | — | — | 7 | 8 861 |
| `crates/factory-contracts/schemas/verification.schema.yaml` | `standards/schemas/factory/verification.schema.yaml` | yaml | — | — | 5 | 19 627 |
| `crates/factory-contracts/schemas/stage-outputs/audiences.schema.json` | `standards/schemas/factory/stage-outputs/audiences.schema.json` | json | `audiences.schema.json` (relative-style; non-conformant) | Audiences — Stage 2 Output | 1 | 2 244 |
| `crates/factory-contracts/schemas/stage-outputs/business-rules.schema.json` | `standards/schemas/factory/stage-outputs/business-rules.schema.json` | json | `business-rules.schema.json` (non-conformant) | Business Rules — Stage 1 Output | 1 | 3 501 |
| `crates/factory-contracts/schemas/stage-outputs/entity-model.schema.json` | `standards/schemas/factory/stage-outputs/entity-model.schema.json` | json | `entity-model.schema.json` (non-conformant) | Entity Model — Stage 1 Output | 1 | 3 079 |
| `crates/factory-contracts/schemas/stage-outputs/sitemap.schema.json` | `standards/schemas/factory/stage-outputs/sitemap.schema.json` | json | `sitemap.schema.json` (non-conformant) | Sitemap — Stage 2 Output | 2 | 2 232 |
| `crates/factory-contracts/schemas/stage-outputs/use-cases.schema.json` | `standards/schemas/factory/stage-outputs/use-cases.schema.json` | json | `use-cases.schema.json` (non-conformant) | Use Cases — Stage 1 Output | 1 | 2 050 |
| `packages/yaml-standards-schema/schemas/coding-standard.schema.json` | (duplicate; see below) | json | `https://open-agentic-platform.dev/schemas/coding-standard.schema.json` | Coding Standard | 9 | 3 376 |
| `schemas/agent-frontmatter.schema.json` | `standards/schemas/frontmatter/agent-frontmatter.schema.json` | json | `https://open-agentic-platform.dev/schemas/agent-frontmatter.schema.json` | Unified Agent and Skill Frontmatter (spec 054) | 26 | 4 987 |
| `schemas/codebase-index-oap.schema.json` | `standards/schemas/spec-spine/codebase-index-oap.schema.json` | json | `https://open-agentic-platform.local/schemas/codebase-index-oap.schema.json` | Open Agentic Platform — enriched codebase index (Cut D W-07a/c) | 8 | 4 707 |
| `schemas/codebase-index.schema.json` | `standards/schemas/spec-spine/codebase-index.schema.json` | json | `https://open-agentic-platform.local/schemas/codebase-index.schema.json` | Open Agentic Platform — compiled codebase index (spec 101) | 5 | 7 844 |
| `schemas/skill-frontmatter.schema.json` | `standards/schemas/frontmatter/skill-frontmatter.schema.json` | json | `https://open-agentic-platform.dev/schemas/skill-frontmatter.schema.json` | OAP skill definition frontmatter | 5 | 1 016 |
| `specs/000-bootstrap-spec-system/contracts/build-meta.schema.json` | `standards/schemas/spec-spine/build-meta.schema.json` | json | `https://open-agentic-platform.local/specs/000-bootstrap-spec-system/build-meta.schema.json` | Open Agentic Platform — non-deterministic build metadata (ephemeral) | 3 | 876 |
| `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | `standards/schemas/spec-spine/registry.schema.json` | json | `https://open-agentic-platform.local/specs/000-bootstrap-spec-system/registry.schema.json` | Open Agentic Platform — compiled spec registry (deterministic MVP) | 5 | 10 437 |
| `standards/schema/standard.schema.json` | `standards/schemas/coding/standard.schema.json` | json | `https://open-agentic-platform.dev/schemas/coding-standard.schema.json` | Coding Standard | 9 | 3 376 |

## Name collisions

| schema name | locations | semantic relationship |
|---|---|---|
| `verification.schema.json` | `crates/agent/src/schemas/verification.schema.json` | **different** — VerificationConfig for axiomregent skill verification (toolchains, skills, governance). Title: "VerificationConfig". |
| `verification.schema.yaml` | `crates/factory-contracts/schemas/verification.schema.yaml` | **different** — Factory pipeline gate contract (cross-stage consistency, adapter checks). Anonymous (no `$id`/`title`). |
| → Combined: "verification" appears as 2 distinct schemas, json + yaml, with **different** content and **different** consumers. |  |  |

No other filename collides across the 19 files.

## `$id` collisions

| `$id` value | locations | semantic relationship |
|---|---|---|
| `https://open-agentic-platform.dev/schemas/coding-standard.schema.json` | `packages/yaml-standards-schema/schemas/coding-standard.schema.json` and `standards/schema/standard.schema.json` | **byte-identical duplicate** (MD5 `d9a31b7fa469dd2355a3f88708603674` both files). Same title ("Coding Standard"), same 9 top-props, same 3 376 bytes. |

**This is a true cross-tree duplicate.** The same schema is published twice: once inside the npm package (`packages/yaml-standards-schema/schemas/`) and once under the (non-target-layout) `standards/schema/` directory. Both have the **dev** $id (note `.dev` host, not `.local`). The duplication appears to be unintentional drift — likely the package's schema was the original and the `standards/schema/standard.schema.json` was added later as part of a config refactor without removing the original. Master plan §Locked target layout puts coding standards under `standards/schemas/coding/standard.schema.json`; resolution in I4 chooses one of:

(a) **Move the canonical one to `standards/schemas/coding/standard.schema.json`** and delete the npm-side copy (`packages/yaml-standards-schema/schemas/coding-standard.schema.json`). Update `packages/yaml-standards-schema/src/loader.ts` (or its analogue) to look up the schema via the new path (or via a typed binding). **Requires consumer-code change.**

(b) **Keep the npm-side copy** as the runtime read target; delete `standards/schema/standard.schema.json`. **Simpler** if the npm package is the only runtime consumer; no consumer-code change.

(c) **Keep both** and document them as authoring vs. publishing copies. **Not recommended** — perpetuates drift; the codebase-indexer's `schemas/*.json` hash input rule (`CLAUDE.md:89`) would double-count.

D1 surfaces this as Group F open question 2. D5 confirms it's a real byte-identical duplicate, not a near-miss.

## Stage-output `$id` non-conformance

Five schemas under `crates/factory-contracts/schemas/stage-outputs/` declare `$id` as a bare filename (`audiences.schema.json`, etc.) rather than a URI. This is JSON-Schema-2020-12 nonconformant — `$id` MUST be an absolute URI per the spec.

| file | current `$id` | corrected `$id` (recommended) |
|---|---|---|
| `audiences.schema.json` | `audiences.schema.json` | `https://open-agentic-platform.local/schemas/factory/stage-outputs/audiences.schema.json` |
| `business-rules.schema.json` | `business-rules.schema.json` | `https://open-agentic-platform.local/schemas/factory/stage-outputs/business-rules.schema.json` |
| `entity-model.schema.json` | `entity-model.schema.json` | `https://open-agentic-platform.local/schemas/factory/stage-outputs/entity-model.schema.json` |
| `sitemap.schema.json` | `sitemap.schema.json` | `https://open-agentic-platform.local/schemas/factory/stage-outputs/sitemap.schema.json` |
| `use-cases.schema.json` | `use-cases.schema.json` | `https://open-agentic-platform.local/schemas/factory/stage-outputs/use-cases.schema.json` |

Master plan §Out of scope defers schema-from-types generation, so `$id` correction is **deferred to follow-up** post-cleanup. I4 only moves files verbatim. D5 surfaces the issue; operator may add a follow-up issue post-Epic 2.

## $id host inconsistency

Three host values appear:

1. **`open-agentic-platform.dev`** (3 schemas) — `agent-frontmatter`, `skill-frontmatter`, `coding-standard` (the duplicate). Suggests "public/registered" identity.
2. **`open-agentic-platform.local`** (4 schemas) — `codebase-index`, `codebase-index-oap`, `registry`, `build-meta`. Suggests "local-only" identity.
3. **`axiomregent://spec/`** (2 schemas) — `agent/verification`, `agent/verify-result`. Custom URI scheme; appears intentional for axiomregent-internal use.

No two schemas share a host within the same "$id namespace" (other than the byte-duplicate above). Decision deferred to schema-from-types follow-up.

## Property-set overlaps (heuristic)

Surfacing pairs with high top-property overlap that aren't already flagged via name/`$id`:

| schema A | schema B | overlap evidence | judgment |
|---|---|---|---|
| `schemas/agent-frontmatter.schema.json` (26 props) | `schemas/skill-frontmatter.schema.json` (5 props) | Both describe markdown-frontmatter shape; skill is a subset of agent (spec 054 unified them) | **intentional subset** — no duplicate; same source-of-truth concept staged across 2 files. Could be unified but is not duplicate. |
| `crates/factory-contracts/schemas/verification.schema.yaml` | `crates/agent/src/schemas/verification.schema.json` | Same name, different content (factory pipeline gates vs axiomregent skill toolchains) | **independent** — name collision only. |

No additional overlaps surfaced.

## Format unification surface

Schemas in YAML (4 files, all under factory):

| file | top-level | size |
|---|---|---|
| `crates/factory-contracts/schemas/adapter-manifest.schema.yaml` | factory-owned | 18 308 |
| `crates/factory-contracts/schemas/build-spec.schema.yaml` | factory-owned | 28 237 |
| `crates/factory-contracts/schemas/pipeline-state.schema.yaml` | factory-owned | 8 861 |
| `crates/factory-contracts/schemas/verification.schema.yaml` | factory-owned | 19 627 |

Stage-outputs under factory are JSON, not YAML (5 files).

Format unification (yaml → json or vice versa) **deferred to follow-up** per master plan §Out of scope. I4 moves files verbatim, preserving format.

## Crate-internal schemas with `include_str!` to update

| consumer | current path | current include_str! | post-I4 target |
|---|---|---|---|
| `tools/spec-compiler/src/schema.rs:13` | `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | `include_str!("../../../specs/000-bootstrap-spec-system/contracts/registry.schema.json")` | path becomes `standards/schemas/spec-spine/registry.schema.json`; include_str depth = 2 (`../../standards/schemas/spec-spine/registry.schema.json`) |
| `tools/spec-compiler/src/schema.rs:16` | `specs/000-bootstrap-spec-system/contracts/build-meta.schema.json` | `include_str!("../../../specs/000-bootstrap-spec-system/contracts/build-meta.schema.json")` | path becomes `standards/schemas/spec-spine/build-meta.schema.json`; same depth update |
| `tools/codebase-indexer/src/schema.rs:8` | `schemas/codebase-index.schema.json` (runtime read, not include_str!) | `repo_root.join("schemas/codebase-index.schema.json")` | path becomes `standards/schemas/spec-spine/codebase-index.schema.json` |
| `tools/codebase-indexer/tests/schema_conformance.rs:29,89,94` | `schemas/codebase-index.schema.json` | `load_schema("schemas/codebase-index.schema.json")` | same — path string updates |
| `tools/spec-compiler/tests/schema_conformance.rs:30,42,75,81` | bootstrap schemas | `load_schema("specs/000-.../contracts/...")` | same — path string updates |
| `packages/yaml-standards-schema/src/loader.ts` (or analogue) | `packages/yaml-standards-schema/schemas/coding-standard.schema.json` (if kept) OR external `standards/schemas/coding/standard.schema.json` (if moved) | runtime read via package-relative path | depends on duplicate resolution (above) |

**Important:** I5 also moves `tools/spec-compiler/` → `tools/spec-spine/spec-compiler/`. The combined I4 + I5 effect is two simultaneous depth changes on the same `include_str!` strings. Operator decides whether to: (a) land I4 and I5 in lockstep with the combined depth update, or (b) land them sequentially with two passes through `tools/spec-compiler/src/schema.rs`.

## Test fixtures referencing schemas

| consumer | current path | impact |
|---|---|---|
| `tools/codebase-indexer/tests/schema_conformance.rs:29-94` | `schemas/codebase-index.schema.json` | 3 test loads update |
| `tools/spec-compiler/tests/schema_conformance.rs:30,42,75,81` | bootstrap schemas | 4 test loads update |
| `tools/spec-compiler/tests/v004_consolidation_excludes.rs:32,36` | `pnpm-workspace.yaml`, `pnpm-lock.yaml` | 2 test fixtures — Group M concern, not D5 |
| `platform/services/stagecraft/web/app/components/artifact-body-viewer.test.ts:126,131,140,142` | `schemas/x.schema.yaml`, `schemas/adapter-manifest.schema.yaml` | Test-only arbitrary path strings as fixture payload; not load-bearing on real schema location |

## Stagecraft runtime walk

`platform/services/stagecraft/api/factory/oapContracts.ts` walks up looking for the directory `crates/factory-contracts/schemas/` (D1 Group D). After I4 the walked target becomes `standards/schemas/factory/`. **Required update**: the walk-up string in `oapContracts.ts:20` updates from `crates/factory-contracts/schemas/` to `standards/schemas/factory/`, and emitted substrate rows' `path:` field shapes change accordingly. Stagecraft tests asserting on substrate path strings (`platform/services/stagecraft/web/app/components/artifact-body-viewer.test.ts:140,142`) also update.

## I4 readiness summary

- **Schemas to move:** 19 (or 18 if duplicate is deleted)
- **True duplicates needing resolution before move:** 1 (`standard.schema.json` ↔ `coding-standard.schema.json`)
- **Format unification deferred:** 4 YAML schemas remain YAML; deferred to follow-up
- **`$id` non-conformance** in 5 stage-outputs schemas; deferred to follow-up
- **Crate-internal `include_str!` to update:** 2 in `tools/spec-compiler/src/schema.rs` (path-literal in source) + 4 in tests
- **Runtime-read paths to update:** 1 in `tools/codebase-indexer/src/schema.rs` + 1 in stagecraft `oapContracts.ts`
- **Test paths to update:** 7 fixture string literals (across both crates)
- **Estimated complexity:** **medium** — bulk move is `git mv`; the duplicate resolution (decision required), the `include_str!` depth change, the stagecraft walker target, and the test fixture path strings are all coupled. I4 lands atomically per master plan.

## Open questions (surface for operator triage)

1. **Coding-standard duplicate resolution** (option a/b/c above). D5 recommends **option (b)** — keep the npm-side copy as authoritative runtime location and delete `standards/schema/standard.schema.json`, accepting that the layout puts a single coding-standard schema inside the npm package's `schemas/` subdir. Alternative: move both to `standards/schemas/coding/` and update the loader.
2. **`crates/agent/src/schemas/` (Group E) disposition** (from D1 Group E open question) — move to `standards/schemas/agent/`, delete, or move-and-keep? Two files are dormant (no callers).
3. **`tools/spec-compiler/src/schema.rs` `include_str!` depth coupling** — I4 + I5 land in lockstep, or I4 (schema move only) lands first leaving I5 (tool move) for a second pass?
4. **Stage-outputs `$id` non-conformance** — defer to post-cleanup follow-up (recommended) or fix as part of I4?
5. **YAML/JSON unification** — master plan defers; D5 confirms no I4 work here.
