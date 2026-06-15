---
id: "217-spec-spine-engine-swap-collapse"
title: "Spec-Spine Engine Swap and Collapse: replace OAP's in-tree generic engine with the published spec-spine library"
feature_branch: "feat/217-spec-spine-engine-swap-collapse"
status: draft
implementation: pending
kind: migration
domain: tooling
created: "2026-06-15"
authors: ["open-agentic-platform"]
language: en
summary: >
  Delete OAP's in-tree generic spec-spine engine (spec-compiler,
  registry-consumer, codebase-indexer, spec-code-coupling-check, and the
  generic half of spec-lint) and replace it with the published external
  spec-spine library (spec-spine-types, spec-spine-core, spec-spine-cli).
  OAP retains only its overlay: the 16-kind enum, shape/category, compliance,
  factory/OWASP machinery, capability/registry/profile, stagecraft://
  provenance, and the OAP-domain lint codes. The engine swap is mechanical
  and corpus-governance work, not a research problem: the library compiled
  OAP's full 217-spec corpus with zero errors on 2026-06-15. A supersession
  and amendment wave retires the establishing specs for the deleted crates
  and repoints the surviving overlay specs to the library seam. Done when
  OAP's own coupling gate is spec-spine couple (green in CI), compile/
  lint/index run via the library, and zero generic engine code remains in-tree.
depends_on:
  - "216-spec-spine-library-grammar-adoption"
  - "130-spec-coupling-primary-owner"
  - "133-amends-aware-coupling-gate"
  - "154-logical-unit-ownership-grammar"
  - "155-logical-unit-resolution-semantics"
  - "101-codebase-index-mvp"
  - "127-spec-code-coupling-gate"
  - "152-path-co-authority"
  - "181-registry-consumer-unit-grammar-authority"
code_aliases: ["SPEC_SPINE_ENGINE_SWAP_COLLAPSE"]
# DRAFT frontmatter carries only edges that are TRUE NOW: 217 establishes
# spec-spine.toml (the Phase 0 root config it introduces), owns its
# featuregraph golden row (extends 034), and depends on 216. The supersession
# and amendment WAVE (documented in section 6) is deliberately NOT declared in
# this draft's frontmatter. Those supersedes/amends edges land in the
# implementation PR, atomically with the predecessor status flips and the binary
# deletion, so the spine never asserts an un-effected supersession while the
# in-tree binaries still exist (CONST-005; the relationship graph is current
# truth, not declared intent). Until then, 001/002/006/133/176/181 remain
# approved and authoritative over their still-present code.
establishes:
  - unit: { kind: file, path: spec-spine.toml }
extends:
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
---

# Feature Specification: Spec-Spine Engine Swap and Collapse

**Feature Branch**: `feat/217-spec-spine-engine-swap-collapse`
**Created**: 2026-06-15
**Status**: Draft
**Predecessor**: spec 216 (grammar adoption), which completed the grammar-convergence
prerequisite; this spec initiates the mechanical deletion and call-site rewiring.

---

## 1. Overview

OAP maintains a large in-tree generic spec-spine engine:
`spec-compiler`, `registry-consumer`, `codebase-indexer`,
`spec-code-coupling-check`, and the generic half of `spec-lint` (all
under `tools/spec-spine/`). The standalone `spec-spine` library
(crates.io: `spec-spine-types`, `spec-spine-core`, `spec-spine-cli`)
provides the same generic capability as a versioned, externally-tested
dependency. Running both is redundancy, not defence in depth: OAP has
been converging grammar onto the library since spec 216, and a
2026-06-15 dry run confirmed that the published 0.4.0 binary compiled
OAP's full 217-spec corpus with zero errors or warnings using the
`spec-spine.toml` config already at the repo root.

This spec governs the deletion of the in-tree generic engine and the
migration of all consumers to the library. OAP retains an overlay: the
16-kind enum, shape/category, compliance, factory/OWASP machinery,
capability/registry/profile, `stagecraft://` provenance, and the
OAP-domain lint codes. The overlay is not generic spec-spine; it is
OAP-specific typed modelling layered on `spec-spine-types` primitives.

**The overlay binary that replaces `registry-consumer`** for
authority-resolver queries is `oap-registry-enrich` (already at
`tools/oap/oap-registry-enrich`). The OAP-specific verbs
(`by-authority`, `validate-graph`, `show-relationships`,
`show-supersession-chain`, `show-constraints-on`) fold into
`oap-registry-enrich`, which reads the registry via
`spec-spine-core::load_registry`. Generic verbs (`list`, `show`,
`status-report`, `relationships`) are served by `spec-spine registry`.

