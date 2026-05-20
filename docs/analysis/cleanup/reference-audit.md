# Reference Audit — Path Group Inventories

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** `git grep -nE`, `find`, manual inspection. Read-only.
**Scope:** 15 path groups (A–O) — every reference to a moving path catalogued with `file:line` evidence so Epic 2 can update them in lockstep with each move.

## Classification key

- **code-import** — Rust `use` / Cargo `path = "..."`, TS `import`, Python `import`. Compile-time / runtime dependency carried in source.
- **path-literal** — string literal containing the path: `include_str!`, runtime `Path::join`, CI workflow `manifest-path`, Makefile recipe, `.gitignore`, `.gitattributes`, glob patterns.
- **spec-implements** — appears under a spec frontmatter `implements:` value (spec 127 coupling-relevant).
- **doc-prose** — markdown narrative, code comments. Not load-bearing; should still be updated for accuracy.
- **gitignore-rule** — `.gitignore` / `.git*` rules.
- **auto-regenerated** — appears in a generated artifact (`build/codebase-index/index.json`, lockfiles, golden fixtures) that regenerates after the move and stages the new paths.

Auto-regenerated entries are listed once per group with their total count; they do not require per-line manual updates in Epic 2 (the move itself triggers regeneration).

## Cross-group conventions

1. **Spec frontmatter (`implements:`)** classifications below are best-effort from frontmatter context only — D4 produces the authoritative manifest of which spec rows change in which Epic 2 phase.
2. **Doc-prose hot spots** consistently include `docs/analysis/spec-spine-cut-d-*.md` and `docs/analysis/init-trace.md`. These are session-frozen analysis artifacts referenced as historical record — they capture the pre-cleanup state intentionally. Epic 2 does not need to "update" them; new analysis can document the post-cleanup state if needed.
3. **`build/codebase-index/index.json`** appears in many groups (Layer 1 inventory + Layer 2 spec→path traceability). After `make registry`, regenerates to reference the new paths. Treated as auto-regenerated.
4. **`pnpm-lock.yaml`** and `crates/Cargo.lock` regenerate on `pnpm install` / `cargo build`. Auto-regenerated.
5. **`crates/featuregraph/tests/golden/features_graph.json`** is a golden test fixture; regenerates via golden-update flow (see featuregraph crate tests). Auto-regenerated, but the fixture-update commit is needed.

---

## Group A — `.specify/`

**Description:** Spec Kit content paths. Deleted in Epic 2 Phase I13 after Phase I3 graduates substantive content to `standards/spec/`. Total: 95 references.

### Load-bearing references (code-import / path-literal in active code or workflow)

| file:line | category | context |
|---|---|---|
| `.claude/commands/init.md:12` | path-literal | "Read all files in `.specify/memory/` if the directory exists" — init protocol step |
| `.claude/commands/init.md:26` | path-literal | `.specify/contract.md` — constitutional contract summary read |
| `.claude/commands/init.md:68` | doc-prose | Template summary "From memory: <key points from `.specify/memory/`>" |
| `.specify/scripts/bash/common.sh:195` | path-literal | Template lookup paths |
| `.specify/scripts/bash/common.sh:202` | path-literal | `local base="$repo_root/.specify/templates"` — base path for template resolution |
| `.specify/scripts/bash/update-agent-context.sh:90` | path-literal | `TEMPLATE_FILE="$REPO_ROOT/.specify/templates/agent-file-template.md"` |
| `.specify/templates/plan-template.md:6` | path-literal | Template self-reference |
| `.specify/contract.md:5` | doc-prose | Precedence rules |
| `.specify/contract.md:24` | doc-prose | Workflow scripts location |
| `.specify/memory/constitution.md:8,9,42,44` | doc-prose | Constitution precedence + cross-references |
| `.github/CODEOWNERS:17` | path-literal | `/.specify/ @bartlomiejkus` |
| `CLAUDE.md:42` | doc-prose | Repository structure table entry: ".specify/ — Spec Kit contract metadata and templates" |

### Spec frontmatter (`implements:`)

| file:line | category | context |
|---|---|---|
| `specs/119-project-as-unit-of-governance/spec.md:41` | spec-implements | `- path: .specify/contract.md` |

### Spec narrative / doc-prose

| file:line | category | context |
|---|---|---|
| `specs/000-bootstrap-spec-system/checklists/requirements.md:25` | doc-prose | "`specs/` vs `.specify/` location decision explicit" |
| `specs/000-bootstrap-spec-system/spec.md:62` | doc-prose | "Initial `.specify/` layout contract" |
| `specs/000-bootstrap-spec-system/spec.md:115` | doc-prose | "Initial `.specify/` contract" section header |
| `specs/000-bootstrap-spec-system/spec.md:121,122,123,130,355` | doc-prose | `.specify/` content references in normative narrative |
| `specs/000-bootstrap-spec-system/quickstart.md:31,36` | doc-prose | `.specify/templates/spec-template.md` lookup |
| `specs/000-bootstrap-spec-system/tasks.md:22` | doc-prose | T003 historical note |
| `specs/004-spec-to-execution-bridge-mvp/plan.md:9,43` | doc-prose | `.specify/templates/` references |
| `specs/004-spec-to-execution-bridge-mvp/tasks.md:6,19` | doc-prose | `.specify/contract.md` references |
| `specs/100-post-convergence-remediation/spec.md:69` | doc-prose | `.specify/templates/spec-template.md` |
| `specs/119-project-as-unit-of-governance/spec.md:261,290` | doc-prose | `.specify/contract.md` amendment narrative |
| `specs/132-constitutional-invariant-freeze/spec.md:46` | doc-prose | `.specify/contract.md` reference |
| `specs/143-presigned-upload-public-endpoint/spec.md:1013` | doc-prose | Convention cite |

### Analysis / planning doc-prose

| file:line | category | context |
|---|---|---|
| `docs/analysis/init-trace.md:18-19,21,50-52,144,147,151,169-171,199-202,213,217-218,232` | doc-prose | Init-trace catalogue — references the `.specify/` shape under audit |
| `docs/analysis/spec-spine-cut-d-architectural-review-prompt.md:304-305` | doc-prose | Architectural-review prompt — references constitution + contract |
| `docs/analysis/spec-spine-cut-d-architectural-review.md:697` | doc-prose | Review report cross-reference |
| `docs/analysis/spec-spine-footprint.md:133-135,355` | doc-prose | Footprint catalogue |
| `docs/analysis/spec-spine-init-trace-prompt.md:111,112,211,225,254,261,262` | doc-prose | Init-trace prompt content |
| `docs/analysis/cleanup/cleanup-master-plan.md:185,208,218,268` | doc-prose | Own master plan referencing `.specify/` deletion |
| `docs/analysis/cleanup/epic-1-discovery-prompt.md:97,98-103,208,517,768,779` | doc-prose | This audit's prompt |

### Auto-regenerated

| file:line | category | context |
|---|---|---|
| `build/codebase-index/index.json:3414` | auto-regenerated | Inventory entry for `.specify/contract.md` (codebase index Layer 1) |

### Summary

- **Total references:** 95
- **code-import:** 0
- **path-literal (load-bearing):** 8 (init.md ×2; specify/scripts ×3; specify/templates ×1; CODEOWNERS ×1; CLAUDE.md table ×1)
- **spec-implements:** 1 (spec 119)
- **doc-prose:** 85 (spec narrative + analysis docs; majority is `specs/000-bootstrap-spec-system/spec.md` + `docs/analysis/*.md` historical material)
- **auto-regenerated:** 1 (`build/codebase-index/index.json:3414`)
- **Bytes in `.specify/`:** 5 files — `contract.md`, `memory/constitution.md`, `templates/*.md`, `scripts/bash/*.sh`, `init-options/` (if any). Graduated to `standards/spec/` in I3; remaining shell scripts retire or move per master plan.

