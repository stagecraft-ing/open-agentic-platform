# Implementation Manifest (synthesized from D1–D9)

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** Synthesis of `docs/analysis/cleanup/{reference-audit, cargo-workspace-inventory, typescript-workspace-inventory, spec-implements-inventory, schema-duplication-audit, workflow-makefile-inventory, vcode-emission-audit, render-path-decomposition, protocol-drift-resolutions}.md`.

## How Epic 2 reads this document

Each I-phase section below lists:

- **Pre-conditions** — what must hold before the phase fires.
- **Discovery references** — which D-phase document(s) provide the per-move manifest.
- **Atomic operations in order** — what runs, in sequence.
- **Files changed in each commit** — the diff shape.
- **Verification after the phase** — what must pass before declaring the phase complete.
- **Trip-wires** — conditions that halt the phase and surface to operator.

Epic 2 agent must read this document end-to-end before firing any phase.

## I1 — Root Cargo workspace consolidation

**Discovery reference:** [`cargo-workspace-inventory.md`](./cargo-workspace-inventory.md).

**Pre-conditions:**
- `cargo test --workspace --manifest-path crates/Cargo.toml` clean.
- `cargo build` succeeds for each standalone tool.
- Operator has resolved D2 open questions 1 (`apps/desktop` SQLite isolation) and 2 (`platform/services/deployd-api-rs` standalone disposition).

**Operations (in order):**
1. Author root `Cargo.toml` with `[workspace]` block listing all members (default: 18 crates from `crates/Cargo.toml` + 12 tools + `tools/shared/spec-types` = 31; plus `apps/desktop/src-tauri` and `platform/services/deployd-api-rs` per operator decision).
2. Delete `crates/Cargo.toml`'s `[workspace]` block — but **don't delete the file**; it becomes a workspace-member manifest (or is removed entirely if the root workspace lists `crates/*` as a glob). Recommendation: drop `crates/Cargo.toml` since it carries only the `[workspace]` block (no `[package]`).
3. Update Cargo path deps to be workspace-aware (optional — see D2 Open Question 4). Default: leave path deps as `path = ".../<sibling>"`.
4. Delete redundant `Cargo.lock` files (per D2 disposition; default 14 deleted, 1 root lockfile remains).
5. Update `.gitignore` lines `23, 25, 26` (the `!tools/policy-compiler/Cargo.lock` etc. re-includes) — those paths no longer exist post-delete.

**Commit shape:** Single commit `refactor(cleanup): consolidate Rust crates into single workspace`.

**Files changed:**
- New: `Cargo.toml` (root).
- Removed: `crates/Cargo.toml`, 14 `Cargo.lock` files (`tools/*/Cargo.lock`, `tools/shared/spec-types/Cargo.lock`, conditionally `apps/desktop/src-tauri/Cargo.lock` + `platform/services/deployd-api-rs/Cargo.lock`).
- Modified: `.gitignore` (drop the `!tools/.../Cargo.lock` re-includes for deleted lockfiles).

**Verification:**
- `cargo build --workspace --release` clean.
- `cargo test --workspace` clean.
- Spec 127 coupling gate: I1 does not move any code path; the gate must not fire. If it does, halt.
- `/init` works (no protocol changes in I1).

**Trip-wires:**
- If consolidation reveals workspace dep cycles (e.g., feature-flag mismatch on shared deps like `policy-kernel` with `default-features = false`), halt and surface.
- If `apps/desktop/src-tauri/` SQLite linking breaks the build when consolidated, fall back to keeping it isolated (per D2 Open Question 1 option (a)) and add a note in `Cargo.toml`.

## I2 — Create target directory skeleton

