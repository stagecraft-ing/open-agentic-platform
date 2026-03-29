> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Feature **032** is complete. Feature **033** (`axiomregent activation`) is **delivered**: `status: active`, tasks T001–T008 checked, `execution/verification.md` + `changeset.md` updated. Spawn wired in `lib.rs`; probe port on **stderr**; MCP + governance UI show sidecar / preflight tier reference. **Next:** **Claude** post-implementation review; then **claude-opus** for next-slice synthesis or **cursor** for 034-class work.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **032 spec:** `specs/032-opc-inspect-governance-wiring-mvp/spec.md` (status: active, delivered)
- **033 spec:** `specs/033-axiomregent-activation/spec.md` (status: active, delivered)
- **Execution:** per-feature `execution/changeset.md`, `execution/verification.md`

## Current execution truth

- Feature 032: T000–T013 complete. Verification green 2026-03-28.
- Feature 033: T001–T008 complete. Sidecar spawned at startup; probe port announced on stderr from `axiomregent` (TCP listener); UI surfaces in MCP manager + governance. Binary bundling: still **macOS arm64 only** in-repo — other targets degrade gracefully (documented in `execution/verification.md`).

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
- Next owner: **claude-opus** (next-slice / priorities) or **cursor** (034-class implementation)
- Last baton update: 2026-03-28 — **Cursor** implemented Feature **033** (spawn, stderr probe port, MCP + governance UI, verification artifacts); baton to **Claude** for post-implementation review
- Requested outputs from **Claude**:
  1. Review 033 implementation vs `specs/033-axiomregent-activation/spec.md` and `execution/verification.md`.
  2. Optionally update `.ai/reviews/claude-review.md` Feature 033 section with final architecture notes (stderr probe, UI entry points).
  3. Update baton: hand to **claude-opus** for synthesis or **cursor** for next feature; run `spec-compiler compile` if available (promotion candidate).
- Recommended files to read:
  - `specs/033-axiomregent-activation/execution/changeset.md`, `execution/verification.md`
  - `apps/desktop/src-tauri/src/lib.rs` (spawn after `SidecarState`)
  - `apps/desktop/src-tauri/src/sidecars.rs`, `crates/axiomregent/src/main.rs` (probe + stderr line)
  - `apps/desktop/src/components/MCPManager.tsx`, `apps/desktop/src/features/governance/GovernanceSurface.tsx`

## Requested next agent output

Claude: confirm 033 delivery against spec; then pass baton for **034-class** planning (registry/scanner) or next slice per `.ai/plans/next-slice.md`.

## Promotion candidates for canonical artifacts

- Run `spec-compiler compile` to validate registry frontmatter after 033 merge
- Scanner / `registry.json` work → Feature 034-class
- Agent execution reroute → Feature 035-class

---

## Recent outputs

- 2026-03-28 (cursor): Feature **033** implementation complete (axiomregent spawn, stderr probe port, MCP/governance UI, execution docs); baton to **claude**
- 2026-03-29 (claude): Confirmed handoff coherence + 033 review spot-check (no drift); baton to cursor
- 2026-03-28 (cursor): Removed obsolete vendor references from `.ai/handoff/current.md`; passed baton to **claude** (pre-033 review pass)
- 2026-03-29 (claude): 033 spec review, data integrity fixes (integration-debt.md restored), baton to cursor
- 2026-03-29 (cursor): Repaired handoff (NUL bytes, lifecycle); scaffolded `specs/033-axiomregent-activation/`
- 2026-03-29 (claude-opus): synthesis in `.ai/plans/next-slice.md`
- 2026-03-28 (cursor): T010–T013 implementation + verification
- 2026-03-28 (claude): Reconciled findings with 032 closure, staged post-032 priorities
