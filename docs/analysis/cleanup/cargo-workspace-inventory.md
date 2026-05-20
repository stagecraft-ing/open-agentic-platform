# Cargo Workspace Inventory

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** `find`, manual `Cargo.toml` inspection. `cargo metadata` deferred — pre-condition for cleanness is already established by branch HEAD passing CI.

## All Cargo.toml files

34 manifests found (excluding `target/`, `node_modules/`, `grammars/`):

| # | path | package.name | kind | has-sibling-Cargo.lock |
|---|---|---|---|---|
| 1 | `crates/Cargo.toml` | — (workspace root) | workspace-root | yes (`crates/Cargo.lock`, 8 321 lines) |
| 2 | `crates/agent/Cargo.toml` | `agent` | workspace-member of `crates/` | no |
| 3 | `crates/agent-frontmatter/Cargo.toml` | `agent-frontmatter` | workspace-member of `crates/` | no |
| 4 | `crates/artifact-extract/Cargo.toml` | `artifact-extract` | workspace-member of `crates/` | no |
| 5 | `crates/axiomregent/Cargo.toml` | `axiomregent` | workspace-member of `crates/` | no |
| 6 | `crates/factory-contracts/Cargo.toml` | `factory-contracts` | workspace-member of `crates/` | no |
| 7 | `crates/factory-engine/Cargo.toml` | `factory-engine` | workspace-member of `crates/` | no |
| 8 | `crates/factory-platform-client/Cargo.toml` | `factory-platform-client` | workspace-member of `crates/` | no |
| 9 | `crates/factory-project-detect/Cargo.toml` | `factory-project-detect` | workspace-member of `crates/` | no |
| 10 | `crates/featuregraph/Cargo.toml` | `featuregraph` | workspace-member of `crates/` | no |
| 11 | `crates/orchestrator/Cargo.toml` | `orchestrator` | workspace-member of `crates/` | no |
| 12 | `crates/policy-kernel/Cargo.toml` | `open_agentic_policy_kernel` | workspace-member of `crates/` | no |
| 13 | `crates/provenance-validator/Cargo.toml` | `provenance-validator` | workspace-member of `crates/` | no |
| 14 | `crates/provider-registry/Cargo.toml` | `provider-registry` | workspace-member of `crates/` | no |
| 15 | `crates/run/Cargo.toml` | `run` | workspace-member of `crates/` | no |
| 16 | `crates/skill-factory/Cargo.toml` | `skill-factory` | workspace-member of `crates/` | no |
| 17 | `crates/standards-loader/Cargo.toml` | `standards-loader` | workspace-member of `crates/` | no |
| 18 | `crates/tool-registry/Cargo.toml` | `tool-registry` | workspace-member of `crates/` | no |
| 19 | `crates/xray/Cargo.toml` | `xray` | workspace-member of `crates/` | no |
| 20 | `apps/desktop/src-tauri/Cargo.toml` | `opc` | **workspace-root** (self-contained, isolated) | yes (10 241 lines) |
| 21 | `platform/services/deployd-api-rs/Cargo.toml` | `deployd-api` | **standalone** | yes (5 067 lines) |
| 22 | `tools/adapter-scopes-compiler/Cargo.toml` | `open_agentic_adapter_scopes_compiler` | **standalone** | yes (598 lines) |
| 23 | `tools/assumption-cascade-check/Cargo.toml` | `open_agentic_assumption_cascade_check` | **standalone** | yes (3 869 lines) |
| 24 | `tools/ci-parity-check/Cargo.toml` | `open_agentic_ci_parity_check` | **standalone** | yes (597 lines) |
| 25 | `tools/codebase-indexer/Cargo.toml` | `open_agentic_codebase_indexer` | **standalone** | yes (1 614 lines) |
| 26 | `tools/oap-code-index-enrich/Cargo.toml` | `open_agentic_code_index_enrich` | **standalone** | yes (1 666 lines) |
| 27 | `tools/oap-registry-enrich/Cargo.toml` | `open_agentic_registry_enrich` | **standalone** | yes (717 lines) |
| 28 | `tools/policy-compiler/Cargo.toml` | `open_agentic_policy_compiler` | **standalone** | yes (1 654 lines) |
| 29 | `tools/registry-consumer/Cargo.toml` | `open_agentic_spec_registry_reader` | **standalone** | yes (566 lines) |
| 30 | `tools/shared/spec-types/Cargo.toml` | `open_agentic_spec_types` | **standalone** | yes (129 lines) |
| 31 | `tools/spec-code-coupling-check/Cargo.toml` | `open_agentic_spec_code_coupling_check` | **standalone** | yes (1 661 lines) |
| 32 | `tools/spec-compiler/Cargo.toml` | `open_agentic_spec_compiler` | **standalone** | yes (1 553 lines) |
| 33 | `tools/spec-lint/Cargo.toml` | `open_agentic_spec_lint` | **standalone** | yes (599 lines) |
| 34 | `tools/stakeholder-doc-lint/Cargo.toml` | `open_agentic_stakeholder_doc_lint` | **standalone** | yes (1 065 lines) |