**The gate is a confirmed drop-in.** `spec-spine couple` replicates OAP's
gate's exit-code contract (0=clean, 1=drift; no branching on any other
code), its waiver mechanism (`Spec-Drift-Waiver:` in PR body),
dependency-only auto-waive, amends-awareness, section-matching, and
claim-precedence (spec 216 FR-002 validation). The CI workflow
(`.github/workflows/ci-spec-code-coupling.yml`) survives with its
binary invocation changed.

---

## 2. Phased Delivery Plan

The implementation follows the phase sequence proven by the WS-B plan.
Each phase is independently deployable (CI stays green at each phase
boundary).

### Phase 0: Prerequisites (already satisfied)

- Spec 216 implementation complete (grammar-convergence prerequisite).
- `spec-spine.toml` config at repo root (namespace=oap, 4 domains,
  16 kinds, dual standalone Rust crates, product/ pnpm workspace,
  stagecraft standalone-npm).
- Library 0.4.0 published to crates.io (or pinned via git tag per the
  WS-B D1 dependency-mode decision).

**FR-000 (gate).** Before Phase 1 begins, `spec-spine compile` run
against the repo root with the committed `spec-spine.toml` exits 0 with
zero errors and zero warnings on the full corpus.

**AC-000.** CI job (or local equivalent) confirms FR-000. The
spec-spine.toml config matches the validated dry-run candidate.

### Phase 1: Build the OAP overlay layer (no deletion yet)

Slim `tools/shared/spec-types` from the full in-tree types crate to an
OAP overlay types crate that re-exports and extends `spec-spine-types`.
Generic primitives (`split_frontmatter_*`, `LogicalUnit`,
`ProvenanceKind`, `RESOLVER_EXCLUSIONS`, `FrontmatterError`) move to
`spec-spine-types` imports; OAP-specific typed modelling (the 16-value
`kind` enum, `VALID_KINDS`, `shape`/`SHAPE_TABLE`, `category`/
`CONVENTIONAL_CATEGORIES`, `VALID_DOMAINS`, capability/registry/profile
fields, `stagecraft://` provenance, legacy `implements`) stays in the
overlay crate. The Cargo.toml for `open_agentic_spec_types` gains
`spec-spine-types` as a direct dependency.

Slim `tools/spec-spine/spec-lint` to the OAP-domain lint codes only
(W-130/131/132 category/kind/shape, V-030/031 domain, W-161
decomposition-origin, L-005 workspace-migration). Generic codes
(W-001..007, V-020, L-001..004) are removed from the in-tree crate; the
OAP overlay lint invokes `spec-spine-core::lint` for the generic pass
and appends its own OAP codes.

Neither the producing binary (spec-compiler) nor the consuming binaries
(registry-consumer, codebase-indexer, spec-code-coupling-check) are
deleted in this phase. Build remains green.

**FR-101 (overlay types).** `open_agentic_spec_types` compiles against
`spec-spine-types`; its public API for OAP-specific types is unchanged
(downstream crates recompile without call-site changes).

**FR-102 (overlay lint).** `tools/spec-spine/spec-lint` compiles as an
OAP overlay: it re-exports or calls the library lint for the generic
pass and appends OAP-domain codes. The `spec-lint --fail-on-warn` CI
invocation exits 0 against the current corpus.

**AC-101.** `cargo build --manifest-path tools/shared/spec-types/Cargo.toml`
succeeds. All downstream crates that `path`-depend on spec-types compile
without modification.

**AC-102.** `spec-lint --fail-on-warn` exits 0. The OAP-domain codes
(W-130/131/132, V-030/031, W-161, L-005) are still exercised by their
existing fixture tests.

### Phase 2: Repoint the 8 consumer crates to the library

Each of the 8 crates that hold path-dependencies on the generic
in-tree crates is repointed:

| Consumer crate | Old dep | New dep |
|---|---|---|
| `oap-registry-enrich` | `registry-consumer` + `spec-types` | `spec-spine-core::load_registry` + overlay types |
| `oap-code-index-enrich` | `codebase-indexer` + `spec-types` | `spec-spine-core::load_index` + overlay types |
| `policy-compiler` | `spec-types` | overlay types |
| `spec-code-coupling-check` | both + `spec-types` | `spec-spine-core::couple_with` + overlay types |
| `featuregraph` | `registry-consumer` | `spec-spine-core::load_registry` (index_bridge.rs also moves from hand-rolled JSON to `load_index`) |
| `factory-engine` | `registry-consumer` | `spec-spine-core::load_registry` |
| `opc-decomposition-pipeline` | `spec-compiler` + `spec-lint` | `spec-spine-core::compile` (caller writes) + overlay lint |
| `product/apps/opc/src-tauri` | `registry-consumer` | `spec-spine-core::load_registry` |

