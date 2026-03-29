# Claude review (working notes)

> **Non-authoritative.** Deep-dive notes; cite files/lines when asserting behavior.

## Scope reviewed

- Feature / slice: Feature 032 (OPC inspect + governance wiring MVP), T000–T013 — **all complete**
- Code paths: Full Tauri backend (`apps/desktop/src-tauri/src/`), all 10 Rust crates, frontend features (`inspect/`, `git/`, `governance/`), spec system (`specs/`, `tools/`), `.ai/` workspace
- Post-T009 review: T010 "View spec" action (`actions.ts`, `RegistrySpecFollowUp.tsx`), `featureSummaries` backend extension (`analysis.rs:130-166`), vitest coverage, verification suite

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

### Feature 032: COMPLETE
- T010–T013 implemented by Cursor (2026-03-28). "View spec" action uses `featureSummaries` from compiled registry. Backend extended to emit `featureSummaries` (id, title, specPath) in `read_registry_summary`. `RegistrySpecFollowUp` component wired into both `InspectSurface` and `GovernanceSurface`. Vitest coverage added. Verification green.

### For post-032 platform thesis:
- **axiomregent activation** — needs its own spec
- **Agent permission enforcement** — needs axiomregent or alternative mechanism
- **Safety tier spec** — `safety.rs` tiers need to be spec-governed, not code-only
- **Feature ID reconciliation** — needs design decision before the two systems diverge further
- **featuregraph scanner fix** — adapt to use `registry.json` instead of `features.yaml`
- **Titor Tauri commands** — 5 stubs blocking temporal safety

## Recommended next move

**Feature 032 is done. Post-032 priorities (in order):**

1. **Spec: axiomregent activation** — write a feature spec for spawning axiomregent at startup and exposing its governed tool surface. This is the single move that transforms the platform from "Claude wrapper" to "governed execution environment."

2. **Spec: agent routing through axiomregent** — replace `--dangerously-skip-permissions` with governed dispatch. Make `enable_file_read/write/network` flags enforceable via safety tiers.

3. **Fix featuregraph scanner** — adapt `Scanner::scan()` to read from `registry.json` instead of the forbidden `spec/features.yaml`. This would immediately promote the governance panel from degraded to fully functional.

4. **Wire titor Tauri commands** — implement the 5 stubbed checkpoint commands. Enables temporal safety net for agent execution.

5. **Spec: safety tier model** — formalize `safety.rs` tier definitions. Make tier assignments governance-visible and spec-governed.

## Promotion candidates

- [x] `execution/verification.md` — governance backend tests and featuregraph degraded state documented (T013, 2026-03-28)
- [x] `execution/changeset.md` — T010–T013 recorded (2026-03-28)
- [x] `specs/033-axiomregent-activation/` — scaffolded (2026-03-29)
- [ ] Post-033 spec candidates — safety tier model, feature ID reconciliation, featuregraph scanner fix (Feature 034-class)

---

## Feature 033 review (2026-03-29)

### Spec assessment: `specs/033-axiomregent-activation/spec.md`

**Verdict: Spec is sound. Two issues to address before implementation.**

#### What's right

- **Scope is correctly bounded.** In-scope: spawn, port discovery, MCP UI visibility, safety tier display. Out-of-scope: agent rerouting, permission enforcement, scanner fix, titor. This is the right sequencing.
- **FR-001 through FR-004 are clear and testable.** Each has a verifiable condition.
- **Degraded state handling required (FR-002)** — consistent with 032's approach of explicit degradation rather than crashes.
- **Contract note about `tauri.conf.json` authority** is correct — `externalBin` already lists `binaries/axiomregent` (verified at `tauri.conf.json:61-63`).

#### Issue 1: axiomregent binary only exists for aarch64-apple-darwin

**Evidence:** `apps/desktop/src-tauri/binaries/` contains only `axiomregent-aarch64-apple-darwin`. No Windows (`x86_64-pc-windows-msvc`) or Linux binaries present.

**Impact:** FR-001 says "on supported builds" — which currently means only Apple Silicon macOS. On Windows/Linux, `spawn_axiomregent` will fail at `app.shell().sidecar("axiomregent")` because the binary doesn't exist.

**Recommendation:** T003 (packaging verification) should explicitly:
1. Document which targets have bundled binaries
2. For missing targets, either cross-compile axiomregent or ensure FR-002 degraded state works cleanly
3. Consider adding a `build:executables` step for axiomregent (similar to gitctx-mcp's `fetch-and-build.js` pattern at `apps/desktop/package.json`)

#### Issue 2: `spawn_axiomregent` uses `app.shell().sidecar()` which requires Tauri shell plugin

**Evidence:** `sidecars.rs:50` calls `app.shell().sidecar("axiomregent")`. The shell plugin is registered in `lib.rs` plugin chain. This should work, but the sidecar name must match the `externalBin` entry exactly (minus architecture suffix).

**Verification needed:** Confirm that Tauri 2's sidecar resolution correctly maps `"axiomregent"` to `binaries/axiomregent-{arch}` with the current `tauri.conf.json` externalBin config. This is a known Tauri 2 pattern but should be smoke-tested per T002.

#### Tasks assessment

- **T001–T002 (startup + smoke):** Correct sequencing. T001 should add `spawn_axiomregent(app)` after `SidecarState` management at `lib.rs:189`. T002 should verify port appears in `SidecarState`.
- **T003–T004 (packaging + verification):** Need to account for binary availability per platform. The gitctx-mcp approach (per-architecture bundled binary) is the pattern to follow.
- **T005–T006 (UI + safety tiers):** Well-scoped. T005 can use `get_sidecar_ports` which is already a Tauri command. T006 can read `safety.rs` tier definitions — but note these are only meaningful once axiomregent is actually dispatching tool calls.
- **T007–T008 (closure):** Standard.

### Feature 032 lifecycle status

**Confirmed: `status: active` is correct.** The registry enum (Feature 000/003) allows only `draft|active|superseded|retired`. There is no `implemented` value. Feature 032 remains `active` — it is current platform truth. Delivery is proven by `tasks.md` (all checked) + `execution/verification.md` (green run 2026-03-28).

### Recommendation

Feature 033 spec is ready for implementation with two additions:
1. Add a note to `spec.md` or `plan.md` acknowledging binary availability constraint (only macOS arm64 currently)
2. T003 should include cross-compilation or graceful degradation as explicit deliverables