## Workspace structure today

Two workspace roots exist; the remaining 14 manifests are standalone.

### Workspace 1 — `crates/Cargo.toml`

18 members, all under `crates/`:

```toml
[workspace]
members = [
    "agent",
    "agent-frontmatter",
    "artifact-extract",
    "axiomregent",
    "factory-contracts",
    "factory-engine",
    "factory-platform-client",
    "factory-project-detect",
    "featuregraph",
    "orchestrator",
    "policy-kernel",
    "provenance-validator",
    "provider-registry",
    "run",
    "skill-factory",
    "standards-loader",
    "tool-registry",
    "xray",
]
resolver = "2"
```

No `[workspace.dependencies]` block — each member declares its own deps.

### Workspace 2 — `apps/desktop/src-tauri/Cargo.toml`

Self-contained, deliberately isolated from `crates/`:

```toml
# Self-contained workspace — isolated from root to prevent libsqlite3-sys
# linking conflicts when both this crate and crates/axiomregent bundle rusqlite.
[workspace]
publish = false
```

Single-member workspace (the `opc` binary itself); imports 13 crates from `crates/` and `tools/` via `path = "../../../..."` deps. The isolation is functional (SQLite linking) — I1 must preserve the same isolation guarantee while folding into a single root workspace, or document the constraint that requires `opc` to remain separated.

### Standalone manifests (14)

- `platform/services/deployd-api-rs/Cargo.toml`
- All 12 `tools/*/Cargo.toml` (excluding `tools/shared/spec-types/` which is a path-dep target but also standalone — it has its own `Cargo.lock`).

Each standalone manifest has its own `Cargo.lock`. Builds invoked via `cargo build --release --manifest-path <path>/Cargo.toml`.

## Path dependencies between crates

47 path-dep declarations across 24 manifests:

### Intra-workspace (inside `crates/`)

`../<sibling>` form — these become workspace member-to-member refs after I1.

| consumer | dep name | path |
|---|---|---|
| `crates/agent/Cargo.toml` | `agent-frontmatter` | `../agent-frontmatter` |
| `crates/artifact-extract/Cargo.toml` | `factory-contracts` | `../factory-contracts` |
| `crates/axiomregent/Cargo.toml` | `featuregraph` | `../featuregraph` |
| `crates/axiomregent/Cargo.toml` | `xray` | `../xray` (features = `analysis-call-graph`) |
| `crates/axiomregent/Cargo.toml` | `agent` | `../agent` |
| `crates/axiomregent/Cargo.toml` | `run` | `../run` |
| `crates/axiomregent/Cargo.toml` | `open_agentic_policy_kernel` | `../policy-kernel` |
| `crates/axiomregent/Cargo.toml` | `tool-registry` | `../tool-registry` |
| `crates/axiomregent/Cargo.toml` | `skill-factory` | `../skill-factory` |
| `crates/factory-contracts/Cargo.toml` | `agent-frontmatter` | `../agent-frontmatter` |
| `crates/factory-engine/Cargo.toml` | `orchestrator` | `../orchestrator` |
| `crates/factory-engine/Cargo.toml` | `factory-contracts` | `../factory-contracts` |
| `crates/factory-engine/Cargo.toml` | `artifact-extract` | `../artifact-extract` |
| `crates/factory-engine/Cargo.toml` | `provenance-validator` | `../provenance-validator` |
| `crates/factory-engine/Cargo.toml` | `standards-loader` | `../standards-loader` |
| `crates/factory-engine/Cargo.toml` | `policy-kernel` | `../policy-kernel` (package = `open_agentic_policy_kernel`) |
| `crates/factory-platform-client/Cargo.toml` | `factory-engine` | `../factory-engine` |
| `crates/factory-platform-client/Cargo.toml` | `factory-contracts` | `../factory-contracts` |
| `crates/factory-project-detect/Cargo.toml` | `factory-contracts` | `../factory-contracts` |
| `crates/featuregraph/Cargo.toml` | `xray` | `../xray` |
| `crates/orchestrator/Cargo.toml` | `provider-registry` | `../provider-registry` (default-features = false; features = anthropic, openai) |
| `crates/orchestrator/Cargo.toml` | `standards-loader` | `../standards-loader` |
| `crates/provenance-validator/Cargo.toml` | `factory-contracts` | `../factory-contracts` |
| `crates/provider-registry/Cargo.toml` | `policy-kernel` | `../policy-kernel` (package = `open_agentic_policy_kernel`, default-features = false) |
| `crates/skill-factory/Cargo.toml` | `agent-frontmatter` | `../agent-frontmatter` |
| `crates/skill-factory/Cargo.toml` | `tool-registry` | `../tool-registry` |
| `crates/tool-registry/Cargo.toml` | `policy-kernel` | `../policy-kernel` (package = `open_agentic_policy_kernel`, default-features = false) |

