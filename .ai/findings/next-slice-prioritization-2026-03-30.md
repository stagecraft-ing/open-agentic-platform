# Next-Slice Prioritization — 2026-03-30

**Author:** claude-opus
**Context:** All P0 features except 043 are feature-complete. 043 has Phases 1–4 approved (crate-level organizer done), with Phase 5 (Tauri wiring) and Phase 6 (verification) remaining.

---

## P0 Status Summary

| Spec | Title | Status | Remaining |
|------|-------|--------|-----------|
| 042 | Multi-Provider Agent Registry | ✅ Complete | — |
| 043 | Agent Organizer | **Phase 4/6** | Phase 5 (Tauri `plan_request`), Phase 6 (verification) |
| 044 | Multi-Agent Orchestration | ✅ Complete | FR-001 NL decomposition deferred; SC-007 verification doc |
| 045 | Claude Code SDK Bridge | ✅ Complete | — |
| 046 | Context Compaction | ✅ Complete | LOW findings only |
| 047 | Governance Control Plane | ✅ Complete | LOW findings only |

---

## Recommendation 1: Finish 043 Phase 5 + 6 (last P0)

**Priority: Immediate. This is the only incomplete P0.**

Phase 5 scope is small — a single Tauri command (`plan_request`) that:
- Accepts request string + optional `PlanContext`
- Loads `AgentRegistrySnapshot` from the existing agent DB/registry (042 integration surface)
- Calls `plan_with_planner()` or `plan()` and returns `ExecutionPlan` JSON
- Thin TypeScript types mirroring the Rust structs

Phase 6 is documentation only — `execution/verification.md` with test commands and latency notes.

**Estimated effort:** Small. The crate API is stable, the trait hook is ready, and the Tauri command pattern is well-established from 044 (`orchestrate_manifest`) and 046 (`git_last_commit`). Cursor can likely complete both phases in a single pass.

**Decision point for Phase 5:** Whether to wire up a real `HaikuOrganizerPlanner` implementation or ship with `DeterministicOrganizerPlanner` and defer Haiku integration. Recommendation: **ship deterministic first**, add Haiku planner as a follow-up. The trait hook makes this a clean upgrade path. Rationale: avoids blocking on API key management and Haiku availability in the desktop runtime.

---

## Recommendation 2: P1 Triage — Top 3 for Next Wave

With all P0s complete (after 043), the next wave should target P1 specs that **solidify the agent execution pipeline** — turning the crate-level foundations (042 registry, 043 organizer, 044 orchestrator, 047 policy) into a usable end-to-end flow.

### Tier A — High value, clear dependencies on completed P0s

| Rank | Spec | Title | Why next |
|------|------|-------|----------|
| 1 | **048** | Hookify Rule Engine | Builds directly on 047 policy kernel. Turns compile-time policy rules into runtime hooks the desktop app can enforce. Without this, 047's gates exist only in tests. |
| 2 | **054** | Agent Frontmatter Schema | Standardizes how agents declare capabilities, model preferences, and constraints. 042 registry and 043 organizer consume agent metadata — a schema makes the pipeline real instead of stub-based. |
| 3 | **051** | Worktree Agents | Enables parallel isolated agent execution (git worktrees). High user-visible value — multiple agents working simultaneously without conflicts. Builds on 044 orchestrator's multi-agent dispatch. |

### Tier B — Important but less dependent on P0 completion

| Rank | Spec | Title | Why defer |
|------|------|-------|-----------|
| 4 | **049** | Permission System | Needed for multi-user/multi-agent security, but 047's policy kernel already provides gate enforcement. Can layer on after hookify. |
| 5 | **052** | State Persistence | Session continuity across restarts. High UX value but independent of the agent pipeline — can develop in parallel. |
| 6 | **056** | Session Memory | Builds on 046 context compaction. Natural extension but not blocking other work. |

### Tier C — Defer

Specs 050, 053, 055, 057–063 are UI features, schema standards, or advanced capabilities that don't block the core pipeline. Defer until Tier A/B are underway.

---

## Recommended Execution Order

```
1. cursor  → 043 Phase 5 + 6 (finish last P0)
   claude  → review Phase 5, then Phase 6

2. cursor  → 048 planning pass (Hookify Rule Engine)
   claude  → 048 plan review
   [repeat cursor/claude cycle through phases]

3. cursor  → 054 planning pass (Agent Frontmatter Schema)
4. cursor  → 051 planning pass (Worktree Agents)
```

Steps 3–4 can overlap with 048 implementation if cursor capacity allows, since 054 and 051 have minimal code overlap with 048.

---

## Open Items to Resolve

- **044 FR-001 (NL decomposition):** Deferred indefinitely. This is the "natural language → task DAG" feature. Currently, orchestration requires explicit YAML manifests. NL decomposition depends on a reliable LLM decomposition pipeline — revisit after 043's Haiku planner proves the LLM-in-the-loop pattern.
- **044 SC-007 (verification doc):** Minor — cursor should write `specs/044-multi-agent-orchestration/execution/verification.md` as part of the next session housekeeping.
- **Haiku planner for 043:** Deferred from Phase 5. The `OrganizerPlanner` trait is ready. Implement after 048 (hookify) establishes the runtime hook pattern, since the Haiku call needs API key management and error handling that hookify may standardize.
