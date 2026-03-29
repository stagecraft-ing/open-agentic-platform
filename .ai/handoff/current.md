> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Features **032–034** **delivered** (`status: active`). **035** scaffolded: **agent governed execution** — route agent dispatch through axiomregent, enforce permission flags + safety tiers, replace `--dangerously-skip-permissions`. Spec + tasks + execution stubs at `specs/035-agent-governed-execution/`. **Next:** **Cursor** implements 035.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **032 spec:** `specs/032-opc-inspect-governance-wiring-mvp/spec.md` (status: active, delivered)
- **033 spec:** `specs/033-axiomregent-activation/spec.md` (status: active, delivered)
- **034 spec:** `specs/034-featuregraph-registry-scanner-fix/spec.md` (status: active, delivered)
- **035 spec:** `specs/035-agent-governed-execution/spec.md` (status: draft, scaffolded)
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

- Current owner: **cursor**
- Next owner: **claude** (post-implementation review)
- Last baton update: 2026-03-29 — **claude-opus** synthesized Feature 035 scope, scaffolded `specs/035-agent-governed-execution/` with spec + tasks + execution stubs; baton to **cursor** for implementation
- Requested outputs from **cursor**:
  1. **T001 spike first** — validate axiomregent-as-MCP-server integration pattern (can Claude CLI use `--mcp-server` pointing at axiomregent's port?). Produce findings before full implementation.
  2. Implement T002–T013 per `specs/035-agent-governed-execution/tasks.md`.
  3. Key files to modify: `agents.rs` (1 site), `claude.rs` (3 sites), `web_server.rs` (3 sites), `router/mod.rs`, `lease.rs`, `CreateAgent.tsx`, `api.ts`.
- Deferred: safety-tier governance spec, feature ID reconciliation, titor command stubs, cross-platform axiomregent binaries.
- Recommended files to read:
  - `specs/035-agent-governed-execution/spec.md` (full spec with architecture diagram)
  - `specs/035-agent-governed-execution/tasks.md` (13 tasks, 5 phases)
  - `crates/agent/src/safety.rs` (existing tier model — no changes needed)
  - `crates/axiomregent/src/router/mod.rs` (tool dispatch + `PermissionDenied` already defined)
  - `crates/axiomregent/src/snapshot/lease.rs` (lease model to extend with permission grants)

## Requested next agent output

**Cursor:** implement **035** (agent governed execution) per scaffolded spec. Start with T001 spike.

## Promotion candidates for canonical artifacts

- **`spec-compiler compile`** — re-run after large spec edits (green at 034 close)
- ~~**034** registry scanner~~ — delivered
- **035** agent governed execution → scaffolded, ready for implementation

---

## Recent outputs

- 2026-03-29 (claude-opus): Synthesized Feature **035** scope (agent governed execution); scaffolded `specs/035-agent-governed-execution/` (spec, tasks, execution stubs); baton → **cursor**
- 2026-03-29 (claude): Feature **034** post-delivery review — all FRs pass, no blockers; original concern §2 (scanner yaml dependency) resolved; baton forward to cursor/claude-opus for **035**
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
