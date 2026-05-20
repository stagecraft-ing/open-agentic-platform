# Workflow & Makefile Inventory

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** `ls .github/workflows/`, `grep -nE`, end-to-end Makefile read.

## Workflows enumerated

22 workflow files under `.github/workflows/`:

```
ai-changelog.yml             cd-tenant-hello.yml      ci-spec-code-coupling.yml
ai-pr-review.yml             ci-axiomregent.yml       ci-stagecraft.yml
build-axiomregent.yml        ci-codebase-index.yml    ci-supply-chain.yml
cd-deployd-api-rs.yml        ci-crates.yml            ci-tenant-hello.yml
cd-stagecraft.yml            ci-deployd-api-rs.yml    release-axiomregent.yml
                             ci-desktop.yml           release-desktop.yml
                             ci-orchestrator.yml      release-tools.yml
                             ci-parity.yml            spec-conformance.yml
                             ci-policy-kernel.yml
```

## Per-workflow path-reference summary

Each row gives a path-reference count by Epic 2 I-phase target. Workflow files have two kinds of refs: **trigger filters** (the `paths:` glob list in `on:` block) and **step body** refs (manifest-path, `./tools/<tool>/target/release/...`, etc.). Both must update in the same commit as the move per spec 127 coupling-gate.

### `spec-conformance.yml` (32 refs)

Single biggest workflow. Builds + tests every spec-spine tool + the OAP enrich tools.

| Epic 2 phase | refs | examples |
|---|---|---|
| I5 (tools restructure) | 22 | `manifest-path tools/spec-compiler/Cargo.toml` (lines 47, 59); `./tools/spec-compiler/target/release/spec-compiler compile` (line 50); same shape for `tools/registry-consumer/` (lines 62, 66–80), `tools/spec-lint/` (83, 86, 89), `tools/codebase-indexer/` (95, 98, 107), `tools/oap-registry-enrich/` (53, 56), `tools/oap-code-index-enrich/` (101, 104), `tools/policy-compiler/` (110, 113) |
| I9 (`build/` → `.derived/`) | 4 | trigger paths (`build/**` at lines 18, 23) — also includes `tools/**`, `specs/**` trigger globs |
| trigger globs | 6 | `tools/**`, `specs/**`, `build/**` (lines 16–23) |

All 22 step-body refs update in I5. Trigger globs at lines 16–23 update in I9 (`build/**` → `.derived/**`); the `tools/**` glob path stays the same string (still under `tools/`) regardless of internal restructure.

### `ci-codebase-index.yml` (10 refs)

Staleness gate for `build/codebase-index/index.json`.

| Epic 2 phase | refs | examples |
|---|---|---|
| I5 | 3 | `tools/codebase-indexer/**` (24, 33), `manifest-path tools/codebase-indexer/Cargo.toml` (55), `./tools/codebase-indexer/target/release/codebase-indexer check` (62) |
| I7 | 4 | trigger globs `apps/**`, `packages/**` (lines 21, 30) |
| I9 | 4 | `build/codebase-index/**` (25, 34), `build/codebase-index/index.json` comment refs (4, 5) |

### `ci-spec-code-coupling.yml` (6 refs)

Spec 127 coupling-gate workflow.

| Epic 2 phase | refs | examples |
|---|---|---|
| I5 | 4 | `manifest-path tools/codebase-indexer/Cargo.toml` (38, 41), `manifest-path tools/spec-code-coupling-check/Cargo.toml` (44), `./tools/spec-code-coupling-check/target/release/spec-code-coupling-check` (56) |
| I9 | 2 | doc comments referring to `build/codebase-index/index.json` (5) |

### `ci-parity.yml` (5 refs)

| Epic 2 phase | refs | examples |
|---|---|---|
| I5 | 5 | `tools/ci-parity-check/**` trigger (15, 20); `manifest-path tools/ci-parity-check/Cargo.toml` (36, 39); `./tools/ci-parity-check/target/release/ci-parity-check` (42) |

### `ci-supply-chain.yml` (10 refs)

The cargo-deny matrix; pins explicit Cargo.toml paths.

