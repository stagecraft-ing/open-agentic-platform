# Integration debt (working synthesis)

> **Non-authoritative.** Tracks **felt** debt across boundaries; formal tracking stays in specs and execution artifacts once promoted.

## Context

- Branch: `main`
- Features 032–037: complete (all delivered 2026-03-29)
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
| **Titor Tauri commands stubbed** | Temporal safety | Enables checkpoint/restore from desktop | **Feature 038 scaffolded** |
| **Feature ID duality** | Data architecture | Enables cross-referencing registry/code features | Design decision needed; no code dependency |
| **Blockoli semantic search stubbed** | Product capability | Enables AI-native code search from desktop | Heavy lift; lowest urgency |

## Notes

- Items 1–6 are now **all resolved** (Features 032–037 + Slice A). The governance stack is complete on macOS arm64 and partially extended to Windows (binary built, CI for remaining targets).
- **Titor** (item 7) is production-ready in the library crate. Gap is purely Tauri state management + command wiring. One session. Spec scaffolded: `specs/038-titor-tauri-command-wiring/`.
- **Feature ID duality** (item 8) has 13 UPPERCASE code IDs vs 38 kebab spec IDs with no bridge. The scanner alias system exists in legacy YAML but not in compiled registry JSON.

## Promotion

- [x] axiomregent activation → `specs/033-axiomregent-activation/` (delivered)
- [x] featuregraph scanner fix → `specs/034-featuregraph-registry-scanner-fix/` (delivered)
- [x] Agent governed execution → `specs/035-agent-governed-execution/` (delivered)
- [x] Safety tier governance → `specs/036-safety-tier-governance/` (delivered)
- [x] Cross-platform axiomregent → `specs/037-cross-platform-axiomregent/` (delivered 2026-03-29)
- [x] Titor command wiring → `specs/038-titor-tauri-command-wiring/` (scaffolded 2026-03-29)
- [ ] Feature ID reconciliation → ADR needed before spec
