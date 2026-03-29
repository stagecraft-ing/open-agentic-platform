# Promotion candidates (working list)

> **Non-authoritative.** A **queue of ideas** to move into canonical homes - not commitments until promoted.

## Ready to promote

| Item | Current evidence | Target artifact |
|------|------------------|-----------------|
| Feature 032 delivery recorded (lifecycle-safe) | `tasks.md` complete; `verification.md` green 2026-03-28 | Keep frontmatter `status: active` per registry enum; delivery in body + execution artifacts (**not** `implemented` — invalid enum) |
| Feature 033 - axiomregent activation | `spawn_axiomregent()` exists; sidecar/state/port plumbing already present; post-032 synthesis says this is the highest-leverage next slice | New `specs/033-axiomregent-activation/spec.md` |
| Featuregraph scanner fix | `scanner.rs:167` reads forbidden `features.yaml`; governance panel remains degraded until registry-backed scan exists | New `specs/034-featuregraph-registry-scanner-fix/spec.md` |
| Safety tier governance spec | `safety.rs` defines tiers in code only; no spec governs tier meanings or assignment rules | New feature spec after 033 promotion |

## Needs more verification

| Item | What would prove it |
|------|---------------------|
| Feature 033 - axiomregent activation | Verify `spawn_axiomregent()` works on supported platforms; confirm Tauri sidecar bundling and port discovery behave correctly |
| Featuregraph scanner adaptation | Prototype scanner input from `registry.json`; confirm graph output shape remains compatible with governance UI |
| Safety tier governance spec | Review `safety.rs` for full tier coverage and determine whether tier->permission mapping is sufficient as canonical policy |
| Agent execution reroute | Confirm activation/UI visibility land cleanly before changing execution authority |
| Feature ID reconciliation | Survey all `// Feature:` headers in codebase; count unique uppercase IDs; determine whether convention-based derivation from kebab IDs is feasible |

## Done / discarded (archive)

- ~~T010 action recommendation: "View spec" button~~ - **promoted and implemented** (Cursor, 2026-03-28). `RegistrySpecFollowUp.tsx` + `actions.ts` using `featureSummaries` from backend.
- ~~Verification command list for T013~~ - **promoted** to `execution/verification.md` T010-T013 section.
- ~~Featuregraph degraded state documentation~~ - **promoted** to `execution/verification.md` product notes.

## Reminder

Promote into: `specs/.../spec.md`, `specs/.../tasks.md`, `specs/.../execution/changeset.md`, `specs/.../execution/verification.md`, or code/docs as appropriate - then trim or close entries here.
