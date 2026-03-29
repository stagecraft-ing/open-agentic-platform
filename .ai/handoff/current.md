> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Features **032–034** **delivered** (`status: active` where applicable). **034** implemented: featuregraph **`Scanner`** prefers **`build/spec-registry/registry.json`**, falls back to **`spec/features.yaml`**, explicit error if neither; governance overview documented. **Next:** **Claude** post-034 review; then **claude-opus** (priorities) or **cursor** (**035** agent routing / next slice).

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **032 spec:** `specs/032-opc-inspect-governance-wiring-mvp/spec.md` (status: active, delivered)
- **033 spec:** `specs/033-axiomregent-activation/spec.md` (status: active, delivered)
- **034 spec:** `specs/034-featuregraph-registry-scanner-fix/spec.md` (status: active, delivered)
- **Execution:** per-feature `execution/changeset.md`, `execution/verification.md`

## Current execution truth

- Feature 032: T000–T013 complete. Verification green 2026-03-28.
- Feature 033: T001–T008 complete (sidecar + UI + verification).
- Feature 034: T001–T006 complete; `execution/verification.md` + `changeset.md` filled; golden graph updated.

## Residual notes (033)

1. **Cross-platform binaries** — Still only `axiomregent-aarch64-apple-darwin` under `src-tauri/binaries/`. Follow-up: `build:executables` / CI matrix when ready.
2. **Golden test** — `crates/axiomregent/tests/mcp_contract.rs` may need `UPDATE_GOLDEN=1` if tools list JSON formatting drifts (orthogonal to sidecar work).

Historical review: `.ai/reviews/claude-review.md` (Feature 033 section).

## Data integrity fixes (this pass)

- `.ai/plans/integration-debt.md` — **restored** (was corrupted: contained concatenation of next-slice + promotion-candidates + current.md)
- `.ai/plans/next-slice.md` — cleaned stale `implemented` references
- `.ai/plans/promotion-candidates.md` — cleaned stale `implemented` reference

## Baton

- Current owner: **claude**
- Next owner: **cursor** (035 scaffold / implementation) or **claude-opus** (next-slice synthesis)
- Last baton update: 2026-03-28 — **Cursor** completed **034** (registry-first scanner, docs, verification, spec active); baton to **Claude** for review
- Requested outputs from **Claude**:
  1. Spot-check **034** vs `specs/034-featuregraph-registry-scanner-fix/spec.md` and `execution/verification.md`.
  2. Optionally refresh `.ai/findings/` if governance path behavior changed materially.
  3. Return baton to **cursor** for **035** (agent routing / governed execution) per `.ai/plans/next-slice.md`, or to **claude-opus** for prioritization.
- Deferred: **035** — scaffold + implement when prioritized.
- Recommended files to read:
  - `specs/034-featuregraph-registry-scanner-fix/execution/changeset.md`
  - `crates/featuregraph/src/registry_source.rs`, `crates/featuregraph/src/scanner.rs`
  - `.ai/plans/next-slice.md`

## Requested next agent output

**Claude:** 034 post-delivery review. **Cursor:** pick up **035** when baton returns.

## Promotion candidates for canonical artifacts

- **`spec-compiler compile`** — re-run after large spec edits (green at 034 close)
- ~~**034** registry scanner~~ — delivered
- **035** agent execution reroute → next major slice

---

## Recent outputs

- 2026-03-28 (cursor): Feature **034** complete (registry-first `Scanner`, `registry_source`, golden update, execution docs, baton → **claude**)
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
