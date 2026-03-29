# Tasks: axiomregent activation

**Input**: `/specs/033-axiomregent-activation/`  
**Prerequisites**: Features **000–004**, **032** (desktop shell)

## Phase 1: Startup

- [ ] T001 Invoke `spawn_axiomregent` from Tauri `setup` / `run` path with explicit error handling
- [ ] T002 Unit or integration smoke: sidecar announces port (dev environment)

## Phase 2: Packaging / verification

- [ ] T003 Document and verify `externalBin` / bundle layout per target OS in `execution/verification.md`
- [ ] T004 Record `cargo check` / desktop build for touched crates in verification

## Phase 3: UI

- [ ] T005 Expose axiomregent availability (and port or “degraded”) in MCP or governance-adjacent UI
- [ ] T006 Read-only safety tier / governance signal from existing Rust helpers where feasible

## Phase 4: Closure

- [ ] T007 Update `execution/changeset.md` with files and PR references
- [ ] T008 Final verification run per `execution/verification.md` checklist
