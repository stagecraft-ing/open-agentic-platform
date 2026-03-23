# Tasks: Spec compiler MVP

**Input**: `/specs/001-spec-compiler-mvp/` (spec, plan, research)  
**Prerequisites**: Feature **000** schemas at `specs/000-bootstrap-spec-system/contracts/`

## Phase 1: Crate skeleton

- [ ] T001 Create `tools/spec-compiler/` with `Cargo.toml`, `src/main.rs`, `src/lib.rs`, workspace-friendly README linking Feature 001 spec
- [ ] T002 Add `clap` subcommand `compile`; stub output writing empty valid `registry.json` shell (manual schema tweak until parsers exist)
- [ ] T003 [P] Add `rust-toolchain.toml` or `rust-version` in `Cargo.toml`

## Phase 2: Read & parse

- [ ] T004 Implement `specs/*/spec.md` discovery (sorted directory walk)
- [ ] T005 Parse YAML frontmatter + markdown; extract H1/H2 headings per [data-model.md](./data-model.md)
- [ ] T006 Map frontmatter → `FeatureRecord` + `extraFrontmatter` (normalized keys only in top-level fields; remainder with schema-allowed value shapes)

## Phase 3: Validation

- [ ] T007 **V-001** / **V-002** / **V-003** implementation
- [ ] T008 **V-004** repo walk excluding standard vendor dirs per [research.md](./research.md) R6
- [ ] T009 Wire `validation` object and non-zero exit on failure per [research.md](./research.md) R4

## Phase 4: Emit & hash

- [ ] T010 Deterministic JSON writer (BTreeMap / canonical sort) for `registry.json`
- [ ] T011 `contentHash` per Feature 000 research D2
- [ ] T012 Emit `build-meta.json` with UTC `builtAt`

## Phase 5: Tests & docs

- [ ] T013 [P] Golden integration test: two runs, identical `registry.json` bytes on fixture
- [ ] T014 [P] Schema validation test (invoke `ajv` in CI script or Rust jsonschema crate—pick one in implementation)
- [ ] T015 Expand root [README.md](../../README.md) with real `cargo build` / run commands once `tools/spec-compiler/` exists (pointer to Feature 001 already added)

## Dependencies

- T004–T006 before T010–T012
- T007–T009 can interleave with emit once `ParsedSpec` exists
- T013 depends on T010–T012
