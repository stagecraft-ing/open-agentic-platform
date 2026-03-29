# Integration debt (working synthesis)

> **Non-authoritative.** Tracks **felt** debt across boundaries; formal tracking stays in specs and execution artifacts once promoted.

## Context

- Branch: `main`
- Feature 032: complete (T000–T013, verification green)
- Feature 033: draft scaffolded (`specs/033-axiomregent-activation/`)
- Related specs: 000 (constitutional), 003–005 (lifecycle/execution), 029–031 (consumer contracts)

## Debt items (ranked)

| Item | Area | Payoff | Status |
|------|------|--------|--------|
| ~~axiomregent not spawned~~ | Runtime / governance enforcement | Activates governed tool surface | **RESOLVED (Feature 033)** — sidecar spawns at startup, probe port on stderr, UI surfaces status |
| **Agent execution ungoverned** | Execution / trust | Makes `enable_file_read/write/network` flags real; routes through safety tiers | Post-033; needs axiomregent live first |
| **featuregraph scanner reads forbidden `features.yaml`** | Governance / data | Unblocks governance panel from permanent degradation | Feature 034-class; can parallel with 033 |
| **Titor Tauri commands stubbed** | Temporal safety | Enables checkpoint/restore/diff/verify from desktop | Independent; lower priority than 033 |
| **Safety tiers not spec-governed** | Governance process | Formalizes `safety.rs` tier model; makes tier assignments auditable | Can parallel with 033 or absorb into it |
| **Feature ID duality** | Data architecture | Enables cross-referencing registry features (kebab) with code attribution (UPPERCASE) | Design decision needed; no code dependency |
| **Blockoli semantic search stubbed** | Product capability | Enables AI-native code search from desktop | Heavy lift; lowest urgency |
| **No audit trail for agent actions** | Observability / compliance | Required for verifiable governance | Parallel with axiomregent activation |

## Notes

- Items 1–2 are the same integration arc: activate axiomregent (033), then route agents through it (034+).
- `--dangerously-skip-permissions` flag appears 7 times and is the single artifact most at odds with the platform's stated purpose. Removing it requires agent routing through axiomregent.
- **axiomregent binary exists only for `aarch64-apple-darwin`** — Windows/Linux binaries need to be built or the sidecar must degrade gracefully on those platforms. This is a real constraint for Feature 033.

## Promotion

- [x] axiomregent activation → `specs/033-axiomregent-activation/` (scaffolded 2026-03-29)
- [ ] featuregraph scanner fix → `specs/034-featuregraph-registry-scanner-fix/` (not yet created)
- [ ] Safety tier governance → separate spec (not yet created)