### Cross-tree from `crates/` into `tools/`

These break workspace encapsulation today (a workspace member of `crates/` reaches into a standalone `tools/` manifest). I1 must collapse both into the single root workspace.

| consumer | dep name | path |
|---|---|---|
| `crates/factory-engine/Cargo.toml` | `open_agentic_spec_registry_reader` | `../../tools/registry-consumer` |
| `crates/featuregraph/Cargo.toml` | `open_agentic_spec_registry_reader` | `../../tools/registry-consumer` |

### Cross-tree from `tools/` into `crates/`

Symmetric: standalone `tools/` reaches into `crates/`.

| consumer | dep name | path |
|---|---|---|
| `tools/assumption-cascade-check/Cargo.toml` | `factory-engine` | `../../crates/factory-engine` |
| `tools/policy-compiler/Cargo.toml` | `open_agentic_policy_kernel` | `../../crates/policy-kernel` |
| `tools/stakeholder-doc-lint/Cargo.toml` | `factory-contracts` | `../../crates/factory-contracts` |
| `tools/stakeholder-doc-lint/Cargo.toml` | `provenance-validator` | `../../crates/provenance-validator` |

### Within `tools/` (sibling deps)

| consumer | dep name | path |
|---|---|---|
| `tools/codebase-indexer/Cargo.toml` | `open_agentic_spec_types` | `../shared/spec-types` |
| `tools/oap-code-index-enrich/Cargo.toml` | `open_agentic_codebase_indexer` | `../codebase-indexer` |
| `tools/oap-code-index-enrich/Cargo.toml` | `open_agentic_spec_types` | `../shared/spec-types` |
| `tools/oap-registry-enrich/Cargo.toml` | `open_agentic_spec_registry_reader` | `../registry-consumer` |
| `tools/oap-registry-enrich/Cargo.toml` | `open_agentic_spec_types` | `../shared/spec-types` |
| `tools/policy-compiler/Cargo.toml` | `open_agentic_spec_types` | `../shared/spec-types` |
| `tools/spec-code-coupling-check/Cargo.toml` | `open_agentic_codebase_indexer` | `../codebase-indexer` |
| `tools/spec-compiler/Cargo.toml` | `open_agentic_spec_types` | `../shared/spec-types` |
| `tools/spec-lint/Cargo.toml` | `open_agentic_spec_types` | `../shared/spec-types` |

### From `apps/desktop/src-tauri/`

Deepest depth (`../../../...`). All 13 deps reach into `crates/` and `tools/`.

