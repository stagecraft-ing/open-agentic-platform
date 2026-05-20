# TypeScript Workspace Inventory

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** `find`, `git grep`, manual `package.json` inspection.

## pnpm-workspace.yaml globs today

```yaml
packages:
  - "apps/*"
  - "packages/*"

onlyBuiltDependencies:
  - "@parcel/watcher"
  - "better-sqlite3"
  - "core-js"
  - "esbuild"
  - "protobufjs"
  - "sharp"
```

After I7 the `packages:` globs change to `product/apps/*` and `product/packages/*` (or are scoped to `product/` if the workspace file itself moves into `product/`). `onlyBuiltDependencies` is unaffected.

## All packages

22 packages under `packages/`. All scoped `@opc/...` except `oap-ctl` (unscoped CLI).

| package path | npm name | version | workspace-dep-count | oap.spec |
|---|---|---|---|---|
| `packages/agent-frontmatter` | `@opc/agent-frontmatter` | 0.1.0 | 0 | `054-agent-frontmatter-schema` |
| `packages/claude-code-bridge` | `@opc/claude-code-bridge` | 0.1.0 | 0 | `045-claude-code-sdk-bridge` |
| `packages/coherence-scoring` | `@opc/coherence-scoring` | 0.1.0 | 0 | `063-coherence-scoring` |
| `packages/conductor-track` | `@opc/conductor-track` | 0.1.0 | 0 | `061-conductor-track-lifecycle` |
| `packages/file-mention` | `@opc/file-mention` | 0.1.0 | 0 | `058-file-mention-system` |
| `packages/git-panel` | `@opc/git-panel` | 0.1.0 | 0 | `059-git-panel` |
| `packages/hookify-rule-engine` | `@opc/hookify-rule-engine` | 0.1.0 | 0 | `048-hookify-rule-engine` |
| `packages/multi-model-chaining` | `@opc/multi-model-chaining` | 0.1.0 | 1 | `062-multi-model-chaining` |
| `packages/notification-orchestrator` | `@opc/notification-orchestrator` | 0.1.0 | 0 | `057-notification-system` |
| `packages/oap-ctl` | `oap-ctl` | 0.1.0 | 0 | `085-remote-control-cli` |
| `packages/panel-event-bus` | `@opc/panel-event-bus` | 0.1.0 | 0 | `060-panel-event-bus` |
| `packages/permission-system` | `@opc/permission-system` | 0.1.0 | 0 | `049-permission-system` |
| `packages/prompt-assembly` | `@opc/prompt-assembly` | 0.1.0 | 1 | `070-prompt-assembly-cache` |
| `packages/provider-registry` | `@opc/provider-registry` | 0.1.0 | 2 | `042-multi-provider-agent-registry` |
| `packages/session-memory` | `@opc/session-memory` | 0.1.0 | 0 | `056-session-memory` |
| `packages/skill-command-factory` | `@opc/skill-command-factory` | 0.1.0 | 0 | `071-skill-command-factory` |
| `packages/tool-renderer` | `@opc/tool-renderer` | 0.1.0 | 0 | `050-tool-renderer-system` |
| `packages/ui` | `@opc/ui` | 0.1.0 | 0 | `032-opc-inspect-governance-wiring-mvp` |
| `packages/verification-profiles` | `@opc/verification-profiles` | 0.1.0 | 0 | `053-verification-profiles` |
| `packages/workspace-sdk` | `@opc/workspace-sdk` | 0.1.0 | 0 | `087-unified-workspace-architecture` |
| `packages/worktree-agents` | `@opc/worktree-agents` | 0.1.0 | 0 | `051-worktree-agents` |
| `packages/yaml-standards-schema` | `@opc/yaml-standards-schema` | 0.1.0 | 0 | `055-yaml-standards-schema` |

## All apps

1 app under `apps/`.

| app path | npm name | version | workspace-dep-count | oap.spec |
|---|---|---|---|---|
| `apps/desktop` | `@opc/desktop` | 0.3.2 | 3 | `032-opc-inspect-governance-wiring-mvp` |

## Workspace dependency graph (`workspace:*` only)

7 workspace deps total across 4 consumers:

| consumer | dep |
|---|---|
| `packages/multi-model-chaining/package.json` | `@opc/provider-registry: workspace:*` |
| `packages/prompt-assembly/package.json` | `@opc/yaml-standards-schema: workspace:*` |
| `packages/provider-registry/package.json` | `@opc/claude-code-bridge: workspace:*` |
| `packages/provider-registry/package.json` | `@opc/permission-system: workspace:*` |
| `apps/desktop/package.json` | `@opc/claude-code-bridge: workspace:*` |
| `apps/desktop/package.json` | `@opc/ui: workspace:*` |
| `apps/desktop/package.json` | `@opc/workspace-sdk: workspace:*` |

**Critical observation:** **no `workspace:` deps use path expressions** — they all use npm scope names (`@opc/<name>`). After I7 moves these packages under `product/`, the `workspace:*` deps continue to resolve correctly because the pnpm workspace globs (updated in I7) still cover them.

## Cross-package relative imports

`git grep -nE "from\s+['\"]\.\.?/.+/packages/" -- '*.ts' '*.tsx' '*.js' '*.jsx' '*.mjs'` returns **zero results**.

All inter-package code reuse happens via the npm scope names (`import { X } from '@opc/<name>'`), not via relative paths reaching into sibling packages.

## Cross-app relative imports

`git grep -nE "from\s+['\"]\.\.?/.+/apps/desktop/" -- '*.ts' '*.tsx' '*.js' '*.jsx' '*.mjs'` returns **zero results**.