---

## Group B — Root `schemas/`

**Description:** Four authored schemas at repo root. Move to `standards/schemas/spec-spine/` (codebase-index variants) and `standards/schemas/frontmatter/` (agent-/skill-frontmatter) per master plan §Locked target layout. Total: 82 references after filtering out crate-internal `*/schemas/`.

### Load-bearing references

| file:line | category | context |
|---|---|---|
| `tools/codebase-indexer/src/schema.rs:6,8` | path-literal | `let schema_path = repo_root.join("schemas/codebase-index.schema.json");` — runtime read |
| `tools/codebase-indexer/src/types.rs:1,25` | doc-prose | Module-doc comment refs to schema files |
| `tools/codebase-indexer/tests/schema_conformance.rs:29,89,94` | path-literal | Test load of `schemas/codebase-index.schema.json` |
| `.githooks/pre-commit:18,46` | path-literal | Pre-commit hook documentation refs `schemas/*.json` glob |
| `CLAUDE.md:89` | path-literal | "codebase index hashes ... `schemas/*.json`" |
| `Makefile:169` | doc-prose | Help text: "`schemas/*.json`, `.github/workflows/*.yml`" |
| `schemas/agent-frontmatter.schema.json:3` | path-literal | `$id`: `https://open-agentic-platform.dev/schemas/agent-frontmatter.schema.json` |
| `schemas/codebase-index-oap.schema.json:3` | path-literal | `$id` URI |
| `schemas/codebase-index.schema.json:3` | path-literal | `$id` URI |
| `schemas/skill-frontmatter.schema.json:3` | path-literal | `$id` URI |

`$id` URIs are absolute URLs that include the path component — moving the file changes the canonical URL. The URLs are not dereferenced at runtime (validators load by file path), but `$id` should track the new location.

### Spec frontmatter (`implements:`)

| file:line | category | context |
|---|---|---|
| `specs/129-granular-package-oap-metadata/spec.md:18` | spec-implements | `- path: schemas/codebase-index.schema.json` |
| `specs/133-amends-aware-coupling-gate/spec.md:23` | spec-implements | `- path: schemas/codebase-index.schema.json` |

### Spec narrative

| file:line | category | context |
|---|---|---|
| `specs/054-agent-frontmatter-schema/spec.md:236,248` | doc-prose | Phase 5 — generate `schemas/agent-frontmatter.schema.json` |
| `specs/101-codebase-index-mvp/spec.md:71,139,277` | doc-prose | Schema location narrative |
| `specs/133-amends-aware-coupling-gate/plan.md:134` | doc-prose | "Update `schemas/codebase-index.schema.json`" |
| `specs/133-amends-aware-coupling-gate/spec.md:187` | doc-prose | Schema update bullet |
| `specs/133-amends-aware-coupling-gate/tasks.md:106` | doc-prose | T022 |
| `specs/074-factory-ingestion/spec.md:57` | doc-prose | "schemas/" diagram (within factory tree) |

### Cross-references in stagecraft tests (path-literal but unrelated)

| file:line | category | context |
|---|---|---|
| `platform/services/stagecraft/web/app/components/artifact-body-viewer.test.ts:126,131,140,142` | path-literal | Test fixtures using `schemas/...` strings — these are arbitrary fixture paths, not refs to repo-root `schemas/`. Note for review: confirm fixtures are not impacted. |

### Analysis docs

| file:line | category | context |
|---|---|---|
| `docs/analysis/init-trace.md:178-180,216` | doc-prose | Schema cross-reference table |
| `docs/analysis/spec-spine-cut-d-architectural-review-prompt.md:255-256,310,312` | doc-prose | Schema-location architecture question |
| `docs/analysis/spec-spine-cut-d-architectural-review.md:609,613-619,672,676,704,851,883-884` | doc-prose | Schema-home analysis |
| `docs/analysis/spec-spine-cut-d-plan.md:111,227-228,539-540,798,802` | doc-prose | Schema-location planning |
| `docs/analysis/spec-spine-cut-d-self-validation-fix-prompt.md:9,134,220,248,286` | doc-prose | Schema-location aside |
| `docs/analysis/spec-spine-init-trace-prompt.md:266` | doc-prose | Schema enumeration |
| `docs/analysis/spec-spine-footprint.md:92,93` | doc-prose | Schema footprint table |

### Auto-regenerated

| file:line | category | context |
|---|---|---|
| `build/codebase-index/index.json:3976,4061` | auto-regenerated | Inventory entries for `schemas/codebase-index.schema.json` |

### Summary

- **Total references:** 82 (after filtering crate-internal schemas)
- **code-import:** 0
- **path-literal (load-bearing):** 9 (indexer schema.rs, conformance tests, pre-commit hook, CLAUDE.md inputs list, 4 `$id` URLs)
- **spec-implements:** 2 (specs 129, 133)
- **doc-prose:** 69 (spec narrative + analysis docs)
- **auto-regenerated:** 2 (`build/codebase-index/index.json`)

---

## Group C — `specs/000-bootstrap-spec-system/contracts/`

**Description:** Two bootstrap schemas (`registry.schema.json`, `build-meta.schema.json`). Move to `standards/schemas/spec-spine/` per master plan. Total: 41 references.

### Load-bearing references

| file:line | category | context |
|---|---|---|
| `tools/spec-compiler/src/schema.rs:9` | doc-prose | Module-doc comment "schemas live under `<root>/specs/000-bootstrap-spec-system/contracts/`" |
| `tools/spec-compiler/src/schema.rs:13` | path-literal | `include_str!("../../../specs/000-bootstrap-spec-system/contracts/registry.schema.json")` |
| `tools/spec-compiler/src/schema.rs:16` | path-literal | `include_str!("../../../specs/000-bootstrap-spec-system/contracts/build-meta.schema.json")` |
| `tools/spec-compiler/tests/schema_conformance.rs:30` | path-literal | `load_schema("specs/000-bootstrap-spec-system/contracts/registry.schema.json")` |
| `tools/spec-compiler/tests/schema_conformance.rs:42` | path-literal | `load_schema(".../build-meta.schema.json")` |
| `tools/spec-compiler/tests/schema_conformance.rs:75,81` | path-literal | Multi-line test loads |
| `specs/000-bootstrap-spec-system/quickstart.md:20,23` | path-literal | `ajv-cli validate -s specs/000-.../contracts/...` shell commands |

### Spec frontmatter (`implements:`)

| file:line | category | context |
|---|---|---|
| `specs/132-constitutional-invariant-freeze/spec.md:21` | spec-implements | `- path: specs/000-bootstrap-spec-system/contracts/registry.schema.json` |

### Spec narrative

| file:line | category | context |
|---|---|---|
| `specs/001-spec-compiler-mvp/data-model.md:19,20` | doc-prose | Schema location table |
| `specs/001-spec-compiler-mvp/spec.md:107,108,113` | doc-prose | FR-003/FR-004 references |
| `specs/001-spec-compiler-mvp/tasks.md:4` | doc-prose | Prerequisites note |
| `specs/002-registry-consumer-mvp/spec.md:147` | doc-prose | SC-004 stability assertion |
| `specs/039-feature-id-reconciliation/spec.md:106` | doc-prose | Schema-edit table |
| `specs/132-constitutional-invariant-freeze/spec.md:96` | doc-prose | Schema additions bullet |
| `specs/147-spec-kind-grammar/plan.md:52,56` | doc-prose | Phase plan file list |
| `specs/147-spec-kind-grammar/registry.schema.json.patch:2` | doc-prose | Patch header reference |
| `docs/adr/0001-feature-id-reconciliation.md:13,71` | doc-prose | ADR cross-reference |

