# Promotion candidates (working list)

> **Non-authoritative.** A **queue of ideas** to move into canonical homes - not commitments until promoted.

## Ready to promote

| Item | Current evidence | Target artifact |
|------|------------------|-----------------|
| Blockoli semantic search discovery | `crates/blockoli/` library exists, desktop UI stub present, no Tauri command wiring | New `specs/040-blockoli-semantic-search/` (after discovery pass) |
| Checkpoint/restore UI | Feature 038 wired all 6 Tauri commands, no desktop UI for temporal controls | New feature spec (after design input) |
| Minor code cleanup batch | V-005 message, lease.rs doc comment, CI timeout portability | Direct code commits (no spec needed) |

## Needs more verification

| Item | What would prove it |
|------|---------------------|
| Blockoli semantic search | Survey `crates/blockoli/` public API; assess embedding model requirements; determine startup cost |
| Checkpoint/restore UI | Design input on desktop UX; determine per-project vs per-agent-session scope |

## Done / discarded (archive)

- ~~T010 action recommendation: "View spec" button~~ - **promoted and implemented** (Cursor, 2026-03-28)
- ~~Verification command list for T013~~ - **promoted** to `execution/verification.md`
- ~~Featuregraph degraded state documentation~~ - **promoted** to `execution/verification.md`
- ~~Feature 033 - axiomregent activation~~ - **DELIVERED** (Feature 033)
- ~~Featuregraph scanner fix~~ - **DELIVERED** (Feature 034)
- ~~Agent governed execution~~ - **DELIVERED** (Feature 035)
- ~~Safety tier governance spec~~ - **DELIVERED** (Feature 036)
- ~~Cross-platform axiomregent~~ - **DELIVERED** (Feature 037)
- ~~Titor command wiring~~ - **DELIVERED** (Feature 038)
- ~~Feature ID reconciliation~~ - **DELIVERED** (Feature 039)

## Reminder

Promote into: `specs/.../spec.md`, `specs/.../tasks.md`, `specs/.../execution/changeset.md`, `specs/.../execution/verification.md`, or code/docs as appropriate - then trim or close entries here.
