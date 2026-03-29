> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Feature **032** complete. Feature **033** **delivered** (`status: active`). **Feature 034** (`featuregraph registry scanner fix`) is **scaffolded** under `specs/034-featuregraph-registry-scanner-fix/` as **`draft`** — registry-first scanner vs `spec/features.yaml`. **`spec-compiler compile`** run green at handoff. **Next:** **Cursor** implements **034** per `tasks.md`; then **Claude** review.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **032 spec:** `specs/032-opc-inspect-governance-wiring-mvp/spec.md` (status: active, delivered)
- **033 spec:** `specs/033-axiomregent-activation/spec.md` (status: active, delivered)
- **034 spec:** `specs/034-featuregraph-registry-scanner-fix/spec.md` (status: draft, scaffolded)
- **Execution:** per-feature `execution/changeset.md`, `execution/verification.md`

## Current execution truth

- Feature 032: T000–T013 complete. Verification green 2026-03-28.
- Feature 033: T001–T008 complete (sidecar + UI + verification).
- Feature 034: spec/plan/tasks/execution stubs added; implementation **not** started.

## Residual notes (033)

1. **Cross-platform binaries** — Still only `axiomregent-aarch64-apple-darwin` under `src-tauri/binaries/`. Follow-up: `build:executables` / CI matrix when ready.
2. **Golden test** — `crates/axiomregent/tests/mcp_contract.rs` may need `UPDATE_GOLDEN=1` if tools list JSON formatting drifts (orthogonal to sidecar work).

Historical review: `.ai/reviews/claude-review.md` (Feature 033 section).

## Data integrity fixes (this pass)

- `.ai/plans/integration-debt.md` — **restored** (was corrupted: contained concatenation of next-slice + promotion-candidates + current.md)
- `.ai/plans/next-slice.md` — cleaned stale `implemented` references
- `.ai/plans/promotion-candidates.md` — cleaned stale `implemented` reference

## Baton

- Current owner: **cursor**
- Next owner: **claude** (post-034 review) or **claude-opus** (synthesis)
- Last baton update: 2026-03-28 — **Cursor**: ran **`spec-compiler compile`** (pass); **chose 034** over 035 (smaller, unblocks governance degraded state); **scaffolded** `specs/034-featuregraph-registry-scanner-fix/`
- Requested outputs from **Cursor** (Feature **034**):
  1. **T001–T002**: Registry → scanner adapter in `crates/featuregraph`; registry-first path in `scanner.rs` with yaml fallback where still needed.
  2. **T003**: Confirm `featuregraph_overview` / governance uses registry-backed data when `build/spec-registry/registry.json` exists.
  3. **T004**: `cargo test -p featuregraph`, `pnpm -C apps/desktop check`.
  4. **T005–T006**: Fill `execution/verification.md`, `changeset.md`, set spec **`status: active`** when done.
- Deferred: **035** (`specs/035-agent-governed-execution/`) — scaffold after 034 unless reprioritized.
- Recommended files to read:
  - `specs/034-featuregraph-registry-scanner-fix/spec.md`, `plan.md`, `tasks.md`
  - `crates/featuregraph/src/scanner.rs`
  - `build/spec-registry/registry.json` (after `spec-compiler compile`)
  - `apps/desktop/src-tauri/src/commands/analysis.rs`

## Requested next agent output

**Cursor:** implement **034** T001–T006. **Claude:** review when Cursor returns baton.

## Promotion candidates for canonical artifacts

- ~~`spec-compiler compile`~~ — run at 034 handoff (green)
- **034** registry scanner → in progress
- **035** agent execution reroute → after 034

---

## Recent outputs

- 2026-03-28 (cursor): `spec-compiler compile` green; scaffolded **`specs/034-featuregraph-registry-scanner-fix/`**; baton → implement 034
- 2026-03-29 (claude): Verified 033 implementation (all FRs pass); updated findings/authority-map/debt; baton to cursor for 034/035
- 2026-03-29 (cursor): Feature **033** implementation complete (axiomregent spawn, stderr probe port, MCP/governance UI, execution docs); baton to **claude**
- 2026-03-29 (claude): Confirmed handoff coherence + 033 review spot-check (no drift); baton to cursor
- 2026-03-28 (cursor): Removed obsolete vendor references from `.ai/handoff/current.md`; passed baton to **claude** (pre-033 review pass)
- 2026-03-29 (claude): 033 spec review, data integrity fixes (integration-debt.md restored), baton to cursor
- 2026-03-29 (cursor): Repaired handoff (NUL bytes, lifecycle); scaffolded `specs/033-axiomregent-activation/`
- 2026-03-29 (claude-opus): synthesis in `.ai/plans/next-slice.md`
- 2026-03-28 (cursor): T010–T013 implementation + verification
- 2026-03-28 (claude): Reconciled findings with 032 closure, staged post-032 priorities
