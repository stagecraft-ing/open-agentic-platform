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

---

## I0 refresh corrections (post-activation reconciliation)

> **Authored:** 2026-05-20 in the Epic 2 I0 pre-flight, after the
> side-quest II activation (commit `131392ff`) excised list-form
> `implements:` from the corpus and migrated path claims to the
> relationship-graph fields.
>
> **Scope:** Two gaps surfaced by Epic 2 I0 against this document:
> a *shape gap* (D4's pre-activation `implements:` line references no
> longer point at `- path:` items in the same form) and a *coverage
> gap* (D4 enumerates only `implements:`; the load-bearing
> path-claiming surface is now distributed across six
> relationship-graph fields per spec 130 §1).
>
> This appendix is **append-only**. The main D4 tables above are
> preserved as the pre-activation snapshot. Epic 2 I3–I9 sweeps
> consume *this appendix* as the authoritative per-phase manifest
> for relationship-graph reference updates; the main tables remain
> useful as the path-string and per-phase-target mapping (each path
> still moves to the same target post-activation).

### Shape gap — D4 line-number references are pre-activation

Every `line` column in the main D4 tables above points at a line
that previously carried a `- path:` entry inside a list-form
`implements:` block. Side-quest II excised list-form `implements:`
in commit `6e326463`; the path strings now live under the
relationship-graph fields (`establishes:`, `extends:`, `refines:`,
`co_authority:`, `supersedes:`, `constrains:` per spec 130 §1).

Concretely:

| spec | D4 line | post-activation line | post-activation field |
|---|---|---|---|
| 119-project-as-unit-of-governance | 41 (`.specify/contract.md`) | 35 | `establishes:` |
| 129-granular-package-oap-metadata | 18 (`schemas/codebase-index.schema.json`) | 20 | `extends:` |
| 132-constitutional-invariant-freeze | 21 (`specs/000-bootstrap-spec-system/contracts/registry.schema.json`) | 27 | `constrains:` |
| 133-amends-aware-coupling-gate | 23 (`schemas/codebase-index.schema.json`) | 22 | `extends:` |
| 112-factory-project-lifecycle | 33 (`crates/factory-contracts/schemas/`) | 48 | `extends:` |
| 118-workflow-spec-traceability | 18, 19 | 19, 20 | `extends:` |
| 127-spec-code-coupling-gate | 23 | 24 | `extends:` |
| 130-spec-coupling-primary-owner | 18 | 23 | `extends:` |
| 133-amends-aware-coupling-gate | 21, 22 | 17, 22 | `establishes:`, `extends:` |
| 151-declarative-cluster-reconciliation | 29, 30, 31 | 21, 37, varies | mix of `establishes:`/`extends:` |

The per-phase sweep operations target the path *string* (which is
stable across activation) under whichever relationship-graph field
currently carries it, not the historic `implements:` line number.

### Coverage gap — D4 enumerates only `implements:`

D4's main tables count only paths that lived under list-form
`implements:` pre-activation. Side-quest II established the
relationship-graph as the load-bearing path-claiming surface, so
**all** path-bearing relationship-graph fields are in scope for
Epic 2 sweeps. The next section enumerates the full surface.

## relationship-graph fields locked at I0

Per operator confirmation (2026-05-20, in response to the I0
trip-wire surfacing `supersedes:` as a path-bearing field outside
the original 5-field hypothesis), the locked **six-field**
path-bearing model is:

### Gate-enforced fields (codebase-indexer `spec_scanner.rs:142-215` reads these into `implementing_paths`; coupling gate consults this set)

| field | shape | authority |
|---|---|---|
| `establishes:` | flat list of path strings | whole-file |
| `extends:` | list of `{spec:, paths: [...], nature:}` items | whole-file |
| `refines:` | list of `{paths: [...], aspect:, refines_specs:}` items | whole-file |
| `co_authority:` | list of `{paths: [...], section:, with_specs:}` items | **section-aware** |

### Corpus-consistency fields (gate does NOT enforce; sweep updates them so the spec corpus stays internally consistent post-move)

| field | shape | notes |
|---|---|---|
| `supersedes:` | list of `{spec:, scope:, paths: [...], rationale:}` items | `paths:` required for `scope: partial`; included for `scope: full` in 073; legacy ID-list form desugars to `scope: full` with no paths (spec 130 §2.4) |
| `constrains:` | list of `{spec:?, kind:, paths: [...]}` items | `paths:` present for `kind: invariant-freeze` (specs 130, 132); `target_specs:` (no paths) for `kind: delivery-sequencing` / `sequencing-plan` (specs 078, 089) — those rows are spec-ID only and out of scope for path sweeps |

