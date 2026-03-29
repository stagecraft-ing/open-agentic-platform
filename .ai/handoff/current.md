> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Feature **032** is complete (`status: active`, delivery proven by tasks + verification). Feature **033** (`axiomregent activation`) is scaffolded under `specs/033-axiomregent-activation/` as `draft`. Claude has reviewed the 033 spec and identified two implementation-relevant issues. **Next:** Cursor implements 033 per `tasks.md`.

## Agent pack change (2026-03-29)

**ChatGPT removed from the agent pack.** Synthesis/prioritization role reassigned to **Claude Opus** (same model family as deep-analysis agent). Reason: ChatGPT introduced an invalid `status: implemented` value that doesn't exist in the registry enum (Feature 000/003), requiring Cursor repair. Using the same model family for both analysis and synthesis avoids cross-model translation errors. Updated files: `.ai/handoff/chatgpt.md` (now Claude Opus role card), `.ai/prompts/chatgpt-slice-synthesis.md` (now Claude Opus prompt).

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
- `.ai/handoff/chatgpt.md` — **replaced** with Claude Opus synthesis role card (was corrupted concatenation)
- `.ai/prompts/chatgpt-slice-synthesis.md` — **updated** to target Claude Opus
- `.ai/plans/next-slice.md` — cleaned stale `implemented` references
- `.ai/plans/promotion-candidates.md` — cleaned stale `implemented` reference

## Baton

- Current owner: **cursor**
- Next owner: **claude** (for post-implementation review) or **claude-opus** (for synthesis if needed)
- Last baton update: 2026-03-29 — Claude reviewed 033 spec, fixed corrupted files, replaced ChatGPT with Claude Opus; baton to Cursor for 033 implementation
- Requested outputs from Cursor:
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

Cursor: implement Feature 033 T001–T008 per spec and tasks.md. Key constraint: axiomregent binary currently only exists for macOS arm64 — handle other platforms as graceful degradation for now.

## Promotion candidates for canonical artifacts

- After 033 lands: run `spec-compiler compile` to validate frontmatter
- Scanner / `registry.json` work → Feature 034-class
- Agent execution reroute → Feature 035-class (requires 033 complete)

---

## Recent outputs

- 2026-03-29 (claude): 033 spec review, data integrity fixes (integration-debt.md restored, chatgpt→claude-opus), baton to cursor
- 2026-03-29 (cursor): Repaired handoff (NUL bytes, lifecycle); scaffolded `specs/033-axiomregent-activation/`
- 2026-03-29 (chatgpt — removed): synthesis in `.ai/plans/next-slice.md`
- 2026-03-28 (cursor): T010–T013 implementation + verification
- 2026-03-28 (claude): Reconciled findings with 032 closure, staged post-032 priorities
