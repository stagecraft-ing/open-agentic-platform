# Spec `implements:` Field Inventory

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** `git grep -nE "^implements:"` + `^\s*-\s*path:` across `specs/*/spec.md`.

## Aggregate counts

- **Total specs in corpus:** 152
- **Specs declaring `implements:`:** 107 (per `git grep -lE "^implements:"`)
- **Specs with empty `implements: []`:** 1 (`specs/081-factory-test-hydration/spec.md:23`)
- **Total `- path:` entries (post-W-05 list polymorphism):** 502 across the corpus (some `- path:` lines also live outside frontmatter as code-examples — D4 sources only spec frontmatter blocks below; the `502` count includes non-frontmatter prose-`- path:` lines and is therefore an upper bound; the per-group enumerations below use the actual frontmatter blocks)

## All `implements:` entries pointing at moving paths

Each row lists `spec id : line` and `path value`. The Epic 2 phase column indicates which I-phase updates the entry (per master plan).

### Group A — `.specify/` (Epic 2 I3 / I13)

| spec | line | implements path | I-phase |
|---|---|---|---|
| `specs/119-project-as-unit-of-governance` | 41 | `.specify/contract.md` | I3 (replaced by `standards/spec/contract.md`) |

### Group B — Root `schemas/` (Epic 2 I4)

| spec | line | implements path | I-phase |
|---|---|---|---|
| `specs/129-granular-package-oap-metadata` | 18 | `schemas/codebase-index.schema.json` | I4 → `standards/schemas/spec-spine/codebase-index.schema.json` |
| `specs/133-amends-aware-coupling-gate` | 23 | `schemas/codebase-index.schema.json` | I4 → `standards/schemas/spec-spine/codebase-index.schema.json` |

### Group C — `specs/000-bootstrap-spec-system/contracts/` (Epic 2 I4)