### Analysis docs

| file:line | category | context |
|---|---|---|
| `docs/analysis/init-trace.md:176,177,214,215` | doc-prose | Schema-table in init trace |
| `docs/analysis/spec-spine-cut-d-architectural-review.md:611-612,693` | doc-prose | Architecture-review file list |
| `docs/analysis/spec-spine-cut-d-self-validation-fix-prompt.md:12,77,107,124,127,223` | doc-prose | Self-validation prompt body |

### Auto-regenerated

| file:line | category | context |
|---|---|---|
| `build/codebase-index/index.json:4041` | auto-regenerated | Inventory entry |

### Summary

- **Total references:** 41
- **code-import:** 0
- **path-literal (load-bearing):** 7 (2 `include_str!`, 4 test loaders, 2 `ajv-cli` invocations in quickstart)
- **spec-implements:** 1 (spec 132)
- **doc-prose:** 32 (spec narrative + analysis docs)
- **auto-regenerated:** 1

---

## Group D — `crates/factory-contracts/schemas/`

**Description:** Factory YAML schemas — 4 top-level (`adapter-manifest`, `build-spec`, `pipeline-state`, `verification`) + 5 under `stage-outputs/` (`audiences`, `business-rules`, `entity-model`, `sitemap`, `use-cases`). Move to `standards/schemas/factory/` per master plan. Total: 22 references.

### Load-bearing references (in active code)

| file:line | category | context |
|---|---|---|
| `platform/services/stagecraft/api/factory/oapContracts.ts:5,20,86-88,110` | path-literal | Stagecraft runtime walks up looking for `crates/factory-contracts/schemas/` and emits substrate rows. Path appears as a directory-walk target and in emitted row `path:` fields. |
| `platform/services/stagecraft/api/factory/projection.ts:99` | doc-prose | Comment: "OAP-owned contract schemas under `crates/factory-contracts/schemas/`" |
| `platform/services/stagecraft/api/factory/substrateBrowser.ts:70` | doc-prose | Comment ref |
| `platform/services/stagecraft/api/factory/syncPipeline.ts:62,167` | doc-prose | Comment refs |

### Spec frontmatter (`implements:`)

| file:line | category | context |
|---|---|---|
| `specs/112-factory-project-lifecycle/spec.md:33` | spec-implements | `- path: crates/factory-contracts/schemas/` |

### Spec narrative

| file:line | category | context |
|---|---|---|
| `specs/074-factory-ingestion/spec.md:344,348` | doc-prose | Schema directory description |
| `specs/108-factory-as-platform-feature/spec.md:316` | doc-prose | Schema enumeration |
| `specs/112-factory-project-lifecycle/spec.md:170` | doc-prose | Schema directory reference |
| `specs/119-project-as-unit-of-governance/spec.md:308` | doc-prose | Schema-edit note |
| `specs/139-factory-artifact-substrate/plan.md:81` | doc-prose | Schema location |
| `specs/139-factory-artifact-substrate/spec.md:737` | doc-prose | "9 files, declared..." |

### Auto-regenerated

| file:line | category | context |
|---|---|---|
| `build/codebase-index/index.json:3063` | auto-regenerated | Inventory entry |

### Summary

- **Total references:** 22
- **code-import:** 0
- **path-literal (load-bearing):** 1 hot spot — stagecraft `oapContracts.ts` walks the dir at runtime; the walk-up target string `crates/factory-contracts/schemas/` is hardcoded (5 line refs)
- **spec-implements:** 1 (spec 112)
- **doc-prose:** 15
- **auto-regenerated:** 1
- **Move impact:** `oapContracts.ts` walk needs the new path under `standards/schemas/factory/` *and* the emitted substrate row `path:` field changes shape; stagecraft tests asserting on path strings (`platform/services/stagecraft/web/app/components/artifact-body-viewer.test.ts:140,142`) also need update.

---

## Group E — `crates/agent/src/schemas/`

**Description:** Two schemas (`verification.schema.json`, `verify-result.schema.json`) physically present, but not currently loaded by code (no `include_str!`, no runtime read found in `crates/agent/`). `$id` uses an opaque `axiomregent://spec/...` scheme rather than the file path. Move to `standards/schemas/agent/` per master plan. Total: 3 references (all in path-itself).

### Load-bearing references

| file:line | category | context |
|---|---|---|
| `crates/agent/src/schemas/verification.schema.json:3` | path-literal | `$id`: `axiomregent://spec/verification.schema.json` — opaque URI, not tied to filesystem path |
| `crates/agent/src/schemas/verify-result.schema.json:3` | path-literal | `$id`: `axiomregent://spec/verify-result.schema.json` |

### Doc-prose

| file:line | category | context |
|---|---|---|
| `docs/analysis/cleanup/cleanup-master-plan.md:90,91` | doc-prose | Target-layout entry |
| `docs/analysis/cleanup/epic-1-discovery-prompt.md:458` | doc-prose | Mention in prompt |

### Open question (no consumer in code)

No `include_str!`, no `Path::join("crates/agent/src/schemas/...")`, no `serde_json::from_str(SCHEMA)` reference found inside `crates/agent/`. The files appear to be dormant schemas. Surface for operator triage in D-phase open questions — possible disposition options:
1. Move to `standards/schemas/agent/` and leave dormant (preserves them for future use).
2. Delete in I4 if confirmed unused.
3. Move and add a verification step that exercises them.

### Summary

- **Total references:** 3 (2 `$id` URIs in the schemas themselves, 1 docs hit)
- **code-import:** 0
- **path-literal:** 2 (`$id` URIs — opaque scheme, no filesystem coupling)
- **doc-prose:** 1
- **Move impact:** lowest of any group — no callers to update.

---

## Group F — `standards/official/` and `standards/schema/`

**Description:** Coding-standard YAML files + their JSON schema. `standards/schema/standard.schema.json` validates `standards/official/*.yaml`. Per master plan target, `standards/official/` is retained at the same path (under `standards/coding/official/` — note path change from `standards/official/` to `standards/coding/official/`). `standards/schema/` moves under `standards/schemas/coding/`. Total: 11 references.

### Load-bearing references

| file:line | category | context |
|---|---|---|
| `crates/standards-loader/src/loader.rs:96` | doc-prose | Module-doc: "Reads `standards/official/`, `standards/community/`, `standards/local/`" |
| `packages/yaml-standards-schema/src/loader.test.ts:156,183` | path-literal | Test fixtures use `standards/official/...` |
| `packages/yaml-standards-schema/src/parser.test.ts:183` | path-literal | Test path literal |

### Spec narrative

| file:line | category | context |
|---|---|---|
| `specs/055-yaml-standards-schema/spec.md:64,80,144` | doc-prose | Standards directory contract |

### Doc-prose (own analysis)

| file:line | category | context |
|---|---|---|
| `docs/analysis/cleanup/epic-1-discovery-prompt.md:125-127,446` | doc-prose | This prompt |

### Summary

- **Total references:** 11 (note: actual reads are by glob — `standards-loader` reads directory dynamically; literal paths only appear in tests + module docs)
- **code-import:** 0
- **path-literal:** 3 (test fixtures × 3)
- **doc-prose:** 8
- **Move impact:** `standards-loader` reads via dynamic config (XDG paths + project root) — the directory path is not hardcoded in the runtime read but is documented. Test fixtures need path updates.
- **Open question:** Master-plan target layout puts coding standards under `standards/coding/official/`. The `standards-loader` defaults look for `standards/official/` directly. I3 needs to either (a) move under `coding/` and update the loader defaults, or (b) keep at `standards/official/` and accept layout drift. Operator decides.

