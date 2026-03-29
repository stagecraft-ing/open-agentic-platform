> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Features **032–037** delivered (037 partial: Windows binary + CI, macOS x86_64/Linux deferred to CI runners). See `specs/037-cross-platform-axiomregent/spec.md`.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **037 spec:** `specs/037-cross-platform-axiomregent/spec.md` (status: **active**, partially implemented)

## Current execution truth

- **037:** T001/T002/T005/T006/T007/T008 complete. T003/T004 deferred to CI. T009 pending.
- Stale `Tier` → `ToolTier` fix in `agent.rs` (036 residual).
- Review found: 1 stale doc comment (`lease.rs:97`), CI smoke test `timeout` portability issue, missing cargo cache.

## Baton

- Current owner: **antigravity**
- Next owner: **claude-opus**
- Last baton update: 2026-03-29 — **claude** reviewed Feature 037. All FRs pass (FR-001 partial: 2/5 binaries bundled, CI covers rest). ToolTier fix verified correct, no other stale `Tier` references in code (1 doc comment in `lease.rs:97`). T003/T004 deferral to CI confirmed as correct call. Three CI/build-script issues identified (smoke test timeout, cargo cache, `--all` flag docs).
- Requested outputs from **antigravity**:
  1. Wide pass: confirm no other stale `Tier`/`SafetyTier` references beyond `lease.rs:97` doc comment.
  2. Fix `lease.rs:97` doc comment: `agent::safety::Tier` → `agent::safety::ToolTier`.
  3. Optionally fix CI smoke test `timeout` portability (replace with cross-platform alternative).
  4. Verify `.exe` binary is correctly tracked in git (it was added with `git add -f` due to `*.exe` gitignore).

- Recommended files to read:
  - `.ai/reviews/claude-review.md` (Feature 037 section at end)
  - `crates/axiomregent/src/snapshot/lease.rs:97` (stale doc comment)
  - `.github/workflows/build-axiomregent.yml:73` (smoke test timeout)
  - `.gitignore` (*.exe rule vs tracked binary)

## Requested next agent output

**Antigravity:** Wide pass on Feature 037, fix stale doc comment, verify git tracking.

## Promotion candidates for canonical artifacts

- **`spec-compiler compile`** — re-run to pick up 037 spec (now active)
- **Cargo cache** — add `Swatinem/rust-cache@v2` to CI workflow (performance optimization)

---

## Recent outputs

- 2026-03-29 (claude): Feature 037 review — all FRs pass (FR-001 partial). ToolTier fix correct, 1 stale doc comment in `lease.rs:97`. CI smoke test `timeout` won't work on macOS (continue-on-error masks it). T003/T004 deferral correct. Updated `claude-review.md`. Baton → **antigravity** for wide pass.
- 2026-03-29 (cursor): Feature 037 implemented — Windows x86_64 binary built (7.3 MB), MCP handshake verified, all 21 tools confirmed. Created build script and CI workflow. Fixed stale `Tier`→`ToolTier` in `agent.rs`. All tests green. Baton → **claude** for review.
- 2026-03-29 (claude-opus): Post-036 synthesis complete. Scaffolded Feature 037. Updated authority-map, next-slice, integration-debt. Baton → **cursor**.
- 2026-03-29 (antigravity): Feature 036 wide pass complete. Baton → **claude-opus**.
- 2026-03-29 (claude): Feature 036 review — all FRs/SCs pass. Baton → **antigravity**.
- 2026-03-29 (cursor): Feature 036 implemented (T001–T007). Baton → **claude**.
- 2026-03-29 (claude-opus): Feature 036 spec scaffolded. Baton → **cursor**.
- 2026-03-29 (antigravity): Slice A wide pass. Baton → **claude-opus**.
- 2026-03-29 (claude): Slice A review. Baton → **antigravity**.
- 2026-03-29 (cursor): Slice A complete. Baton → **claude**.
- 2026-03-29 (claude-opus): Post-035 synthesis. Baton → **cursor**.
