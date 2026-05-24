# 165 — Tasks

## In this PR (deterministic backbone)

- [x] T-001 — Create branch `165-opc-decomposition-pipeline`.
- [x] T-002 — Author `plan.md` + this tasks file.
- [ ] T-003 — Scaffold `crates/opc-decomposition-pipeline/`: Cargo.toml,
  src/lib.rs, src/types.rs, src/persistence.rs, src/error.rs, src/stages/
  module with stage stubs, src/pipeline.rs orchestrator stub. Register
  crate in root `Cargo.toml`.
- [ ] T-004 — Stage 1: wire `artifact_extract::extract_deterministic` for
  each file in the bundle directory; emit `ExtractionRecord` JSONL under
  `<run>/s1-extraction/`. Handle `ExtractError::RequiresAgent` by
  recording a `requires-agent` placeholder rather than failing the run.
- [ ] T-005 — Stage 2: run `xray::scan_target(project_root, None)`, then
  `xray::fingerprint::generate_fingerprint(&index)`. Persist
  `index.json` + `fingerprint.json` under `<run>/s2-fingerprint/`.
- [ ] T-006 — Stage 3: cluster source files. Default impl groups by
  top-level directory; `embeddings` feature flag adds fastembed-backed
  clustering via xray's `analysis-embeddings`. Emit cluster summaries
  under `<run>/s3-clusters/clusters.json`.
- [ ] T-007 — Stage 4: invoke `xray::analysis::call_graph::analyze_directory`,
  persist graph + summary under `<run>/s4-callgraph/`.
- [ ] T-008 — Stage 5: temporal lineage. Shell out to git via
  `std::process::Command`; per logical unit emit
  `{ unit, first_commit, last_commit, churn }`. Degraded: no `.git` →
  `unknown` lineage. Persist under `<run>/s5-lineage/lineage.jsonl`.
- [ ] T-009 — Stage 6: deterministic synthesiser. For each cluster
  produced by stage 3, emit one `spec.md` under
  `<run>/s6-synthesis/specs/NNN-slug/spec.md`. Spec satisfies emission
  contract (status: draft, origin retroactive, kind, references with
  decomposition-origin role and provenance pointing at the originating
  cluster + fingerprint).
- [ ] T-010 — Orchestrator: `PipelineRunner::run(PipelineConfig)`. Walks
  stages, persists outputs, computes per-stage content hash, writes
  `<run>/run.json` manifest.
- [ ] T-011 — Tauri commands: `decomposition_run(project_path) ->
  RunSummary`, `decomposition_list_runs(project_path) ->
  Vec<RunSummary>`, `decomposition_get_run(project_path, run_id) ->
  RunDetail`. Registered in `src-tauri/src/lib.rs`.
- [ ] T-012 — Integration tests: `tests/fixture_min_repo.rs` runs the
  pipeline against a copied fixture project; asserts:
  - ≥ 1 draft spec emitted (SC-001),
  - Emitted spec passes `spec-lint` at warn-or-better (SC-002),
  - Emitted spec carries `role: decomposition-origin` (SC-003),
  - Second run with unchanged tree completes ≤ 30s and reuses cached
    stage outputs (SC-004).
- [ ] T-013 — Spec 165 frontmatter: flip `implementation: pending →
  in-progress`; declare `establishes:` with logical-unit grammar
  pointing at `crates/opc-decomposition-pipeline/` and the new Tauri
  command files.
- [ ] T-014 — Regenerate codebase index (`make pr-prep`).
- [ ] T-015 — Open PR, watch CI, fix red checks.

## Follow-ups (separate PRs)

- F-001 — Real LLM-backed `Synthesiser` impl behind the same trait.
  Selects model via platform config (spec 165 §5 defers this).
- F-002 — Promotion flow: write staged specs into `<project>/specs/`;
  invoke project's spec-compiler; run coupling gate as a sanity check
  (FR-008, SC-005).
- F-003 — Governance certificate emission at promotion (spec 102,
  FR-009, SC-006).
- F-004 — React panel in `product/apps/opc`: "Decompose project"
  action button, staging-area browser, promotion UI.
- F-005 — Checkpoint integration (spec 095): run the pipeline as a
  branch-of-thought, allow multiple synthesis trajectories.
- F-006 — Persistent embedding cache for stage 3 (the originally-named
  axiomregent substrate). Cross-run reuse.