| consumer | dep name | path |
|---|---|---|
| `apps/desktop/src-tauri/Cargo.toml` | `xray` | `../../../crates/xray` |
| `apps/desktop/src-tauri/Cargo.toml` | `featuregraph` | `../../../crates/featuregraph` |
| `apps/desktop/src-tauri/Cargo.toml` | `open_agentic_spec_registry_reader` | `../../../tools/registry-consumer` |
| `apps/desktop/src-tauri/Cargo.toml` | `agent` | `../../../crates/agent` |
| `apps/desktop/src-tauri/Cargo.toml` | `run` | `../../../crates/run` |
| `apps/desktop/src-tauri/Cargo.toml` | `orchestrator` | `../../../crates/orchestrator` |
| `apps/desktop/src-tauri/Cargo.toml` | `provider-registry` | `../../../crates/provider-registry` (default-features = false; features = anthropic, openai) |
| `apps/desktop/src-tauri/Cargo.toml` | `policy-kernel` | `../../../crates/policy-kernel` (package = `open_agentic_policy_kernel`, default-features = false) |
| `apps/desktop/src-tauri/Cargo.toml` | `factory-engine` | `../../../crates/factory-engine` |
| `apps/desktop/src-tauri/Cargo.toml` | `factory-contracts` | `../../../crates/factory-contracts` |
| `apps/desktop/src-tauri/Cargo.toml` | `factory-platform-client` | `../../../crates/factory-platform-client` |
| `apps/desktop/src-tauri/Cargo.toml` | `factory-project-detect` | `../../../crates/factory-project-detect` |
| `apps/desktop/src-tauri/Cargo.toml` | `provenance-validator` | `../../../crates/provenance-validator` |

## Crates not in any workspace today

14 manifests are standalone:

- `apps/desktop/src-tauri/Cargo.toml` (single-member workspace = effectively standalone for I1 purposes)
- `platform/services/deployd-api-rs/Cargo.toml`
- All 12 `tools/*/Cargo.toml` files
- `tools/shared/spec-types/Cargo.toml`

Of these:

- **`platform/services/deployd-api-rs/Cargo.toml`** is the deployd Rust service; per master plan, `platform/` is unchanged structurally — its standalone status is intentional and remains so post-cleanup. Should it be folded into the root workspace in I1? The locked target layout under `Cargo.toml # one root workspace` suggests yes, but `platform/` is its own evolution unit. **Open question** for operator.
- **`apps/desktop/src-tauri/Cargo.toml`** is functionally isolated due to SQLite linking conflicts. I1 must either (a) keep it isolated, accepting layout drift from "one root workspace," (b) fold in and find a SQLite resolution. Master plan §Locked target layout suggests one root workspace. **Open question** — recommendation pending; check `apps/desktop/src-tauri/Cargo.toml` header comment for the linking-conflict context.
- **All 12 `tools/*/Cargo.toml`** are independent today. Each has its own `Cargo.lock`. I1 folds them into the root workspace.
- **`tools/shared/spec-types/Cargo.toml`** is a path-dep target for 6 sibling tools but is itself standalone with its own `Cargo.lock`. I1 folds it in.

## Crates with their own Cargo.lock (to be removed in I1)

16 lockfiles total today:

| path | lines | disposition in I1 |
|---|---|---|
| `crates/Cargo.lock` | 8 321 | becomes the root `Cargo.lock` |
| `apps/desktop/src-tauri/Cargo.lock` | 10 241 | if folded into root: removed. If kept isolated: stays. |
| `platform/services/deployd-api-rs/Cargo.lock` | 5 067 | if folded: removed. If kept standalone: stays. |
| `tools/assumption-cascade-check/Cargo.lock` | 3 869 | removed |
| `tools/oap-code-index-enrich/Cargo.lock` | 1 666 | removed |
| `tools/spec-code-coupling-check/Cargo.lock` | 1 661 | removed |
| `tools/policy-compiler/Cargo.lock` | 1 654 | removed |
| `tools/codebase-indexer/Cargo.lock` | 1 614 | removed |
| `tools/spec-compiler/Cargo.lock` | 1 553 | removed |
| `tools/stakeholder-doc-lint/Cargo.lock` | 1 065 | removed |
| `tools/oap-registry-enrich/Cargo.lock` | 717 | removed |
| `tools/spec-lint/Cargo.lock` | 599 | removed |
| `tools/adapter-scopes-compiler/Cargo.lock` | 598 | removed |
| `tools/ci-parity-check/Cargo.lock` | 597 | removed |
| `tools/registry-consumer/Cargo.lock` | 566 | removed |
| `tools/shared/spec-types/Cargo.lock` | 129 | removed |

