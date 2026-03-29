# Next slice (working synthesis)

> **Non-authoritative.** This is a **staging** view for the smallest next increment — not a replacement for `specs/.../tasks.md` or `plan.md`. Promote agreed work into canonical tasks.

## Context

- Branch: `main`
- **Feature 032: COMPLETE** (T000–T013 all done, verification green)
- Next priorities: post-032 convergence work

## Smallest high-leverage slice (proposal): Feature 033 — axiomregent activation

### Why this is the next convergence slice

axiomregent is the only component that unifies governance enforcement, safety tiers, snapshot management, workspace operations, and feature analysis into a single dispatch surface. It is compiled, tested, has sidecar infrastructure (`spawn_axiomregent`, `SidecarState`, port discovery) — and is never started. One function call away from transforming the platform.

### Proposed scope

1. **Write `specs/033-axiomregent-activation/spec.md`** — formalize sidecar activation, tool surface, and safety tier enforcement as a governed feature
2. **Call `spawn_axiomregent(app)` in `lib.rs` setup** — activate the sidecar on app startup
3. **Verify sidecar binary bundling** — confirm `axiomregent` binary is available as Tauri sidecar on all target platforms
4. **Expose axiomregent tools in MCP management UI** — frontend can list/call axiomregent tools alongside gitctx
5. **Add safety tier display** — show tier classification in governance panel

### What this explicitly does NOT include (defer to Feature 034+)

- Rerouting agent execution through axiomregent (separate feature — changes execution authority)
- Replacing `--dangerously-skip-permissions` (depends on agent routing)
- Fixing featuregraph scanner (independent work, can parallel)
- Titor command wiring (independent)

### Parallel work (can run alongside 033)

- **Fix featuregraph scanner** — adapt `Scanner::scan()` to read `registry.json`; promotes governance panel from degraded to full
- **Wire titor commands** — implement 5 stubbed Tauri commands

## Dependencies / risks

- axiomregent binary must be bundled as Tauri sidecar (check `tauri.conf.json` sidecar config)
- Port discovery (`OPC_AXIOMREGENT_PORT=`) must work on Windows (current code uses `cmd.spawn()` via Tauri shell plugin)
- axiomregent's `Scanner::scan()` (via featuregraph) will also hit `features.yaml` issue — may need scanner fix first or accept degraded featuregraph tools

## After promotion (canonical)

- [ ] Create `specs/033-axiomregent-activation/` with spec.md, plan.md, tasks.md
- [ ] Update Feature 032 status from `active` to implemented in spec frontmatter (per Feature 003 lifecycle)
- [ ] Track scanner fix and titor wiring as separate feature specs or 033-adjacent tasks