---

## Group G — Spec-spine tools (move to `tools/spec-spine/`)

**Description:** 5 binaries — `spec-compiler`, `registry-consumer`, `codebase-indexer`, `spec-lint`, `spec-code-coupling-check`. Move from `tools/<name>/` to `tools/spec-spine/<name>/` per master plan I5. Total: 705 references (659 external to the tools themselves).

The volume here is dominated by Makefile/workflow recipes and historical spec verification artifacts that record `cargo build --manifest-path tools/<tool>/Cargo.toml` invocations. Most spec/*/execution/verification.md hits are completed-feature records — they are evidence of past work, not active build instructions. The master plan §Cross-epic invariants treats verification.md files as historical (no need to retroactively update).

### Hot spots (external refs only)

| file (refs) | category | impact |
|---|---|---|
| `Makefile` (45 refs) | path-literal | Recipes invoke `cargo build --manifest-path tools/<tool>/Cargo.toml` and run binaries from `./tools/<tool>/target/release/<tool>` — every recipe with these patterns updates in I5. D6 produces line-by-line mapping. |
| `.github/workflows/spec-conformance.yml` (22 refs) | path-literal | Workflow steps build + run the binaries. Every step updates in I5. D6 lists exact lines. |
| `tools/ci-parity-check/src/lib.rs` (12 refs) | path-literal | `CONSUMERS` / `PRODUCERS` registries hardcode tool paths to assert Makefile↔workflow parity (spec 104). Updates in I5. |
| `CLAUDE.md` (12 refs) | doc-prose | Repository structure + build command examples. Updates in I5. |
| `docs/analysis/spec-spine-footprint.md` (23 refs) | doc-prose | Frozen footprint; do not update. |
| `docs/analysis/spec-spine-cut-d-*.md` (44 refs total across verification, self-validation-fix-prompt, plan, architectural-review) | doc-prose | Frozen analysis; do not update. |
| `specs/<NNN>/execution/verification.md` (47+ refs across 029, 030, 031, 032, 007, 010, 011, 018, 145) | doc-prose | Completed-feature historical records. Do not rewrite history. |
| `specs/147-spec-kind-grammar/plan.md` (7), `specs/147-spec-kind-grammar/spec.md` (7), `specs/127-spec-code-coupling-gate/spec.md` (7), `specs/133-amends-aware-coupling-gate/{spec,plan,tasks}.md` (15+6+6), `specs/145-deployd-durability/tasks.md` (6) | doc-prose | Active spec content referencing tool paths in narrative; update in I5 alongside spec moves. |

### Load-bearing references in active code

| file:line | category | context |
|---|---|---|
| `tools/spec-compiler/src/schema.rs:13,16` | path-literal (cross-group) | `include_str!("../../../specs/000-bootstrap-spec-system/contracts/...")` — depth changes when spec-compiler relocates to `tools/spec-spine/spec-compiler/` (4 levels up instead of 3). |
| `tools/registry-consumer/tests/cli.rs:1466-...2183` | path-literal | `include_str!("fixtures/...")` — relative to crate root, so stays correct after move (Cargo handles the relocation). |
| `tools/ci-parity-check/src/lib.rs:592` | path-literal | Hardcoded path `./tools/adapter-scopes-compiler/target/release/adapter-scopes-compiler` — this is Group H, but relevant for I5 ordering: ci-parity-check needs path updates when sibling tools move. |
| `tools/spec-compiler/src/lib.rs:1102` | path-literal | `"pnpm-workspace.yaml" | "pnpm-lock.yaml" | ".sops.yaml"` — V-004 exclusion list. Group M-adjacent; npm-files-at-root remain at repo root post-move under `product/` only if I7 also updates this exclusion. |

### Cargo path dependencies referencing Group G tools

| file:line | context |
|---|---|
| `crates/factory-engine/Cargo.toml:open_agentic_spec_registry_reader = { path = "../../tools/registry-consumer" }` | Cross-tree path dep. Becomes a workspace dep after I1; literal path updates when registry-consumer moves under `tools/spec-spine/` in I5. |
| `crates/featuregraph/Cargo.toml:open_agentic_spec_registry_reader = { path = "../../tools/registry-consumer" }` | Same. |
| `apps/desktop/src-tauri/Cargo.toml:open_agentic_spec_registry_reader = { path = "../../../tools/registry-consumer" }` | Same; depth changes after both I5 and I7. |
| `tools/oap-registry-enrich/Cargo.toml:open_agentic_spec_registry_reader = { path = "../registry-consumer" }` | Sibling path dep — after I5 splits into `oap/` and `spec-spine/`, becomes `../../spec-spine/registry-consumer`. |
| `tools/oap-code-index-enrich/Cargo.toml:open_agentic_codebase_indexer = { path = "../codebase-indexer" }` | Sibling path dep; same change as above. |
| `tools/spec-code-coupling-check/Cargo.toml:open_agentic_codebase_indexer = { path = "../codebase-indexer" }` | After I5 both end up in `tools/spec-spine/`, so the relative path stays `../codebase-indexer`. |
| `tools/{spec-compiler,spec-lint,policy-compiler,codebase-indexer,oap-code-index-enrich,oap-registry-enrich}/Cargo.toml: open_agentic_spec_types = { path = "../shared/spec-types" }` | Sibling path dep into `tools/shared/`. After I5, tools under `tools/spec-spine/` reach `../../shared/spec-types`; tools under `tools/oap/` reach `../../shared/spec-types`. |

### `.github/CODEOWNERS`

| file:line | context |
|---|---|
| `.github/CODEOWNERS:21-24` | `/tools/registry-consumer/`, `/tools/spec-compiler/`, `/tools/spec-lint/`, `/tools/codebase-indexer/` — owner-path entries update in I5 |

### Auto-regenerated

`build/codebase-index/index.json` (multiple lines) — inventory entries regenerate.

### Summary

- **Total references:** 705 (659 external)
- **code-import (Cargo path deps):** 8 declarations across `crates/`, `tools/`, `apps/`
- **path-literal in active code:** ~70 (Makefile recipes 45, workflow `spec-conformance.yml` 22, ci-parity-check fixtures 12, include_str! in spec-compiler 2, etc.)
- **spec-implements:** see D4
- **doc-prose:** ~580 (specs/*/execution/verification.md frozen historical records ≈ 160; specs/*/spec.md + plan.md + tasks.md ≈ 80; CLAUDE.md ≈ 12; docs/analysis frozen ≈ 100; .claude/commands ≈ 30; remaining inventory)
- **auto-regenerated:** ~50 in `build/codebase-index/index.json`

---

## Group H — OAP-specific tools (move to `tools/oap/`)

**Description:** 8 binaries — `oap-registry-enrich`, `oap-code-index-enrich`, `policy-compiler`, `adapter-scopes-compiler`, `assumption-cascade-check`, `ci-parity-check`, `schema-parity-check`, `stakeholder-doc-lint`. Move to `tools/oap/<name>/` per master plan I5. Total: 154 references (149 external).

### Hot spots (external refs only)

| file (refs) | category | impact |
|---|---|---|
| `Makefile` (≈30 refs) | path-literal | Recipes: `tools/oap-registry-enrich/`, `tools/oap-code-index-enrich/`, `tools/policy-compiler/`, `tools/assumption-cascade-check/`, `tools/ci-parity-check/`, `tools/schema-parity-check/`, `tools/stakeholder-doc-lint/`, `tools/adapter-scopes-compiler/`. D6 enumerates. |
| `.github/workflows/spec-conformance.yml` (≈10 refs), `.github/workflows/ci-parity.yml` (6 refs), `.github/workflows/ci-supply-chain.yml` (5 refs) | path-literal | Build + run invocations. |
| `.github/workflows/release-tools.yml` (1) | doc-prose | Comment refs |
| `.gitignore:25,26` | path-literal | `!tools/oap-registry-enrich/Cargo.lock`, `!tools/oap-code-index-enrich/Cargo.lock` — allowlist for lockfiles |
| `.gitignore:23` | path-literal | `!tools/policy-compiler/Cargo.lock` |
| `CLAUDE.md:117,120` | doc-prose | Examples |
| `README.md:201,214,215` | doc-prose | Quickstart examples |
| `AGENTS.md:14` | path-literal | "oap-code-index-enrich render → build/codebase-index/CODEBASE-INDEX.md" — protocol step (D-2.10 / D8) |
| `tools/codebase-indexer/src/types.rs:156` | doc-prose | Cross-reference comment to `tools/oap-code-index-enrich/src/types.rs` |
| `tools/codebase-indexer/src/manifest.rs:313,331,347` | doc-prose | Comment in manifest walker referencing `tools/*/` siblings |
| `tools/spec-compiler/src/lib.rs:1482` | doc-prose | Comment cross-ref |
| `tools/ci-parity-check/src/main.rs:60` | doc-prose | User-facing error string ref |
| `tools/ci-parity-check/src/lib.rs:592` | path-literal | Hardcoded path `./tools/adapter-scopes-compiler/target/release/...` |
| `tools/schema-parity-check/index.mjs:34` | path-literal | Self-reference in usage comment |
| `tools/schema-parity-check/walk-descriptor.mjs:10` | doc-prose | Self-reference |
| `tools/schema-parity-check/walk-descriptor.test.mjs:7` | doc-prose | Self-reference |

### Spec frontmatter (`implements:`)

| file:line | category | context |
|---|---|---|
| `specs/122-stakeholder-doc-inversion/spec.md:42` | spec-implements | `- path: tools/stakeholder-doc-lint/Cargo.toml` |
| `specs/125-schema-parity-walker-rebuild/spec.md:27` | spec-implements | `- path: tools/schema-parity-check/index.mjs` |
| `specs/134-fast-local-ci-mode/spec.md:18` | spec-implements | `- path: tools/ci-parity-check/src/lib.rs` |
| `specs/135-fast-ci-as-default/spec.md:20` | spec-implements | `- path: tools/ci-parity-check/src/lib.rs` |

### Spec narrative

| file:line | category | context |
|---|---|---|
| `specs/047-governance-control-plane/spec.md:270,271`, `.../execution/verification.md:26` | doc-prose | policy-compiler narrative |
| `specs/104-makefile-ci-parity-contract/spec.md:144,165,217,300` | doc-prose | ci-parity-check narrative |
| `specs/105-scripts-to-binaries-migration/spec.md:88,147` | doc-prose | adapter-scopes-compiler + policy-compiler narrative |
| `specs/116-supply-chain-policy-gates/spec.md:224-227` | doc-prose | Multi-tool list |
| `specs/122-stakeholder-doc-inversion/spec.md:348` | doc-prose | stakeholder-doc-lint reference |
| `specs/125-schema-parity-walker-rebuild/spec.md:48,75,116,167,237,243`, `.../plan.md:14,26,29,95,120`, `.../tasks.md:15,67,69,85,88,90,130` | doc-prose | schema-parity-check narrative |
| `specs/131-adversarial-prompt-refusal-policy/spec.md:127,197` | doc-prose | policy-compiler narrative |
| `specs/134-fast-local-ci-mode/spec.md:178`, `specs/135-fast-ci-as-default/spec.md:226,286,288` | doc-prose | ci-parity-check narrative |

### Tests + coverage docs

| file:line | category | context |
|---|---|---|
| `crates/factory-engine/tests/spec-122-coverage.md:18,21,43` | doc-prose | Coverage table refs to stakeholder-doc-lint + schema-parity-check |
| `crates/provenance-validator/tests/spec-121-coverage.md:34` | doc-prose | Coverage table |
| `crates/featuregraph/tests/golden/features_graph.json` (multiple lines) | auto-regenerated | Golden fixture entries |

### Analysis docs

| file:line | category | context |
|---|---|---|
| `docs/analysis/spec-spine-footprint.md:125,143,145-148,222,225` | doc-prose | Footprint catalogue |
| `docs/analysis/spec-spine-cut-d-{plan,verification,run-report,architectural-review}.md` | doc-prose | Multiple refs; frozen. |
| `.claude/commands/validate-and-fix.md:25` | doc-prose | Validate-and-fix tool refs |
| `docs/analysis/cleanup/epic-1-discovery-prompt.md:147-154` | doc-prose | This prompt |

### Summary

- **Total references:** 154 (149 external)
- **code-import (Cargo path deps):** see Cargo deps in Group G summary — `oap-registry-enrich` and `oap-code-index-enrich` import sibling tools; the Cargo `path = ".."` values change when the four-way split happens (`spec-spine/`, `oap/`, `shared/`, `vendor/`).
- **path-literal (load-bearing):** ~55 (Makefile, workflows, gitignore, AGENTS.md protocol, ci-parity-check hardcoded path)
- **spec-implements:** 4 (specs 122, 125, 134, 135)
- **doc-prose:** ~85
- **auto-regenerated:** ~10 in `build/codebase-index/index.json` + golden features_graph.json

---

## Group I — `tools/shared/spec-types/`

**Description:** Shared Rust crate housing frontmatter + V-/W-code constants (post-W-01). Master plan keeps it at `tools/shared/spec-types/` — verify references stay valid. Total: 45 references.

### Load-bearing references

| file:line | category | context |
|---|---|---|
| `tools/{spec-compiler,spec-lint,codebase-indexer,policy-compiler,oap-code-index-enrich,oap-registry-enrich}/Cargo.toml` | code-import | All 6 carry `open_agentic_spec_types = { path = "../shared/spec-types" }`. After I5 tools split into `spec-spine/`, `oap/`, the relative path becomes `../../shared/spec-types`. |
| `.github/workflows/ci-supply-chain.yml:86` | path-literal | `tools/shared/spec-types/Cargo.toml` in supply-chain pin list |
| `Makefile:584` | path-literal | `tools/shared/frontmatter/Cargo.toml` — refers to the *deleted* W-01 crate (`tools/shared/frontmatter/` was absorbed into `spec-types`); Makefile reference is **stale**. Surface as open question. |

### Doc-prose

| file:line | category | context |
|---|---|---|
| `DEVELOPERS.md` (no direct hit — confirmed via earlier list); `docs/ARCHITECTURE.md:21` | doc-prose | Crate-tree narrative |
| `tools/codebase-indexer/src/manifest.rs:313,331` | doc-prose | Comment cross-ref |
| `crates/factory-engine/tests/spec-122-coverage.md:8` (n/a) | n/a | None directly |
| Analysis docs: `docs/analysis/spec-spine-cut-d-{architectural-review-prompt,architectural-review,plan,verification,run-report,verification-prompt}.md` | doc-prose | ≈30 refs; frozen. |

### Auto-regenerated

| file:line | category | context |
|---|---|---|
| `build/codebase-index/index.json:1305,1403` | auto-regenerated | Inventory entries |

### Summary

- **Total references:** 45
- **code-import (Cargo path deps):** 6 (sibling tools)
- **path-literal:** 2 (workflow yaml, Makefile — note the stale `frontmatter/` ref on Makefile line 584)
- **doc-prose:** ~35
- **auto-regenerated:** 2
- **Open question:** Master plan's target layout retains `tools/shared/spec-types/` — but the Makefile:584 `tools/shared/frontmatter/Cargo.toml` reference points at a path the W-01 cleanup already deleted. Surface for I-phase: drop or fix this Makefile line.

---

## Group J — `grammars/`

**Description:** 5 tree-sitter grammars at repo root. Move to `tools/vendor/grammars/` per master plan I6. Total: 14 references.

### Load-bearing references

| file:line | category | context |
|---|---|---|
| `.claude/commands/cleanup.md:125` | doc-prose | "Grammar files in `grammars/` (used at build time or runtime)" |
| `tools/codebase-indexer/src/manifest.rs:347` | doc-prose | Walker comment "grammars/*/" |
| `grammars/tree-sitter-{c,javascript,python,rust,typescript}/.gitattributes:22,26` | path-literal | Within-grammar linguist rules; move with the directory and continue to work. Not an external ref. |

### Spec narrative

| file:line | category | context |
|---|---|---|
| `specs/032-opc-inspect-governance-wiring-mvp/execution/verification.md:38,47,60` | doc-prose | Tauri build instructions; cargo invocations reference grammars/ as build-time input |
| `specs/073-axiomregent-unification/spec.md:238,489` | doc-prose | Axiomregent build narrative |

### Doc-prose

| file:line | category | context |
|---|---|---|
| `docs/ARCHITECTURE.md:21` | doc-prose | Crate-tree narrative |
| `docs/analysis/cleanup/cleanup-master-plan.md:122` | doc-prose | Target-layout entry |
| `docs/analysis/cleanup/epic-1-discovery-prompt.md:160-162,520` | doc-prose | This prompt |

### Auto-regenerated

| file:line | category | context |
|---|---|---|
| `build/codebase-index/index.json:635-687,5167-5171` | auto-regenerated | Inventory entries (5 grammars + manifest list) |

### Summary

- **Total references:** 14
- **code-import:** 0 (axiomregent's `build.rs` reads `grammars/` directories via `cc` crate at compile time — see spec 073 §238. The build script reaches the grammars via relative path from `crates/axiomregent/build.rs`; D2 captures the exact path. After I6 the relative path changes from `../../grammars/tree-sitter-X` to `../../tools/vendor/grammars/tree-sitter-X`.)
- **path-literal:** 1 (build.rs path computation in axiomregent — must verify and capture in D2)
- **doc-prose:** 9
- **auto-regenerated:** 10
- **Move impact:** axiomregent `build.rs` is the only runtime-affecting ref; otherwise low-impact.

---

## Group K — `apps/desktop/`

**Description:** Tauri desktop app. Move to `product/apps/desktop/` per master plan I7. Total: 441 references (437 external).

### Hot spots

| file (refs) | category | impact |
|---|---|---|
| `apps/desktop/src/**` (internal) | n/a | Move with the dir; internal relative imports preserved. |
| `apps/desktop/src-tauri/Cargo.toml` (multi) | code-import | Path deps: `../../../crates/...`. After I1 root workspace + I7 move under `product/`, path resolution changes (`../../../crates/...` → `../../../../crates/...`). Workspace dep notation eliminates these literals. |
| `.github/workflows/build-axiomregent.yml` (8 refs) | path-literal | `cp apps/desktop/src-tauri/binaries/...` — bundled-binary copy paths |
| `.github/workflows/ci-desktop.yml` (multiple) | path-literal | Workflow paths |
| `pnpm-workspace.yaml` | path-literal | `apps/*` glob — updates to `product/apps/*` in I7 |
| `tools/spec-compiler/tests/v004_consolidation_excludes.rs:32,36` | path-literal | Exclusion list for V-004 — references workspace files |
| `crates/featuregraph/tests/golden/features_graph.json` (multiple) | auto-regenerated | Golden fixture |
| `apps/desktop/README.md:22` | doc-prose | Internal `README.md` (move with crate) |

### Spec frontmatter (`implements:`)

D4 will enumerate; many specs reference `apps/desktop/...` paths (042, 045, 050, 066, 069, 081, 087, 110, 137, etc. — D4 confirms full list).

### Spec narrative

Multiple specs (032 OPC wiring, 045 claude-code-sdk, 050 tool-renderer, 042 multi-provider, etc.) reference desktop paths in narrative.

### Auto-regenerated

`build/codebase-index/index.json` carries ≈ 90 entries for `apps/desktop/...`; regenerates.

### Summary

- **Total references:** 441 (437 external; ~4 internal self-refs in the desktop tree)
- **code-import (Cargo path deps):** 14 declarations in `apps/desktop/src-tauri/Cargo.toml` reach into `../../../crates/...` and `../../../tools/registry-consumer`. After I1, workspace deps replace path literals; after I7, residual path strings deepen by one level.
- **path-literal:** ≈ 25 (workflows + Makefile + spec-compiler exclusion list)
- **spec-implements:** see D4
- **doc-prose:** ≈ 300 in specs + analysis + .claude
- **auto-regenerated:** ≈ 90

---

## Group L — `packages/`

**Description:** 25 TypeScript workspace packages. Move to `product/packages/` per master plan I7. Total: 224 references (223 external).

### Hot spots

| file (refs) | category | impact |
|---|---|---|
| `pnpm-lock.yaml` (22 refs) | auto-regenerated | Lockfile entries — regenerate on `pnpm install` after workspace globs update. |
| `pnpm-workspace.yaml` | path-literal | `packages/*` glob — updates to `product/packages/*` in I7. |
| `build/codebase-index/index.json` (48 refs) | auto-regenerated | Layer 1 inventory entries. |
| `crates/featuregraph/tests/golden/features_graph.json` (20 refs) | auto-regenerated | Golden fixture. |
| `apps/desktop/src-tauri/src/commands/claude.rs:154,158,161,1200` | path-literal | Runtime path: `packages/provider-registry/dist/node-sidecar.js` — Tauri loads this sidecar from a path resolved relative to repo root. After I7 the path becomes `product/packages/provider-registry/dist/node-sidecar.js`. |
| `apps/desktop/vite.config.ts:18` | path-literal | "packages/ui/src to resolve their deps" — comment but indicates Vite config behavior |
| `apps/desktop/src/lib/contextCompaction.test.ts:634,657` | path-literal | Test fixture strings (`packages/api/handler.rs` — arbitrary fixture content, unrelated). Note for review. |
| `.claude/commands/cleanup.md:48,62,94,118` | path-literal | Cleanup glob patterns |
| `.claude/commands/code-review.md:29` | doc-prose | Glob "packages/**/*.ts" |
| `.github/spec-coupling-bypass.txt:16` | doc-prose | Spec coupling bypass narrative |
| `.github/workflows/ci-codebase-index.yml:22,31`, `.github/workflows/ci-desktop.yml:21,26` | path-literal | Workflow trigger paths |

