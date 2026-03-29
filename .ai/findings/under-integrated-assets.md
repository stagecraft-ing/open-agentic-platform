# Under-integrated assets (working notes)

> **Non-authoritative.** Product scope still flows from `specs/`; this lists **opportunity**, not commitment.

## Purpose

Spot **existing** crates, packages, commands, or UI shells that could add leverage with **small** wiring work (Feature **032**-style slices).

## Canonical references (read first)

- `specs/032-opc-inspect-governance-wiring-mvp/spec.md` (in/out of scope)
- Repo layout: `apps/`, `packages/`, `crates/` as present

## Candidates

| Asset | Current use | Possible integration | Effort (S/M/L) | Spec tie-in |
|-------|-------------|----------------------|----------------|-------------|
| **axiomregent** MCP router | Compiled binary; `spawn_axiomregent()` defined in `sidecars.rs:48` but never called | Call `spawn_axiomregent(app)` in `lib.rs` setup; expose tools via MCP management UI; route agent execution through it | **S** to spawn, **M** to route agents | Post-032; needs own spec for activation + agent routing |
| **titor** checkpoint library | Library complete (~17k LOC); `titor_init()` wired; 5 other Tauri commands `todo!()` | Implement `titor_checkpoint/list/restore/diff/verify` using existing Titor API + Tauri managed state | **M** (5 commands, state management) | Post-032; temporal safety for agent execution |
| **featuregraph scanner** | Works if `spec/features.yaml` exists; currently always fails | Adapt `Scanner::scan()` to read from `registry.json` or parse spec markdown directly | **M** (scanner rewrite) | Would fix governance panel degraded state; could be 032-adjacent |
| **`featuregraph_impact`** Tauri command | Implemented (`analysis.rs:76-82`) but inherits scanner dependency | Same fix as scanner above; once scanner works, impact analysis works | **S** (blocked by scanner fix) | Enables "which features does this change affect?" from inspect |
| **blockoli** semantic search | Crate skeleton with fastembed/qdrant deps; Tauri commands `todo!()` | Connect embedding pipeline + implement Tauri commands | **L** (backend not wired) | Post-032; separate feature spec needed |
| **safety.rs tier model** | Implemented in code; only consulted inside axiomregent | Surface tier classification in governance UI; use for agent execution gating | **S** to display, **M** to enforce | Post-032; needs own spec |
| **agent permission flags** | Stored in SQLite, shown in UI, never enforced | Translate flags into execution constraints (either via axiomregent tiers or Claude permission args) | **M** (requires axiomregent activation or alternative enforcement) | Post-032; depends on axiomregent spec |
| **asterisk/stackwalk** AST analysis | Both crates work; `stackwalk_index` wired to Tauri; `asterisk` is near-duplicate | Deduplicate (remove asterisk, alias to stackwalk); connect call graph to feature attribution | **S** (dedup), **M** (feature attribution link) | Low priority; nice-to-have |

## Highest leverage (ranked)

1. **Spawn axiomregent** — smallest effort, unlocks governed tool surface
2. **Fix featuregraph scanner** — unblocks governance panel from permanent degradation
3. **Wire titor commands** — enables temporal safety net
4. **Route agents through axiomregent** — makes permission enforcement real

## Candidate promotions

- [ ] New task in `tasks.md` — not for 032 (out of scope); track in post-032 planning
- [ ] Defer / out of scope note in `plan.md` — axiomregent activation, titor wiring, scanner fix are post-032 but should be explicitly noted as next priorities