Same observation — `apps/desktop` does not import other apps via path (there are no other apps), and other packages do not reach into `apps/desktop/` via path.

## Non-workspace package.json files

Listed for completeness; these are outside the pnpm workspace and unchanged by I7.

| path | npm name | note |
|---|---|---|
| `package.json` (root) | `@opc/root` workspace orchestrator | Moves to `product/package.json` in I7 per master plan §Locked target layout (`product/package.json`). |
| `platform/services/stagecraft/package.json` | stagecraft service | Uses npm (not pnpm); excluded from root workspace per `DEVELOPERS.md:85,86,88` + `platform/CLAUDE.md:19`. Unchanged. |
| `platform/services/tenant-hello/package.json` | tenant-hello demo | Same exclusion (npm-managed). Unchanged. |

## Runtime path literals (from D1, Groups K + L recap)

Path-literal references to `apps/desktop/` and `packages/` that need updates in I7 alongside the move:

| file:line | category | update target |
|---|---|---|
| `apps/desktop/src-tauri/src/commands/claude.rs:154,158,161,1200` | path-literal | `packages/provider-registry/dist/node-sidecar.js` → `product/packages/provider-registry/dist/node-sidecar.js` |
| `apps/desktop/vite.config.ts:18` | path-literal (comment) | `packages/ui/src` → `product/packages/ui/src` (comment update; vite resolves via tsconfig paths, not literal) |
| `tools/spec-compiler/tests/v004_consolidation_excludes.rs:32,36` | path-literal | V-004 exclusion fixture; `pnpm-workspace.yaml` + `pnpm-lock.yaml` move per Group M |
| `tools/spec-compiler/src/lib.rs:1102` | path-literal | `"pnpm-workspace.yaml"` exclusion match in V-004 walker; remains correct because the matcher looks at filename only |
| `tools/codebase-indexer/src/lib.rs:446,447` | path-literal | `repo_root.join("pnpm-workspace.yaml")` runtime read — must look in `product/` after I7 |
| `tools/codebase-indexer/src/manifest.rs:377,378` | path-literal | Same — workspace-globs runtime read |
| `pnpm-workspace.yaml` | path-literal | `apps/*`, `packages/*` globs become `product/apps/*`, `product/packages/*` (or scoped if file moves into `product/`) |

## Phase I7 readiness summary

- **Packages to move:** 22 (`packages/*`) → `product/packages/*`
- **Apps to move:** 1 (`apps/desktop`) → `product/apps/desktop`
- **Root npm files to move:** 4 (`package.json`, `package-lock.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`) → `product/`
- **Relative imports to update:** **0** (no cross-package or cross-app relative imports exist)
- **`workspace:*` deps:** 7 — unaffected by move (resolution by npm scope name, not path)
- **`pnpm-workspace.yaml` globs:** updates from `apps/*`, `packages/*` to `product/apps/*`, `product/packages/*` (or just `apps/*`, `packages/*` scoped under the new product/ root, depending on where the workspace.yaml lands)
- **`codebase-indexer` runtime read of `pnpm-workspace.yaml`** at `repo_root.join("pnpm-workspace.yaml")` — **must update** to look at `product/pnpm-workspace.yaml` (or fall back). Two call sites: `tools/codebase-indexer/src/lib.rs:446,447`; `tools/codebase-indexer/src/manifest.rs:377,378`. This is the single load-bearing code change for I7 beyond Cargo path deepening.
- **`apps/desktop/src-tauri/Cargo.toml` path deps deepen by one level** after I7 (`../../../crates/...` → `../../../../crates/...`). 13 path-dep declarations affected; reformulated as workspace deps in I1 eliminates the literal paths and removes the need to update at I7 time.
- **Tauri sidecar runtime path** (`apps/desktop/src-tauri/src/commands/claude.rs:154-161,1200`) needs the new `product/packages/provider-registry/dist/node-sidecar.js` path — single hot spot to update inside the moving tree.
- **Featuregraph golden fixture** (`crates/featuregraph/tests/golden/features_graph.json`) and **codebase index** (`build/codebase-index/index.json`) regenerate automatically after the move.
- **Estimated complexity:** **medium** — bulk move is mechanical (`git mv` of 23 directories + 4 root files), but the codebase-indexer loader change is load-bearing and must land in the same commit so `make registry` keeps working.

## Open questions (surface for operator triage)

1. **Workspace YAML location.** Does `pnpm-workspace.yaml` live at repo root (with `product/apps/*`, `product/packages/*` globs) or inside `product/` (with `apps/*`, `packages/*` globs)? Master plan §Locked target layout shows it inside `product/`. That's coherent with the "one product layer" framing; the indexer loader needs the path update either way.
2. **`apps/desktop/src-tauri/` Cargo workspace** — see D2 Open Question 1 (SQLite isolation). I7 changes path-dep depths regardless; coupled with I1's decision.
3. **`@opc/root` scoping** — root `package.json` carries pnpm workspace orchestration. Does it adopt the `oap.spec` convention? (Currently no `oap` field at root.)
4. **TypeScript paths config** — `apps/desktop/tsconfig.json` and similar may carry `paths:` mappings to `packages/...`. Not surveyed in this phase; verify in I7 atomic-move PR before declaring complete.

## Cross-phase notes

- I7 lands after I1 (root Cargo workspace) so that Cargo path-dep deepening can use `workspace = true` in the root Cargo.toml rather than literal `../../../...`.
- I9 (`build/` → `.derived/`) is independent of I7 — no TS files reference `build/` paths in a load-bearing way (only docs/comments).