### Spec frontmatter (`implements:`)

D4 enumerates.

### Spec narrative

Multiple specs (042, 043, 045, 048, 049, 050, 066, 069, 081, 085, 087, 110, 139) reference `packages/<name>/...` paths.

### Auto-regenerated

`pnpm-lock.yaml`, `build/codebase-index/index.json`, `crates/featuregraph/tests/golden/features_graph.json`, `apps/desktop/src-tauri/Cargo.lock` (if it references workspace packages).

### Summary

- **Total references:** 224 (223 external)
- **code-import:** 0 (TS workspace deps use npm package names, not paths)
- **path-literal:** ≈ 15 (Tauri sidecar runtime path, vite config, .claude/commands, workflow triggers, spec-compiler exclusion list)
- **spec-implements:** see D4
- **doc-prose:** ≈ 90
- **auto-regenerated:** ≈ 110

---

## Group M — Root npm files

**Description:** 4 files at root — `package.json`, `package-lock.json`, `pnpm-workspace.yaml`, `pnpm-lock.yaml`. Move to `product/` per master plan I7. Total: 44 references (filtered).

### Load-bearing references

| file:line | category | context |
|---|---|---|
| `.claude/commands/cleanup.md:97` | path-literal | `--ignore "node_modules,dist,build,.git,*.d.ts,pnpm-lock.yaml"` |
| `.githooks/pre-commit:16,45` | path-literal | "indexer hashes Cargo.toml, package.json, pnpm-workspace.yaml" |
| `CLAUDE.md:89` | path-literal | Same — codebase-indexer inputs list |
| `Makefile:166` | path-literal | Help text: "Cargo.toml, workspace + tool Cargo.tomls, package.json, pnpm-workspace.yaml" |
| `DEVELOPERS.md:85,86,88` | path-literal | Workspace docs |
| `platform/.prettierignore:4,5` | path-literal | `pnpm-lock.yaml`, `pnpm-workspace.yaml` — ignored under platform/ |
| `platform/CLAUDE.md:19` | doc-prose | Workspace exclusion narrative |
| `platform/services/stagecraft/Dockerfile:19` | doc-prose | Comment |
| `platform/services/stagecraft/docs/encore-custom-dockerfile.md:122` | doc-prose | Dockerfile example |
| `platform/services/tenant-hello/Dockerfile:3` | path-literal | `COPY package.json package-lock.json* ./` — applies to tenant-hello, not root |
| `platform/services/stagecraft/api/factory/projection.test.ts:121`, `.../translator.test.ts:151` | path-literal | Tests using `"package-lock.json"` as fixture |
| `specs/032-opc-inspect-governance-wiring-mvp/execution/verification.md:35,44,58,62` | doc-prose | OPC wiring narrative; historical |
| `specs/087-unified-workspace-architecture/spec.md:480` | doc-prose | NF-007.1 maintenance note |
| `specs/088-factory-upstream-sync/spec.md:198` | doc-prose | Lockfile classification table |
| `specs/116-supply-chain-policy-gates/spec.md:189` | doc-prose | Allowlist file list |
| `specs/151-declarative-cluster-reconciliation/spec.md:1264,1265` | doc-prose | Exemption rationale |
| `tools/codebase-indexer/src/lib.rs:446,447` | path-literal | `let pnpm_ws = repo_root.join("pnpm-workspace.yaml");` — runtime workspace-globs read |
| `tools/codebase-indexer/src/manifest.rs:377,378` | path-literal | Same — workspace globs read |
| `tools/spec-compiler/src/lib.rs:1102` | path-literal | `"pnpm-workspace.yaml" | "pnpm-lock.yaml" | ".sops.yaml"` — V-004 exclusion match |
| `tools/spec-compiler/tests/v004_consolidation_excludes.rs:32,36` | path-literal | Test fixtures for V-004 exclusion |
| `grammars/tree-sitter-*/.gitattributes:22,26` | path-literal | linguist rules within grammars (unrelated to root files) |

