# Integration risks (working notes)

> **Non-authoritative.** Formal risk acceptance belongs in spec/execution records after review.

## Purpose

List **cross-cutting** failure modes: version skew, contract drift, optional vs required deps, MCP/sidecar availability, and test gaps.

## Canonical references (read first)

- `specs/032-opc-inspect-governance-wiring-mvp/spec.md`, `execution/verification.md`
- Consumer contract specs `029`–`031` if touching registry surfaces

## Risks

| Risk | Likelihood | Impact | Mitigation / owner | Evidence |
|------|------------|--------|---------------------|----------|
| **Governance display without enforcement** — agent permission flags stored/shown but never enforced; `--dangerously-skip-permissions` hardcoded | Certain (current state) | High (trust gap) | Post-032: activate axiomregent + route agents through governed dispatch | `agents.rs:774`, `claude.rs:969` |
| **featuregraph scanner always fails** — reads `spec/features.yaml` which doesn't exist and is forbidden by Feature 000 | Certain (current state) | Medium (governance panel degraded, not broken) | Post-032: adapt scanner to read `registry.json` or markdown | `scanner.rs:167` |
| **Feature ID duality** — kebab spec IDs vs UPPERCASE code IDs, no mapping | Certain (current state) | Low now, compounds over time | Post-032: reconciliation spec | `registry.schema.json`, `scanner.rs:34` |
| **T010 could accidentally depend on broken featuregraph** — if action uses `featuregraph_impact`, results will be partial | Medium (depends on T010 choice) | Low (degraded, not crash) | Choose "Open spec" action (option A) which uses registry data only | See `open-questions.md` Q1 |
| **Ephemeral MCP per gitctx call** — spawn/kill per request, 10s timeout | Low (acceptable for current usage) | Low | Move to sidecar pattern if frequency increases | `mcp.rs:17,121-184` |
| **No structured audit trail** — agent actions streamed for UI, not persisted to structured store | Certain (current state) | Medium (post-032) | Build audit log when axiomregent activates | Output goes to `ProcessHandle.live_output` in memory |
| **Test coverage gaps on new governance path** — `analysis.rs` has unit tests for `read_registry_summary` but no integration test for full `featuregraph_overview` command | Medium | Low (bounded to governance panel) | T012 should add integration test with temp registry file | `analysis.rs:138-174` |

## 032-specific risks (T010–T013)

| Risk | Mitigation |
|------|-----------|
| T010 action scope creep — adding too much action surface | Stick to "Open spec file" — zero backend, uses existing tab infrastructure |
| T011 docs could go stale quickly | Keep minimal; reference spec and changeset rather than duplicating |
| T012/T013 verification misses degraded-state documentation | Explicitly record featuregraph "unavailable" as expected in verification.md |

## Candidate promotions

- [ ] `execution/changeset.md` — note that featuregraph degradation is bounded and expected for 032 MVP
- [ ] `execution/verification.md` — add governance backend test command and expected degraded state documentation
