# Promotion candidates (working list)

> **Non-authoritative.** A **queue of ideas** to move into canonical homes — not commitments until promoted.

## Ready to promote

| Item | Current evidence | Target artifact |
|------|------------------|-----------------|
| T010 action recommendation: "View spec" button using registry `specPath` | Source-grounded analysis in `findings/open-questions.md` Q1; `specPath` confirmed in `registry.schema.json` | T010 implementation by Cursor |
| Verification command list for T013 | Listed in `findings/open-questions.md` Q3; covers frontend build, Tauri compile, governance tests, consumer contracts, spec compiler | `execution/verification.md` |
| Featuregraph degraded state is expected for 032 MVP | `analysis.rs:55-58` returns `{status: "unavailable"}` gracefully; `scanner.rs:167` reads nonexistent file; FR-003 explicitly allows "explicit handling for unavailable data" | `execution/verification.md` as documented expected behavior |

## Needs more verification

| Item | What would prove it |
|------|---------------------|
| Post-032: axiomregent activation spec | Verify `spawn_axiomregent()` actually works when called; test sidecar binary bundling on all platforms |
| Post-032: featuregraph scanner adaptation | Prototype reading from `registry.json` instead of `features.yaml`; confirm graph output is compatible |
| Post-032: safety tier spec | Review `safety.rs` tier assignments for completeness; determine if tier→permission mapping is sufficient |
| Post-032: feature ID reconciliation | Survey all `// Feature:` headers in codebase; count unique UPPERCASE IDs; determine if convention-based derivation from kebab IDs is feasible |

## Done / discarded (archive)

- (none yet)

## Reminder

Promote into: `specs/.../spec.md`, `specs/.../tasks.md`, `specs/.../execution/changeset.md`, `specs/.../execution/verification.md`, or code/docs as appropriate — then trim or close entries here.