**Discovery reference:** [`cleanup-master-plan.md` §Locked target layout](./cleanup-master-plan.md#locked-target-layout).

**Pre-conditions:** I1 complete.

**Operations (in order):**
1. Create empty target directories: `standards/spec/`, `standards/coding/`, `standards/schemas/spec-spine/`, `standards/schemas/frontmatter/`, `standards/schemas/factory/`, `standards/schemas/factory/stage-outputs/`, `standards/schemas/agent/`, `standards/schemas/coding/`, `tools/spec-spine/`, `tools/oap/`, `tools/vendor/`, `tools/vendor/grammars/`, `product/`, `product/apps/`, `product/packages/`, `docs/contracts/`, `.derived/`.
2. Add `.gitkeep` files to empty directories to make them git-trackable.
3. Update `.gitignore` to add `.derived/**` rules (the new gitignored generated-artifacts root); preserve current `build/**` rules until I9 renames.

**Commit shape:** Single commit `refactor(cleanup): create target directory skeleton`.

**Files changed:** ~20 new `.gitkeep` files; `.gitignore` updated.

**Verification:**
- `git status` shows the new directories tracked via `.gitkeep`.
- `cargo build --workspace` still clean (no code changes).
- `/init` works.

**Trip-wires:**
- If any target directory name collides with an existing file (e.g., a `tools/vendor` already exists as a file), halt and surface.

## I3 — Standards content graduation

**Discovery reference:** [`reference-audit.md` Group A](./reference-audit.md#group-a--specify) + [`protocol-drift-resolutions.md` D-2.4, D-2.8](./protocol-drift-resolutions.md#d-2-4--specifycontractmd-read-missing-from-agentsmd).

**Pre-conditions:** I2 complete.

**Operations (in order):**
1. `git mv .specify/contract.md standards/spec/contract.md`.
2. `git mv .specify/memory/constitution.md standards/spec/constitution.md`.
3. `git mv .specify/templates standards/spec/templates`.
4. `git mv .specify/scripts standards/spec/scripts` (or relocate per operator decision — D1 §open questions).
5. Add placeholder `standards/spec/spec-format.md` and `standards/spec/semver-policy.md` per master plan §Out of scope (placeholders, content deferred).
6. Update `specs/119-project-as-unit-of-governance/spec.md:41` `implements:` entry from `.specify/contract.md` to `standards/spec/contract.md`.
7. Update CLAUDE.md:42 repository structure entry from `.specify/` to `standards/spec/`.
8. Update CODEOWNERS `.github/CODEOWNERS:17`: `.specify/` → `standards/spec/`.
9. Update narrative refs in specs (`specs/000-bootstrap-spec-system/spec.md`, etc.) that reference `.specify/` to `standards/spec/` — these are doc-prose updates; spec 127 gate may fire on any spec edit, but each spec's text-only change does not require an `implements:` update.

**Commit shape:** Single commit `refactor(cleanup): graduate standards content from .specify/`.

**Files changed:**
- Moves: 4 (`contract.md`, `constitution.md`, `templates/`, `scripts/`).
- New: 2 placeholders (`spec-format.md`, `semver-policy.md`).
- Spec frontmatter: 1 (`specs/119-project-as-unit-of-governance/spec.md:41`).
- Doc-prose: ~5 narrative updates across `specs/000-bootstrap-spec-system/`, `CLAUDE.md`, `CODEOWNERS`, `specs/004-spec-to-execution-bridge-mvp/`, `specs/119-project-as-unit-of-governance/`.

**Verification:**
- `cargo test --workspace` clean.
- Spec 127 coupling gate: spec 119's `implements:` updates in the same commit; gate must not fire on the spec/code coupling.
- `/init` works (init.md still reads `.specify/contract.md` until I10; surface as known-stale until then).

**Trip-wires:**
- If `.specify/scripts/bash/` shell scripts reference each other via `.specify/...` paths, those references stay correct only if the scripts move as a unit (which the `git mv` of `.specify/scripts/` ensures).

## I4 — Schema co-location

**Discovery reference:** [`schema-duplication-audit.md`](./schema-duplication-audit.md) + [`reference-audit.md` Groups B–E](./reference-audit.md#group-b--root-schemas).

**Pre-conditions:** I2 complete. **Operator has resolved D5 Open Questions 1 (coding-standard duplicate) and 2 (`crates/agent/src/schemas/` disposition).**

**Operations (in order):**

The schema move is fragile because of `include_str!` depth changes. Operations are batched per crate.

**Batch 1: Bootstrap schemas (Group C → spec-spine).**
1. `git mv specs/000-bootstrap-spec-system/contracts/registry.schema.json standards/schemas/spec-spine/registry.schema.json`.
2. `git mv specs/000-bootstrap-spec-system/contracts/build-meta.schema.json standards/schemas/spec-spine/build-meta.schema.json`.
3. Delete the now-empty `specs/000-bootstrap-spec-system/contracts/` directory.
4. Update `tools/spec-compiler/src/schema.rs:13` `include_str!` from `../../../specs/000-bootstrap-spec-system/contracts/registry.schema.json` to `../../../standards/schemas/spec-spine/registry.schema.json`.
5. Update `tools/spec-compiler/src/schema.rs:16` `include_str!` similarly for `build-meta.schema.json`.
6. Update `tools/spec-compiler/tests/schema_conformance.rs:30,42,75,81` test loaders to `standards/schemas/spec-spine/...`.
7. Update `specs/132-constitutional-invariant-freeze/spec.md:21` `implements:` entry.
8. Update narrative refs in 32 doc-prose hits across specs and docs.

**Batch 2: Root schemas (Group B → spec-spine + frontmatter).**
1. `git mv schemas/codebase-index.schema.json standards/schemas/spec-spine/codebase-index.schema.json`.
2. `git mv schemas/codebase-index-oap.schema.json standards/schemas/spec-spine/codebase-index-oap.schema.json`.
3. `git mv schemas/agent-frontmatter.schema.json standards/schemas/frontmatter/agent-frontmatter.schema.json`.
4. `git mv schemas/skill-frontmatter.schema.json standards/schemas/frontmatter/skill-frontmatter.schema.json`.
5. Delete the now-empty root `schemas/` directory.
6. Update `tools/codebase-indexer/src/schema.rs:8` runtime read from `schemas/codebase-index.schema.json` to `standards/schemas/spec-spine/codebase-index.schema.json`.
7. Update `tools/codebase-indexer/tests/schema_conformance.rs:29,89,94` test loaders.
8. Update `$id` URIs in the 4 moved schemas (optional — D5 surfaces this as deferred follow-up; default: leave URIs unchanged).
9. Update `specs/129-granular-package-oap-metadata/spec.md:18` and `specs/133-amends-aware-coupling-gate/spec.md:23` `implements:` entries.
10. Update narrative refs.

**Batch 3: Factory schemas (Group D → factory).**
1. `git mv crates/factory-contracts/schemas standards/schemas/factory`.
2. Update `platform/services/stagecraft/api/factory/oapContracts.ts:20` walk-up target from `crates/factory-contracts/schemas/` to `standards/schemas/factory/` (and adjust emitted substrate row `path:` field shapes).
3. Update narrative refs (`projection.ts:99`, `substrateBrowser.ts:70`, `syncPipeline.ts:62,167` — these are comments).
4. Update `specs/112-factory-project-lifecycle/spec.md:33` `implements:` entry.
5. Update stagecraft test fixture strings (`artifact-body-viewer.test.ts:140,142`).

**Batch 4: Agent schemas (Group E → agent).**
1. Per D5 Open Question 2 resolution, either move-and-keep or delete. If move: `git mv crates/agent/src/schemas standards/schemas/agent`. If delete: `rm -rf crates/agent/src/schemas`.
2. No `implements:` updates (no spec references these).

**Batch 5: Coding-standard duplicate resolution (D5 Open Question 1).**
- Per chosen option (a, b, or c): if (b) — delete `standards/schema/standard.schema.json`; if (a) — delete `packages/yaml-standards-schema/schemas/coding-standard.schema.json` and update the npm package's loader to read from `standards/schemas/coding/standard.schema.json`. Either way, no spec `implements:` updates needed (no spec references either path).

**Commit shape:** Single commit `refactor(cleanup): consolidate authored schemas under standards/schemas/` (one atomic commit covering all 5 batches to keep `cargo build` clean throughout).

**Files changed:**
- Moves: ~13 schema files.
- Modified Rust: 4 files (`tools/spec-compiler/src/schema.rs`, `tools/codebase-indexer/src/schema.rs`, 2 test files).
- Modified TS: 5 files (stagecraft factory/* + test).
- Spec frontmatter: 4 specs (112, 129, 132, 133).
- Doc-prose: ~50 narrative updates across specs + analysis docs.

**Verification:**
- `cargo build --workspace --release` clean.
- `cargo test --workspace` clean (golden-test failure for codebase-indexer's schema load path is the trip-wire to watch).
- `make registry` succeeds (regenerates `index.json` with new schema paths).
- Spec 127 coupling gate clears (4 frontmatter updates land in the same commit as the schema moves).

**Trip-wires:**
- If any `include_str!` fails (compile-time include miss), halt and surface the specific call.
- If stagecraft walk emits empty substrate rows (because the walked path is wrong), halt and surface — the path string update in `oapContracts.ts` is the load-bearing change.

## I5 — Tools restructure

**Discovery reference:** [`reference-audit.md` Groups G, H, I](./reference-audit.md#group-g--spec-spine-tools-move-to-toolsspec-spine) + [`spec-implements-inventory.md` Groups G, H](./spec-implements-inventory.md#group-g--spec-spine-tools-epic-2-i5) + [`workflow-makefile-inventory.md`](./workflow-makefile-inventory.md) + [`cargo-workspace-inventory.md`](./cargo-workspace-inventory.md).

**Pre-conditions:** I1 + I2 complete.

**Operations (in order):**

I5 has the largest blast radius — ~52 workflow line updates, ~80 Makefile line updates, ~40 spec `implements:` rows, ~70 Cargo path-dep declarations, and the root `Cargo.toml` workspace `members` list. Group as one atomic commit (the Cargo workspace members must reference real paths at all times; partial moves break `cargo build`).

1. **Move spec-spine tools to `tools/spec-spine/`:**
   - `git mv tools/spec-compiler tools/spec-spine/spec-compiler`
   - `git mv tools/registry-consumer tools/spec-spine/registry-consumer`
   - `git mv tools/codebase-indexer tools/spec-spine/codebase-indexer`
   - `git mv tools/spec-lint tools/spec-spine/spec-lint`
   - `git mv tools/spec-code-coupling-check tools/spec-spine/spec-code-coupling-check`
2. **Move OAP-specific tools to `tools/oap/`:**
   - `git mv tools/oap-registry-enrich tools/oap/oap-registry-enrich`
   - `git mv tools/oap-code-index-enrich tools/oap/oap-code-index-enrich`
   - `git mv tools/policy-compiler tools/oap/policy-compiler`
   - `git mv tools/adapter-scopes-compiler tools/oap/adapter-scopes-compiler`
   - `git mv tools/assumption-cascade-check tools/oap/assumption-cascade-check`
   - `git mv tools/ci-parity-check tools/oap/ci-parity-check`
   - `git mv tools/schema-parity-check tools/oap/schema-parity-check`
   - `git mv tools/stakeholder-doc-lint tools/oap/stakeholder-doc-lint`
3. **`tools/shared/spec-types/` stays put** (path unchanged per master plan).
4. **Update Cargo path deps** in 9 files (sibling-tool path adjustments — see [`cargo-workspace-inventory.md` "Within tools/" table](./cargo-workspace-inventory.md#within-tools-sibling-deps)):
   - `tools/spec-spine/spec-compiler/Cargo.toml`: `../shared/spec-types` → `../../shared/spec-types`
   - `tools/spec-spine/spec-lint/Cargo.toml`: same
   - `tools/spec-spine/codebase-indexer/Cargo.toml`: same
   - `tools/spec-spine/spec-code-coupling-check/Cargo.toml`: `../codebase-indexer` (stays — both in spec-spine/)
   - `tools/oap/policy-compiler/Cargo.toml`: `../shared/spec-types` → `../../shared/spec-types`; `../../crates/policy-kernel` → `../../../crates/policy-kernel`
   - `tools/oap/oap-registry-enrich/Cargo.toml`: `../registry-consumer` → `../../spec-spine/registry-consumer`; `../shared/spec-types` → `../../shared/spec-types`
   - `tools/oap/oap-code-index-enrich/Cargo.toml`: `../codebase-indexer` → `../../spec-spine/codebase-indexer`; `../shared/spec-types` → `../../shared/spec-types`
   - `tools/oap/stakeholder-doc-lint/Cargo.toml`: `../../crates/factory-contracts` → `../../../crates/factory-contracts`; `../../crates/provenance-validator` → `../../../crates/provenance-validator`
   - `tools/oap/assumption-cascade-check/Cargo.toml`: `../../crates/factory-engine` → `../../../crates/factory-engine`
5. **Update Cargo path deps in `crates/` and `apps/desktop/src-tauri/Cargo.toml`** for cross-tree deps into tools:
   - `crates/factory-engine/Cargo.toml`: `../../tools/registry-consumer` → `../../tools/spec-spine/registry-consumer`
   - `crates/featuregraph/Cargo.toml`: same
   - `apps/desktop/src-tauri/Cargo.toml`: `../../../tools/registry-consumer` → `../../../tools/spec-spine/registry-consumer`
6. **Update root `Cargo.toml` `members` array** to reflect new tool paths.
7. **Update Makefile** (~80 line updates per [`workflow-makefile-inventory.md`](./workflow-makefile-inventory.md)).
   - Also drop the stale `tools/shared/frontmatter/Cargo.toml` reference at `Makefile:584` (D6 Open Question, D1 Group I — the file was deleted in W-01).
8. **Update workflows** (~52 line updates across `spec-conformance.yml`, `ci-codebase-index.yml`, `ci-spec-code-coupling.yml`, `ci-parity.yml`, `ci-supply-chain.yml`, `release-tools.yml`).
9. **Update `tools/oap/ci-parity-check/src/lib.rs:592`** hardcoded path: `./tools/adapter-scopes-compiler/target/release/adapter-scopes-compiler` → `./tools/oap/adapter-scopes-compiler/target/release/adapter-scopes-compiler`.
10. **Update `.github/CODEOWNERS`** lines 21–29 to reflect new tool paths.
11. **Update spec `implements:` rows** — 39 frontmatter rows across 38 specs (per [`spec-implements-inventory.md`](./spec-implements-inventory.md#group-g--spec-spine-tools-epic-2-i5)). Bulk sed-replace with operator review of diff before commit:
    - `tools/registry-consumer` → `tools/spec-spine/registry-consumer` (25 specs)
    - `tools/spec-compiler` → `tools/spec-spine/spec-compiler` (5 specs)
    - `tools/spec-code-coupling-check` → `tools/spec-spine/spec-code-coupling-check` (3 specs)
    - `tools/codebase-indexer` → `tools/spec-spine/codebase-indexer` (3 specs)
    - `tools/ci-parity-check` → `tools/oap/ci-parity-check` (2 specs)
    - `tools/stakeholder-doc-lint` → `tools/oap/stakeholder-doc-lint` (1 spec)
    - `tools/schema-parity-check` → `tools/oap/schema-parity-check` (1 spec)
    - `tools/policy-compiler` → `tools/oap/policy-compiler` (1 spec)
    - `tools/adapter-scopes-compiler` → `tools/oap/adapter-scopes-compiler` (1 spec)
12. **Update doc-prose** in CLAUDE.md, AGENTS.md, README.md, docs/ARCHITECTURE.md, .claude/commands/*.md, .claude/agents/*.md.

**Commit shape:** Single atomic commit `refactor(cleanup): subdivide tools/ into spec-spine, shared, oap, vendor`. Large commit — but each phase invariant requires atomic.

**Files changed:**
- Moves: 13 directories.
- Cargo manifests: 9 in `tools/` + 3 in `crates/` + `apps/desktop/src-tauri/` + root `Cargo.toml` = 14 manifests.
- Makefile: ~80 line updates.
- Workflows: 6 files, ~52 line updates.
- CODEOWNERS: ~9 lines.
- Specs: 38 frontmatter rows.
- Doc-prose: ~100 narrative updates.
- Single source file: `tools/oap/ci-parity-check/src/lib.rs:592`.

**Verification:**
- `cargo build --workspace --release` clean.
- `cargo test --workspace` clean.
- All 22 workflows lint clean (`actionlint` if available).
- `make ci-parity` succeeds (ci-parity-check's CONSUMERS/PRODUCERS registries must reflect new paths).
- `make registry` succeeds.
- Spec 127 coupling gate clears (38 frontmatter updates land with the move).
- `/init` continues to work (no protocol changes; AGENTS.md tool refs unchanged).

**Trip-wires:**
- If a spec frontmatter update is missed and the coupling gate fires, halt and surface the specific spec.
- If `release-tools.yml` matrix expansion fails (tool not found at new path), halt and surface.
- If a stale `Makefile` line points at a deleted tool path, halt and surface — D6's `Makefile:584` stale ref is the canonical example; verify no other stale refs survive.

## I6 — Grammars vendor move

**Discovery reference:** [`reference-audit.md` Group J](./reference-audit.md#group-j--grammars).

**Pre-conditions:** I2 complete.

**Operations (in order):**
1. `git mv grammars tools/vendor/grammars`.
2. Update `crates/axiomregent/build.rs` to walk the new path. (D2 captures this as the load-bearing ref; verify the exact line.)
3. Update narrative refs (`.claude/commands/cleanup.md:125`, `tools/spec-spine/codebase-indexer/src/manifest.rs:347`, `docs/ARCHITECTURE.md:21`).

**Commit shape:** Single commit `refactor(cleanup): move tree-sitter grammars to tools/vendor/`.

**Files changed:**
- Move: 1 directory (5 grammars).
- Source: 1 (`crates/axiomregent/build.rs`).
- Doc-prose: ~3 narrative updates.

**Verification:**
- `cargo build --release --manifest-path crates/axiomregent/Cargo.toml` clean (build.rs reaches the new grammars path).
- `cargo test --workspace` clean.
- No spec `implements:` updates (per D4 Group J).

**Trip-wires:**
- If axiomregent build fails because build.rs's path-walk does not find grammars, halt and surface the exact build.rs line.

## I7 — Product layer

**Discovery reference:** [`typescript-workspace-inventory.md`](./typescript-workspace-inventory.md) + [`reference-audit.md` Groups K, L, M](./reference-audit.md#group-k--appsdesktop) + [`spec-implements-inventory.md` Groups K, L](./spec-implements-inventory.md#group-k--appsdesktop-epic-2-i7).

**Pre-conditions:** I1 + I2 complete.

**Operations (in order):**

I7 has the second-largest blast radius — packages + apps + root npm files all move together. Pre-condition: master plan §Locked target layout puts root npm files inside `product/`; the indexer loader for `pnpm-workspace.yaml` updates accordingly.

1. **Move apps:** `git mv apps/desktop product/apps/desktop`.
2. **Move packages:** `git mv packages product/packages`.
3. **Move root npm files:**
   - `git mv package.json product/package.json`
   - `git mv package-lock.json product/package-lock.json`
   - `git mv pnpm-workspace.yaml product/pnpm-workspace.yaml`
   - `git mv pnpm-lock.yaml product/pnpm-lock.yaml`
4. **Update `pnpm-workspace.yaml` globs** (if file moved into `product/`, globs stay `apps/*`, `packages/*` since they're now relative to `product/`).
5. **Update `apps/desktop/src-tauri/Cargo.toml` path deps** — 13 declarations deepen by one level (`../../../crates/...` → `../../../../crates/...`).
6. **Update root `Cargo.toml` `members` array** to reflect new apps/desktop path.
7. **Update `tools/spec-spine/codebase-indexer/src/lib.rs:446,447`** and **`tools/spec-spine/codebase-indexer/src/manifest.rs:377,378`** to read `pnpm-workspace.yaml` from `product/` (not repo root).
8. **Update `apps/desktop/src-tauri/src/commands/claude.rs:154,158,161,1200`** runtime sidecar path: `packages/provider-registry/dist/node-sidecar.js` → `product/packages/provider-registry/dist/node-sidecar.js`.
9. **Update workflows** — `ci-codebase-index.yml`, `ci-desktop.yml`, `build-axiomregent.yml`, `release-desktop.yml`, `ci-supply-chain.yml` (~65 line updates per D6).
10. **Update Makefile** — `ci-desktop`, `ci-fast-desktop`, `clean` recipes (~25 line updates per D6).
11. **Update spec `implements:` rows** — 27 frontmatter rows across 16 specs for `apps/desktop` + sub-paths; 4 rows for `packages/`.
12. **Update `tools/spec-spine/spec-compiler/tests/v004_consolidation_excludes.rs:32,36`** fixture path strings.
13. **Update doc-prose** in CLAUDE.md, AGENTS.md, README.md, DEVELOPERS.md, docs/ARCHITECTURE.md.

**Commit shape:** Single atomic commit `refactor(cleanup): consolidate end-user product layer under product/`.

**Files changed:**
- Moves: 23 directories (22 packages + 1 app) + 4 root npm files = 27 paths.
- Cargo manifests: `apps/desktop/src-tauri/Cargo.toml` + root `Cargo.toml`.
- Source: `tools/spec-spine/codebase-indexer/src/{lib,manifest}.rs` + `apps/desktop/src-tauri/src/commands/claude.rs`.
- Workflows: 5 files, ~65 line updates.
- Makefile: ~25 line updates.
- Specs: 31 frontmatter rows across 17 specs.
- Doc-prose: ~100 narrative updates.

**Verification:**
- `cargo build --workspace --release` clean.
- `pnpm install` succeeds (workspace globs resolve under `product/`).
- `pnpm -r build` succeeds in product/.
- `cargo build --release --manifest-path apps/desktop/src-tauri/Cargo.toml` clean (deepened path deps resolve).
- `make registry` succeeds (codebase-indexer's `pnpm-workspace.yaml` read now points at `product/`).
- Spec 127 coupling gate clears.

**Trip-wires:**
- If codebase-indexer's runtime read of `pnpm-workspace.yaml` fails (path miss), `make registry` fails and halts the phase.
- If Tauri sidecar runtime path is not updated, the desktop app's claude bridge breaks. Verify by running `make dev` post-commit before declaring complete.

## I8 — Docs consolidation

**Discovery reference:** [`reference-audit.md` Group N](./reference-audit.md#group-n--root-loose-docs).

**Pre-conditions:** I2 complete.

**Operations (in order):**
1. `git mv DEVELOPERS.md docs/DEVELOPERS.md`.
2. `git mv CONTRIBUTING.md docs/CONTRIBUTING.md`.
3. `git mv RELEASE-VERIFICATION.md docs/RELEASE-VERIFICATION.md`.
4. Update markdown link targets in `CLAUDE.md:68`, `CONTRIBUTING.md:144` (self-link → relative to new home), `README.md:355`.
5. Update `.github/spec-coupling-bypass.txt:29` (`DEVELOPERS.md` → `docs/DEVELOPERS.md`).
6. Update `platform/infra/hetzner/setup.sh:12` comment.
7. Update spec `implements:` row: `specs/151-declarative-cluster-reconciliation/spec.md:31`.
8. Update narrative refs in specs (086, 102, 117, 127, 151).

**Commit shape:** Single commit `refactor(cleanup): consolidate loose top-level docs under docs/`.

**Files changed:**
- Moves: 3 files.
- Modified: ~10 (markdown link targets + bypass list + comment + spec + spec narratives).

**Verification:**
- `cargo test --workspace` clean (no Rust changes).
- Spec 127 coupling gate clears (spec 151 `implements:` updates in same commit).
- `/init` continues to work.

**Trip-wires:**
- If a downstream tool reads any of the moved files by literal root path (none found in D1), surface and halt.

## I9 — `build/` → `.derived/` rename

**Discovery reference:** [`reference-audit.md` Group O](./reference-audit.md#group-o--build-rename-to-derived) + [`protocol-drift-resolutions.md` D-2.2](./protocol-drift-resolutions.md#d-2-2--structural-index-read-path-spec-103-violation-in-initmd).

**Pre-conditions:** I2 + I6 complete.

**Operations (in order):**
1. `git mv build .derived` (renames the entire directory including `.derived/spec-registry/`, `.derived/codebase-index/`, `.derived/schema-parity/`).
2. Update `.gitignore` lines 300, 315, 322–324 from `build/spec-registry/` etc. to `.derived/spec-registry/` etc. (and the re-include rules for `.derived/codebase-index/index.json`).
3. Update all `repo_root.join("build/...")` runtime literals (~20 hits across Rust crates per D1 Group O):
   - `apps/desktop/src-tauri/src/commands/analysis.rs:22,28`
   - `crates/factory-contracts/src/{knowledge,provenance,stakeholder_docs}.rs` (5 lines)
   - `crates/factory-engine/src/governance_certificate.rs:713,766`
   - `crates/featuregraph/src/index_bridge.rs:119`
   - `crates/featuregraph/src/scanner.rs:174,176,349`
   - `crates/featuregraph/tests/golden.rs:17`
4. Update `.githooks/pre-commit:20,30,48,52` paths.
5. Update Makefile lines 143, 151, 174–177, 824–826.
6. Update `.github/workflows/spec-conformance.yml`, `ci-codebase-index.yml`, `ci-spec-code-coupling.yml` (~8 line updates).
7. Update `.claude/rules/governed-artifact-reads.md` consumer-binary table (lines 15–18, 26, 35).
8. Update `.claude/agents/{architect,explorer,implementer,reviewer}.md` narrative refs.
9. Update `.claude/commands/init.md:30` (still references `build/`; will be replaced in I10 via D-2.2 resolution — but the path-string update happens here in I9).
10. Update CLAUDE.md, README.md, AGENTS.md, CONTRIBUTING.md, docs/ARCHITECTURE.md narrative refs.
11. Update `specs/118-workflow-spec-traceability/spec.md:19` `implements:` row.
12. Update narrative refs in specs (many `specs/000-bootstrap-spec-system/`, `specs/001-spec-compiler-mvp/`, etc.).

**Commit shape:** Single atomic commit `refactor(cleanup): rename build/ to .derived/`.

**Files changed:**
- Move: 1 directory (3 subdirectories preserved).
- `.gitignore`: ~5 lines.
- Rust runtime literals: ~20 across 7 crates.
- `.githooks/pre-commit`: 4 lines.
- Makefile: ~10 lines.
- Workflows: 3 files, ~8 lines.
- Rules + agents + commands: ~10 files, ~20 lines.
- Spec frontmatter: 1 (spec 118).
- Doc-prose: ~50 narrative updates.

**Verification:**
- `cargo build --workspace --release` clean.
- `cargo test --workspace` clean (golden tests in featuregraph use the path; verify regenerate).
- `make registry` succeeds (now emits to `.derived/`).
- `make pr-prep` succeeds.
- Spec 127 coupling gate clears.
- `/init` works (init.md path-string updated to `.derived/`; logical change in I10).

**Trip-wires:**
- If a Rust crate's `repo_root.join("build/...")` is missed, that crate's runtime breaks; surface the specific file:line.
- If `make pr-prep` fails with "build/ doesn't exist", an existing recipe was missed.

## I10 — Protocol drift resolution

**Discovery reference:** [`protocol-drift-resolutions.md`](./protocol-drift-resolutions.md).

**Pre-conditions:** I3, I5, I7, I9, I11 complete (per D9 dependency graph).

**Operations (in order):**
1. Update `AGENTS.md` "New Sessions" Step 0 to list all three rule files (D-2.1).
2. Update `AGENTS.md` Step 1 parallel reads: add `standards/spec/contract.md` (D-2.4), add `standards/spec/constitution.md` (D-2.8), add `ls tools/`, `ls product/apps/`, `ls docs/` (D-2.9), split render line into `codebase-indexer render` + optional `oap-code-index-enrich render` (D-2.10).
3. Add header note about implicit AGENTS.md self-read (D-2.3).
4. Add Step 2 note about template ownership (D-2.11).
5. Reduce `.claude/commands/init.md`:
   - Step 0: remove direct memory load; defer to AGENTS.md Step 0 (D-2.1, D-2.8).
   - Step 1: drop AGENTS.md from identity-reads list (D-2.3); drop `.specify/contract.md` (D-2.4); replace `ls specs/` with `registry-consumer list --ids-only` (D-2.5); replace `build/codebase-index/index.json` direct read with governed `codebase-indexer render` (D-2.2); add `registry-consumer status-report --json --nonzero-only` (D-2.6); change `-15` to `-10` (D-2.7); remove `ls tools/`, `ls apps/`, `ls docs/` (D-2.9).
   - Step 2: add `## lifecycle:` section to summary template (D-2.6).
6. Update `CLAUDE.md:23-28` rule-paragraph to back-reference (D-2.1).
7. Update `.claude/rules/governed-artifact-reads.md` consumer-binary table (D-2.10): reflect `codebase-indexer render` restoration + `.derived/` paths.

**Commit shape:** Single commit `refactor(cleanup): align /init protocol; AGENTS.md canonical`.

**Files changed:** 4 (AGENTS.md, init.md, CLAUDE.md, governed-artifact-reads.md).

**Verification:**
- Manually invoke `/init` post-commit; verify the structured summary emits with all 11 drift items resolved.
- `cargo test --workspace` clean (no Rust changes).
- Spec 127 coupling gate: no `implements:` updates (the rule files / AGENTS.md aren't in any spec's `implements:`); gate must not fire.

**Trip-wires:**
- If `/init` fails to parse the updated AGENTS.md "New Sessions" section (formatting drift), halt and surface.

## I11 — Render-path resolution

**Discovery reference:** [`render-path-decomposition.md`](./render-path-decomposition.md).

**Pre-conditions:** I5 + I9 complete.

**Operations (in order):**
1. Add `render` subcommand to `tools/spec-spine/codebase-indexer/src/main.rs` (CLI dispatch).
2. Add new file `tools/spec-spine/codebase-indexer/src/render.rs` exposing `render_generic(&Index) -> String`.
3. Refactor `tools/oap/oap-code-index-enrich/src/render.rs` to delegate the Layer 1+2+Diagnostics block via `open_agentic_codebase_indexer::render::render_generic(&core)`, then append Layer 3/4/5 blocks itself.
4. Update Makefile: split `index-render` recipe into `index-render` (generic, codebase-indexer) + retain `oap-code-index-enrich render` invocation as overlay step.
5. Update `tools/spec-spine/codebase-indexer/Cargo.toml` to expose the render module as a `pub use` (the OAP enricher delegates via the lib).
6. Add a golden test for `codebase-indexer render` (snapshot the generic output for a fixture).
7. Update `.claude/rules/governed-artifact-reads.md` consumer-binary table — codebase-indexer regains `render` in its subcommand list. (Some of this already updated in I10's D-2.10 prep.)

**Commit shape:** Single commit `refactor(cleanup): decompose render path; generic core + OAP overlay`.

**Files changed:**
- New: `tools/spec-spine/codebase-indexer/src/render.rs`.
- Modified: `tools/spec-spine/codebase-indexer/src/{lib,main}.rs`; `tools/oap/oap-code-index-enrich/src/render.rs`.
- Makefile: 1 recipe split.
- Test: 1 new golden.

**Verification:**
- `cargo test --workspace` clean (incl. new golden).
- `codebase-indexer render` produces the generic markdown.
- `oap-code-index-enrich render` produces the enriched markdown (overlays the generic).
- Running both in sequence: enriched view is what lands on disk.

**Trip-wires:**
- If the OAP enricher's delegated render diverges byte-for-byte from the standalone generic render, halt — the delegation contract is broken.

## I12 — V-code emission audit fixes

**Discovery reference:** [`vcode-emission-audit.md`](./vcode-emission-audit.md).

**Pre-conditions:** I5 complete (spec-compiler at `tools/spec-spine/spec-compiler/`).

**Operations (in order):**
1. Fix V-002 (b) — extraFrontmatter over-size truncation. In `tools/spec-spine/spec-compiler/src/lib.rs:1278-1285`, after the V-002 (b) violation, truncate `extra` to its first 8 entries (deterministic alphabetical order). The violation remains the source of truth.
2. Add regression test for V-002 (b) — fixture with 9-key extraFrontmatter; assert registry shape has 8 keys + violation present.

**Commit shape:** Single commit `fix(cleanup): V-002 (b) follows producer-side enforcement pattern`.

**Files changed:**
- Modified: `tools/spec-spine/spec-compiler/src/lib.rs` (~10 lines).
- New: `tools/spec-spine/spec-compiler/tests/v002_extra_frontmatter_truncation.rs`.

**Verification:**
- `cargo test --workspace` clean (incl. new test).
- `make registry` produces schema-conformant `registry.json`.

**Trip-wires:** none expected; single-function edit.

## I13 — Final cleanup; delete `.specify/`

**Discovery reference:** [`reference-audit.md` Group A](./reference-audit.md#group-a--specify).

**Pre-conditions:** I3 complete (content already graduated).

**Operations (in order):**
1. `rm -rf .specify/` (verify all content has been graduated; D1 Group A shows the load-bearing refs).
2. Remove `.github/CODEOWNERS:17` `/.specify/` line.
3. Update CLAUDE.md:42 if the repository structure table still mentions `.specify/`.

**Commit shape:** Single commit `refactor(cleanup): delete vestigial .specify/; cleanup complete`.

**Files changed:**
- Removed: `.specify/` (all remaining content).
- Modified: `.github/CODEOWNERS`, `CLAUDE.md` (if stale).

**Verification:**
- `cargo test --workspace` clean.
- `make ci` clean (fast loop).
- `/init` works (post-I10 protocol no longer reads `.specify/`).
- `make pr-prep` clean.
- Spec 127 coupling gate clears.

**Trip-wires:**
- If any binary or workflow still references `.specify/...`, surface and halt — that means earlier I-phases (esp. I3, I5) missed an update. The audit should not allow this; the trip-wire is a safety net.

## Cross-phase dependency graph

```
I1 (root Cargo workspace) ───┐
                             ├──→ I5 (tools restructure)
I2 (skeleton) ───────────────┤
                             ├──→ I3 (standards graduation)
                             │
                             ├──→ I4 (schema co-location)
                             │
                             ├──→ I6 (grammars vendor)
                             │
                             ├──→ I7 (product layer)
                             │
                             ├──→ I8 (docs consolidation)
                             │
                             └──→ I9 (build/ → .derived/)
                                       │
                                       ▼
I5 ───────────────────────────────→  I11 (render path)
                                       │
                                       ▼
I3, I5, I7, I9, I11 ────────────→  I10 (protocol drift)
                                       │
I5 ────────────────────────────────→  I12 (V-code fix)
                                       │
I3 ────────────────────────────────→  I13 (delete .specify/)
                                       │
All prior ───────────────────────→    [merge ready]
```

Linearizable order: I1 → I2 → I3, I4, I6, I8 (parallel-safe after I2) → I5 → I7 (after I5 for path-dep depth) → I9 → I11 → I10 → I12 → I13.

Operator may parallelize I3/I4/I6/I8 in sub-batches if convenient, but each I-phase produces an atomic commit.

## Trip-wires (operator-halt conditions)

Compiled from per-phase trip-wires:

- **I1:** workspace dep cycles, SQLite linking conflict resurfaces.
- **I2:** target directory name collides with existing file.
- **I3:** `.specify/scripts/bash/` script self-references break post-move.
- **I4:** `include_str!` miss, stagecraft empty substrate rows.
- **I5:** missed spec frontmatter (coupling gate fires), `release-tools.yml` matrix fails, stale Makefile lines.
- **I6:** axiomregent `build.rs` path-walk fails.
- **I7:** codebase-indexer `pnpm-workspace.yaml` read fails, Tauri sidecar runtime path stale.
- **I8:** downstream tool reads moved doc by literal root path.
- **I9:** Rust crate's `repo_root.join("build/...")` missed, `make pr-prep` fails.
- **I10:** `/init` fails to parse updated AGENTS.md "New Sessions" formatting.
- **I11:** OAP enricher's delegated render diverges byte-for-byte from the standalone generic render.
- **I12:** none expected.
- **I13:** any `.specify/...` reference survives.

## Epic 2 execution summary

- **Total estimated commits:** 13 (one per I-phase, single-commit-per-phase target).
- **Total files moved:** ~70 paths (4 root npm + 3 root docs + ~14 schemas + ~23 packages/apps + 5 grammars + 13 tools + 1 `build/` rename + various `.specify/` content).
- **Total file content changes:** ~700 lines across Rust source, Cargo manifests, Makefile, workflows, specs, and docs.
- **Spec frontmatter rows updated:** 77 across I3, I4, I5, I7, I8, I9 (per D4).
- **Sequential execution time estimate:** **4–8 hours** for a focused operator-driven session, conditional on no trip-wires firing. With trip-wires that surface meaningful issues (e.g., operator decisions on D2 Open Questions), the timeline extends.

## Caveat

This manifest is the contract for Epic 2. The operator may:

1. **Hand-edit the manifest** before Epic 2 fires; the amended version is canonical.
2. **Re-run Epic 1 in correction mode** for specific phases if the manifest has issues.
3. **Pause the cleanup** for further design.

The Epic 2 agent halts and surfaces if reality during execution diverges from this manifest — it does NOT improvise. The eight collected open-questions across D1–D9 are the operator-triage items at epic boundary.

## Index of D1–D9 source manifests

For Epic 2 cross-reference convenience:

- [reference-audit.md](./reference-audit.md) — D1, path groups A–O
- [cargo-workspace-inventory.md](./cargo-workspace-inventory.md) — D2, Cargo workspace structure
- [typescript-workspace-inventory.md](./typescript-workspace-inventory.md) — D3, pnpm workspace
- [spec-implements-inventory.md](./spec-implements-inventory.md) — D4, per-spec `implements:` rows
- [schema-duplication-audit.md](./schema-duplication-audit.md) — D5, schema co-location
- [workflow-makefile-inventory.md](./workflow-makefile-inventory.md) — D6, CI + Makefile updates
- [vcode-emission-audit.md](./vcode-emission-audit.md) — D7, V-code emission patterns
- [render-path-decomposition.md](./render-path-decomposition.md) — D8, generic/OAP render split
- [protocol-drift-resolutions.md](./protocol-drift-resolutions.md) — D9, /init drift fixes
