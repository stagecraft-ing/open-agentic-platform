> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Keep **Feature 032** (OPC inspect + governance wiring MVP) moving toward completion on `main`: close remaining slices (follow-up action, docs, verification) without diluting spec-first governance. Use this file to pass baton between tools; **promote** durable outcomes into `specs/032-opc-inspect-governance-wiring-mvp/` and its `execution/` artifacts.

## Canonical feature authority

- **Spec:** `specs/032-opc-inspect-governance-wiring-mvp/spec.md`
- **Tasks / status:** `specs/032-opc-inspect-governance-wiring-mvp/tasks.md`
- **Plan:** `specs/032-opc-inspect-governance-wiring-mvp/plan.md`
- **Execution:** `specs/032-opc-inspect-governance-wiring-mvp/execution/changeset.md`, `execution/verification.md`

Do not treat this handoff file as a second task system — follow **`tasks.md`** for ordered work.

## Current execution truth

- Feature 032 is **in progress** per `tasks.md`: inspect, git, and governance wiring slices are done (T000–T009); **T010–T013** (action path, docs, targeted/full verification) remain open.
- Registry-consumer contract work (**029–031**) is prerequisite substrate; avoid weakening settled consumer contracts while wiring OPC inspect.
- Claude has completed deep analysis pass: all `.ai/findings/` and `.ai/reviews/claude-review.md` populated with source-grounded evidence.

## Files touched

- `.ai/findings/runtime-path.md` — full verified runtime path with evidence references
- `.ai/findings/authority-map.md` — enforced vs displayed governance matrix
- `.ai/findings/under-integrated-assets.md` — ranked integration opportunities with effort estimates
- `.ai/findings/open-questions.md` — T010 action recommendation, verification commands, resolved questions
- `.ai/findings/integration-risks.md` — 7 ranked risks with mitigation paths
- `.ai/reviews/claude-review.md` — concerns, resolved items, recommended next moves
- `.ai/plans/next-slice.md` — T010–T013 implementation sketch for Cursor
- `.ai/plans/promotion-candidates.md` — items ready to move to canonical artifacts
- `.ai/plans/integration-debt.md` — ranked post-032 debt with suggested order

## What is working

- Spec-first layout with per-feature `spec.md`, `tasks.md`, `plan.md`, and `execution/` artifacts.
- Feature 032 end-to-end inspect journey: xray scan → git context (native + MCP enrichment) → governance panel (registry OK, featuregraph gracefully degraded).
- Governance panel correctly handles unavailable data with bounded degraded states (FR-003 satisfied).

## What is stubbed / broken

- **featuregraph half of governance panel always returns "unavailable"** — `Scanner::scan()` reads `spec/features.yaml` which doesn't exist. This is bounded and expected for 032 MVP.
- **Remaining stubs**: titor commands (5 `todo!()`), blockoli commands (2 `todo!()`), axiomregent not spawned. All post-032 scope.

## Decisions made

- Adopt `.ai/` as **non-authoritative** collaboration surface.
- **T010 action: "View spec" button** — opens `specs/{id}/spec.md` in a `claude-md` tab using registry `specPath` field. Zero backend work, no `features.yaml` dependency. See `findings/open-questions.md` Q1 for full rationale.
- featuregraph degraded state is **expected and documented** for 032 MVP (FR-003 explicitly allows it).
- Post-032 priorities: axiomregent activation → agent permission enforcement → scanner fix → titor wiring.

## Open questions

- Should T010 also include a secondary "Check impact" action alongside "View spec"? Recommend: no, keep T010 minimal; impact check depends on featuregraph which is degraded.
- What test framework is available in `apps/desktop/` for T012? Cursor should check for vitest/jest config.

## Baton

- Current owner: **cursor**
- Next owner: chatgpt (for synthesis/prioritization after 032 closes) or claude (if implementation questions arise)
- Last baton update: 2026-03-28 — Claude deep analysis complete; baton passed to Cursor for T010–T013 implementation
- Requested outputs from Cursor:
  1. **T010**: Implement "View spec" action per `plans/next-slice.md` — create `actions.ts`, update `InspectSurface.tsx`, add test
  2. **T011**: Update `apps/desktop/README.md` + `execution/changeset.md`
  3. **T012**: Add targeted tests for inspect/git/governance/action surfaces
  4. **T013**: Run verification suite (commands in `findings/open-questions.md` Q3), record in `execution/verification.md`
  5. Update `tasks.md` checkboxes as each task completes
- Recommended files to read:
  - `.ai/handoff/current.md` (this file)
  - `.ai/plans/next-slice.md` (implementation sketch)
  - `.ai/findings/open-questions.md` (T010 rationale, verification commands)
  - `.ai/findings/runtime-path.md` (what's real vs stubbed)
  - `specs/032-opc-inspect-governance-wiring-mvp/spec.md`, `tasks.md`
  - `apps/desktop/src/features/inspect/InspectSurface.tsx` (T010 target)
  - `apps/desktop/src-tauri/src/commands/analysis.rs` (governance backend)

## Requested next agent output

Cursor: implement T010–T013 per the sketch in `plans/next-slice.md`. For each task, update `tasks.md` checkbox and add a verification entry. When all four are done, update `execution/changeset.md` with the final PR reference and run the verification suite. Then commit, push, and hand baton to **chatgpt** for post-032 synthesis.

## Promotion candidates for canonical artifacts

- T010 implementation → code + test
- T013 verification results → `execution/verification.md`
- Post-032 integration debt → future feature specs (axiomregent activation, safety tiers, feature ID reconciliation)

---

## Recent outputs

- 2026-03-28 (claude): `.ai/findings/runtime-path.md`, `.ai/findings/authority-map.md`, `.ai/findings/under-integrated-assets.md`, `.ai/findings/open-questions.md`, `.ai/findings/integration-risks.md`, `.ai/reviews/claude-review.md`, `.ai/plans/next-slice.md`, `.ai/plans/promotion-candidates.md`, `.ai/plans/integration-debt.md`
