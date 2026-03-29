# Next slice (working synthesis)

> **Non-authoritative.** This is a **staging** view for the smallest next increment — not a replacement for `specs/.../tasks.md` or `plan.md`. Promote agreed work into canonical tasks.

## Context

- Branch: `main`
- **Features 032–041: COMPLETE** — all delivered 2026-03-29, verification green
- **Slice A (post-035 hardening): COMPLETE** — no-lease bypass fixed, NF-001 benchmark, max_tier rationale documented
- Synthesis by: **claude-opus** (2026-03-29)

## Platform state after Feature 041

The governed execution thesis is **live, spec-governed, enforcement-complete, cross-platform, temporally recoverable, identity-reconciled, AI-native search enabled, and checkpoint/restore UI live**:

| Milestone | Feature | Status |
|-----------|---------|--------|
| Inspect + governance wiring | 032 | Active, complete |
| axiomregent sidecar alive | 033 | Active, complete |
| featuregraph reads registry | 034 | Active, complete |
| Agent execution governed | 035 | Active, complete |
| Safety tier governance | 036 | Active, complete |
| Cross-platform axiomregent | 037 | Active, complete (T003/T004 deferred to CI) |
| Titor Tauri command wiring | 038 | Active, complete |
| Feature ID reconciliation | 039 | Active, complete |
| Blockoli semantic search wiring | 040 | Active, complete |
| Checkpoint/restore UI | 041 | Active, complete |

**All authority-map items from 032 through 041 are RESOLVED.** The original four critical concerns from the 032 review are all closed:
1. ~~Governance is display-only~~ → Feature 035
2. ~~Scanner depends on nonexistent artifact~~ → Feature 034
3. ~~axiomregent is dead code~~ → Feature 033
4. ~~Dual feature identity with no bridge~~ → Feature 039

Features 040–041 delivered the **product surface expansion**: blockoli semantic code search (040) wired end-to-end, and checkpoint/restore UI (041) exposing all 6 titor temporal safety commands through a singleton panel. Remaining work: **CI/infra polish + minor code cleanup** (Slice H).

## Residuals inventory

### Cross-platform axiomregent CI residuals (LOW)

T003 (macOS x86_64) and T004 (Linux x86_64/arm64) deferred to CI runners. CI workflow exists (`.github/workflows/build-axiomregent.yml`) but hasn't run yet. Will resolve automatically when CI runners are available.

### Minor code cleanup (LOW)

- `tools/spec-compiler/src/lib.rs:591` — V-005 second violation message names wrong feature for its path (review item from claude)
- `crates/axiomregent/src/snapshot/lease.rs:97` — stale doc comment `agent::safety::Tier` → `agent::safety::ToolTier` (carried since 037 review)
- CI smoke test `timeout` command portability on macOS (`build-axiomregent.yml:73`)

### ~~Blockoli semantic search~~ — RESOLVED (Feature 040)

`BlockoliState` wired, both `blockoli_index_project` and `blockoli_search` commands implemented, app-data SQLite, embedded asterisk config. All FRs/SCs pass. Build green.

## Ordered next-slice priority (post-041)

### ~~Slice F: Blockoli semantic search wiring (Feature 040) — COMPLETE~~

**Delivered 2026-03-29.** Spec promoted `draft` → `active`. All 6 FRs pass, all 5 SCs pass. No blockoli HTTP route regressions. `cargo check` green. Review: `.ai/reviews/claude-review.md` (lines 645–698).

### ~~Slice G: Desktop UI for checkpoint/restore (Feature 041) — COMPLETE~~

**Delivered 2026-03-29.** Singleton `checkpoint` tab in titlebar tools dropdown. `CheckpointSurface` exposes all 6 titor commands (init, checkpoint, list, restore, diff, verify). Project-scoped design (user picks directory). `tsc --noEmit` clean.

### Slice H: Minor code cleanup batch

**Why lowest:** All items are non-blocking cosmetic fixes. Can be batched into a single commit when convenient. No spec needed.

Items:
- V-005 message wording fix (`lib.rs:591`)
- Stale doc comment (`lease.rs:97`)
- CI smoke test portability (`build-axiomregent.yml:73`)

## Fork resolution

**Chosen path: product surface expansion complete.**

Features 032–041 established the governed execution thesis (033–036), extended it cross-platform (037), wired temporal safety (038), reconciled identities (039), added AI-native code search (040), and surfaced checkpoint/restore in the desktop UI (041). The platform is now capability-complete for governance with two product surfaces (semantic search + checkpoint/restore). The only remaining work is Slice H (minor code cleanup batch).

## Recommended promotion set

### Promote now

- **Minor code cleanup batch** — V-005 message wording, `lease.rs:97` doc comment, CI portability (Slice H)

### Promote next

- New product surface or feature direction (post-041 — awaiting direction)