### Auto-regenerated

`build/codebase-index/index.json` — Layer 1 inventory includes these files.

### Summary

- **Total references:** 44 (excluding spurious grammars `.gitattributes` linguist rules unrelated to root)
- **code-import:** 0
- **path-literal (load-bearing):** 7 (codebase-indexer lib.rs:446,447; manifest.rs:377,378; spec-compiler lib.rs:1102; spec-compiler test v004_consolidation_excludes.rs ×2)
- **doc-prose:** ≈ 25
- **auto-regenerated:** several in `build/codebase-index/index.json`
- **Move impact:** **HIGH** — `codebase-indexer` reads `pnpm-workspace.yaml` from repo root at runtime. If I7 moves these to `product/`, the indexer must either (a) look in `product/pnpm-workspace.yaml` (loader update), or (b) keep a symlink/forward-stub at root, or (c) the master plan changes to keep these at root. The master plan §Locked target layout puts them under `product/`; D3 + D6 need to confirm the indexer change and Makefile/workflow updates needed in I7.

---

## Group N — Root loose docs

**Description:** 3 files at root — `DEVELOPERS.md`, `CONTRIBUTING.md`, `RELEASE-VERIFICATION.md`. Move to `docs/` per master plan I8. Total: ≈ 22 references.

### Load-bearing references

