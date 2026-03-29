# Claude review (working notes)

> **Non-authoritative.** Deep-dive notes; cite files/lines when asserting behavior.

## Scope reviewed

- Feature / slice: Feature 032 (OPC inspect + governance wiring MVP), T000–T013
- Code paths: Full Tauri backend (`apps/desktop/src-tauri/src/`), all 10 Rust crates, frontend features (`inspect/`, `git/`, `governance/`), spec system (`specs/`, `tools/`), `.ai/` workspace

## Main concerns

### 1. Governance is display-only (CRITICAL — post-032)

Every Claude/agent execution uses `--dangerously-skip-permissions` (`claude.rs:969`, `agents.rs:774`, `web_server.rs:494,607,695`). Agent permission flags (`enable_file_read/write/network`) in SQLite are stored and shown in UI but never translated into execution constraints. The governed execution thesis is not on the runtime path.

### 2. featuregraph scanner has a structural dependency on a nonexistent forbidden artifact

`Scanner::scan()` at `scanner.rs:167` reads `spec/features.yaml`. This file doesn't exist and is forbidden by Feature 000 (V-004). The governance panel gracefully handles this (`analysis.rs:55-58`), but the featuregraph half can never succeed on this repo.

### 3. axiomregent is the platform's most valuable integration and it's dead code

`spawn_axiomregent()` at `sidecars.rs:48` is fully implemented. `SidecarState` is managed at `lib.rs:189`. Port discovery works. The binary compiles. It is never called. This one missing function call represents the gap between "Claude wrapper" and "governed execution environment."

### 4. Dual feature identity systems with no bridge

Spec IDs (kebab: `032-opc-inspect-governance-wiring-mvp`) and code attribution IDs (UPPERCASE: `FEATUREGRAPH_REGISTRY`) coexist in the same governance response but cannot be cross-referenced.

## What appears resolved

- **Git authority is clean.** FR-002 is correctly implemented: native git is primary, gitctx MCP is additive. Well-separated in code (`useGitContext` vs `useGitCtxEnrichment`).
- **Registry authority is clean.** `read_registry_summary()` reads the deterministic, CI-gated `registry.json`. Contract-tested via Feature 029.
- **Inspect → enrich → display governance loop works.** T000–T009 delivered a real end-to-end inspect flow: xray scan, git context, governance status (degraded but explicit).
- **`.ai/` workspace is well-designed.** Non-authoritative, promotion-gated, baton-based. Follows the platform's own governance philosophy.
- **Degraded state handling is honest.** PR-6's governance implementation returns `{status: "degraded"}` with per-source reasons rather than hiding failures. This is good design.

## What still blocks convergence

### For 032 completion (T010–T013):
- **T010 needs a decision on action type.** Recommendation: "Open spec file" button using registry `specPath` field. Zero backend work, no `features.yaml` dependency.
- **T012/T013 need verification commands.** Listed in `open-questions.md` Q3.

### For post-032 platform thesis:
- **axiomregent activation** — needs its own spec
- **Agent permission enforcement** — needs axiomregent or alternative mechanism
- **Safety tier spec** — `safety.rs` tiers need to be spec-governed, not code-only
- **Feature ID reconciliation** — needs design decision before the two systems diverge further
- **featuregraph scanner fix** — adapt to use `registry.json` instead of `features.yaml`
- **Titor Tauri commands** — 5 stubs blocking temporal safety

## Recommended next move

**For Cursor (T010–T013 implementation):**

1. **T010:** Add "View spec" button to `InspectSurface.tsx` when registry data includes features with `specPath`. Create `apps/desktop/src/features/inspect/actions.ts` with `openSpecAction(specPath)` that opens a `claude-md` tab. Add fixture-backed test.

2. **T011:** Update `apps/desktop/README.md` with inspect/governance user flow. Update `specs/032-opc-inspect-governance-wiring-mvp/execution/changeset.md` with final touchpoints.

3. **T012:** Add tests for:
   - Inspect surface renders xray data
   - Git panel renders branch/status data
   - Governance panel renders registry summary + handles degraded featuregraph
   - Action button renders when specPath available

4. **T013:** Run full verification suite (see `open-questions.md` Q3 for commands), record results in `verification.md`.

## Promotion candidates

- [ ] `execution/verification.md` — add governance backend test command (`cargo test ... commands::analysis::tests::`) and expected featuregraph degraded state documentation
- [ ] `execution/changeset.md` — update with T010–T013 status when implemented
- [ ] Post-032 spec candidates — axiomregent activation, safety tier model, feature ID reconciliation (track as planned next features, not 032 scope)