The one raw artifact reader (`crates/featuregraph/src/index_bridge.rs`)
moves from hand-rolled `serde_json::from_str` to
`spec-spine-core::load_index`. `opc-decomposition-pipeline/promotion.rs`
moves from `spec_compiler::compile_and_write` to
`spec_spine_core::compile` plus a caller-side write (the library returns
`CompileOutcome`).

The `spec-code-coupling-check` crate's internal logic is replaced by a
thin shim that delegates to `spec-spine-core::couple_with`; the binary
itself will be deleted in Phase 3.

**FR-201 (library seam).** Every consumer crate listed above compiles
against the library seam. No consumer retains a direct path-dep on an
in-tree generic engine crate after this phase.

**FR-202 (compile API shape).** `opc-decomposition-pipeline/promotion.rs`
calls `spec_spine_core::compile` and writes the `CompileOutcome` to
disk. The in-tree `spec_compiler::compile_and_write` entry point is no
longer called.

**FR-203 (raw-reader elimination).** `crates/featuregraph/src/index_bridge.rs`
uses `spec-spine-core::load_index` exclusively. Zero direct
`serde_json::from_str` calls against `.derived/**/*.json` remain in any
consumer crate (governed-reads principle, spec 103).

**AC-201.** `cargo build --workspace` (excluding the in-tree generic
engine crates themselves) succeeds with no path-dep compilation errors.

**AC-202.** The featuregraph golden test passes (registry compiled first
per `feedback_golden_after_registry`).

**AC-203.** `spec-spine compile` exits 0 on the full corpus; the
generated registry is byte-identical to the in-tree spec-compiler's
output.

### Phase 3: Delete the generic engine

Remove the four generic binaries and the now-redundant generic
spec-lint crate:

- `tools/spec-spine/spec-compiler` (pkg `open_agentic_spec_compiler`)
- `tools/spec-spine/registry-consumer` (pkg `open_agentic_spec_registry_reader`)
- `tools/spec-spine/codebase-indexer` (pkg `open_agentic_codebase_indexer`)
- `tools/spec-spine/spec-code-coupling-check` (pkg `open_agentic_spec_code_coupling_check`)
- The generic half of `tools/spec-spine/spec-lint` (the OAP overlay
  lint crate remains; only the generic codes are removed, having moved
  to the library in Phase 1).

Drop each from `[workspace] members` in the root `Cargo.toml`. The
`.derived/codebase-index/` artifact, `codebase-index.schema.json`, and
the CI workflow `ci-spec-code-coupling.yml` **survive** (only the
producing and enforcing binaries change).

`oap-registry-enrich` absorbs the OAP-specific authority-resolver verbs
previously in `registry-consumer`. The `by-authority`, `validate-graph`,
`show-relationships`, `show-supersession-chain`, and
`show-constraints-on` subcommands are implemented there, reading via
`spec-spine-core::load_registry`.

**FR-301 (deletion).** All four generic engine crates are deleted from
`tools/spec-spine/`. No `Cargo.toml` in the workspace retains a
`path = "tools/spec-spine/<name>"` dependency after this phase.

**FR-302 (oap-registry-enrich authority verbs).** `oap-registry-enrich`
exposes `by-authority <path>`, `validate-graph`, `show-relationships`,
`show-supersession-chain`, and `show-constraints-on` subcommands. The
authority logic from spec 181 is re-implemented here, reading the
registry via `spec-spine-core::load_registry`. Exit codes and output
format are preserved (callers are unaffected).

**FR-303 (codebase-index artifact survives).** The
`.derived/codebase-index/` artifact (produced by `spec-spine index
compile`) is byte-identical in shape to the in-tree indexer's output
(same schema version). The committed `index.json` is regenerated in
this phase's commit.

**AC-301.** `cargo build --workspace` succeeds with the deleted crates
absent. No dangling `path` or `crate` dependencies remain.

**AC-302.** `oap-registry-enrich by-authority <path>` produces output
consistent with the pre-deletion `registry-consumer by-authority` for
the same path (authority set preserved).

**AC-303.** `spec-spine index check` exits 0 (fresh index). `spec-spine
couple` exits 0 (coupling gate clean). The CI coupling workflow passes
locally.

