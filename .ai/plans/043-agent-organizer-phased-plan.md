# 043 Agent Organizer / Meta-Orchestrator — phased implementation plan

> **Non-authoritative.** Planning scratch for agent coordination only. Canonical contract remains `specs/043-agent-organizer/spec.md`.

## Goal

Implement Feature 043: a meta-orchestrator that scores request complexity deterministically, applies mandatory dispatch triggers, assembles agent teams and phased workflows (Haiku for planning), and emits JSON `ExecutionPlan` for downstream execution (Feature 035 / 044).

## Pre-implementation decisions (O-001 to O-004)

- **O-001 (crate placement):** implement core types and deterministic scoring in `crates/agent/` as `plan`, `complexity`, `dispatch`, and `registry` modules (spec Architecture table). Re-export a single `plan_request`-oriented API for Tauri to call.
- **O-002 (registry source):** accept an `AgentRegistrySnapshot` (agent id + capability/description text) injected into `plan()` from the desktop layer (SQLite / 042 provider surface), not a hard-coded list. Unit tests use in-memory fixtures. Satisfies FR-010 by returning `mode: "direct"` + warning when empty or unavailable.
- **O-003 (Haiku boundary):** keep deterministic scoring and trigger evaluation **outside** any LLM call (NF-002). After `mode` is `delegated`, invoke a pluggable `OrganizerPlanner` trait: default production backend calls Haiku via existing bridge/SDK paths; tests use a deterministic stub that fills team/workflow from fixtures.
- **O-004 (JSON contract):** define `ExecutionPlan` with `serde` + JSON schema alignment to the TypeScript interface in the spec; stable field ordering for snapshots optional, but round-trip serde tests are required (NF-003).

## Implementation slices

### Phase 1 — ExecutionPlan + deterministic complexity (FR-002, NF-002, NF-003)

Deliverables:
- Add `crates/agent/src/plan.rs`: `ExecutionPlan`, nested types (`complexity`, `team`, `workflow`, `warnings`), `serde` derives, `request_id` (UUID v4 from caller or generated in API).
- Add `crates/agent/src/complexity.rs`: implement the signal table and caps exactly as in the Architecture section (prompt length, verbs, connectors, tech breadth, scope phrases, file/path hints).
- Export `score_complexity(prompt: &str) -> ComplexityBreakdown` with per-signal numeric contributions for audit (`signals` map).

Validation:
- Unit tests for SC-005 (identical prompts → identical score/breakdown).
- Boundary tests for score bands (25/50/75) and each signal cap.

### Phase 2 — Dispatch protocol (FR-003, FR-004, FR-005, FR-006)

Deliverables:
- Add `crates/agent/src/dispatch.rs`: evaluate mandatory direct vs mandatory delegate substring/pattern lists (spec lists + extensible config); apply overrides before score-based branch; produce `mode`, optional `mandatory_trigger` string, and merged complexity block.
- Map numeric score to `band` enum (`simple` | `moderate` | `complex` | `highly_complex`) per spec table.

Validation:
- SC-001 / SC-002 style fixtures (simple typo vs large multi-domain request).
- SC-003 / SC-004: mandatory delegate always `delegated`; mandatory direct always `direct` even when score disagrees.

### Phase 3 — Registry integration + team size rules (FR-007, FR-010)

Deliverables:
- Add `crates/agent/src/registry.rs`: `AgentRegistrySnapshot`, trait or struct for listing agents with ids + descriptions; `plan()` accepts snapshot + request.
- Deterministic **team cardinality** from band (e.g. moderate 1–2, complex 2–3, highly complex 3–5) without LLM; placeholder agent selection (first-N or keyword match) acceptable only behind `OrganizerPlanner` stub — document that real selection is Phase 4.

Validation:
- FR-010: empty snapshot → `direct` + warning, no panic.
- Team array length respects 1–5 and band rules when registry has enough agents.

### Phase 4 — Haiku planner: team roles + phased workflow (FR-001, FR-008, FR-009)

Deliverables:
- Implement `OrganizerPlanner` production backend: prompt template + Haiku model id; input = request text + `ComplexityBreakdown` + serialized registry entries; output parsed into `team.agents` (roles, justifications, model tier) and `workflow.phases` (ids, depends_on, success_gate, per-phase model).
- Organizer model fixed to `haiku` for the meta-call (FR-009); phase/agent model fields advisory per spec Contract notes.
- Fallback: if LLM unavailable, degrade to stub team + minimal single-phase workflow with warning.

Validation:
- Parse/fail tests for malformed LLM JSON (graceful error or warning).
- SC-006 smoke: planner timeout budget documented (measure in Phase 6).

### Phase 5 — Tauri + desktop wiring

Deliverables:
- New Tauri command (e.g. `plan_request`) in `apps/desktop/src-tauri` per spec table: accepts request string + optional plan context; loads registry snapshot from existing agent DB/registry integration; returns JSON `ExecutionPlan`.
- Thin TypeScript types mirroring Rust or generated from shared JSON schema if already present.

Validation:
- Integration test or manual script path documented in `execution/verification.md`.

### Phase 6 — Verification artifacts + performance (NF-001, SC-008)

Deliverables:
- `specs/043-agent-organizer/execution/verification.md`: list `cargo test` for `agent` crate, any vitest/e2e for Tauri command, and p95 planning note (Haiku latency under 2000-token prompts).

Validation:
- SC-008 satisfied by checked commands and recorded outputs.

## Dependency notes

- **042:** registry content must be readable from the app; reuse IDs/descriptions already stored for provider/agent catalog.
- **035 / 044:** execution consumes `ExecutionPlan`; no execution logic in 043.

## Open risks (from spec)

- R-001 edge cases: mitigated by mandatory triggers in Phase 2.
- R-002 Haiku quality: mitigated by structured prompt + examples in Phase 4.
- R-003 missing registry: FR-010 path in Phase 3.