| Epic 2 phase | refs | examples |
|---|---|---|
| I5 | 8 | `tools/spec-compiler/Cargo.toml` (79), `tools/registry-consumer/Cargo.toml` (80), `tools/spec-lint/Cargo.toml` (81), `tools/codebase-indexer/Cargo.toml` (82), `tools/policy-compiler/Cargo.toml` (83), `tools/adapter-scopes-compiler/Cargo.toml` (84), `tools/ci-parity-check/Cargo.toml` (85), `tools/shared/spec-types/Cargo.toml` (86) |
| I7 | 1 | `apps/desktop/src-tauri/Cargo.toml` (78) |
| (path unchanged) | 1 | `tools/shared/spec-types/Cargo.toml` (86) stays same |

### `ci-crates.yml` (6 refs)

Workspace-wide `cargo` invocations against `crates/Cargo.toml`.

| Epic 2 phase | refs | examples |
|---|---|---|
| I1 | 3+ | `manifest-path crates/Cargo.toml` (43); trigger `crates/**` (19, 22). Once I1 consolidates the root workspace, `crates/Cargo.toml` becomes the leaf workspace member root — or the trigger broadens to repo root. **Operator-decision point: whether the I1 root `Cargo.toml` replaces `crates/Cargo.toml` or sits alongside.** |
| I7 | (potential) | If I7 moves apps/desktop, no direct ref here but indirectly the `--workspace` runs change scope |

### `ci-orchestrator.yml` (7 refs)

| Epic 2 phase | refs | examples |
|---|---|---|
| I1 | 4 | trigger globs `crates/orchestrator/**`, `crates/provider-registry/**`, `crates/standards-loader/**` (12–14, 17–19); `manifest-path crates/orchestrator/Cargo.toml` (40); `cache-workspaces: crates/orchestrator` (41). After I1, may collapse into a workspace-wide runner. |

### `ci-policy-kernel.yml` (4 refs)

| Epic 2 phase | refs | examples |
|---|---|---|
| I1 | 4 | trigger globs `crates/policy-kernel/**` (11, 14); `manifest-path crates/policy-kernel/Cargo.toml` (35) |

### `ci-axiomregent.yml` (2 refs)

| Epic 2 phase | refs | examples |
|---|---|---|
| I1 | 2 | `manifest-path crates/axiomregent/Cargo.toml` (47); comment ref to `crates/policy-kernel/Cargo.toml` (51) |

### `ci-desktop.yml` (24+ refs)

| Epic 2 phase | refs | examples |
|---|---|---|
| I7 | 21+ | trigger globs `apps/desktop/**`, `packages/**`, `crates/**` (lines 19–26); workspace `apps/desktop/src-tauri` (56); dist + binaries stubs `apps/desktop/dist/index.html` (61–62), `apps/desktop/src-tauri/binaries/` (66–68); `manifest-path apps/desktop/src-tauri/Cargo.toml` (71, 74, 77, 80); `apps/desktop/src-tauri/Cargo.toml` version check (85); `apps/desktop/package.json` version check (86) |

### `build-axiomregent.yml` (32 refs)

| Epic 2 phase | refs | examples |
|---|---|---|
| I1 | 4 | trigger globs `crates/axiomregent/**`, `crates/agent/**`, `crates/featuregraph/**`, etc. (12–17, 20–25); `manifest-path crates/axiomregent/Cargo.toml` (67, 127) |
| I7 | 28+ | `apps/desktop/src-tauri/binaries/...` (72–93, 131–138) — bundled-binary copy destinations |

### `release-axiomregent.yml` (12 refs)

| Epic 2 phase | refs | examples |
|---|---|---|
| I1 | 6 | `manifest-path crates/axiomregent/Cargo.toml` (63, 115); `crates/target/...` (71, 120); `crates/Cargo.lock` (165, 178, 181) |
| I7 | (none direct, but bundle outputs feed into apps/desktop downstream) | — |

### `release-desktop.yml` (15+ refs)

| Epic 2 phase | refs | examples |
|---|---|---|
| I1 | 4 | `crates/axiomregent/Cargo.toml` (76, 131); `crates/target/...` (82, 136) |
| I7 | 11+ | `apps/desktop/src-tauri/binaries/...` (81–84, 97, 135–137, 143, 216, 220); `apps/desktop` projectPath (269, 296); `apps/desktop/src-tauri/target` (320); `apps/desktop` (339); `apps/desktop/src-tauri/target/sbom-desktop-...cdx.json` (341) |

