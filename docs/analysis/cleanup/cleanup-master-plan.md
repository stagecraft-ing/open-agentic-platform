# Spec-Spine / OAP Structural Cleanup — Master Plan

**Branch:** `cut-d/autonomous-run-20260519-025506` (continuing as the
one-branch-to-fix-it-all).

**Scope:** Restructure the repository's top-level layout to match the
locked target shape, consolidate schemas under a single home, establish
a root Cargo workspace, resolve the eleven `/init` trace drift items,
resolve the render-path question (D-1 Option 3), generalize the V-007
emission lesson across all V-codes, and delete the vestigial Spec Kit
artifacts. After completion the branch is merge-ready.

**Posture:** No "tolerate-and-defer." Every reservation the
architectural review surfaced, every drift item the `/init` trace
found, every structural inconsistency identified is resolved on this
branch. Coherency, uniformity, compactness.

## Execution model

Two epics, two CC sessions, separated by operator review.

### Epic 1 — Discovery

One CC session. Read-only audits. Produces 10 manifest documents that
Epic 2 consumes as its rails. Every move, every reference, every
duplicate, every drift item is catalogued with file:line evidence
before any code is moved.

10 phases, each ending in one commit (or two for the implementation
manifest synthesis phase). All commits are `docs(cleanup): ...`
shapes. No code changes anywhere outside `docs/analysis/cleanup/`.

### Epic 2 — Implementation

One CC session, fired only after operator approves Epic 1's output.
Reads the implementation manifest from Epic 1 and executes phase by
phase. Each phase ends in a commit (or several small commits within
the phase if the manifest dictates). Agent works autonomously through
all 13 implementation phases.

13 phases, each ending in one or more commits (`refactor(cleanup):
...`, `fix(cleanup): ...` shapes).

## Locked target layout

```
open-agentic-platform/
  README.md
  LICENSE
  Makefile
  Cargo.toml                      # one root workspace
  Cargo.lock                      # single lockfile
  rust-toolchain.toml
  deny.toml
  CLAUDE.md
  AGENTS.md

  standards/                      # all normative reference content
    spec/                         # the spec-spine standard
      constitution.md
      contract.md
      spec-format.md              # NEW: placeholder, content deferred
      semver-policy.md            # NEW: placeholder, content deferred
      grammar/                    # generated, populated later
      codes/                      # V/W codes, generated later
      templates/
    coding/
      official/
    schemas/                      # all authored schemas, co-located
      spec-spine/
        registry.schema.json
        build-meta.schema.json
        codebase-index.schema.json
        codebase-index-oap.schema.json
      frontmatter/
        agent-frontmatter.schema.json
        skill-frontmatter.schema.json
      factory/
        adapter-manifest.schema.yaml
        build-spec.schema.yaml
        pipeline-state.schema.yaml
        verification.schema.yaml
        stage-outputs/
          audiences.schema.json
          business-rules.schema.json
          entity-model.schema.json
          sitemap.schema.json
          use-cases.schema.json
      agent/
        verification.schema.json
        verify-result.schema.json
      coding/
        standard.schema.json

  specs/                          # 152-spec corpus, location unchanged
    000-bootstrap-spec-system/    # contracts/ subdir removed
    001-spec-compiler-mvp/
    ...
    151-declarative-cluster-reconciliation/

  tools/
    spec-spine/                   # extractable bundle
      spec-compiler/
      registry-consumer/
      codebase-indexer/
      spec-lint/
      spec-code-coupling-check/
      scripts/
        bash/
    shared/
      spec-types/
    oap/
      oap-registry-enrich/
      oap-code-index-enrich/
      policy-compiler/
      adapter-scopes-compiler/
      assumption-cascade-check/
      ci-parity-check/
      schema-parity-check/
      stakeholder-doc-lint/
    vendor/
      grammars/
        tree-sitter-c/
        tree-sitter-javascript/
        tree-sitter-python/
        tree-sitter-rust/
        tree-sitter-typescript/

  crates/                         # 18 OAP Rust libraries, flat
    agent/                        # src/schemas/ removed
    agent-frontmatter/
    artifact-extract/
    axiomregent/
    factory-contracts/            # schemas/ removed
    factory-engine/
    factory-platform-client/
    factory-project-detect/
    featuregraph/
    orchestrator/
    policy-kernel/
    provenance-validator/
    provider-registry/
    run/
    skill-factory/
    standards-loader/
    tool-registry/
    xray/

  product/                        # end-user product layer
    apps/
      desktop/
    packages/                     # 25 TypeScript packages
    package.json
    package-lock.json
    pnpm-workspace.yaml
    pnpm-lock.yaml

  platform/                       # organizational platform layer
    (internal structure unchanged)

  docs/
    ARCHITECTURE.md
    DEVELOPERS.md
    CONTRIBUTING.md
    RELEASE-VERIFICATION.md
    adr/
    analysis/
    runbooks/
    contracts/                    # contract-governance per binary, populated later

  .derived/                       # gitignored generated artifacts
    spec-registry/
    codebase-index/
    schema-parity/

  .claude/
    agents/
    commands/
    rules/

  .github/
  .githooks/
```