| file:line | category | context |
|---|---|---|
| `.github/spec-coupling-bypass.txt:29` | path-literal | `DEVELOPERS.md` in bypass list |
| `CLAUDE.md:68` | doc-prose | Markdown link `[DEVELOPERS.md](DEVELOPERS.md)` |
| `CONTRIBUTING.md:144` | doc-prose | Self-reference (move with the file) |
| `README.md:355` | doc-prose | Markdown link to DEVELOPERS.md |
| `platform/infra/hetzner/setup.sh:12` | doc-prose | Comment |

### Spec frontmatter (`implements:`)

| file:line | category | context |
|---|---|---|
| `specs/151-declarative-cluster-reconciliation/spec.md:31` | spec-implements | `- path: DEVELOPERS.md` |

### Spec narrative

| file:line | category | context |
|---|---|---|
| `specs/127-spec-code-coupling-gate/spec.md:126` | doc-prose | Bypass narrative listing root docs |
| `specs/086-open-source-launch/spec.md:26,40,50,70` | doc-prose | CONTRIBUTING.md narrative |
| `specs/102-governed-excellence/spec.md:313` | doc-prose | FR-021 CONTRIBUTING.md doc requirement |
| `specs/117-release-artifact-attestations/spec.md:76,97,220,308` | doc-prose | RELEASE-VERIFICATION.md narrative |
| `specs/151-declarative-cluster-reconciliation/spec.md:1189,1202,1213,tasks.md:23` | doc-prose | DEVELOPERS.md additions |

### Auto-regenerated

`build/codebase-index/index.json:5047` (DEVELOPERS.md inventory entry).

### Summary

- **Total references:** 22
- **code-import:** 0
- **path-literal:** 2 (spec-coupling-bypass.txt; CONTRIBUTING.md self-link)
- **spec-implements:** 1 (spec 151)
- **doc-prose:** 18
- **auto-regenerated:** 1
- **Move impact:** Markdown links `[DEVELOPERS.md](DEVELOPERS.md)` need update to `docs/DEVELOPERS.md`. Spec 151's `implements:` updates in same commit. Bypass list updates.

---

## Group O — `build/` (rename to `.derived/`)

**Description:** Compiler output directories — `build/spec-registry/`, `build/codebase-index/`, `build/schema-parity/`. Rename to `.derived/` per master plan I9. Total: 264 references.

### Hot spots

