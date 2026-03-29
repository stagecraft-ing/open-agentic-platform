# Next slice (working synthesis)

> **Non-authoritative.** This is a **staging** view for the smallest next increment — not a replacement for `specs/.../tasks.md` or `plan.md`. Promote agreed work into canonical tasks.

## Context

- Branch: `main`
- **Features 032–039: COMPLETE** — all delivered 2026-03-29, verification green
- **Slice A (post-035 hardening): COMPLETE** — no-lease bypass fixed, NF-001 benchmark, max_tier rationale documented
- Synthesis by: **claude-opus** (2026-03-29)

## Platform state after Feature 039

The governed execution thesis is **live, spec-governed, enforcement-complete, cross-platform, temporally recoverable, and identity-reconciled**:

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

**All authority-map items from 032 through 039 are RESOLVED.** The original four critical concerns from the 032 review are all closed:
1. ~~Governance is display-only~~ → Feature 035
2. ~~Scanner depends on nonexistent artifact~~ → Feature 034
3. ~~axiomregent is dead code~~ → Feature 033
4. ~~Dual feature identity with no bridge~~ → Feature 039

The platform has reached a **capability plateau**: the governed execution story is complete end-to-end. Remaining work falls into three categories: (1) **product surface expansion** (blockoli, checkpoint UI), (2) **CI/infra polish** (cross-platform binaries, smoke test portability), (3) **minor code cleanup** (V-005 message, stale doc comment).

## Residuals inventory

### Cross-platform axiomregent CI residuals (LOW)

T003 (macOS x86_64) and T004 (Linux x86_64/arm64) deferred to CI runners. CI workflow exists (`.github/workflows/build-axiomregent.yml`) but hasn't run yet. Will resolve automatically when CI runners are available.

### Minor code cleanup (LOW)

- `tools/spec-compiler/src/lib.rs:591` — V-005 second violation message names wrong feature for its path (review item from claude)
- `crates/axiomregent/src/snapshot/lease.rs:97` — stale doc comment `agent::safety::Tier` → `agent::safety::ToolTier` (carried since 037 review)
- CI smoke test `timeout` command portability on macOS (`build-axiomregent.yml:73`)

### Blockoli semantic search (LOW — heavy lift)

Desktop UI stub exists but backend is not wired. The `crates/blockoli/` library exists but Tauri command integration has not been scoped.

## Ordered next-slice priority

### Slice F: Blockoli semantic search wiring (Feature 040) — SCAFFOLDED

**Why first among remaining items:** The only substantial new capability left to unlock. The governance stack, temporal safety, and data architecture are all complete. Product-visible value now comes from exposing AI-native capabilities through the desktop app.

**Discovery complete.** Key findings:
- `crates/blockoli/` — 1,284 LOC, fastembed (384-dim vectors), KD-tree similarity, SQLite persistence, asterisk AST parsing
- Two `todo!()` stubs in `commands/search.rs` already registered in Tauri invoke handler
- `SemanticSearchPanel.tsx` fully implemented frontend — just needs working backend
- Integration pattern: `BlockoliState` managed state (like `TitorState`), direct `VectorStore::SQLiteStore(conn)` construction to override hardcoded DB path
- Risk: fastembed model download (~30 MB) on first embedding call

**Spec:** `specs/040-blockoli-semantic-search-wiring/spec.md` (6 FRs, 5 SCs). Ready for cursor implementation.

### Slice G: Desktop UI for checkpoint/restore

**Why second:** Feature 038 wired the backend commands. The next product-visible step is a UI for checkpoint/restore in the desktop app. Depends on design decisions about where checkpoint controls appear (per-project? per-agent-session?).

**Not ready to scaffold.** Needs design input.

### Slice H: Minor code cleanup batch

**Why lowest:** All items are non-blocking cosmetic fixes. Can be batched into a single commit when convenient. No spec needed.

Items:
- V-005 message wording fix (`lib.rs:591`)
- Stale doc comment (`lease.rs:97`)
- CI smoke test portability (`build-axiomregent.yml:73`)

## Fork resolution

**Chosen path: expand product surface.**

Features 032–039 established and completed the governed execution thesis including temporal safety and identity reconciliation. The platform is now capability-complete for governance. The next meaningful increment is product capability expansion (blockoli, checkpoint UI). The minor cleanup items can be batched at any convenient moment.

## Recommended promotion set

### Promote now

- **Feature 040 spec** — `specs/040-blockoli-semantic-search-wiring/spec.md` (drafted, ready for implementation)

### Promote next

- **Checkpoint/restore UI design** — design input on desktop UX for temporal controls