**Authoritative source for the eight-field relationship-graph
model:** spec 130 §1 ("Eight frontmatter fields — establishes,
extends, refines, supersedes, amends, co_authority, constrains,
origin — encode how specs relate to code and to each other").

**Authoritative source for the gate-enforced subset:**
`tools/codebase-indexer/src/spec_scanner.rs:142-215` (the `parse_implements`
helper) — only the four gate-enforced fields are read; the indexer
does not currently read `supersedes:` or `constrains:` into the
`implementing_paths` view that the coupling gate consumes.

## relationship-graph fields enumerated but not path-bearing

The following relationship-graph fields exist in the spec model
per spec 130 but do not currently carry code paths in the corpus
post-activation. **No path-string sweeps for these fields** during
Epic 2:

| field | corpus shape | rationale |
|---|---|---|
| `amends:` | spec-ID list only (e.g., `amends: ["000", "087"]`) — 17 specs use this form | Spec 130 §2.5 admits an object form `{spec:, paths:, change_type:}`, but no corpus spec currently uses it; all 17 instances are scalar ID lists. The Epic 2 prompt's §Post-activation field model claimed `amends:` was "paths arrays whole-file" — that turned out to be a documentation inaccuracy; corpus reality is ID-only. |
| `origin:` | metadata-only (e.g., `origin: { retroactive: true }`) — 7 specs use this form | Bootstrap-only marker per spec 130 §2.8; carries no code paths. |

## relationship-graph reference enumeration (Epic 2 in-scope)

**159 path-bearing entries** in scope for Epic 2 sweeps, classified
by I-phase and D1 path-group. Format: `spec_id:line | field | path[ | section=<section>] -> target_path`. Section anchors appear only on
`co_authority:` entries.

### Phase I3 / Group A — `.specify/` (1 entry)

| spec | line | field | path | section | target |
|---|---|---|---|---|---|
| 119-project-as-unit-of-governance | 35 | `establishes` | `.specify/contract.md` | — | `standards/spec/contract.md` |

### Phase I4 / Group B — Root `schemas/` (3 entries)

| spec | line | field | path | section | target |
|---|---|---|---|---|---|
| 054-agent-frontmatter-schema | 33 | `establishes` | `schemas/agent-frontmatter.schema.json` | — | `standards/schemas/frontmatter/agent-frontmatter.schema.json` |
| 101-codebase-index-mvp | 33 | `establishes` | `schemas/codebase-index.schema.json` | — | `standards/schemas/spec-spine/codebase-index.schema.json` |
| 129-granular-package-oap-metadata | 20 | `extends` | `schemas/codebase-index.schema.json` | — | `standards/schemas/spec-spine/codebase-index.schema.json` |

### Phase I4 / Group C — `specs/000-bootstrap-spec-system/contracts/` (5 entries)

| spec | line | field | path | section | target |
|---|---|---|---|---|---|
| 039-feature-id-reconciliation | 20 | `extends` | `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | — | `standards/schemas/spec-spine/registry.schema.json` |
| 130-spec-coupling-primary-owner | 19 | `establishes` | `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | — | `standards/schemas/spec-spine/registry.schema.json` |
| 130-spec-coupling-primary-owner | 32 | `constrains` | `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | — | `standards/schemas/spec-spine/registry.schema.json` |
| 132-constitutional-invariant-freeze | 27 | `constrains` | `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | — | `standards/schemas/spec-spine/registry.schema.json` |
| 147-spec-kind-grammar | 41 | `extends` | `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | — | `standards/schemas/spec-spine/registry.schema.json` |

### Phase I4 / Group D — `crates/factory-contracts/schemas/` (1 entry)

| spec | line | field | path | section | target |
|---|---|---|---|---|---|
| 112-factory-project-lifecycle | 48 | `extends` | `crates/factory-contracts/schemas/` | — | `standards/schemas/factory/` |

### Phase I5 / Group G — Spec-spine tools (73 entries)

The 73 entries cover the `tools/{spec-compiler,registry-consumer,codebase-indexer,spec-lint,spec-code-coupling-check}` family. The path-suffix is preserved through the move; the prefix change is `tools/<tool>` → `tools/spec-spine/<tool>`. Two `co_authority:` entries are included (specs 133:26 and 152:27, both targeting `tools/spec-code-coupling-check/src/lib.rs` at sections `authority-derivation` and `section-matching` respectively).

Full list (sorted by spec):

| spec | line | field | path | section | target |
|---|---|---|---|---|---|
| 001-spec-compiler-mvp | 24 | `establishes` | `tools/spec-compiler/src/lib.rs` | — | `tools/spec-spine/spec-compiler/src/lib.rs` |
| 001-spec-compiler-mvp | 25 | `establishes` | `tools/spec-compiler/src/main.rs` | — | `tools/spec-spine/spec-compiler/src/main.rs` |
| 002-registry-consumer-mvp | 17 | `establishes` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 002-registry-consumer-mvp | 18 | `establishes` | `tools/registry-consumer/src/lib.rs` | — | `tools/spec-spine/registry-consumer/src/lib.rs` |
| 002-registry-consumer-mvp | 19 | `establishes` | `tools/registry-consumer/src/main.rs` | — | `tools/spec-spine/registry-consumer/src/main.rs` |
| 002-registry-consumer-mvp | 20 | `establishes` | `tools/registry-consumer/Cargo.toml` | — | `tools/spec-spine/registry-consumer/Cargo.toml` |
| 003-feature-lifecycle-mvp | 19 | `refines` | `tools/spec-compiler/src/lib.rs` | — | `tools/spec-spine/spec-compiler/src/lib.rs` |
| 003-feature-lifecycle-mvp | 20 | `refines` | `tools/registry-consumer/src/lib.rs` | — | `tools/spec-spine/registry-consumer/src/lib.rs` |
| 007-registry-consumer-status-report-mvp | 19 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 008-registry-consumer-status-report-json-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 009-registry-consumer-status-report-nonzero-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 010-registry-consumer-status-report-json-contract-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 010-registry-consumer-status-report-json-contract-mvp | 22 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 011-registry-consumer-status-report-status-filter-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 012-registry-consumer-list-json-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 013-registry-consumer-show-json-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 014-registry-consumer-show-compact-json-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 015-registry-consumer-list-compact-json-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 016-registry-consumer-status-report-compact-json-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 017-registry-consumer-shared-json-serialization-helper-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 017-registry-consumer-shared-json-serialization-helper-mvp | 22 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 018-registry-consumer-list-show-json-contract-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 018-registry-consumer-list-show-json-contract-mvp | 22 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 019-registry-consumer-readme-examples-contract-mvp | 19 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 019-registry-consumer-readme-examples-contract-mvp | 23 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 020-registry-consumer-error-contract-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 020-registry-consumer-error-contract-mvp | 22 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 021-registry-consumer-field-shape-invariants-contract-mvp | 19 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 021-registry-consumer-field-shape-invariants-contract-mvp | 23 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 022-registry-consumer-help-usage-contract-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 022-registry-consumer-help-usage-contract-mvp | 22 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 023-registry-consumer-flag-conflict-argument-validation-contract-mvp | 19 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 023-registry-consumer-flag-conflict-argument-validation-contract-mvp | 23 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 024-registry-consumer-version-banner-contract-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 024-registry-consumer-version-banner-contract-mvp | 22 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 025-registry-consumer-default-path-contract-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 025-registry-consumer-default-path-contract-mvp | 22 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 026-registry-consumer-allow-invalid-contract-mvp | 19 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 026-registry-consumer-allow-invalid-contract-mvp | 23 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 027-registry-consumer-sorting-order-contract-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 027-registry-consumer-sorting-order-contract-mvp | 22 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 028-registry-consumer-channel-discipline-contract-mvp | 19 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 028-registry-consumer-channel-discipline-contract-mvp | 23 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 029-registry-consumer-contract-governance-gate-mvp | 19 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 029-registry-consumer-contract-governance-gate-mvp | 23 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 030-registry-consumer-internal-output-exit-refactor-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 031-registry-consumer-list-ids-only-contract-mvp | 18 | `extends` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 031-registry-consumer-list-ids-only-contract-mvp | 22 | `refines` | `tools/registry-consumer` | — | `tools/spec-spine/registry-consumer` |
| 039-feature-id-reconciliation | 19 | `extends` | `tools/spec-compiler/src/lib.rs` | — | `tools/spec-spine/spec-compiler/src/lib.rs` |
| 091-registry-enrichment | 20 | `extends` | `tools/spec-compiler/src/lib.rs` | — | `tools/spec-spine/spec-compiler/src/lib.rs` |
| 101-codebase-index-mvp | 28 | `establishes` | `tools/codebase-indexer/src/main.rs` | — | `tools/spec-spine/codebase-indexer/src/main.rs` |
| 101-codebase-index-mvp | 29 | `establishes` | `tools/codebase-indexer/src/lib.rs` | — | `tools/spec-spine/codebase-indexer/src/lib.rs` |
| 101-codebase-index-mvp | 30 | `establishes` | `tools/codebase-indexer/src/spec_scanner.rs` | — | `tools/spec-spine/codebase-indexer/src/spec_scanner.rs` |
| 101-codebase-index-mvp | 31 | `establishes` | `tools/codebase-indexer/src/manifest.rs` | — | `tools/spec-spine/codebase-indexer/src/manifest.rs` |
| 102-governed-excellence | 53 | `extends` | `tools/spec-compiler/src/lib.rs` | — | `tools/spec-spine/spec-compiler/src/lib.rs` |
| 118-workflow-spec-traceability | 19 | `extends` | `tools/codebase-indexer` | — | `tools/spec-spine/codebase-indexer` |
| 127-spec-code-coupling-gate | 24 | `extends` | `tools/spec-code-coupling-check` | — | `tools/spec-spine/spec-code-coupling-check` |
| 129-granular-package-oap-metadata | 19 | `extends` | `tools/codebase-indexer` | — | `tools/spec-spine/codebase-indexer` |
| 130-spec-coupling-primary-owner | 23 | `extends` | `tools/spec-compiler/src/lib.rs` | — | `tools/spec-spine/spec-compiler/src/lib.rs` |
| 130-spec-coupling-primary-owner | 27 | `extends` | `tools/codebase-indexer/src/spec_scanner.rs` | — | `tools/spec-spine/codebase-indexer/src/spec_scanner.rs` |
| 132-constitutional-invariant-freeze | 22 | `extends` | `tools/spec-compiler` | — | `tools/spec-spine/spec-compiler` |
| 133-amends-aware-coupling-gate | 17 | `establishes` | `tools/spec-code-coupling-check/src/lib.rs` | — | `tools/spec-spine/spec-code-coupling-check/src/lib.rs` |
| 133-amends-aware-coupling-gate | 18 | `establishes` | `tools/spec-code-coupling-check/src/main.rs` | — | `tools/spec-spine/spec-code-coupling-check/src/main.rs` |
| 133-amends-aware-coupling-gate | 22 | `extends` | `tools/codebase-indexer/src/lib.rs` | — | `tools/spec-spine/codebase-indexer/src/lib.rs` |
| 133-amends-aware-coupling-gate | 26 | `co_authority` | `tools/spec-code-coupling-check/src/lib.rs` | `authority-derivation` | `tools/spec-spine/spec-code-coupling-check/src/lib.rs` |
| 147-spec-kind-grammar | 29 | `extends` | `tools/spec-compiler/src/lib.rs` | — | `tools/spec-spine/spec-compiler/src/lib.rs` |
| 147-spec-kind-grammar | 33 | `extends` | `tools/spec-lint/src/lib.rs` | — | `tools/spec-spine/spec-lint/src/lib.rs` |
| 147-spec-kind-grammar | 37 | `extends` | `tools/codebase-indexer/src/spec_scanner.rs` | — | `tools/spec-spine/codebase-indexer/src/spec_scanner.rs` |
| 147-spec-kind-grammar | 45 | `extends` | `tools/codebase-indexer/src/lib.rs` | — | `tools/spec-spine/codebase-indexer/src/lib.rs` |
| 151-declarative-cluster-reconciliation | 21 | `establishes` | `tools/spec-compiler/tests/v004_consolidation_excludes.rs` | — | `tools/spec-spine/spec-compiler/tests/v004_consolidation_excludes.rs` |
| 151-declarative-cluster-reconciliation | 37 | `extends` | `tools/spec-compiler/src/lib.rs` | — | `tools/spec-spine/spec-compiler/src/lib.rs` |
| 152-path-co-authority | 23 | `extends` | `tools/spec-code-coupling-check/src/lib.rs` | — | `tools/spec-spine/spec-code-coupling-check/src/lib.rs` |
| 152-path-co-authority | 27 | `co_authority` | `tools/spec-code-coupling-check/src/lib.rs` | `section-matching` | `tools/spec-spine/spec-code-coupling-check/src/lib.rs` |

### Phase I5 / Group H — OAP tools (8 entries)

| spec | line | field | path | section | target |
|---|---|---|---|---|---|
| 104-makefile-ci-parity-contract | 17 | `establishes` | `tools/ci-parity-check/src/lib.rs` | — | `tools/oap/ci-parity-check/src/lib.rs` |
| 104-makefile-ci-parity-contract | 18 | `establishes` | `tools/ci-parity-check/src/main.rs` | — | `tools/oap/ci-parity-check/src/main.rs` |
| 105-scripts-to-binaries-migration | 18 | `establishes` | `tools/adapter-scopes-compiler/src/main.rs` | — | `tools/oap/adapter-scopes-compiler/src/main.rs` |
| 105-scripts-to-binaries-migration | 19 | `establishes` | `tools/adapter-scopes-compiler/src/lib.rs` | — | `tools/oap/adapter-scopes-compiler/src/lib.rs` |
| 122-stakeholder-doc-inversion | 40 | `establishes` | `tools/stakeholder-doc-lint/Cargo.toml` | — | `tools/oap/stakeholder-doc-lint/Cargo.toml` |
| 131-adversarial-prompt-refusal-policy | 23 | `extends` | `tools/policy-compiler` | — | `tools/oap/policy-compiler` |
| 134-fast-local-ci-mode | 31 | `extends` | `tools/ci-parity-check/src/lib.rs` | — | `tools/oap/ci-parity-check/src/lib.rs` |
| 135-fast-ci-as-default | 33 | `extends` | `tools/ci-parity-check/src/lib.rs` | — | `tools/oap/ci-parity-check/src/lib.rs` |

### Phase I7 / Group K — `apps/desktop/` (52 entries)

| spec | line | field | path | section | target |
|---|---|---|---|---|---|
| 033-axiomregent-activation | 24 | `establishes` | `apps/desktop/src-tauri/src/commands/sidecars.rs` | — | `product/apps/desktop/src-tauri/src/commands/sidecars.rs` |
| 035-agent-governed-execution | 22 | `extends` | `apps/desktop/src-tauri/src/commands/agents.rs` | — | `product/apps/desktop/src-tauri/src/commands/agents.rs` |
| 035-agent-governed-execution | 23 | `extends` | `apps/desktop/src-tauri/src/commands/claude.rs` | — | `product/apps/desktop/src-tauri/src/commands/claude.rs` |
| 037-cross-platform-axiomregent | 21 | `extends` | `apps/desktop/src-tauri/binaries` | — | `product/apps/desktop/src-tauri/binaries` |
| 038-titor-tauri-command-wiring | 14 | `establishes` | `apps/desktop/src-tauri/src/commands/titor.rs` | — | `product/apps/desktop/src-tauri/src/commands/titor.rs` |
| 040-blockoli-semantic-search-wiring | 14 | `establishes` | `apps/desktop/src-tauri/src/commands/search.rs` | — | `product/apps/desktop/src-tauri/src/commands/search.rs` |
| 041-checkpoint-restore-ui | 16 | `establishes` | `apps/desktop/src/features/checkpoint/CheckpointSurface.tsx` | — | `product/apps/desktop/src/features/checkpoint/CheckpointSurface.tsx` |
| 041-checkpoint-restore-ui | 17 | `establishes` | `apps/desktop/src/features/checkpoint/useCheckpointFlow.ts` | — | `product/apps/desktop/src/features/checkpoint/useCheckpointFlow.ts` |
| 041-checkpoint-restore-ui | 18 | `establishes` | `apps/desktop/src/features/checkpoint/types.ts` | — | `product/apps/desktop/src/features/checkpoint/types.ts` |
| 045-claude-code-sdk-bridge | 17 | `extends` | `apps/desktop/src-tauri/src/commands/claude.rs` | — | `product/apps/desktop/src-tauri/src/commands/claude.rs` |
| 051-worktree-agents | 21 | `establishes` | `apps/desktop/src-tauri/src/commands/worktree_agents.rs` | — | `product/apps/desktop/src-tauri/src/commands/worktree_agents.rs` |
| 057-notification-system | 20 | `establishes` | `apps/desktop/src/lib/notificationOrchestrator.ts` | — | `product/apps/desktop/src/lib/notificationOrchestrator.ts` |
| 057-notification-system | 21 | `establishes` | `apps/desktop/src/lib/notificationChannels.ts` | — | `product/apps/desktop/src/lib/notificationChannels.ts` |
| 058-file-mention-system | 21 | `establishes` | `apps/desktop/src/components/FileMentionAutocomplete.tsx` | — | `product/apps/desktop/src/components/FileMentionAutocomplete.tsx` |
| 058-file-mention-system | 22 | `establishes` | `apps/desktop/src/lib/fileMentionSystem.ts` | — | `product/apps/desktop/src/lib/fileMentionSystem.ts` |
| 059-git-panel | 20 | `establishes` | `apps/desktop/src/components/GitPanel.tsx` | — | `product/apps/desktop/src/components/GitPanel.tsx` |
| 059-git-panel | 21 | `establishes` | `apps/desktop/src-tauri/src/commands/git.rs` | — | `product/apps/desktop/src-tauri/src/commands/git.rs` |
| 060-panel-event-bus | 24 | `establishes` | `apps/desktop/src/lib/panelEventBus.ts` | — | `product/apps/desktop/src/lib/panelEventBus.ts` |
| 064-websocket-reconnection | 23 | `establishes` | `apps/desktop/src/lib/wsReconnection.ts` | — | `product/apps/desktop/src/lib/wsReconnection.ts` |
| 064-websocket-reconnection | 27 | `extends` | `apps/desktop` | — | `product/apps/desktop` |
| 065-encrypted-keychain | 24 | `establishes` | `apps/desktop/src-tauri/src/keychain.rs` | — | `product/apps/desktop/src-tauri/src/keychain.rs` |
| 065-encrypted-keychain | 25 | `establishes` | `apps/desktop/src/components/CredentialPicker.tsx` | — | `product/apps/desktop/src/components/CredentialPicker.tsx` |
| 065-encrypted-keychain | 29 | `extends` | `apps/desktop` | — | `product/apps/desktop` |
| 073-axiomregent-unification | 22 | `supersedes` | `apps/desktop/src-tauri/src/commands/titor.rs` | — | `product/apps/desktop/src-tauri/src/commands/titor.rs` |
| 073-axiomregent-unification | 26 | `supersedes` | `apps/desktop/src-tauri/src/commands/search.rs` | — | `product/apps/desktop/src-tauri/src/commands/search.rs` |
| 076-factory-desktop-panel | 17 | `establishes` | `apps/desktop/src/components/FactoryPipelinePanel.tsx` | — | `product/apps/desktop/src/components/FactoryPipelinePanel.tsx` |
| 076-factory-desktop-panel | 21 | `extends` | `apps/desktop` | — | `product/apps/desktop` |
| 083-xray-ui-upgrade | 25 | `extends` | `apps/desktop` | — | `product/apps/desktop` |
| 084-opc-settings-reconciliation | 21 | `establishes` | `apps/desktop/src/lib/settingsManager.ts` | — | `product/apps/desktop/src/lib/settingsManager.ts` |
| 084-opc-settings-reconciliation | 25 | `extends` | `apps/desktop/src-tauri/src/commands/claude.rs` | — | `product/apps/desktop/src-tauri/src/commands/claude.rs` |
| 084-opc-settings-reconciliation | 26 | `extends` | `apps/desktop/src/lib/api.ts` | — | `product/apps/desktop/src/lib/api.ts` |
| 084-opc-settings-reconciliation | 27 | `extends` | `apps/desktop/src/components/ProjectSettings.tsx` | — | `product/apps/desktop/src/components/ProjectSettings.tsx` |
| 085-remote-control-cli | 26 | `extends` | `apps/desktop/src-tauri/src/web_server.rs` | — | `product/apps/desktop/src-tauri/src/web_server.rs` |
| 090-governance-non-optionality | 31 | `extends` | `apps/desktop/src-tauri/src/commands/orchestrator.rs` | — | `product/apps/desktop/src-tauri/src/commands/orchestrator.rs` |
| 100-post-convergence-remediation | 15 | `refines` | `apps/desktop/src-tauri/tauri.conf.json` | — | `product/apps/desktop/src-tauri/tauri.conf.json` |
| 110-stagecraft-to-opc-factory-trigger | 29 | `establishes` | `apps/desktop/src-tauri/src/commands/sync_client.rs` | — | `product/apps/desktop/src-tauri/src/commands/sync_client.rs` |
| 111-org-agent-catalog-sync | 38 | `establishes` | `apps/desktop/src-tauri/src/commands/agents.rs` | — | `product/apps/desktop/src-tauri/src/commands/agents.rs` |
| 111-org-agent-catalog-sync | 39 | `establishes` | `apps/desktop/src-tauri/src/commands/agent_catalog_sync.rs` | — | `product/apps/desktop/src-tauri/src/commands/agent_catalog_sync.rs` |
| 111-org-agent-catalog-sync | 40 | `establishes` | `apps/desktop/src-tauri/src/commands/stagecraft_client.rs` | — | `product/apps/desktop/src-tauri/src/commands/stagecraft_client.rs` |
| 112-factory-project-lifecycle | 41 | `establishes` | `apps/desktop/src-tauri/src/commands/factory_project.rs` | — | `product/apps/desktop/src-tauri/src/commands/factory_project.rs` |
| 112-factory-project-lifecycle | 42 | `establishes` | `apps/desktop/src-tauri/src/commands/keychain.rs` | — | `product/apps/desktop/src-tauri/src/commands/keychain.rs` |
| 112-factory-project-lifecycle | 43 | `establishes` | `apps/desktop/src/routes/factory/ProjectCockpit.tsx` | — | `product/apps/desktop/src/routes/factory/ProjectCockpit.tsx` |
| 119-project-as-unit-of-governance | 60 | `refines` | `apps/desktop` | — | `product/apps/desktop` |
| 120-factory-extraction-stage | 38 | `establishes` | `apps/desktop/src/components/factory/ArtifactInspector.tsx` | — | `product/apps/desktop/src/components/factory/ArtifactInspector.tsx` |
| 120-factory-extraction-stage | 48 | `extends` | `apps/desktop/src-tauri/src/commands/factory.rs` | — | `product/apps/desktop/src-tauri/src/commands/factory.rs` |
| 120-factory-extraction-stage | 49 | `extends` | `apps/desktop/src-tauri/src/commands/stagecraft_client.rs` | — | `product/apps/desktop/src-tauri/src/commands/stagecraft_client.rs` |
| 122-stakeholder-doc-inversion | 41 | `establishes` | `apps/desktop/src/components/factory/StageCdReview.tsx` | — | `product/apps/desktop/src/components/factory/StageCdReview.tsx` |
| 123-agent-catalog-org-rescope | 45 | `extends` | `apps/desktop/src-tauri/src/commands/agent_catalog_sync.rs` | — | `product/apps/desktop/src-tauri/src/commands/agent_catalog_sync.rs` |
| 124-opc-factory-run-platform-integration | 34 | `extends` | `apps/desktop/src-tauri/src/commands/factory.rs` | — | `product/apps/desktop/src-tauri/src/commands/factory.rs` |
| 126-desktop-agent-picker-ui | 31 | `extends` | `apps/desktop/src/components/AgentPicker.tsx` | — | `product/apps/desktop/src/components/AgentPicker.tsx` |
| 126-desktop-agent-picker-ui | 32 | `extends` | `apps/desktop/src/lib/agentPicker.ts` | — | `product/apps/desktop/src/lib/agentPicker.ts` |
| 139-factory-artifact-substrate | 68 | `refines` | `apps/desktop/src-tauri/src/commands/factory.rs` | — | `product/apps/desktop/src-tauri/src/commands/factory.rs` |

### Phase I7 / Group L — `packages/` (12 entries)

| spec | line | field | path | section | target |
|---|---|---|---|---|---|
| 045-claude-code-sdk-bridge | 13 | `establishes` | `packages/claude-code-bridge` | — | `product/packages/claude-code-bridge` |
| 046-context-compaction | 18 | `establishes` | `packages/prompt-assembly` | — | `product/packages/prompt-assembly` |
| 048-hookify-rule-engine | 21 | `establishes` | `packages/hookify-rule-engine` | — | `product/packages/hookify-rule-engine` |
| 049-permission-system | 21 | `establishes` | `packages/permission-system` | — | `product/packages/permission-system` |
| 050-tool-renderer-system | 20 | `establishes` | `packages/tool-renderer` | — | `product/packages/tool-renderer` |
| 051-worktree-agents | 20 | `establishes` | `packages/worktree-agents` | — | `product/packages/worktree-agents` |
| 056-session-memory | 21 | `establishes` | `packages/memory-mcp` | — | `product/packages/memory-mcp` |
| 066-vscode-extension | 22 | `establishes` | `packages/vscode-extension` | — | `product/packages/vscode-extension` |
| 069-lifecycle-hook-runtime | 23 | `extends` | `packages/hookify-rule-engine` | — | `product/packages/hookify-rule-engine` |
| 085-remote-control-cli | 22 | `establishes` | `packages/oap-ctl/src/cli.js` | — | `product/packages/oap-ctl/src/cli.js` |
| 087-unified-workspace-architecture | 37 | `establishes` | `packages/project-sdk` | — | `product/packages/project-sdk` |
| 110-stagecraft-to-opc-factory-trigger | 46 | `extends` | `packages/oap-ctl/src/cli.js` | — | `product/packages/oap-ctl/src/cli.js` |

### Phase I8 / Group N — Root loose docs (1 entry)

| spec | line | field | path | section | target |
|---|---|---|---|---|---|
| 086-open-source-launch | 23 | `establishes` | `CONTRIBUTING.md` | — | `docs/CONTRIBUTING.md` |

### Phase I9 / Group O — `build/` → `.derived/` (3 entries)

| spec | line | field | path | section | target |
|---|---|---|---|---|---|
| 001-spec-compiler-mvp | 26 | `establishes` | `build/spec-registry/registry.json` | — | `.derived/spec-registry/registry.json` |
| 101-codebase-index-mvp | 32 | `establishes` | `build/codebase-index/index.json` | — | `.derived/codebase-index/index.json` |
| 118-workflow-spec-traceability | 20 | `extends` | `build/codebase-index/CODEBASE-INDEX.md` | — | `.derived/codebase-index/CODEBASE-INDEX.md` |

### Per-phase counts (I0 refresh, relationship-graph fields)

| Phase | Group(s) | Entries | Field breakdown |
|---|---|---|---|
| I3 | A | 1 | establishes=1 |
| I4 | B | 3 | establishes=2, extends=1 |
| I4 | C | 5 | establishes=1, extends=2, constrains=2 |
| I4 | D | 1 | extends=1 |
| I5 | G | 73 | establishes=15, extends=39, refines=17, co_authority=2 |
| I5 | H | 8 | establishes=5, extends=3 |
| I7 | K | 52 | establishes=28, extends=18, refines=4, supersedes=2 |
| I7 | L | 12 | establishes=10, extends=2 |
| I8 | N | 1 | establishes=1 |
| I9 | O | 3 | establishes=2, extends=1 |
| **Total** | — | **159** | — |

### Per-phase sweep targets (sorted by I-phase, summary form)

| I-phase | spec count | path-bearing rows | fields touched |
|---|---|---|---|
| I3 | 1 | 1 | establishes |
| I4 | 6 (specs 039, 054, 101, 112, 129, 130, 132, 147 — some overlap) | 10 (B=3, C=5, D=1, plus C's constrains=2) | establishes, extends, constrains |
| I5 | ≈ 38 | 81 (G=73, H=8) | establishes, extends, refines, co_authority |
| I6 | 0 | 0 | — |
| I7 | ≈ 27 | 64 (K=52, L=12) | establishes, extends, refines, supersedes |
| I8 | 1 | 1 | establishes |
| I9 | 3 | 3 | establishes, extends |
| **Total** | — | **159** | — |

### Per-phase corpus-consistency sweep targets (`supersedes:`/`constrains:` — not gate-enforced)

Per operator resolution, these entries must be updated by the
moving phase to keep the spec corpus internally consistent, even
though the coupling gate does not currently consult them:

| spec | line | field | path | I-phase |
|---|---|---|---|---|
| 073-axiomregent-unification | 22 | `supersedes` | `apps/desktop/src-tauri/src/commands/titor.rs` | I7 |
| 073-axiomregent-unification | 26 | `supersedes` | `apps/desktop/src-tauri/src/commands/search.rs` | I7 |
| 130-spec-coupling-primary-owner | 32 | `constrains` | `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | I4 |
| 132-constitutional-invariant-freeze | 27 | `constrains` | `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | I4 |

### Out-of-scope path-bearing entries (noted, not swept)

The I0 enumeration surfaced **478 additional path-bearing entries**
that point at paths NOT in any Epic 2 move group. Examples (not
exhaustive):

- `crates/...` paths — crates/ root layout is structurally unchanged
  by Epic 2 (I1 consolidates the workspace, not the crate paths).
- `platform/...` paths — `platform/` is read-only per Epic 2
  cross-phase rule 7.
- `factory/...` paths — `factory/` is not in any Epic 2 move group
  (spec 088's `supersedes: factory/upstream-map.yaml` is captured
  here for explicit note; no sweep needed).
- `.claude/...` paths — narrative content owned by I10; not move
  targets.
- Root files referenced but not moved (`SECURITY.md`,
  `CODE_OF_CONDUCT.md`, `CHANGELOG.md`, `docs/ARCHITECTURE.md`).

Explicit notes on supersedes/constrains entries surfaced during
enumeration:

- **Spec 088 `supersedes: factory/upstream-map.yaml`** — `factory/`
  is not in any Epic 2 move group. No sweep needed.
- **Spec 114 `supersedes: platform/services/stagecraft/api/projects/clone.ts`** —
  `platform/` is read-only per cross-phase rule 7. No sweep needed.

### Corpus-consistency check protocol (per operator resolution)

Every Epic 2 phase that moves a path also runs an explicit
corpus-consistency cross-check after the commit:

```
git grep -nE "^(supersedes|constrains):" specs/*/spec.md
```

and confirms no entry still references a pre-move path from that
phase's move set. The end-of-phase reporting block gains:

```
Corpus-consistency check (supersedes/constrains): PASS|FAIL
```

FAIL halts per cross-phase rule 5 (no autonomous resolution).
