# Next slice (working synthesis)

> **Non-authoritative.** This is a **staging** view for the smallest next increment — not a replacement for `specs/.../tasks.md` or `plan.md`. Promote agreed work into canonical tasks.

## Context

- Branch: `main`
- **Features 032–037: COMPLETE** — all delivered 2026-03-29, verification green
- **Slice A (post-035 hardening): COMPLETE** — no-lease bypass fixed, NF-001 benchmark, max_tier rationale documented
- **Feature 037 (cross-platform axiomregent): COMPLETE** — Windows binary built and verified, CI workflow created, T003/T004 deferred to CI runners
- Synthesis by: **claude-opus** (2026-03-29)

## Platform state after Feature 037

The governed execution thesis is **live, spec-governed, enforcement-complete on macOS arm64, and partially extended to Windows**:

| Milestone | Feature | Status |
|-----------|---------|--------|
| Inspect + governance wiring | 032 | Active, complete |
| axiomregent sidecar alive | 033 | Active, complete |
| featuregraph reads registry | 034 | Active, complete |
| Agent execution governed | 035 | Active, complete |
| Safety tier governance | 036 | Active, complete |
| Cross-platform axiomregent | 037 | Active, complete (T003/T004 deferred to CI) |

Every authority-map item from 032 through 037 is RESOLVED except **Titor** (HIGH — library complete, commands stubbed) and **Feature ID duality** (MEDIUM — 13 UPPERCASE vs 38 kebab IDs, no bridge).

The platform's critical gap has shifted from **"governance only works on one platform"** (post-036) to **"temporal safety net not accessible from desktop"** (post-037).

## Residuals inventory

### Titor command stubs (HIGH — authority-map)

5 of 6 Tauri commands are `todo!()`. The titor library is production-ready (~17k LOC: checkpoint, restore, diff, verify, timeline, GC). Gap: no `TitorState` in Tauri `AppState`, no per-root-path instance tracking. One Cursor session to wire.

**Fix:** Create `TitorState`, wire 5 commands, fix `titor_init`. Spec scaffolded: `specs/038-titor-tauri-command-wiring/`.

### Feature ID duality (MEDIUM — authority-map)

13 UPPERCASE code IDs vs 38 kebab spec IDs, no mapping. The scanner's alias system works in legacy YAML mode but not in the compiled registry. Needs an ADR before implementation.

### Cross-platform axiomregent CI residuals (LOW)

T003 (macOS x86_64) and T004 (Linux x86_64/arm64) deferred to CI runners. CI workflow exists (`.github/workflows/build-axiomregent.yml`) but hasn't run yet. Will resolve automatically when CI runners are available.

### Ongoing minor items

- **NF-001 automated latency gate** — RESOLVED (Slice A). Sub-µs per call.
- **Scanner error wording** — RESOLVED (Slice A). Leads with `spec-compiler compile`.

## Ordered next-slice priority

### Slice D: Titor Tauri command wiring (Feature 038) — SPEC SCAFFOLDED

**Why first:** Last HIGH item in the authority-map. Library is production-ready, gap is well-scoped. Enables the temporal safety net: checkpoint before agent actions, restore on failure.

Spec: `specs/038-titor-tauri-command-wiring/spec.md` (status: **draft**, 11 tasks scaffolded)

Scope:
1. **Create `TitorState`** — `HashMap<PathBuf, Arc<Mutex<Titor>>>` in Tauri `AppState` (T001).
2. **Register in app setup** — `.manage(TitorState::new())` in `lib.rs` (T002).
3. **Fix `titor_init`** — persist instance into `TitorState` instead of discarding (T003).
4. **Wire 5 commands** — `titor_checkpoint`, `titor_list`, `titor_restore`, `titor_diff`, `titor_verify` using existing library API (T004–T008).
5. **Verification** — round-trip init→checkpoint→list→verify→diff→restore (T009).

Key risk: `Titor` may not be `Send + Sync` due to internal SQLite connection. Use `Mutex` wrapper (same pattern as `CheckpointManager`).

Estimated effort: 1 Cursor session. No new crate dependencies.

### Slice E: Feature ID reconciliation (ADR + Feature 039)

**Why second:** Growing urgency (38 features and counting) but no enforcement impact today. Needs a design decision before implementation.

Scope:
1. **ADR**: choose canonical ID format, define mapping strategy.
2. **Options**: (a) add `aliases` field to compiled registry JSON, (b) derive UPPERCASE from kebab via convention, (c) adopt kebab everywhere and migrate code headers.
3. **Implement chosen strategy** in `Scanner` and `spec-compiler`.

### Slice F: Blockoli semantic search (future)

Lowest urgency. Heavy lift. Desktop UI stub exists but backend is not wired.

## Fork resolution

**Chosen path: complete capabilities → reconcile identifiers → expand product.**

Features 032–037 established and broadened the governed execution thesis. The next priority is completing the temporal safety net (Slice D), then reconciling identifiers (Slice E). Product features like semantic search follow.

## Recommended promotion set

### Promote now

- **Feature 038 spec** — `specs/038-titor-tauri-command-wiring/` (scaffolded 2026-03-29). Ready for cursor implementation.

### Promote next

- **Feature 039 ADR** — feature ID reconciliation (needs design decision)