| spec | line | implements path | I-phase |
|---|---|---|---|
| `specs/132-constitutional-invariant-freeze` | 21 | `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | I4 → `standards/schemas/spec-spine/registry.schema.json` |

### Group D — `crates/factory-contracts/schemas/` (Epic 2 I4)

| spec | line | implements path | I-phase |
|---|---|---|---|
| `specs/112-factory-project-lifecycle` | 33 | `crates/factory-contracts/schemas/` | I4 → `standards/schemas/factory/` |

### Group E — `crates/agent/src/schemas/` (Epic 2 I4)

No `implements:` references found. (Schemas are present but no spec claims them as its implementation surface; see D5 + D1 Group E open question.)

### Group F — `standards/` (Epic 2 I3)

No `implements:` references found. (`standards-loader` is owned by `specs/055-yaml-standards-schema/spec.md` whose `implements:` points at `crates/standards-loader` — not the YAML files themselves.)

### Group G — Spec-spine tools (Epic 2 I5)

32 entries across 31 specs (spec 003 lists two G paths):

| spec | line | implements path | I-phase target |
|---|---|---|---|
| `specs/003-feature-lifecycle-mvp` | 18 | `tools/spec-compiler` | `tools/spec-spine/spec-compiler` |
| `specs/003-feature-lifecycle-mvp` | 19 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/007-registry-consumer-status-report-mvp` | 17 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/008-registry-consumer-status-report-json-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/009-registry-consumer-status-report-nonzero-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/010-registry-consumer-status-report-json-contract-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/011-registry-consumer-status-report-status-filter-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/012-registry-consumer-list-json-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/013-registry-consumer-show-json-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/014-registry-consumer-show-compact-json-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/015-registry-consumer-list-compact-json-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/016-registry-consumer-status-report-compact-json-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/017-registry-consumer-shared-json-serialization-helper-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/018-registry-consumer-list-show-json-contract-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/019-registry-consumer-readme-examples-contract-mvp` | 17 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/020-registry-consumer-error-contract-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/021-registry-consumer-field-shape-invariants-contract-mvp` | 17 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/022-registry-consumer-help-usage-contract-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/023-registry-consumer-flag-conflict-argument-validation-contract-mvp` | 17 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/024-registry-consumer-version-banner-contract-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/025-registry-consumer-default-path-contract-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/026-registry-consumer-allow-invalid-contract-mvp` | 17 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/027-registry-consumer-sorting-order-contract-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/028-registry-consumer-channel-discipline-contract-mvp` | 17 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/029-registry-consumer-contract-governance-gate-mvp` | 17 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/030-registry-consumer-internal-output-exit-refactor-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/031-registry-consumer-list-ids-only-contract-mvp` | 16 | `tools/registry-consumer` | `tools/spec-spine/registry-consumer` |
| `specs/039-feature-id-reconciliation` | 17 | `tools/spec-compiler` | `tools/spec-spine/spec-compiler` |
| `specs/091-registry-enrichment` | 18 | `tools/spec-compiler` | `tools/spec-spine/spec-compiler` |
| `specs/102-governed-excellence` | 34 | `tools/spec-compiler` | `tools/spec-spine/spec-compiler` |
| `specs/118-workflow-spec-traceability` | 18 | `tools/codebase-indexer` | `tools/spec-spine/codebase-indexer` |
| `specs/127-spec-code-coupling-gate` | 23 | `tools/spec-code-coupling-check` | `tools/spec-spine/spec-code-coupling-check` |
| `specs/129-granular-package-oap-metadata` | 17 | `tools/codebase-indexer` | `tools/spec-spine/codebase-indexer` |
| `specs/130-spec-coupling-primary-owner` | 18 | `tools/spec-code-coupling-check` | `tools/spec-spine/spec-code-coupling-check` |
| `specs/132-constitutional-invariant-freeze` | 20 | `tools/spec-compiler` | `tools/spec-spine/spec-compiler` |
| `specs/133-amends-aware-coupling-gate` | 21 | `tools/spec-code-coupling-check` | `tools/spec-spine/spec-code-coupling-check` |
| `specs/133-amends-aware-coupling-gate` | 22 | `tools/codebase-indexer` | `tools/spec-spine/codebase-indexer` |
| `specs/151-declarative-cluster-reconciliation` | 29 | `tools/spec-compiler/src/lib.rs` | `tools/spec-spine/spec-compiler/src/lib.rs` |
| `specs/151-declarative-cluster-reconciliation` | 30 | `tools/spec-compiler/tests/v004_consolidation_excludes.rs` | `tools/spec-spine/spec-compiler/tests/v004_consolidation_excludes.rs` |

### Group H — OAP tools (Epic 2 I5)

7 entries:

| spec | line | implements path | I-phase target |
|---|---|---|---|
| `specs/104-makefile-ci-parity-contract` | 18 | `tools/ci-parity-check` | `tools/oap/ci-parity-check` |
| `specs/105-scripts-to-binaries-migration` | 18 | `tools/adapter-scopes-compiler` | `tools/oap/adapter-scopes-compiler` |
| `specs/122-stakeholder-doc-inversion` | 42 | `tools/stakeholder-doc-lint/Cargo.toml` | `tools/oap/stakeholder-doc-lint/Cargo.toml` |
| `specs/125-schema-parity-walker-rebuild` | 27 | `tools/schema-parity-check/index.mjs` | `tools/oap/schema-parity-check/index.mjs` |
| `specs/131-adversarial-prompt-refusal-policy` | 21 | `tools/policy-compiler` | `tools/oap/policy-compiler` |
| `specs/134-fast-local-ci-mode` | 18 | `tools/ci-parity-check/src/lib.rs` | `tools/oap/ci-parity-check/src/lib.rs` |
| `specs/135-fast-ci-as-default` | 20 | `tools/ci-parity-check/src/lib.rs` | `tools/oap/ci-parity-check/src/lib.rs` |

### Group I — `tools/shared/` (path unchanged in target layout)

No `implements:` entries found. The shared crate `open_agentic_spec_types` is owned by spec 000 via `[package.metadata.oap].spec` but no spec frontmatter `implements:` references it. **No I-phase update required.**

### Group J — `grammars/` (Epic 2 I6)

No `implements:` entries found. axiomregent (spec 073) owns the grammar consumption via `build.rs`; the `implements:` list for spec 073 points at `crates/axiomregent` (the crate path, not the grammars). **No frontmatter update required in I6 for grammar paths.**

### Group K — `apps/desktop/` (Epic 2 I7)

27 entries across 16 specs:

| spec | line | implements path | I-phase target |
|---|---|---|---|
| `specs/041-checkpoint-restore-ui` | 16 | `apps/desktop` | `product/apps/desktop` |
| `specs/064-websocket-reconnection` | 23 | `apps/desktop` | `product/apps/desktop` |
| `specs/065-encrypted-keychain` | 24 | `apps/desktop` | `product/apps/desktop` |
| `specs/076-factory-desktop-panel` | 17 | `apps/desktop` | `product/apps/desktop` |
| `specs/083-xray-ui-upgrade` | 23 | `apps/desktop` | `product/apps/desktop` |
| `specs/084-opc-settings-reconciliation` | 21 | `apps/desktop` | `product/apps/desktop` |
| `specs/110-stagecraft-to-opc-factory-trigger` | 32 | `apps/desktop/src-tauri/src/commands/factory.rs` | `product/apps/desktop/src-tauri/src/commands/factory.rs` |
| `specs/110-stagecraft-to-opc-factory-trigger` | 33 | `apps/desktop/src-tauri/src/commands/stagecraft_client.rs` | `product/apps/desktop/...` |
| `specs/110-stagecraft-to-opc-factory-trigger` | 34 | `apps/desktop/src/routes/factory` | `product/apps/desktop/...` |
| `specs/111-org-agent-catalog-sync` | 38 | `apps/desktop/src-tauri/src/commands/agents.rs` | `product/apps/desktop/...` |
| `specs/111-org-agent-catalog-sync` | 39 | `apps/desktop/src-tauri/src/commands/agent_catalog_sync.rs` | `product/apps/desktop/...` |
| `specs/111-org-agent-catalog-sync` | 40 | `apps/desktop/src-tauri/src/commands/stagecraft_client.rs` | `product/apps/desktop/...` |
| `specs/111-org-agent-catalog-sync` | 41 | `apps/desktop/src-tauri/src/commands/sync_client.rs` | `product/apps/desktop/...` |
| `specs/111-org-agent-catalog-sync` | 42 | `apps/desktop/src-tauri/src/lib.rs` | `product/apps/desktop/...` |
| `specs/112-factory-project-lifecycle` | 43 | `apps/desktop/src-tauri/src/commands/factory_project.rs` | `product/apps/desktop/...` |
| `specs/112-factory-project-lifecycle` | 44 | `apps/desktop/src-tauri/src/commands/keychain.rs` | `product/apps/desktop/...` |
| `specs/112-factory-project-lifecycle` | 45 | `apps/desktop/src/routes/factory/ProjectCockpit.tsx` | `product/apps/desktop/...` |
| `specs/119-project-as-unit-of-governance` | 40 | `apps/desktop` | `product/apps/desktop` |
| `specs/120-factory-extraction-stage` | 42 | `apps/desktop/src-tauri/src/commands/factory.rs` | `product/apps/desktop/...` |
| `specs/120-factory-extraction-stage` | 43 | `apps/desktop/src-tauri/src/commands/stagecraft_client.rs` | `product/apps/desktop/...` |
| `specs/120-factory-extraction-stage` | 44 | `apps/desktop/src/components/factory/ArtifactInspector.tsx` | `product/apps/desktop/...` |
| `specs/122-stakeholder-doc-inversion` | 43 | `apps/desktop/src/components/factory/StageCdReview.tsx` | `product/apps/desktop/...` |
| `specs/123-agent-catalog-org-rescope` | 41 | `apps/desktop/src-tauri/src/commands/agent_catalog_sync.rs` | `product/apps/desktop/...` |
| `specs/124-opc-factory-run-platform-integration` | 29 | `apps/desktop/src-tauri/src/commands/factory.rs` | `product/apps/desktop/...` |
| `specs/126-desktop-agent-picker-ui` | 29 | `apps/desktop/src/components/AgentPicker.tsx` | `product/apps/desktop/...` |
| `specs/126-desktop-agent-picker-ui` | 30 | `apps/desktop/src/lib/agentPicker.ts` | `product/apps/desktop/...` |
| `specs/139-factory-artifact-substrate` | 54 | `apps/desktop/src-tauri/src/commands/factory.rs` | `product/apps/desktop/...` |

### Group L — `packages/` (Epic 2 I7)

4 entries:

| spec | line | implements path | I-phase target |
|---|---|---|---|
| `specs/005-verification-reconciliation-mvp` | 21 | `packages/verification-profiles` | `product/packages/verification-profiles` |
| `specs/046-context-compaction` | 18 | `packages/prompt-assembly` | `product/packages/prompt-assembly` |
| `specs/069-lifecycle-hook-runtime` | 21 | `packages/hookify-rule-engine` | `product/packages/hookify-rule-engine` |
| `specs/110-stagecraft-to-opc-factory-trigger` | 36 | `packages/oap-ctl/src/cli.js` | `product/packages/oap-ctl/src/cli.js` |

### Group M — Root npm files (Epic 2 I7)

No `implements:` entries pointing at root `package.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`, or `package-lock.json`.

### Group N — Root loose docs (Epic 2 I8)

| spec | line | implements path | I-phase target |
|---|---|---|---|
| `specs/151-declarative-cluster-reconciliation` | 31 | `DEVELOPERS.md` | `docs/DEVELOPERS.md` |

### Group O — `build/` → `.derived/` (Epic 2 I9)

| spec | line | implements path | I-phase target |
|---|---|---|---|
| `specs/118-workflow-spec-traceability` | 19 | `build/codebase-index/CODEBASE-INDEX.md` | `.derived/codebase-index/CODEBASE-INDEX.md` |

## Specs with multi-path `implements:` (post-W-05 polymorphism)

W-05 widened `implements:` from scalar to list-of-paths. Multi-path specs that update in this cleanup:

| spec | path count | groups touched |
|---|---|---|
| `specs/003-feature-lifecycle-mvp` | 2 | G (spec-compiler + registry-consumer) |
| `specs/039-feature-id-reconciliation` | ≥ 2 | G (spec-compiler) + crates/featuregraph |
| `specs/091-registry-enrichment` | ≥ 1 | G |
| `specs/102-governed-excellence` | multi | G + others (tools/oap-registry-enrich via `[package.metadata.oap]` only) |
| `specs/110-stagecraft-to-opc-factory-trigger` | 5+ | K (apps/desktop ×3) + L (packages/oap-ctl ×1) + platform |
| `specs/111-org-agent-catalog-sync` | 5 | K (apps/desktop ×5) |
| `specs/112-factory-project-lifecycle` | 3+ | D (factory-contracts/schemas) + K (apps/desktop ×3) |
| `specs/119-project-as-unit-of-governance` | 2+ | A (.specify/contract.md) + K (apps/desktop) |
| `specs/120-factory-extraction-stage` | 3 | K (apps/desktop ×3) |
| `specs/122-stakeholder-doc-inversion` | 2 | H (stakeholder-doc-lint) + K (apps/desktop) |
| `specs/126-desktop-agent-picker-ui` | 2 | K (apps/desktop ×2) |
| `specs/129-granular-package-oap-metadata` | 2 | B (schemas/codebase-index.schema.json) + G (codebase-indexer) |
| `specs/132-constitutional-invariant-freeze` | 2 | C (bootstrap registry.schema.json) + G (spec-compiler) |
| `specs/133-amends-aware-coupling-gate` | 3 | B (schemas/codebase-index.schema.json) + G (spec-code-coupling-check + codebase-indexer) |
| `specs/151-declarative-cluster-reconciliation` | 3+ | G (spec-compiler/src/lib.rs + tests/v004…) + N (DEVELOPERS.md) |

Multi-path specs require careful per-entry handling in their I-phase: each `- path:` line updates independently, and a single spec may touch multiple I-phases. The W-05 list polymorphism makes this safe — every entry is replaced in-place.

## Per-phase counts (summary)

| Epic 2 phase | concern | spec frontmatter rows to update |
|---|---|---|
| I3 | Standards content graduation (`.specify/` → `standards/spec/`) | 1 (spec 119) |
| I4 | Schema co-location | 5 (specs 112, 129, 132, 133 ×2) |
| I5 | Tools restructure | 39 frontmatter rows (G: 32, H: 7) across 38 distinct specs |
| I6 | Grammars vendor move | 0 |
| I7 | Product layer | 31 rows (K: 27, L: 4) across ≈ 17 specs |
| I8 | Docs consolidation | 1 (spec 151) |
| I9 | `build/` → `.derived/` rename | 1 (spec 118) |
| **Total** | | **77** frontmatter rows across ≈ 60 distinct specs |

(Counts may shift slightly depending on whether the `package.metadata.oap` Cargo-side spec attribution counts here — it does not, because that's not a frontmatter `implements:` row; D2 enumerates the Cargo side. The 77 above reflects spec-frontmatter rows only.)

## Spec-127 coupling-gate readiness

Spec 127 fires when a diff touches a path declared in any spec's `implements:` list but does not also modify that spec's `spec.md`. Epic 2's per-phase commit shape MUST land the code move and the `implements:` update in **the same commit** to avoid the gate firing.

Special cases:

- **I5 (tools restructure)** is the largest spec-update batch. The 39 frontmatter rows above can be batched into one commit alongside the directory-move + Cargo.toml `members` update, but the commit will be large (~ 40 frontmatter lines + git-mv operations + workspace member array). D6 produces the workflow/Makefile update list that lands in the same commit.
- **I7 (product layer)** is the second-largest batch. 31 frontmatter rows + the `git mv apps/desktop product/apps/desktop` + `git mv packages product/packages` + the loader update in `tools/codebase-indexer/src/{lib,manifest}.rs`. Atomic per master plan.
- **I4 (schema co-location)** is fragile because three groups (B, C, D) co-move; per-spec updates touch 5 frontmatter rows total. Atomic across all schema moves.
- **Specs with multi-phase touch** (e.g., 119 touches I3 + I7; 132 touches I4 + I5; 133 touches I4 + I5; 151 touches I5 + I8) — operator decides whether to (a) split frontmatter into per-phase sub-commits, or (b) update both entries in whichever phase lands later. Recommendation: update each phase's entry in its own commit; coupling-gate bypass list (`.github/spec-coupling-bypass.txt`) explicitly does not exempt `specs/`. The spec's `implements:` array can carry mixed old + new paths transiently if the gate's behavior allows; D7's "W-NNN follows producer-side enforcement" lessons may apply here too.

## Open questions (surface for operator triage)

1. **Spec `031-registry-consumer-list-ids-only-contract-mvp` and other registry-consumer specs (007–031)** all carry identical `- path: tools/registry-consumer`. Confirm batch-update strategy: single sed-style replace across all 25+ specs, or per-spec inspection? Recommendation: scripted bulk update with mandatory operator review of diff before commit.
2. **Specs touching multiple I-phases** (119, 132, 133, 151, 122 has H + K) — split into per-phase frontmatter sub-commits or land in last-touching phase? See I5/I7 above.
3. **Multi-path specs in I7 (apps/desktop sub-paths)** — when `apps/desktop` is split into `product/apps/desktop`, all sub-path entries (e.g., `apps/desktop/src-tauri/src/commands/factory.rs`) update mechanically by prefixing `product/`. Confirm sed-pattern strategy is acceptable.
4. **Spec 081 `implements: []`** — explicitly empty list. No update required; flag for D7 / D10 in case the empty-list shape needs the V-NNN audit treatment.

## Phase coupling-gate readiness summary

- **Total spec rows to update:** ≈ 77 across I3–I9
- **Per-phase counts:** I3:1, I4:5, I5:39, I6:0, I7:31, I8:1, I9:1
- **Specs touching multiple phases:** 5 (119, 132, 133, 151, 122)
- **Estimated complexity:** **medium-high** — the mechanical work is sed-replaceable, but per-spec operator review is mandatory because every `implements:` change is a load-bearing spec amendment under spec 127.