`.specify/` is fully deleted. Spec Kit is superseded.

## Epic 1 phase index — Discovery

| Phase | Concern | Deliverable | Commit shape |
|---|---|---|---|
| D1 | Path reference audit (14 path groups) | `docs/analysis/cleanup/reference-audit.md` | `docs(cleanup): add path reference audit` |
| D2 | Cargo workspace inventory | `docs/analysis/cleanup/cargo-workspace-inventory.md` | `docs(cleanup): add Cargo workspace inventory` |
| D3 | TypeScript workspace inventory | `docs/analysis/cleanup/typescript-workspace-inventory.md` | `docs(cleanup): add TypeScript workspace inventory` |
| D4 | Spec `implements:` field inventory | `docs/analysis/cleanup/spec-implements-inventory.md` | `docs(cleanup): add spec implements inventory` |
| D5 | Schema duplication audit | `docs/analysis/cleanup/schema-duplication-audit.md` | `docs(cleanup): add schema duplication audit` |
| D6 | Workflow and Makefile inventory | `docs/analysis/cleanup/workflow-makefile-inventory.md` | `docs(cleanup): add workflow and Makefile inventory` |
| D7 | V-code emission audit | `docs/analysis/cleanup/vcode-emission-audit.md` | `docs(cleanup): add V-code emission audit` |
| D8 | Render-path decomposition design (D-1 Option 3) | `docs/analysis/cleanup/render-path-decomposition.md` | `docs(cleanup): add render path decomposition design` |
| D9 | Protocol drift resolutions (11 D-2.* items) | `docs/analysis/cleanup/protocol-drift-resolutions.md` | `docs(cleanup): add protocol drift resolutions` |
| D10 | Implementation manifest (synthesis) | `docs/analysis/cleanup/implementation-manifest.md` | `docs(cleanup): add implementation manifest` |

## Epic 2 phase index — Implementation

| Phase | Concern | Depends on | Commit shape |
|---|---|---|---|
| I1 | Root Cargo workspace consolidation | D2 | `refactor(cleanup): consolidate Rust crates into single workspace` |
| I2 | Create target directory skeleton | — | `refactor(cleanup): create target directory skeleton` |
| I3 | Standards content graduation | D1, I2 | `refactor(cleanup): graduate standards content from .specify/` |
| I4 | Schema co-location | D1, D5, I2 | `refactor(cleanup): consolidate authored schemas under standards/schemas/` |
| I5 | Tools restructure | D1, D4, I1, I2 | `refactor(cleanup): subdivide tools/ into spec-spine, shared, oap, vendor` |
| I6 | Grammars vendor move | D1, I2 | `refactor(cleanup): move tree-sitter grammars to tools/vendor/` |
| I7 | Product layer | D1, D3, I1, I2 | `refactor(cleanup): consolidate end-user product layer under product/` |
| I8 | Docs consolidation | D1, I2 | `refactor(cleanup): consolidate loose top-level docs under docs/` |
| I9 | `build/` → `.derived/` rename | D1, D6 | `refactor(cleanup): rename build/ to .derived/` |
| I10 | Protocol drift resolution | D9, I3–I9 | `refactor(cleanup): align /init protocol; AGENTS.md canonical` |
| I11 | Render-path resolution | D8, I5, I10 | `refactor(cleanup): decompose render path; generic core + OAP overlay` |
| I12 | V-code emission audit fixes | D7, I4 | `fix(cleanup): V-NNN follows producer-side enforcement pattern` (one or more) |
| I13 | Final cleanup; delete .specify/ | all prior | `refactor(cleanup): delete vestigial .specify/; cleanup complete` |

