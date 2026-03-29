# Integration debt (working synthesis)

> **Non-authoritative.** Tracks **felt** debt across boundaries; formal tracking stays in specs and execution artifacts once promoted.

## Context

- Branch: `main`
- Related specs: 032 (active, T010–T013 remaining), 000 (constitutional), 003–005 (lifecycle/execution)

## Debt items

| Item | Area | Payoff | Suggested order |
|------|------|--------|-------------------|
| **axiomregent not spawned** | Runtime / governance enforcement | Activates governed tool surface (gov.preflight, gov.drift, snapshot.*, workspace.*, agent.*, run.*); transforms app from Claude wrapper to control plane | **1st** post-032 |
| **Agent execution ungoverned** | Execution / trust | Makes `enable_file_read/write/network` flags real; routes through safety tiers; closes display-vs-enforcement gap | **2nd** (depends on axiomregent) |
| **featuregraph scanner reads nonexistent `features.yaml`** | Governance / data | Unblocks governance panel from permanent degradation; enables feature attribution across specs and code | **3rd** (independent of axiomregent) |
| **Titor Tauri commands stubbed** | Temporal safety | Enables checkpoint/restore/diff/verify from desktop; provides rollback safety net for agent execution | **4th** (independent) |
| **Safety tiers not spec-governed** | Governance process | Formalizes `safety.rs` tier model; makes tier assignments auditable and changeable via spec process | **5th** (can parallel with 1-4) |
| **Feature ID duality** | Data architecture | Enables cross-referencing registry features with code attribution; prevents two naming systems from diverging | **6th** (design decision first) |
| **Blockoli semantic search stubbed** | Product capability | Enables AI-native code search from desktop | **7th** (heavy lift, lower urgency) |
| **No audit trail for agent actions** | Observability / compliance | Required for verifiable governance; tracks what axiomregent allowed/denied | Parallel with axiomregent activation |

## Notes

- Items 1–2 are the same integration: activate axiomregent, then route agents through it. Together they close the platform's biggest thesis gap.
- Item 3 (scanner fix) could be done independently and would immediately improve the governance panel from degraded to functional.
- Items 1–4 represent the "next convergence slice" after 032 — a natural Feature 033 scope.
- The `--dangerously-skip-permissions` flag appears 7 times and is the single artifact most at odds with the platform's stated purpose.

## Promotion

- [ ] File post-032 feature specs — at minimum: axiomregent activation, safety tier model
- [ ] Track scanner fix as either 032-adjacent or post-032 depending on scope appetite
