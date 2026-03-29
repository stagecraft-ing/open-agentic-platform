# Integration debt (working synthesis)

> **Non-authoritative.** Tracks **felt** debt across boundaries; formal tracking stays in specs and execution artifacts once promoted.

## Context

- Branch: `main`
- Features 032–039: complete (all delivered 2026-03-29)
- Slice A (post-035 hardening): complete
- Related specs: 000 (constitutional), 003–005 (lifecycle/execution), 029–031 (consumer contracts), 032–039 (governance stack + cross-platform + temporal safety + identity reconciliation)

## Debt items (ranked)

| Item | Area | Payoff | Status |
|------|------|--------|--------|
| ~~axiomregent not spawned~~ | Runtime / governance enforcement | Activates governed tool surface | **RESOLVED (Feature 033)** |
| ~~Agent execution ungoverned~~ | Execution / trust | Makes permission flags real | **RESOLVED (Feature 035)** |
| ~~No-lease bypass~~ | Security / enforcement gap | Closes silent permission skip | **RESOLVED (Slice A)** |
| ~~Safety tiers not spec-governed~~ | Governance process | Formalizes tier model | **RESOLVED (Feature 036)** |
| ~~featuregraph scanner reads forbidden `features.yaml`~~ | Governance / data | Unblocks governance panel | **RESOLVED (Feature 034)** |
| ~~Cross-platform axiomregent~~ | Platform coverage | Governance on Windows/Linux | **RESOLVED (Feature 037)** — Windows binary built, CI workflow for all targets |
| ~~Titor Tauri commands stubbed~~ | Temporal safety | Enables checkpoint/restore from desktop | **RESOLVED (Feature 038)** — `TitorState` + all 6 commands wired, round-trip verified |
| ~~Feature ID duality~~ | Data architecture | Enables cross-referencing registry/code features | **RESOLVED (Feature 039)** — ADR 0001 accepted, `codeAliases` in schema 1.1.0, compiler + scanner + frontmatter. All 12 tokens bridged, zero orphans. |
| **Blockoli semantic search stubbed** | Product capability | Enables AI-native code search from desktop | Heavy lift; lowest urgency |

## Notes

- Items 1–8 are now **all resolved** (Features 032–039 + Slice A). The governance stack is complete on macOS arm64, partially extended to Windows, the temporal safety net is wired, and the dual identity system is bridged.
- **Only remaining item:** Blockoli semantic search (lowest urgency, heavy lift, requires discovery pass).

## Promotion

- [x] axiomregent activation → `specs/033-axiomregent-activation/` (delivered)
- [x] featuregraph scanner fix → `specs/034-featuregraph-registry-scanner-fix/` (delivered)
- [x] Agent governed execution → `specs/035-agent-governed-execution/` (delivered)
- [x] Safety tier governance → `specs/036-safety-tier-governance/` (delivered)
- [x] Cross-platform axiomregent → `specs/037-cross-platform-axiomregent/` (delivered 2026-03-29)
- [x] Titor command wiring → `specs/038-titor-tauri-command-wiring/` (delivered 2026-03-29)
- [x] Feature ID reconciliation (ADR) → `docs/adr/0001-feature-id-reconciliation.md` — accepted
- [x] Feature ID reconciliation (implementation) → `specs/039-feature-id-reconciliation/` — delivered, all 9 tasks complete, reviewed by claude
