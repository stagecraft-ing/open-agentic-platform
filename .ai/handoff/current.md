> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Feature **032** is complete (`status: active`, delivery proven by tasks + verification). Feature **033** (`axiomregent activation`) is scaffolded under `specs/033-axiomregent-activation/` as `draft`. Claude has reviewed the 033 spec and identified two implementation-relevant issues. Handoff docs were cleaned (agent pack, no obsolete vendor references). **Next:** Claude quick review pass, then **Cursor** implements 033 per `tasks.md`.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **032 spec:** `specs/032-opc-inspect-governance-wiring-mvp/spec.md` (status: active, delivered)
- **033 spec:** `specs/033-axiomregent-activation/spec.md` (status: draft)
- **Execution:** per-feature `execution/changeset.md`, `execution/verification.md`

## Current execution truth

- Feature 032: T000–T013 complete. Verification green 2026-03-28.
- Feature 033: Spec/plan/tasks scaffolded. Claude review complete — **two issues identified** (see below).

## Claude review findings for 033

1. **axiomregent binary only exists for `aarch64-apple-darwin`** — `apps/desktop/src-tauri/binaries/` has no Windows or Linux binaries. T003 must explicitly handle this: either cross-compile or ensure graceful degradation per FR-002.

2. **Sidecar name resolution needs smoke test** — `sidecars.rs:50` uses `app.shell().sidecar("axiomregent")` which must map to `binaries/axiomregent-{arch}` via `tauri.conf.json` externalBin. Standard Tauri 2 pattern but untested.

Full review in `.ai/reviews/claude-review.md` (Feature 033 review section).

## Data integrity fixes (this pass)

- `.ai/plans/integration-debt.md` — **restored** (was corrupted: contained concatenation of next-slice + promotion-candidates + current.md)
- `.ai/plans/next-slice.md` — cleaned stale `implemented` references
- `.ai/plans/promotion-candidates.md` — cleaned stale `implemented` reference

## Baton

- Current owner: **cursor**
- Next owner: **claude** (post-implementation review)
- Last baton update: 2026-03-29 — Claude confirmed handoff coherence + 033 review still matches code (SidecarState at lib.rs:189, externalBin at tauri.conf.json:61, only aarch64-apple-darwin binary). Baton to cursor for 033 implementation.
- Requested outputs from Claude:
  1. Skim `.ai/handoff/current.md` and confirm **Agent pack** + baton wording are coherent.
  2. Confirm `.ai/reviews/claude-review.md` (Feature 033 section) still matches `sidecars.rs`, `lib.rs`, `tauri.conf.json`, and `apps/desktop/src-tauri/binaries/` (patch review if drifted).
  3. Return baton to **cursor**: set **current owner** → **cursor**, **next owner** → **claude**, keep the **Requested outputs from Cursor** list below unchanged unless a spec blocker appears.
- Requested outputs from Cursor (when baton is back with **cursor**):
  1. **T001**: Add `spawn_axiomregent(app)` call in `lib.rs` after `SidecarState` management (~line 189). Handle spawn failure gracefully (FR-002).
  2. **T002**: Smoke test — verify port announcement works on current platform (macOS arm64).
  3. **T003**: Document binary availability per platform. For Windows: either cross-compile axiomregent or document degraded state. Consider following `gitctx-mcp` bundling pattern (`scripts/fetch-and-build.js`).
  4. **T004–T008**: Per `specs/033-axiomregent-activation/tasks.md`.
  5. Update `tasks.md` checkboxes as each task completes.
- Recommended files to read:
  - `specs/033-axiomregent-activation/spec.md`, `plan.md`, `tasks.md`
  - `.ai/reviews/claude-review.md` (Feature 033 review section — binary availability issue)
  - `apps/desktop/src-tauri/src/sidecars.rs` (spawn implementation)
  - `apps/desktop/src-tauri/src/lib.rs:188-189` (where to add spawn call)
  - `apps/desktop/src-tauri/tauri.conf.json:61-63` (externalBin config)

## Requested next agent output

Claude: handoff consistency + 033 review spot-check; then pass baton to **cursor** for Feature 033 T001–T008. Key constraint for implementation: axiomregent binary currently only exists for macOS arm64 — handle other platforms as graceful degradation for now.

## Promotion candidates for canonical artifacts

- After 033 lands: run `spec-compiler compile` to validate frontmatter
- Scanner / `registry.json` work → Feature 034-class
- Agent execution reroute → Feature 035-class (requires 033 complete)

---

## Recent outputs

- 2026-03-29 (claude): Confirmed handoff coherence + 033 review spot-check (no drift); baton to cursor
- 2026-03-28 (cursor): Removed obsolete vendor references from `.ai/handoff/current.md`; passed baton to **claude** (pre-033 review pass)
- 2026-03-29 (claude): 033 spec review, data integrity fixes (integration-debt.md restored), baton to cursor
- 2026-03-29 (cursor): Repaired handoff (NUL bytes, lifecycle); scaffolded `specs/033-axiomregent-activation/`
- 2026-03-29 (claude-opus): synthesis in `.ai/plans/next-slice.md`
- 2026-03-28 (cursor): T010–T013 implementation + verification
- 2026-03-28 (claude): Reconciled findings with 032 closure, staged post-032 priorities