### `release-tools.yml` (10+ refs)

| Epic 2 phase | refs | examples |
|---|---|---|
| I5 | 10+ | `manifest-path tools/spec-compiler/Cargo.toml` (108), `manifest-path tools/registry-consumer/Cargo.toml` (111), `manifest-path tools/spec-lint/Cargo.toml` (114), `manifest-path tools/codebase-indexer/Cargo.toml` (123); shell pattern `tools/${tool}/target/${TARGET}/release/...` (148); SBOM scope comment (205) |

### `ci-stagecraft.yml` (10 refs)

All `platform/services/stagecraft/` — **unchanged** by master plan (platform/ structurally untouched). No I-phase updates.

### `cd-stagecraft.yml` / `cd-deployd-api-rs.yml` / `cd-tenant-hello.yml` / `ci-deployd-api-rs.yml` / `ci-tenant-hello.yml` (~30 refs total)

All under `platform/...`. **Unchanged** by cleanup.

### `ai-changelog.yml` / `ai-pr-review.yml` (0 path refs)

No load-bearing refs to moving paths. Untouched.

## Per-phase workflow change manifest

| Epic 2 phase | workflows touched | total line updates |
|---|---|---|
| I1 (root Cargo workspace) | `ci-orchestrator.yml`, `ci-policy-kernel.yml`, `ci-axiomregent.yml`, `ci-crates.yml`, `build-axiomregent.yml`, `release-axiomregent.yml`, `release-desktop.yml`, `ci-supply-chain.yml` | ~25 lines (mostly `manifest-path` and `cache-workspaces` changes; if root workspace remains compatible with existing manifest-path strings, fewer lines change) |
| I5 (tools restructure) | `spec-conformance.yml` (22), `ci-codebase-index.yml` (3), `ci-spec-code-coupling.yml` (4), `ci-parity.yml` (5), `ci-supply-chain.yml` (8), `release-tools.yml` (10) | **~52 lines** |
| I6 (grammars vendor) | (no direct workflow refs — grammars are consumed by axiomregent build.rs, captured under I1's axiomregent workflows) | 0 |
| I7 (product layer) | `ci-codebase-index.yml` (4), `ci-desktop.yml` (21+), `build-axiomregent.yml` (28+), `release-desktop.yml` (11+), `ci-supply-chain.yml` (1) | **~65 lines** |
| I9 (`build/` → `.derived/`) | `spec-conformance.yml` (2 trigger globs), `ci-codebase-index.yml` (4), `ci-spec-code-coupling.yml` (2) | **~8 lines** |

**Grand total workflow line updates across Epic 2:** ≈ 150 lines spread across 11 of the 22 workflows.

## Root Makefile (`Makefile`, 888 lines)

77 lines reference `tools/` directly; significant additional refs to `specs/`, `build/`, `apps/`, `packages/`, `platform/`, `standards/`.

### Recipes by I-phase impact

| Epic 2 phase | recipes / line ranges | nature |
|---|---|---|
| I5 (tools restructure) | `setup` (43–80, builds spec-spine + codebase-indexer + registry-consumer), `axiomregent` builds (not affected by I5), `registry` target chain (140), `oap-registry-enrich` (148–149), `oap-code-index-enrich` (157–158), `pr-prep` (171), `spec-compile` (181), `spec-tools` (184–188), `index` + `index-render` (286, 297–298), `ci-rust` (incl. ci-tools 428–467), `ci-fast-rust`, `ci-fast-tools` (705–711), `ci-schema-parity` (531, 790), `ci-spec-code-coupling` (545–556), `ci-fast-spec-coupling` (793–798), `ci-supply-chain-cargo` (575–584), `ci-parity` (621–622), `clean` (824–826) | **~80 line updates** to manifest-paths and binary paths; recipe **identifiers** unchanged |
| I7 (product layer) | `ci-desktop` (471–491), `ci-fast-desktop` (750–770), `clean` (827–828) | **~25 line updates** to `apps/desktop/...` paths |
| I9 (`build/` → `.derived/`) | `pr-prep` (174–177), `clean` (824–826), all comments referring to `build/spec-registry/`, `build/codebase-index/`, `build/schema-parity/` | **~10 line updates** |
| I1 (root Cargo workspace) | Indirect — many `manifest-path tools/<tool>/Cargo.toml` become `cargo … --package <name>` after consolidation, or stay verbatim if the consolidation preserves the manifest-path style. Operator-decision point. | **0 mandatory** changes (manifest-path still works); **bulk style refactor possible** |
| **Stale ref to remove** | `Makefile:584` references `tools/shared/frontmatter/Cargo.toml` (deleted in W-01). Surfaces from D1 Group I open question. | **1 line removal**, ideally landed early as a cleanup precursor |

### Help text and comments

The Makefile carries extensive `##` doc-prose at lines 95, 111, 124, 139, 143, 151, 160–170, 207, 251, 294, 353, 383, 510, 630, 633, 700, 819, 868–869 referencing tool paths and apps/packages paths. Each I-phase commit updates the corresponding comment block as part of the same atomic change.

### `dev` / `dev-platform` recipes

Lines 313–346: invoke `pnpm run dev` in `apps/desktop` and `npm` in `platform/services/stagecraft`. I7 updates `apps/desktop` paths to `product/apps/desktop`.

## `platform/Makefile` (259 lines)

Read end-to-end. References:

- All `platform/services/...` and `platform/charts/...` — **unchanged** by cleanup.
- No refs to repo-root `tools/`, `apps/`, `packages/`, `crates/`, `schemas/`, `build/`, `standards/`, `.specify/`.

**No I-phase updates needed for `platform/Makefile`.**

## I-phase readiness summary

- **Total workflow file updates:** 22 workflows, of which 11 carry moving-path refs. Estimated ~150 line updates across Epic 2.
- **Total Makefile recipe updates:** ~115 line updates concentrated in I5 + I7 + I9.
- **`platform/Makefile`:** unchanged.
- **`Makefile:584` stale ref to `tools/shared/frontmatter/Cargo.toml`** — D6 confirms; recommend removing as part of I5 or earlier as a precursor commit. Removing is safe because the file does not exist (path was deleted in W-01).
- **Estimated complexity:** **medium-high**. Mechanical sed-replace works for `tools/<tool>/` → `tools/spec-spine/<tool>/` or `tools/oap/<tool>/` patterns, but each I-phase commit needs operator review since miss-categorising a tool (spec-spine vs oap) would create CI breakage.

## Open questions (surface for operator triage)

1. **I1 root workspace style.** Does the I1 root `Cargo.toml` keep individual `manifest-path tools/<tool>/Cargo.toml` style (Makefile + workflows stay readable, individual tool builds remain possible) or switch to `cargo --package <name>` (more idiomatic, less verbose, but loses the "build this tool only" semantics)? Recommendation: keep `manifest-path` style for compatibility; switch is a follow-up.
2. **I5 tool category (spec-spine vs oap) per tool.** D6 assumes the master plan §Locked target layout categorisation. Confirm before I5 fires.
3. **`tools/ci-parity-check/src/lib.rs:592`** carries a hardcoded path `./tools/adapter-scopes-compiler/target/release/adapter-scopes-compiler` (D1 Group H). After I5 this becomes `./tools/oap/adapter-scopes-compiler/target/release/...`. Confirm ci-parity-check is updated in same I5 commit (it lives under `tools/oap/` post-move).
4. **`build/codebase-index/index.json` mid-rename**. I9 renames `build/` → `.derived/`. The Makefile `pr-prep` recipe (171–179) currently has `git diff --quiet build/codebase-index/index.json` and `git add build/codebase-index/index.json` (lines 174–177). Both update in I9. Per spec 103, the codebase-indexer's `check` subcommand wraps the read; the recipe relies on raw `git diff` over the path — that's a path-literal that needs updating, not a consumer-binary call.
5. **`.specify/` tree** does not appear in any workflow (no trigger globs, no recipe steps). I3 + I13 do not touch workflows.