Note: `.gitignore` re-includes specific lockfiles via `!tools/policy-compiler/Cargo.lock`, `!tools/oap-registry-enrich/Cargo.lock`, `!tools/oap-code-index-enrich/Cargo.lock` (`.gitignore:23,25,26`) — see Group H in D1. I1 removes those `!`-re-includes when the lockfiles are deleted.

## Cargo `[package.metadata.oap]` (spec attribution)

All 33 leaf manifests (excluding `crates/Cargo.toml` which is workspace-only) declare a spec under `[package.metadata.oap].spec`. This is the spec 127 coupling-gate signal; D4 enumerates which specs need `implements:` updates when paths move. I1 itself does not move any manifest path — it only adds them to a workspace and removes their individual lockfiles. **No `implements:` updates are expected to fire from I1 alone**.

Detected spec attribution (the field as `spec = "<id>"`):

| manifest | spec |
|---|---|
| `crates/agent-frontmatter/Cargo.toml` | `054-agent-frontmatter-schema` |
| `crates/agent/Cargo.toml` | `035-agent-governed-execution` |
| `crates/artifact-extract/Cargo.toml` | `120-factory-extraction-stage` |
| `crates/axiomregent/Cargo.toml` | `073-axiomregent-unification` |
| `crates/factory-contracts/Cargo.toml` | `074-factory-ingestion` |
| `crates/factory-engine/Cargo.toml` | `075-factory-workflow-engine` |
| `crates/factory-platform-client/Cargo.toml` | `124-opc-factory-run-platform-integration` |
| `crates/factory-project-detect/Cargo.toml` | `112-factory-project-lifecycle` |
| `crates/featuregraph/Cargo.toml` | `034-featuregraph-registry-scanner-fix` |
| `crates/orchestrator/Cargo.toml` | `052-state-persistence` |
| `crates/policy-kernel/Cargo.toml` | `047-governance-control-plane` |
| `crates/provenance-validator/Cargo.toml` | `121-claim-provenance-enforcement` |
| `crates/provider-registry/Cargo.toml` | `042-multi-provider-agent-registry` |
| `crates/run/Cargo.toml` | `052-state-persistence` |
| `crates/skill-factory/Cargo.toml` | `071-skill-command-factory` |
| `crates/standards-loader/Cargo.toml` | `055-yaml-standards-schema` |
| `crates/tool-registry/Cargo.toml` | `067-tool-definition-registry` |
| `crates/xray/Cargo.toml` | `032-opc-inspect-governance-wiring-mvp` |
| `platform/services/deployd-api-rs/Cargo.toml` | `073-axiomregent-unification` |
| `tools/adapter-scopes-compiler/Cargo.toml` | `105-scripts-to-binaries-migration` |
| `tools/assumption-cascade-check/Cargo.toml` | `121-claim-provenance-enforcement` |
| `tools/ci-parity-check/Cargo.toml` | `104-makefile-ci-parity-contract` |
| `tools/codebase-indexer/Cargo.toml` | `101-codebase-index-mvp` |
| `tools/oap-code-index-enrich/Cargo.toml` | `101-codebase-index-mvp` |
| `tools/oap-registry-enrich/Cargo.toml` | `102-governed-excellence` |
| `tools/policy-compiler/Cargo.toml` | `047-governance-control-plane` |
| `tools/registry-consumer/Cargo.toml` | `002-registry-consumer-mvp` |
| `tools/shared/spec-types/Cargo.toml` | `000-bootstrap-spec-system` |
| `tools/spec-code-coupling-check/Cargo.toml` | `127-spec-code-coupling-gate` |
| `tools/spec-compiler/Cargo.toml` | `001-spec-compiler-mvp` |
| `tools/spec-lint/Cargo.toml` | `006-conformance-lint-mvp` |
| `tools/stakeholder-doc-lint/Cargo.toml` | `122-stakeholder-doc-inversion` |

## Phase I1 readiness summary

- **Crates to add to root workspace:** 13 (`apps/desktop/src-tauri/`, `platform/services/deployd-api-rs/`, 12× `tools/*/`, `tools/shared/spec-types/`)
  - Conditional on operator decision: depending on `apps/desktop` and `platform/services/deployd-api-rs` disposition, this could drop to 10 or 11.
