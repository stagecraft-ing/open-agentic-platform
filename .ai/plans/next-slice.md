# Next slice (working synthesis)

> **Non-authoritative.** This is a **staging** view for the smallest next increment — not a replacement for `specs/.../tasks.md` or `plan.md`. Promote agreed work into canonical tasks.

## Context

- Branch: `main`
- **Features 032–036: COMPLETE** — all delivered 2026-03-29, verification green
- **Slice A (post-035 hardening): COMPLETE** — no-lease bypass fixed, NF-001 benchmark, max_tier rationale documented
- Synthesis by: **claude-opus** (2026-03-29)

## Platform state after Feature 036

The governed execution thesis is **live, spec-governed, and enforcement-complete on macOS arm64**:

| Milestone | Feature | Status |
|-----------|---------|--------|
| Inspect + governance wiring | 032 | Active, complete |
| axiomregent sidecar alive | 033 | Active, complete |
| featuregraph reads registry | 034 | Active, complete |
| Agent execution governed | 035 | Active, complete |
| Safety tier governance | 036 | Active, complete |

Every authority-map item from 032 through 036 is RESOLVED except **Titor** (HIGH — library complete, commands stubbed) and **Feature ID duality** (MEDIUM — 13 UPPERCASE vs 36 kebab IDs, no bridge).

The platform's critical gap has shifted from **"governance isn't enforced"** (pre-035) to **"governance only works on one platform"** (post-036).

## Residuals inventory

### From Feature 033 (cross-platform — HIGH priority)

**Only macOS arm64 axiomregent binary bundled.** Windows/Linux degrade to bypass mode — the entire governance thesis is inoperative on those platforms. The development team is on Windows, making this especially impactful.

**Fix:** Build axiomregent for 5 targets, bundle in `src-tauri/binaries/`. Pattern: follow `gitctx-mcp` binary resolution in `mcp.rs:29-60`. Spec scaffolded: `specs/037-cross-platform-axiomregent/`.

### Titor command stubs (HIGH — authority-map)

5 of 6 Tauri commands are `todo!()`. The titor library is production-ready (~17k LOC: checkpoint, restore, diff, verify, timeline, GC). Gap: no `TitorState` in Tauri `AppState`, no per-root-path instance tracking. One Cursor session to wire.

### Feature ID duality (MEDIUM — authority-map)

13 UPPERCASE code IDs vs 36 kebab spec IDs, no mapping. The scanner's alias system works in legacy YAML mode but not in the compiled registry. Needs an ADR before implementation.

### Ongoing minor items

- **NF-001 automated latency gate** — RESOLVED (Slice A). Sub-µs per call.
- **Scanner error wording** — RESOLVED (Slice A). Leads with `spec-compiler compile`.

## Ordered next-slice priority

### Slice C: Cross-platform axiomregent binaries (Feature 037) — SPEC SCAFFOLDED

**Why first:** The development team is on Windows. Governed execution is fully functional in code but inoperative on Windows due to missing binary. This is the single highest-impact change: one binary file enables the entire governance stack (035 permission enforcement + 036 tier governance) on the team's primary platform.

Spec: `specs/037-cross-platform-axiomregent/spec.md` (status: **draft**, 9 tasks scaffolded)

Scope:
1. **Build Windows binary** — `cargo build --release --target x86_64-pc-windows-msvc -p axiomregent` (T002). Highest priority single deliverable.
2. **Build script** for reproducible builds across all 5 targets (T001).
3. **Verify sidecar spawn on Windows** — confirm port discovery, governance UI, governed dispatch (T005).
4. **Build remaining targets** — macOS x86_64, Linux x86_64/arm64 (T003–T004).
5. **CI workflow** for automated builds (T006).

Key risk: `rusqlite` and `zstd` bundle C source code requiring platform-native C compilers. On Windows, MSVC toolchain handles this natively. CI matrix (one runner per OS) avoids cross-compilation complexity.

### Slice D: Titor Tauri command wiring (Feature 038)

**Why second:** Last HIGH item in the authority-map. Library is production-ready, gap is well-scoped. Enables the temporal safety net: checkpoint before agent actions, restore on failure.

Scope:
1. **Create `TitorState`** — `HashMap<PathBuf, Arc<Mutex<Titor>>>` in Tauri `AppState`.
2. **Wire 5 commands** — `titor_checkpoint`, `titor_list`, `titor_restore`, `titor_diff`, `titor_verify` using existing library API.
3. **Fix `titor_init`** — currently creates instance then discards it. Persist into `TitorState`.
4. **Expose in agent execution UI** — checkpoint/restore controls.
5. **Verification** — round-trip checkpoint→execute→restore.

Estimated effort: 1 Cursor session. No new crate dependencies.

### Slice E: Feature ID reconciliation (ADR)

**Why third:** Growing urgency (36 features and counting) but no enforcement impact today. Needs a design decision before implementation.

Scope:
1. **ADR**: choose canonical ID format, define mapping strategy.
2. **Options**: (a) add `aliases` field to compiled registry JSON, (b) derive UPPERCASE from kebab via convention, (c) adopt kebab everywhere and migrate code headers.
3. **Implement chosen strategy** in `Scanner` and `spec-compiler`.

## Fork resolution

**Chosen path: broaden platform → complete capabilities → reconcile identifiers.**

Features 032–036 established the governed execution thesis on macOS. The next priority is to bring that governance to all platforms (Slice C), then complete the temporal safety net (Slice D). Identifier reconciliation (Slice E) follows as a data-architecture improvement.

## Recommended promotion set

### Promote now

- **Feature 037 spec** — `specs/037-cross-platform-axiomregent/` (scaffolded 2026-03-29). Ready for cursor implementation.

### Promote next

- **Feature 038 spec** — titor Tauri command wiring (scaffold after 037 delivery)
- **Feature 039 ADR** — feature ID reconciliation (needs design decision)
