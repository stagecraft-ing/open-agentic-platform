# 165 — Implementation plan

> Status: in progress (deterministic backbone landing).
> Owner: bart
> Branch: `165-opc-decomposition-pipeline`

## Scope of this landing

Initial PR delivers the **deterministic backbone** of the pipeline:

- Stages 1-5 wired to existing substrates (`artifact-extract`, `xray`).
- Stage 6 implemented as a **deterministic baseline synthesiser** that
  emits a draft `spec.md` satisfying spec 161 emission contract, spec 147
  kind grammar, and spec 154 logical-unit grammar.
- Run persistence under `<project>/.opc/decomposition/<run-id>/`.
- Tauri command surface in `product/apps/opc/src-tauri` so the
  pipeline is invocable from OPC.
- Fixture-based integration tests proving SC-001 / SC-002 / SC-003 / SC-004.

## Explicitly out of this PR (follow-up specs/PRs)

- **Real LLM stage-6 synthesis.** Spec §5 defers stage-6 model selection
  to platform-level config; this PR ships a deterministic baseline. A
  future PR swaps the `Synthesiser` trait impl for an LLM-backed one.
- **Promotion + governance certificate flow.** FR-008 / FR-009 / SC-005 /
  SC-006 require writing into `<project>/specs/`, re-running the
  project's spec-compiler + coupling gate, and emitting a spec-102
  certificate at promotion. Tracked separately.
- **React UI panel.** FR-001 surfaces a "Decompose project" action; this
  PR exposes the Tauri command but does not render a polished panel in
  the desktop app. The Requirements view (spec 163) is itself draft.

## Substrate decisions

### Stage 3 substrate: xray, not axiomregent

The spec §2.1 names `crates/axiomregent` (via its `search` module) as the
stage-3 substrate. In practice, axiomregent's semantic search and xray's
`analysis-embeddings` feature both wrap `fastembed`; the vector-matching
primitive is identical. axiomregent layers persistent indexing via
hiqlite + reqwest + octocrab on top, which this pipeline does not need
and which would inflate the crate's dependency tree by ~25 transitive
deps.

This PR uses `xray/analysis-embeddings` for stage 3. FR-010 explicitly
defines the xray-only fallback as the degraded path; the deterministic
backbone treats it as the primary path. A future spec can introduce an
axiomregent-backed persistent-index implementation behind the same
`Clustering` trait if cross-run cache-reuse becomes load-bearing.

This is an engineering choice, not a spec amendment: the spec's intent
("semantic clustering via vector matching") is preserved.

### Stage 6 synthesiser: deterministic baseline

The trait `Synthesiser` takes `EvidenceBundle` (the merged outputs of
stages 1-5) and returns `Vec<DraftSpec>`. The baseline implementation
walks xray's clusters + call-graph entry points and emits one
`spec.md` per significant cluster, populating:

- `status: draft`
- `origin: retroactive: true`
- `kind: capability` (spec 147)
- `category:` from the cluster's dominant top-level directory
- `establishes:` with logical-unit declarations (spec 154) for each path
- `references: [{ role: decomposition-origin, unit: ..., provenance: ... }]`
  per spec 161

The synthesis is mechanical: enough to pass spec-lint, satisfy the
emission contract, and round-trip through the spec-compiler. The LLM
swap improves the prose quality of the summary, not the contract.

## Phases

1. **Scaffold** — crate at `crates/opc-decomposition-pipeline/`, wired
   into the workspace, types and traits defined, stages stubbed.
2. **Stage 1: extraction** — call `artifact_extract::extract_deterministic`
   per knowledge object; emit JSONL `ExtractionOutput` under
   `s1-extraction/`. Degraded: empty output when bundle missing.
3. **Stage 2: structural fingerprint** — call `xray::scan_target` then
   `xray::fingerprint::generate_fingerprint`; persist the full
   `XrayIndex` and the `Fingerprint` under `s2-fingerprint/`.
4. **Stage 3: semantic clustering** — call into `xray::analysis::embeddings`
   to embed code blocks; group blocks whose cosine similarity > 0.85
   into clusters; emit cluster summaries under `s3-clusters/`. Degraded:
   skip embeddings (no feature, or load failure) → cluster by top-level
   directory only.
5. **Stage 4: call graph** — call `xray::analysis::call_graph::analyze_directory`;
   persist graph + summary under `s4-callgraph/`.
6. **Stage 5: temporal lineage** — invoke `git log --follow` per logical
   unit; persist `(unit, first_commit, last_commit, churn)` tuples under
   `s5-lineage/`. Degraded: `unknown` lineage when no `.git`.
7. **Stage 6: synthesiser** — deterministic baseline emits `<staging>/specs/*/spec.md`.
8. **Orchestrator + persistence** — `PipelineRunner::run(config)` walks
   stages 1→6, content-addresses each output, caches re-runs.
9. **Tauri command surface** — `decomposition_run`, `decomposition_list_runs`,
   `decomposition_get_run` in `src-tauri/src/commands/`.
10. **Integration tests + fixture project** — `tests/fixture_min_repo.rs`
    asserts emitted draft passes spec-lint and carries
    `role: decomposition-origin`.
11. **Spec status + index refresh + PR** — flip `implementation:
    pending → in-progress`, declare `establishes:` with logical units,
    regenerate `.derived/codebase-index/`, open PR.
12. **CI watch** — iterate on red checks until green.

## Risks

- **xray embeddings feature pulls fastembed** — heavy build. Mitigated:
  feature-gate stage 3's embedding path. Default build skips it.
- **Tauri's separate workspace** — `product/apps/opc/src-tauri` is
  excluded from the root workspace and cannot depend on the new crate
  via path without adding it as a path dep in its own Cargo.toml. The
  Tauri commands call into the new crate as a normal Cargo path
  dependency.
- **CONST-005** — stage 3 substrate substitution (xray vs axiomregent)
  is documented above as an engineering choice that preserves spec
  intent. Not a spec edit; not gate-driven.
