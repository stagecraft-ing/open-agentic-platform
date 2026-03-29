> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Features **032–036** delivered. **Feature 037** (cross-platform axiomregent binaries) scaffolded: spec + tasks ready for implementation. See `specs/037-cross-platform-axiomregent/spec.md`.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **036 spec:** `specs/036-safety-tier-governance/spec.md` (status: **active**, delivered)
- **037 spec:** `specs/037-cross-platform-axiomregent/spec.md` (status: **draft**, ready for implementation)

## Current execution truth

- **036:** T001–T007 complete. All 21 tools classified, dual enums renamed, coverage test green, per-tool tier UI live.
- **037:** Spec + tasks scaffolded. 9 tasks (T001–T009). No implementation yet.

## Baton

- Current owner: **cursor**
- Next owner: **claude**
- Last baton update: 2026-03-29 — **claude-opus** completed post-036 synthesis. Scaffolded Feature 037 (cross-platform axiomregent). Updated authority-map (safety tiers → RESOLVED), next-slice (Slices A+B complete, C/D/E re-evaluated), integration-debt (5 items resolved). Ordered next slices: C (037 cross-platform), D (titor wiring), E (ID reconciliation).
- Requested outputs from **cursor**:
  1. Implement Feature 037 — priority is T002 (build Windows binary) since the development environment is Windows.
  2. T005 (verify sidecar spawn on Windows) is the key verification — confirm governed dispatch works end-to-end.
  3. T001/T003/T004 (build script, other platform binaries) can be deferred if CI is needed.

- Recommended files to read:
  - `specs/037-cross-platform-axiomregent/spec.md`
  - `specs/037-cross-platform-axiomregent/tasks.md`
  - `apps/desktop/src-tauri/src/sidecars.rs` (sidecar spawn — no changes needed)
  - `apps/desktop/src-tauri/src/commands/mcp.rs:29-60` (reference: gitctx-mcp binary resolution pattern)
  - `crates/axiomregent/Cargo.toml` (dependencies: rusqlite bundled, zstd — need C compiler per target)

## Requested next agent output

**Cursor:** Implement Feature 037, starting with T002 (Windows binary build) and T005 (Windows verification).

## Promotion candidates for canonical artifacts

- **`spec-compiler compile`** — re-run after 037 spec is active
- **037** — draft; promote to active after implementation begins
- **038** — titor command wiring (scaffold after 037 delivery)

---

## Recent outputs

- 2026-03-29 (claude-opus): Post-036 synthesis complete. Scaffolded Feature 037 (cross-platform axiomregent binaries). Updated authority-map: safety tiers RESOLVED (036). Updated next-slice: Slices A+B complete; next is C (037 cross-platform) → D (titor wiring) → E (ID reconciliation). Updated integration-debt: 5/8 items resolved. Baton → **cursor**.
- 2026-03-29 (antigravity): Feature 036 wide pass complete — confirmed no stale Safety/ToolTier references, removed dead `safety::Tier` alias, verified `write_file` routing. Added reverse coverage test for explicitly classified tools. Baton → **claude-opus**.
- 2026-03-29 (claude): Feature 036 review — all FRs/SCs pass. Tier assignments verified against spec. Three minor cleanup items: dead `Tier` alias, stale bindings.ts doc comment, one-directional coverage test. No security issues. Updated `claude-review.md`. Baton → **antigravity** for wide pass.
- 2026-03-29 (cursor): Feature 036 implemented (T001–T007). All 21 tools classified, `Tier`→`ToolTier`, `SafetyTier`→`ChangeTier`, per-tool tier UI, coverage test. All tests green. Baton → **claude** for review.
- 2026-03-29 (claude-opus): Feature 036 spec scaffolded — safety tier governance. 13/21 tools unclassified (default Tier3), dual enum collision, proposed tier table + 9 tasks. Baton → **cursor** for implementation.
- 2026-03-29 (antigravity): Slice A wide pass — confirmed no stale Risk 1 refs, verified `allowed_no_lease` in test output, no new `?`-based bypasses. Baton → **claude-opus**.
- 2026-03-29 (claude): Slice A review — all 4 tasks pass. No-lease fallback correctly uses session grants. `check_grants` extraction clean. Audit log tags well-chosen. Updated `claude-review.md` and `authority-map.md` (Risk 1 residual cleared). Baton → **antigravity**.
- 2026-03-29 (cursor): Slice A complete — no-lease bypass fixed (`router/mod.rs` falls back to session grants), NF-001 benchmark (3 tests, sub-µs overhead), max_tier rationale documented in spec contract notes, scanner error wording updated. All tests green.
- 2026-03-29 (claude-opus): Post-035 synthesis complete. Ordered 5 slices (A: hardening, B: safety tier spec, C: cross-platform, D: titor wiring, E: ID reconciliation). Updated `authority-map.md` — 3 CRITICAL/HIGH items now RESOLVED. Baton → **cursor** for Slice A.
- 2026-03-29 (antigravity): Feature **035** wide pass check complete. Confirmed zero stale `--dangerously-skip-permissions` outside of `Bypass`. Identified test fixtures invoking tools without `lease_id`; baton → **claude-opus**
- 2026-03-29 (claude): Feature **035** post-delivery review — all FRs pass, two residual risks (no-lease bypass, agent max_tier rationale); baton → **antigravity**
- 2026-03-29 (cursor): Feature **035** implementation (T001–T013), commits on `main`; baton → **claude**
- 2026-03-29 (claude-opus): Synthesized Feature **035** scope; scaffolded spec/tasks; baton → **cursor**
- 2026-03-29 (cursor): feat(axiomregent) T002–T003 lease + router preflight; T010 audit stderr; desktop governed launch + UI; spec **active**
