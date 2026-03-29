# Integration debt (working synthesis)

> **Non-authoritative.** Tracks **felt** debt across boundaries; formal tracking stays in specs and execution artifacts once promoted.

## Context

- Branch: `main`
- Features 032–038: complete (all delivered 2026-03-29)
- Slice A (post-035 hardening): complete
- Related specs: 000 (constitutional), 003–005 (lifecycle/execution), 029–031 (consumer contracts), 032–037 (governance stack + cross-platform)

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
| **Feature ID duality** | Data architecture | Enables cross-referencing registry/code features | **ADR proposed** — `docs/adr/0001-feature-id-reconciliation.md`; implementation (Feature 039) pending |
| **Blockoli semantic search stubbed** | Product capability | Enables AI-native code search from desktop | Heavy lift; lowest urgency |

## Notes

- Items 1–7 are now **all resolved** (Features 032–038 + Slice A). The governance stack is complete on macOS arm64, partially extended to Windows, and the temporal safety net is wired.
- **Feature ID duality** (item 8): ADR 0001 proposes kebab `id` + optional `codeAliases` in compiled registry. Scanner/compiler/schema work remains after review.

## Promotion

- [x] axiomregent activation → `specs/033-axiomregent-activation/` (delivered)
- [x] featuregraph scanner fix → `specs/034-featuregraph-registry-scanner-fix/` (delivered)
- [x] Agent governed execution → `specs/035-agent-governed-execution/` (delivered)
- [x] Safety tier governance → `specs/036-safety-tier-governance/` (delivered)
- [x] Cross-platform axiomregent → `specs/037-cross-platform-axiomregent/` (delivered 2026-03-29)
- [x] Titor command wiring → `specs/038-titor-tauri-command-wiring/` (delivered 2026-03-29)
- [x] Feature ID reconciliation (ADR) → `docs/adr/0001-feature-id-reconciliation.md` — Feature 039 spec + implementation pending