## Cross-epic invariants

These hold from epic start to epic end:

1. **Branch protection.** No commit prior to the cleanup is modified.
   The branch is append-only.
2. **`cargo test --workspace` clean at every commit.** Every commit
   leaves the workspace test-passing. If a phase cannot achieve this,
   it halts and surfaces to the operator.
3. **`/init` works at every commit.** Daily-driver capability is
   preserved throughout. Within a phase, `/init` may temporarily
   reference moved paths; by phase end, all references are aligned.
4. **Spec 127 coupling gate satisfied at every commit.** Code moves
   land with their `implements:` updates in the same commit. `make
   pr-prep` succeeds at every commit.
5. **Spec 103 governed-artifact-reads observed.** No ad-hoc parsing
   of `build/**` (or `.derived/**` post-I9). Consumer binaries only.
6. **One concern per phase.** No "while I'm here." Cross-phase
   concerns wait for their own phase.
7. **Reversibility.** Each phase's commits can be `git revert`'d
   without cascading damage.
8. **No new specs.** This is restructure, not feature work. New files
   (`spec-format.md`, `semver-policy.md`) are placeholders with TODO
   content; substantive authoring is deferred to post-cleanup work.

## Operator review at epic boundary

After Epic 1 completes, the operator reviews the 10 audit documents
and the implementation manifest. The manifest is the contract for
Epic 2 — it should be reviewable in its own right as a coherent plan
before Epic 2 fires.

If the operator finds issues with the manifest (a missed reference,
an ambiguous resolution, a phase ordering concern), they decide
whether:

- Epic 1 re-runs in correction mode for specific phases.
- The manifest is hand-edited and Epic 2 fires against the corrected
  version.
- The cleanup is paused for further design.

The branch is always a valid checkpoint between epics.

## What lands at the end

- Locked target layout achieved.
- One root Cargo workspace, one Cargo.lock.
- All authored schemas co-located under `standards/schemas/`.
- `.specify/` deleted.
- Standards content graduated to `standards/spec/`.
- `tools/` subdivided into `spec-spine/`, `shared/`, `oap/`, `vendor/`.
- `apps/`, `packages/`, npm files relocated under `product/`.
- Loose top-level docs consolidated under `docs/`.
- `build/` renamed to `.derived/`.
- `/init` protocol drift resolved; AGENTS.md is single canonical
  source; spec 103 violation closed.
- Render path resolved via generic-template + OAP-overlay pattern.
- V-code emission audit complete; all V-codes follow the V-007
  pattern.
- Branch is merge-ready to main.

## Out of scope

Tracked as follow-up but not landed in this branch:

- Schema-from-types generator (architectural review §Q5). Requires
  design work.
- `spec-format.md` and `semver-policy.md` substantive content.
- Cargo deps for tree-sitter grammars (replacing vendored content).
- shared-types decomposition into three crates (architectural review
  §Q1a).
- `registry-consumer` naming triple resolution.
- featuregraph extraction or further OAP-side cleanup.
- `coding-standard.schema.json` cross-tree duplication (surfaced in
  Epic 1 D5; resolved as follow-up).

## First action

Operator: fire Epic 1's prompt
(`docs/analysis/cleanup/epic-1-discovery-prompt.md`) in a fresh CC
session. Epic 1 produces all discovery artifacts and the
implementation manifest. No code changes.