| file (refs) | category | impact |
|---|---|---|
| `.claude/commands/init.md:30` | path-literal | "build/codebase-index/index.json -- compiled structural inventory" — needs path update |
| `.claude/agents/{architect,explorer}.md` | doc-prose | Multiple refs to `build/spec-registry/registry.json` |
| `.claude/rules/governed-artifact-reads.md:15-18,26,35` | path-literal | Consumer-binary mapping table + bad/good patterns |
| `.githooks/pre-commit:20,30,48,52` | path-literal | "build/codebase-index/index.json in the diff" + `git add build/codebase-index/index.json` |
| `.github/workflows/ci-codebase-index.yml:4,25,34` | path-literal | Trigger paths + comments |
| `.github/workflows/ci-spec-code-coupling.yml:5` | doc-prose | Comment |
| `.gitignore:300,315,322,323,324` | gitignore-rule | `build/spec-registry/` ignored; `build/codebase-index/` with re-include of `index.json`. After I9 these become `.derived/...` patterns. |
| `.specify/contract.md:15` | doc-prose | Machine-truth narrative |
| `AGENTS.md:14` | path-literal | "build/codebase-index/CODEBASE-INDEX.md — rendered structural summary" — protocol step |
| `CLAUDE.md:16,66,67,68,89` | doc-prose | Multiple narrative refs |
| `CONTRIBUTING.md:141` | doc-prose | `build/codebase-index/CODEBASE-INDEX.md` link |
| `Makefile:143,151,174-177` | path-literal | Recipe comments + diff checks |
| `README.md:215` | doc-prose | `cat build/codebase-index/CODEBASE-INDEX.md` |
| `apps/desktop/README.md:22` | doc-prose | Refs `build/spec-registry/registry.json` |
| `apps/desktop/src-tauri/src/commands/analysis.rs:22,28` | path-literal | `let registry_path = repo_root.join("build/spec-registry/registry.json");` |
| `apps/desktop/src/stores/README.md:3` | doc-prose | Markdown link |
| `crates/factory-contracts/src/{knowledge,provenance,stakeholder_docs}.rs` | path-literal | `workspace_root().join("build/schema-parity/...")` — parity fingerprint dest paths (×4 lines across 3 files) |
| `crates/factory-engine/src/bin/{build_certificate,factory_run,verify_certificate}.rs` | doc-prose | CLI arg doc comments |
| `crates/factory-engine/src/governance_certificate.rs:713,766` | path-literal | `let registry_path = repo_root.join("build/spec-registry/registry.json");` |
| `crates/featuregraph/src/index_bridge.rs:119` | path-literal | `let index_path = Path::new("build/codebase-index/index.json");` |
| `crates/featuregraph/src/registry_source.rs:5` | doc-prose | Module doc |
| `crates/featuregraph/src/scanner.rs:174,176,349` | path-literal | Registry path resolution + error message |
| `crates/featuregraph/tests/golden.rs:17` | path-literal | Test path check |
| `docs/ARCHITECTURE.md:13,106` | doc-prose | Architecture narrative |
| `docs/adr/0001-feature-id-reconciliation.md:62` | doc-prose | Cross-ref |

### Spec narrative

Many specs reference `build/spec-registry/` and `build/codebase-index/` in narrative. Auto-regenerated artifact lookup; update narrative in I9.

### Summary

- **Total references:** 264
- **code-import:** 0
- **path-literal (load-bearing):** ≈ 20 (apps/desktop analysis.rs ×2; factory-contracts schema-parity ×4; factory-engine governance ×2; featuregraph ×4; .githooks/pre-commit ×3; AGENTS.md protocol; .claude/commands/init.md; .claude/rules; Makefile diff check; ci workflow triggers ×3)
- **gitignore-rule:** 5 (`.gitignore` entries — updated in I9)
- **doc-prose:** ≈ 235
- **Move impact:** **HIGH**. Multiple Rust crates carry `repo_root.join("build/...")` literals. I9 must update all of them in one atomic commit (coupling-gate fires if code path moves but `implements:` doesn't). Spec 103 governed-artifact-reads rule itself names `build/**`; the rule file updates in I9.

---

## Refinement notes

- **Group B (root `schemas/`)** initial pattern `^schemas/` and `[^/]schemas/` returned excess hits from crate-internal `*/schemas/` directories. Refined by filtering out `crates/*/schemas/`, `tools/*/schemas/`, `packages/*/schemas/`, `apps/*/schemas/`, `stagecraft/*/schemas/`, `platform/*/schemas/`. Note that `platform/services/stagecraft/web/app/components/artifact-body-viewer.test.ts:126,131,140,142` uses string literals containing "schemas/..." as test-fixture artifact paths — these are unrelated to the repo-root `schemas/` directory and surface only because the literal pattern matches.
- **Group J (grammars)** initial pattern `^grammars/` matched zero standalone hits because git-grep applies pattern per file, not per content. Refined to `grammars/tree-sitter-` plus a follow-up direct ls of the directory. Note `apps/desktop/src-tauri/Cargo.toml` does not directly reference `grammars/` — axiomregent's `build.rs` walks via the cc crate; that walk path lives in `crates/axiomregent/build.rs` (verify in D2).
- **Group K (`apps/desktop/`)** and **Group L (`packages/`)** patterns excluded explicit `^apps/desktop/` and `^packages/` self-prefixes to focus on external references. Internal references (within the moved tree) preserve correctness automatically.
- **Group I (`tools/shared/`)** surfaced a stale reference in `Makefile:584` pointing at the deleted `tools/shared/frontmatter/Cargo.toml`. Documented in summary as open question.
- **Group N (root loose docs)** uses pattern `^DEVELOPERS\.md`, etc., to scope to files-at-root references. Markdown links `[DEVELOPERS.md](DEVELOPERS.md)` are still picked up because git-grep matches per-line content.

## Open questions (surface for operator triage)

1. **Group E (`crates/agent/src/schemas/`)** has no callers in code. Move-and-keep, move-and-delete, or move-and-add-consumer? Recommendation pending operator decision.
2. **Group F (`standards/`)** — master plan target layout shows `standards/coding/official/` but the runtime `standards-loader` reads `standards/official/` by default. I3 either restructures or accepts layout drift. Operator decides which.
3. **Group I (`tools/shared/`)** — `Makefile:584` references the deleted `tools/shared/frontmatter/Cargo.toml`. Remove the stale line as part of I5, or earlier (it's already broken)?
4. **Group M (root npm files moving to `product/`)** — `codebase-indexer` reads `pnpm-workspace.yaml` from `repo_root`. I7 needs loader update (search `product/pnpm-workspace.yaml` first, fall back to root, or move the entire workspace contract under `product/`). Confirm exact resolution path in D3 + I7.
5. **Group O (`build/` → `.derived/`)** — multiple Rust crates carry hardcoded `repo_root.join("build/...")` literals. I9 needs one atomic commit touching every crate; ordering w.r.t. coupling-gate has to be verified per crate.
6. **Group B / E format unification** — Group D schemas are `.yaml`; Groups B, C, E, F use `.json`. Master plan defers format unification to follow-up; D5 captures explicitly. Confirm I4 only moves schemas verbatim (no format change).
7. **Frozen analysis docs** — every `docs/analysis/spec-spine-cut-d-*.md` and `docs/analysis/init-trace.md` reference moves under audit. These are session-frozen historical records. Confirm policy: leave as-is (recommended) vs. update post-cleanup with retrospective notes.

## Phase D1 readiness summary

- **15 groups inventoried** with file:line evidence for every load-bearing reference.
- **~2,100 total references** across all groups; ~80 are load-bearing path-literal or Cargo path-dep; the rest are doc-prose (≈ 70% in spec narrative + frozen analysis docs) and auto-regenerated (≈ 15% in `build/codebase-index/index.json`, lockfiles, golden fixtures).
- **D4 (spec implements inventory)** consumes the spec-implements rows from each group to build the per-spec change manifest.
- **D6 (workflow + Makefile inventory)** consumes the Makefile + workflow rows from each group to build the per-recipe change manifest.
- **Open questions** surface 7 items for operator triage at epic boundary.

This audit is descriptive. No path is moved, no reference is changed. Epic 2 reads this as its source of truth for which references update in which I-phase.
