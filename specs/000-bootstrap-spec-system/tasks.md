# Tasks: Bootstrap spec system

**Input**: Design documents from `/specs/000-bootstrap-spec-system/`  
**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/registry.schema.json](./contracts/registry.schema.json)

**Tests**: Golden-output and schema-validation tests are **required** for this feature’s compiler scaffold.

**Organization**: Phases follow bootstrap dependencies; no product user stories beyond the spec’s compiler-centric stories.

## Format: `[ID] [P?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)

---

## Phase 1: Setup

**Purpose**: Reserve repository layout for compiler output and tooling.

- [ ] T001 Create `tools/spec-compiler/` skeleton with README pointing to Feature 000 spec
- [ ] T002 Add `build/spec-registry/` to `.gitignore` (compiler output) and document optional committed golden fixtures in `research.md` open items
- [x] T003 [P] Add a minimal `CONTRIBUTING.md` section or pointer in root `README.md` stating markdown-only / JSON-only rules (one paragraph, links to `specs/000-bootstrap-spec-system/spec.md`) — **done**: root `README.md` links to Feature 000 and `.specify/contract.md`

---

## Phase 2: Foundational (compiler core)

**Purpose**: Implement MVP spec compiler per [plan.md](./plan.md)

- [ ] T004 Implement markdown discovery for `specs/*/spec.md` under repository root
- [ ] T005 Parse YAML frontmatter + extract `sectionHeadings` per [data-model.md](./data-model.md)
- [ ] T006 Emit `registry.json` with **sorted JSON keys** and sorted violation lists; write to `build/spec-registry/registry.json`
- [ ] T007 Compute `build.contentHash` per [research.md](./research.md) decision D2
- [ ] T008 Implement validation codes **V-001**–**V-003** minimum; stub **V-004**–**V-005** with explicit `NOT IMPLEMENTED` violations until policy paths are wired

---

## Phase 3: User Story 1 — Clean compile (P1)

**Goal**: Valid specs produce `validation.passed: true` for a fixture tree.

**Independent Test**: Golden test compares emitted JSON to expected file for a tiny `specs/` fixture.

- [ ] T009 [US1] Add `tools/spec-compiler/tests/fixtures/minimal/` with one valid feature spec
- [ ] T010 [US1] Golden test: two runs produce byte-identical `registry.json`
- [ ] T011 [US1] Schema validation test using [contracts/registry.schema.json](./contracts/registry.schema.json)

---

## Phase 4: User Story 2 — Forbidden YAML policy (P2)

**Goal**: **V-004** fails CI when a standalone `.yaml` appears in authored paths.

- [ ] T012 [US2] Implement path policy scanner for `V-004` per [research.md](./research.md) D4
- [ ] T013 [US2] Add negative fixture proving failure path

---

## Phase 5: User Story 3 — Consumer stability (P3)

**Goal**: Downstream tools can rely on schema only.

- [ ] T014 [US3] Publish stable `$id` for [contracts/registry.schema.json](./contracts/registry.schema.json) (update if repo URL known) and document in [quickstart.md](./quickstart.md)

---

## Phase 6: Polish

- [ ] T015 [P] Wire **V-005** heuristic or document deferral with explicit violation code if postponed
- [ ] T016 Run `/speckit.checklist` parity manually: ensure [checklists/requirements.md](./checklists/requirements.md) complete

---

## Dependencies & Execution Order

- T004–T008 before T009–T011
- T012–T013 can parallelize with T009–T011 only after T008 emits stable shapes
- T014 depends on stable schema path and emitted JSON

---

## Notes

- Future features (**featuregraph**, **xray**, **axiomregent**) MUST NOT be started until this phase’s **T011** passes for the committed fixture set.
