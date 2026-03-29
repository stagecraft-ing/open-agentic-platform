# Promotion candidates (working list)

> **Non-authoritative.** A **queue of ideas** to move into canonical homes — not commitments until promoted.

## Ready to promote

| Item | Current evidence | Target artifact |
|------|------------------|-----------------|
| Feature 032 status → `implemented` | `tasks.md` shows T000–T013 all [x]; `verification.md` has green run 2026-03-28 | `specs/032-opc-inspect-governance-wiring-mvp/spec.md` frontmatter `status: implemented` (per Feature 003 lifecycle) |
| Post-032 axiomregent activation spec | Analysis in `plans/next-slice.md`; `spawn_axiomregent()` exists at `sidecars.rs:48`; infrastructure ready | New `specs/033-axiomregent-activation/spec.md` |
| Post-032 featuregraph scanner fix | `scanner.rs:167` reads forbidden `features.yaml`; governance panel permanently degraded | Either standalone spec or 033-adjacent task |
| Post-032 safety tier spec | `safety.rs` defines tiers in code only; no spec governs tier assignments | New spec (can parallel with 033) |

## Needs more verification

| Item | What would prove it |
|------|---------------------|
| Post-032: axiomregent activation spec | Verify `spawn_axiomregent()` actually works when called; test sidecar binary bundling on all platforms |
| Post-032: featuregraph scanner adaptation | Prototype reading from `registry.json` instead of `features.yaml`; confirm graph output is compatible |
| Post-032: safety tier spec | Review `safety.rs` tier assignments for completeness; determine if tier→permission mapping is sufficient |
| Post-032: feature ID reconciliation | Survey all `// Feature:` headers in codebase; count unique UPPERCASE IDs; determine if convention-based derivation from kebab IDs is feasible |

## Done / discarded (archive)

- ~~T010 action recommendation: "View spec" button~~ — **promoted and implemented** (Cursor, 2026-03-28). `RegistrySpecFollowUp.tsx` + `actions.ts` using `featureSummaries` from backend.
- ~~Verification command list for T013~~ — **promoted** to `execution/verification.md` T010–T013 section.
- ~~Featuregraph degraded state documentation~~ — **promoted** to `execution/verification.md` product notes.

## Reminder

Promote into: `specs/.../spec.md`, `specs/.../tasks.md`, `specs/.../execution/changeset.md`, `specs/.../execution/verification.md`, or code/docs as appropriate — then trim or close entries here.