### Phase 4: Rewire call sites

Every invocation of the deleted binaries is replaced:

- **Makefile**: `make registry`, `make ci`, `make ci-strict`,
  `make pr-prep`, `make setup`, and all spec-compiler/indexer/coupling-
  check/registry-consumer invocations repoint to `spec-spine compile`,
  `spec-spine index compile`, `spec-spine index check`, `spec-spine
  couple`, `spec-spine registry list`, etc., plus `oap-registry-enrich`
  for the OAP authority verbs. The Makefile section co-authority owners
  (~8 specs) whose sections are edited are in the diff per their
  `co_authority` declarations.
- **CI workflow** `ci-spec-code-coupling.yml`: binary invocation
  changes to `spec-spine index check && spec-spine couple`. The file
  path and workflow structure survive.
- **`release-tools.yml`**: the `oap-tools-*.tar.gz` bundle drops the
  generic engine binaries; the release artifact now bundles only OAP
  overlay binaries plus the `spec-spine` binary (distributed separately
  or bundled per the crates.io release model).
- **`.claude/rules/governed-artifact-reads.md`**: the consumer table is
  repointed (spec 103 co-authority; amend record in 103's spec.md).
  `registry-consumer` becomes `spec-spine registry` (for list/show/
  status-report) and `oap-registry-enrich` (for authority verbs).
  `codebase-indexer` becomes `spec-spine index`. The bad-pattern
  examples are updated.
- **`.claude/skills/`** (init, setup, cleanup): tool invocations
  referencing the deleted binaries are updated.
- **`.githooks/pre-commit`** and the merge driver: hardcoded binary
  paths referencing the in-tree indexer are updated.
- **`platform/services/stagecraft/`**: `REGISTRY_CONSUMER_BIN` and
  its fallback logic repoints to `spec-spine registry` or
  `oap-registry-enrich` as appropriate (WS-B risk item R5).
- **`AGENTS.md`** and **`CLAUDE.md`**: binary name references updated.
- **`docs/`**: developer and architecture docs updated.

**FR-401 (Makefile rewire).** All Makefile targets that invoke the
deleted binaries use `spec-spine` subcommands or `oap-registry-enrich`.
`make ci` and `make pr-prep` execute successfully against the
post-deletion repo.

**FR-402 (governed-reads rule update).** `.claude/rules/governed-artifact-reads.md`
consumer table reflects the new consumer binaries. The rule's
enforcement logic and examples are accurate.

**FR-403 (stagecraft binary).** `REGISTRY_CONSUMER_BIN` in stagecraft
resolves to `spec-spine registry` or `oap-registry-enrich`; the service
starts without referencing a deleted binary.

**FR-404 (zero dead references).** A grep of the repo for the deleted
binary package names (`open_agentic_spec_compiler`,
`open_agentic_spec_registry_reader`, `open_agentic_codebase_indexer`,
`open_agentic_spec_code_coupling_check`) finds zero references outside
of spec.md files and git history. A grep for the deleted binary filenames
(`spec-compiler`, `registry-consumer`, `codebase-indexer`,
`spec-code-coupling-check`) in Makefile, workflows, `.claude/`, and
`AGENTS.md` / `CLAUDE.md` finds zero references.

**AC-401.** `make registry` exits 0 (calls `spec-spine compile`).

**AC-402.** `make pr-prep` exits 0 (calls `spec-spine index check` +
`spec-spine couple`).

**AC-403.** `make ci` exits 0 (full local validation suite, post-deletion).

**AC-404.** FR-404 grep check passes.

### Phase 5: Corpus governance wave

The supersession and amendment wave lands in the same PR as the
deletion (Phase 3) so the coupling gate never goes red: the wave is the
mechanism by which the gate knows to accept the deletion. Each
disposition is documented in section 6 (Supersession and Amendment Wave).

The bulk lifecycle migration of the 002-031 registry-consumer contract
series lands here. Each spec in the series carries an `extends` or
`refines` edge targeting `kind: crate, id: open_agentic_spec_registry_reader`;
that crate no longer exists. The migration:

- Sets `status: superseded` and `superseded_by: "217-spec-spine-engine-swap-collapse"`
  on each spec in the 003-031 series (002 is handled by the full
  supersede in this spec's frontmatter above).
- Strips the `open_agentic_spec_registry_reader` crate-unit from each
  spec's `extends`/`refines` edges (crate-unit with an unknown crate id
  yields an I-003 diagnostic in the library indexer; these are not
  live code claims and must be removed).
- For spec 029 (which also references a CI-path note about the
  registry-consumer binary path), updates that reference to
  `spec-spine registry`.

**FR-501 (wave integrity).** Every spec in the 003-031 series either
(a) carries `status: superseded, superseded_by: "217-spec-spine-engine-swap-collapse"`, or
(b) has its crate-unit stripped if it established only behavioural
contracts on the crate (no surviving code authority). The library
indexer reports zero I-003 diagnostics for `open_agentic_spec_registry_reader`
after the wave.

**FR-502 (wave is honest).** Each supersession and amendment disposition
reflects a genuine capability move or binary deletion. No spec is
superseded or amended purely to satisfy the gate. The auditor test is:
"would this disposition be correct even if the coupling gate did not
exist?"

**AC-501.** `spec-spine lint` (and the OAP overlay lint) exit 0 on the
post-wave corpus. No I-003 diagnostics for the deleted crate IDs.

**AC-502.** `spec-spine couple` exits 0 on the wave PR diff: every
edited path has its authority spec in the diff or a `Spec-Drift-Waiver:`
line in the PR body.

### Phase 6: Verify

Full CI pass on the combined deletion + wave PR:

- `spec-spine compile` exits 0; registry byte-identical to pre-deletion
  (same grammar, same corpus, same library version).
- `spec-spine index compile` + `spec-spine index check` exit 0; fresh
  `index.json` committed.
- `spec-spine couple` exits 0 on the PR diff.
- `spec-spine lint` exits 0 (OAP overlay lint appends clean).
- `oap-registry-enrich enrich` (compliance/OWASP report) emits; the
  report output is non-empty and structurally valid.
- Featuregraph golden regenerated (registry compiled first; per session
  memory pattern `feedback_golden_after_registry`).
- OPC reads the registry via `spec-spine-core::load_registry`; the
  desktop app starts and governs correctly.

---

## 3. Functional Requirements Summary

| ID | Phase | Requirement |
|---|---|---|
| FR-000 | 0 | Library 0.4.0 compiles full corpus, zero errors |
| FR-101 | 1 | Overlay types crate compiles against spec-spine-types |
| FR-102 | 1 | Overlay lint compiles; OAP-domain codes preserved |
| FR-201 | 2 | All 8 consumer crates compile against library seam |
| FR-202 | 2 | opc-decomposition-pipeline uses spec-spine-core::compile |
| FR-203 | 2 | featuregraph index_bridge uses spec-spine-core::load_index |
| FR-210 | 2 | oap-registry-enrich by-authority reads via load_registry |
| FR-301 | 3 | All four generic engine crates deleted from workspace |
| FR-302 | 3 | oap-registry-enrich exposes authority-resolver verbs |
| FR-303 | 3 | .derived/codebase-index/ artifact shape preserved |
| FR-401 | 4 | Makefile rewired to spec-spine subcommands |
| FR-402 | 4 | governed-reads rule consumer table updated |
| FR-403 | 4 | stagecraft REGISTRY_CONSUMER_BIN repointed |
| FR-404 | 4 | Zero dead references to deleted binary names |
| FR-501 | 5 | 003-031 wave: status superseded + crate-units stripped |
| FR-502 | 5 | Wave is honest (capability-moved rationale per spec) |
| FR-BLAST | 6 | Before/after authorities(P) diff shows zero paths lose all live owners |

---

## 4. Acceptance Criteria Summary

| ID | Phase | Criterion |
|---|---|---|
| AC-000 | 0 | spec-spine compile exits 0 on full corpus |
| AC-101 | 1 | spec-types overlay crate builds; downstream crates compile |
| AC-102 | 1 | spec-lint --fail-on-warn exits 0 |
| AC-201 | 2 | Workspace build succeeds (generic crates excluded) |
| AC-202 | 2 | Featuregraph golden passes (registry compiled first) |
| AC-203 | 2 | spec-spine compile output byte-identical to in-tree compiler |
| AC-301 | 3 | Workspace build succeeds post-deletion |
| AC-302 | 3 | oap-registry-enrich by-authority preserves authority output |
| AC-303 | 3 | spec-spine index check exits 0; spec-spine couple exits 0 |
| AC-401 | 4 | make registry exits 0 |
| AC-402 | 4 | make pr-prep exits 0 |
| AC-403 | 4 | make ci exits 0 |
| AC-404 | 4 | Dead-reference grep passes |
| AC-501 | 5 | spec-spine lint exits 0; zero I-003 for deleted crate IDs |
| AC-502 | 5 | spec-spine couple exits 0 on wave PR diff |
| AC-BLAST | 6 | Authorities diff confirms zero paths lose all live owners |

---

## 5. Authority-Preservation Gate (Blast Radius)

The **before/after `authorities(P)` corpus diff** is the landing gate
for this migration. The corpus diff is computed by running
`spec-spine couple --report-authorities` (or equivalent) against the
pre-deletion and post-deletion states and diffing the output.

**The mission halts** if any path in the OAP corpus unexpectedly loses
all live owners after the wave. A path losing a superseded predecessor
is expected (spec 216 Phase 2b already implemented supersession
filtering). A path losing its only live owner is a defect.

The pre-deletion baseline authority set must be verified for the three
change classes:

1. **Full-supersession deletions** (001, 002, 006, 133, 176, 181):
   every path previously owned by these specs must have a live
   alternative owner after the wave. In practice: the library's own
   claim on its API surfaces replaces the deleted specs' claims, or
   OAP overlay specs (via `extends`/`refines` edges) remain live owners.
2. **002-031 bulk migration**: the `kind: crate, id:
   open_agentic_spec_registry_reader` units are not physical paths; no
   `authorities(P)` path is affected. The migration is a metadata
   operation, not a code-authority operation.
3. **Amend targets** (103, 127, 128, 130, 184): the amended specs
   remain live owners of their edited paths. Amending a spec's spec.md
   does not remove its code authority.

**FR-BLAST.** A before/after `authorities(P)` diff is generated in the
verification phase (Phase 6). The diff shows zero paths where the
post-deletion authority set is empty and the pre-deletion set was
non-empty (excluding paths whose predecessor was already superseded by
spec 216 Phase 2b).

**AC-BLAST.** The authorities diff confirms: (a) zero paths lose all
live owners, (b) paths previously owned solely by a deleted spec have a
live alternative owner declared via an OAP overlay `extends`/`refines`
edge or the library's own claim surface.

---

## 6. Supersession and Amendment Wave

This section is the authoritative record of each affected spec's
disposition. The "honest rationale" column states why the disposition is
correct independent of gate mechanics. CONST-005 applies: every row
must survive the auditor test ("would this disposition be correct even
if the coupling gate did not exist?").

**When these edges land.** This wave is a PLAN. The `supersedes:` edges
on spec 217 and the `superseded_by:` / `status: superseded` flips on the
predecessors below are added by the implementation PR (Phase 5),
atomically with the binary deletion. They are deliberately absent from
this draft's frontmatter: until the in-tree binaries are actually
deleted, 001/002/006/133/176/181 remain approved and authoritative over
their still-present code, and the spine must not assert a supersession
that has not yet taken effect. Likewise the `amends:` edges to
103/127/128/130/184 land when their spec.md files are actually edited in
the implementation PR.

### 6.1 Full supersession by spec 217

| Spec | Title (abbreviated) | Disposition | Honest rationale |
|---|---|---|---|
| 001-spec-compiler-mvp | spec-compiler binary | FULL-SUPERSEDE | In-tree binary deleted; compile capability is now spec-spine-core::compile |
| 002-registry-consumer-mvp | registry-consumer binary | FULL-SUPERSEDE | In-tree binary deleted; authority-resolver verbs move to oap-registry-enrich; list/show/status-report move to spec-spine registry |
| 006-conformance-lint-mvp | spec-lint (generic) | FULL-SUPERSEDE | Generic lint codes move to spec-spine lint; in-tree binary deleted; OAP overlay lint is a separate crate not established by 006 |
| 133-amends-aware-coupling-gate | coupling-gate derivation algorithm | FULL-SUPERSEDE | The in-tree coupling-check binary is deleted; its algorithm is now in spec-spine-core::couple_with (confirmed drop-in); the CI workflow survives under spec 127 |
| 176-amends-aware-section-satisfaction-parity | section-satisfaction bug-fix | FULL-SUPERSEDE | Closed amendment on a deleted binary; the section-aware amends satisfaction it fixed is now in spec-spine-core; no in-tree subject remains |
| 181-registry-consumer-unit-grammar-authority | authority-resolver unit-grammar | FULL-SUPERSEDE | The refined binary (registry-consumer) is deleted; the unit-grammar authority resolver is re-implemented in oap-registry-enrich under its own ownership; spec 181's refines subject no longer exists in-tree |

### 6.2 Amendment (repoint to library/overlay; artifact or workflow survives)

These specs survive because their established artifacts or workflows
are not deleted. Their spec.md is amended to document the binary or
seam change.

| Spec | Title (abbreviated) | Disposition | What survives | Honest rationale |
|---|---|---|---|---|
| 101-codebase-index-mvp | codebase-indexer | AMEND | `.derived/codebase-index/` artifact, `codebase-index.schema.json`, `spec-spine index` subcommand replaces the binary | The artifact and schema survive unchanged; only the producing binary is deleted and replaced by the library indexer. Spec 101 continues to govern the artifact contract. |
| 127-spec-code-coupling-gate | CI workflow + make target | AMEND | `ci-spec-code-coupling.yml`, `make pr-prep`, contributor flow | The CI workflow and Makefile target survive; only the binary invocation changes from `spec-code-coupling-check` to `spec-spine couple`. Spec 127 governs the workflow, not the binary. |
| 130-spec-coupling-primary-owner | relationship graph + spec-types | AMEND | `tools/shared/spec-types/src/lib.rs` (as overlay), `standards/schemas/spec-spine/registry.schema.json` | Spec 130 establishes both files; the spec-types file survives as the OAP overlay crate (its established identity changes from full-types to overlay-types); the schema file is unchanged. |
| 132-constitutional-invariant-freeze | invariant-freeze mechanism | AMEND | `standards/schemas/spec-spine/registry.schema.json` (frozen), V-011 migrates to library | Spec 132's frozen schema invariant survives; V-011 moves to spec-spine-core. Amend 132 to record that V-011 is now a library-validated code. |
| 147-spec-kind-grammar | kind grammar + V-012..V-019 | AMEND | OAP overlay types (16-kind enum, VALID_KINDS, SHAPE_TABLE, W-130/131/132) | The OAP-domain validation codes and the kind enum survive in the overlay; the generic V-012..V-019 codes move to the library. Amend 147 to record which codes moved and which remain in the overlay. |
| 152-path-co-authority | named-anchor sectioning | AMEND | Section-matching logic moves to spec-spine-core | Spec 152's spec.md and the CI behaviour it governs survive; the code it established in the coupling-check binary is superseded by the library's section-matching. Amend 152 to record the re-homing. |
| 154-logical-unit-ownership-grammar | unit grammar + indexer | AMEND | Unit-grammar field definitions in overlay spec-types, `codebase-index.schema.json` | Spec 154's value (the unit grammar and the index schema) survives; only the producing binary is deleted. Amend 154 to record that the indexer source is now spec-spine-core. |
| 155-logical-unit-resolution-semantics | unit resolution precision | AMEND | V-024 and the unit-resolution fixes move to spec-spine-types | The precision fixes (V-024, module/directory/symbol resolution rules) are in the library. Amend 155 to record the re-homing; the spec's design rationale remains valid. |
| 161-knowledge-requirements-provenance-emission | W-161 decomposition-origin lint | AMEND | W-161 survives in OAP overlay lint | The W-161 code is an OAP-domain code (decomposition-origin provenance); it stays in the overlay lint. Amend 161 to record that its in-tree lint surface is now the overlay binary. |
| 179-domain-frontmatter-field | domain field + V-030/V-031 | AMEND | V-030/V-031 survive in OAP overlay; domain field grammar survives in overlay types | The domain field and OAP-domain validation codes are OAP-specific; they stay in the overlay. The `registry-consumer --domain` filter moves to `spec-spine registry list --domain`. Amend 179 to record the filter migration. |

### 6.3 Minor amendment (binary name change only)

| Spec | Title | Disposition | Rationale |
|---|---|---|---|
| 128-spec-lint-default-fail-on-warn | spec-lint strict posture | AMEND (minor) | The strict posture policy survives; only the Makefile invocation changes from `spec-lint` to `spec-spine lint` (generic) + `oap-spec-lint` (OAP overlay). Amend 128 to repoint its Makefile section authority annotation. |

### 6.4 Bulk lifecycle migration: 003-031 registry-consumer contract series

Specs 003-031 (excluding 006, already in the full-supersede table)
extend or refine `kind: crate, id: open_agentic_spec_registry_reader`.
That crate is deleted. These specs are behavioural contracts on a
deleted binary; the contracts are absorbed by the library's own contract
surface (`spec-spine registry` verbs).

**Disposition for each spec in 003-031 (excluding 006):**
- `status: superseded`
- `superseded_by: "217-spec-spine-engine-swap-collapse"`
- Strip the `open_agentic_spec_registry_reader` crate-unit from
  `extends:`/`refines:` edges (these are not live code claims and
  would yield I-003 diagnostics on a non-existent crate).

Exception: spec 029 additionally references a CI-path note about the
registry-consumer binary path. Update that reference to `spec-spine
registry` before marking superseded.

**Why this is honest:** these specs wrote behavioural contracts for a
binary that no longer exists. Their contractual intent (the output
format, the exit codes, the field shapes) is honoured by the library.
The contracts are not violated; their subject has moved. Marking them
superseded is accurate status, not gate-appeasement.

---

## 7. Coupling: How Spec 217's Own Edits Are Coupling-Clean

This spec edits a large surface. Each edit class is accounted for:

**Spec 217 spec.md itself** (this file): no coupling rule applies to
spec.md files (they are not `implements` claimants).

**`spec-spine.toml`** (new file, established by this spec's
`establishes:` edge): no predecessor owner; the edge is authoritative.

**Featuregraph golden** (`crates/featuregraph/tests/golden/features_graph.json`):
governed by the `extends: {spec: "034-featuregraph-registry-scanner-fix", nature: additive}` edge in this spec's frontmatter.

**Makefile sections**: each section is governed by its section's
co-authority owner set. This spec's implementing PR edits those
sections and includes the respective owner specs in the diff (or carries
Spec-Drift-Waiver for any section owner that is superseded by this
spec in the same PR).

**`ci-spec-code-coupling.yml`**: established by spec 127. This spec's
`amends: ["127-spec-code-coupling-gate"]` edge confers co-authority; spec
127's spec.md is in the diff (the amendment record is added to it).

**`.claude/rules/governed-artifact-reads.md`**: owned by spec 103. This
spec's `amends: ["103-init-protocol-governed-reads"]` edge confers
co-authority; spec 103's spec.md is in the diff.

**`tools/shared/spec-types/src/lib.rs`**: established by spec 130. This
spec's `amends: ["130-spec-coupling-primary-owner"]` edge confers
co-authority; spec 130's spec.md is in the diff.

**Deleted source files** (the four generic engine crates): deletion of
a file that is `establishes`-claimed by a superseded spec is coupling-
clean once the establishing spec's frontmatter carries
`superseded_by: "217-spec-spine-engine-swap-collapse"` in the same PR.
The coupling gate excludes superseded specs from `legitimate_owners(P)`
per spec 216 Phase 2b (FR-011). The wave lands in the same PR as the
deletion, so the gate sees the superseded status before evaluating the
deleted paths.

**CLAUDE.md and AGENTS.md**: amended by spec 184-claude-shared-config-governance / spec 103 edges
declared above.

**Spec 217 is the superseder, not an amender, for 001/002/006/133/176/181.**
For each of those specs, the implementation PR carries that spec's
spec.md with `superseded_by: "217-spec-spine-engine-swap-collapse"`.
That is the coupling-clean path for a superseding spec deleting the
established files of a fully-superseded predecessor.

---

## 8. Success Criteria

**SC-001 (zero generic engine).** After the landing PR, no source file
under `tools/spec-spine/` belongs to any of the four deleted crate
packages. The directory `tools/spec-spine/` contains only the OAP
overlay lint crate (if not moved to `tools/oap/`) and any non-crate
artifacts (test fixtures that are now governed by the library).

**SC-002 (gate is the library).** OAP's CI coupling gate is
`spec-spine couple`. The `ci-spec-code-coupling.yml` workflow invokes
no in-tree binary. Exit codes 0/1 are preserved.

**SC-003 (overlay survives).** The OAP overlay (16-kind enum,
shape/category, compliance, factory/OWASP report, capability/registry/
profile, stagecraft:// provenance, W-130/131/132, V-030/031, W-161,
L-005) is intact and tested. No OAP-specific typed modelling is lost.

**SC-004 (authority preservation).** The before/after `authorities(P)`
corpus diff confirms zero paths lose all live owners (FR-BLAST /
AC-BLAST).

**SC-005 (wave is honest).** Every superseded spec has a
`superseded_by:` pointer to 217. Every amended spec has an
`amendment_record:` entry naming 217 and a prose callout documenting
what changed. No spec is superseded or amended without an honest
rationale that is independent of gate mechanics.

**SC-006 (featuregraph golden).** The featuregraph golden test passes
against the library-compiled registry. The golden is regenerated in the
landing commit with the registry compiled first.

**SC-007 (OPC desktop app).** The OPC desktop app builds and reads the
registry via `spec-spine-core::load_registry`. The governance panel
hydrates correctly from the library-produced registry.
