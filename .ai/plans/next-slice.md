# Next slice (working synthesis)

> **Non-authoritative.** This is a **staging** view for the smallest next increment — not a replacement for `specs/.../tasks.md` or `plan.md`. Promote agreed work into canonical tasks.

## Context

- Branch: `main`
- Active spec folder (canonical): `specs/032-opc-inspect-governance-wiring-mvp/`
- Remaining tasks: T010, T011, T012, T013

## Smallest high-leverage slice (proposal)

### T010: "View spec" action from inspect results

1. Create `apps/desktop/src/features/inspect/actions.ts`:
   - Export `openSpecAction(specPath: string)` — opens spec file in a `claude-md` tab via the existing tab system
   - Export `getAvailableActions(registryData)` — returns action descriptors when registry features have `specPath`

2. Update `apps/desktop/src/features/inspect/InspectSurface.tsx`:
   - When governance panel returns registry features, render "View spec" button per feature (or a single "View spec" for the repo's primary spec)
   - Button calls `openSpecAction` which creates a new `claude-md` tab pointing at the spec file

3. Add `apps/desktop/src/features/inspect/__tests__/inspect-actions.test.tsx`:
   - Fixture: registry response with `specPath` → action button renders
   - Fixture: registry response without `specPath` → no action button
   - Fixture: registry unavailable → no action section

### T011: Minimal docs

4. Update `apps/desktop/README.md` — add section on inspect/governance flow
5. Update `specs/032-opc-inspect-governance-wiring-mvp/execution/changeset.md` — record T010–T011 completion

### T012: Targeted tests

6. Add or verify tests for:
   - Inspect surface state machine (success/loading/error/degraded)
   - Git panel data rendering
   - Governance panel registry summary + degraded featuregraph handling
   - Action button presence/absence based on registry data

### T013: Full verification

7. Run verification suite and record in `execution/verification.md`:
   ```bash
   pnpm -C apps/desktop build
   cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
   cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml commands::analysis::tests::
   cargo test --manifest-path tools/registry-consumer/Cargo.toml --all --quiet
   cargo build --release --manifest-path tools/spec-compiler/Cargo.toml && ./tools/spec-compiler/target/release/spec-compiler compile
   ```

## Why now

- T010–T013 are the final tasks for Feature 032
- T010 "View spec" is the safest action choice: zero backend work, no dependency on broken `features.yaml`, uses existing tab infrastructure, exercises real compiled registry data
- Completing 032 unblocks post-032 convergence work (axiomregent activation, titor wiring, scanner fix)

## Dependencies / risks

- T010 depends on registry data having `specPath` field — verified present in `registry.schema.json` and emitted by spec-compiler
- T010 does NOT depend on featuregraph (avoids `features.yaml` gap)
- T012 test infrastructure may need vitest/jest setup if not already present in desktop app

## After promotion (canonical)

- [ ] Update `tasks.md` checkboxes for T010–T013 when done
- [ ] Update `execution/changeset.md` with final PR reference
- [ ] Update `execution/verification.md` with T013 results
- [ ] Close Feature 032 (status: active → implemented per Feature 003 lifecycle)