- **Crates already in a workspace:** 18 (under `crates/Cargo.toml`)
- **Path deps to be re-expressed:** 47 declarations across 24 manifests
  - Within `crates/` (intra-workspace siblings): 27 — already use `../<sibling>`; either keep as-is or hoist to `[workspace.dependencies]` once root workspace exists.
  - Cross-tree `crates/` → `tools/`: 2
  - Cross-tree `tools/` → `crates/`: 4
  - Within `tools/`: 9
  - From `apps/desktop/src-tauri/`: 13 (after I7 `apps/desktop/` move to `product/apps/desktop/`, paths deepen by one level)
- **Cargo.lock files to be deleted:** 15 (assuming both `apps/desktop` and `platform/deployd-api-rs` are folded in; 13 if `apps/desktop` stays isolated; 14 if only `deployd-api-rs` stays standalone)
- **Spec-attribution entries:** 32 manifests carry `[package.metadata.oap].spec` — these are spec 127 coupling-gate signals but I1 itself does not change any code paths or `implements:` targets, so the gate should not fire on I1.
- **Workspace dep hoisting** (optional, deferred to follow-up): once a root workspace exists, the 24 within-workspace path deps can be expressed as `<dep>.workspace = true` after declaring `[workspace.dependencies]`. Master plan §Locked target layout doesn't mandate this; I1 only consolidates roots.
- **`apps/desktop/src-tauri/Cargo.toml` SQLite isolation** is a real constraint, not aesthetic. I1 must either (a) preserve the isolation (keeping it as a separate workspace inside the otherwise consolidated tree), (b) find a SQLite linking resolution before folding in. Master plan §Locked target layout shows one root workspace; **operator triage required**.
- **`platform/services/deployd-api-rs/`** lives under `platform/` which master plan §Locked target layout marks as "(internal structure unchanged)." Folding it into the root workspace **breaks the structural-unchanged invariant** for platform/. **Operator triage required**: keep standalone (preserve platform/ isolation, accept Cargo workspace drift) or fold (cleaner workspace, breaks platform/ promise).
- **Estimated complexity:** **medium**.
  - Mechanical part (write root `Cargo.toml`, list members, delete lockfiles) is low.
  - Conditional parts (apps/desktop isolation, deployd-api inclusion) require operator decision.
  - Within-workspace `path =` deps continue to work once members are listed; no per-dep edits needed for the basic fold.
  - Verification: after I1, `cargo build --workspace --release` should succeed and produce a single `Cargo.lock`. If apps/desktop or deployd-api stays separate, that workspace verifies independently.

## Open questions (surface for operator triage)

1. **`apps/desktop/src-tauri/` SQLite linking conflict.** The header comment in `apps/desktop/src-tauri/Cargo.toml:1-2` says "Self-contained workspace — isolated from root to prevent libsqlite3-sys linking conflicts when both this crate and crates/axiomregent bundle rusqlite." Should I1 (a) keep it isolated and document the layout exception, (b) find a SQLite linking resolution as part of I1, or (c) defer to follow-up and accept that "one root workspace" is "one root workspace plus apps/desktop"? Recommendation pending operator decision.
2. **`platform/services/deployd-api-rs/` standalone status.** Master plan §Locked target layout marks `platform/` as "internal structure unchanged" but also says "one root workspace." These conflict for deployd-api-rs. Resolution options: (a) keep deployd-api standalone, accept "one root workspace" is "one root workspace plus platform/services/deployd-api-rs/"; (b) fold deployd-api in, accept platform/ structural drift; (c) define a sub-workspace under `platform/` and leave platform/ otherwise unchanged.
3. **Workspace member ordering after move phases.** I5 moves `tools/<name>/` → `tools/spec-spine/<name>/` or `tools/oap/<name>/`. The root workspace `members` array updates in I5 too — the I1 → I5 sequence means the `members` array is written twice. Confirm that's acceptable, or stage I1 + I5 ordering so members are listed once correctly.
4. **`[workspace.dependencies]` hoisting.** Master plan doesn't mandate this. Defer to follow-up?

## Cross-phase notes

- I1 lands before I5 (tools restructure), so the workspace fold happens with current tool paths; I5 then updates `members` entries.
- I7 (product layer) moves `apps/desktop/` to `product/apps/desktop/` — the root workspace `members` entry updates in I7 too.
- I9 (`build/` → `.derived/`) doesn't touch any Cargo manifest; no workspace impact.
