# Handoff Prompt — Implement Spec 123

Paste this entire prompt into a fresh Claude Code session at the repo root (`/Users/bart/Dev2/open-agentic-platform`). The session will execute spec 123 phase-by-phase, committing after each phase, until the spec frontmatter is flipped to `status: approved` / `implementation: complete`.

---

# BEGIN HANDOFF

You are implementing spec **`123-agent-catalog-org-rescope`** in the `open-agentic-platform` repository. This is an autonomous, multi-phase implementation. Follow the orchestrator behavioral rules in `.claude/rules/orchestrator-rules.md` exactly.

## Mandatory first action

Run `/init` to load session context (memory, codebase index, spec registry, recent activity, agent and command list). The init protocol is defined in `AGENTS.md`.

## What you are implementing

Spec 123 moves the agent catalog from project scope (set by spec 119) back to org scope. Projects become catalog *consumers* via a new `project_agent_bindings` join table that pins one org agent at one immutable version per project — **no per-binding override of the agent definition**. Spec 123 amends spec 119 narrowly (the agent-scoping decision only); the rest of 119's workspace→project collapse stands.

Read these three files in full before starting Phase 0:

1. `specs/123-agent-catalog-org-rescope/spec.md` — the contract you are implementing
2. `specs/123-agent-catalog-org-rescope/plan.md` — the sequencing rationale, decisions, and risks
3. `specs/123-agent-catalog-org-rescope/tasks.md` — the per-phase task list with per-task identifiers (T001, T010, …) and commit messages

Also read these references as needed (do not pre-load all):

- `specs/119-project-as-unit-of-governance/spec.md` — what is being amended
- `specs/111-org-agent-catalog-sync/spec.md` — the original org-level design this restores
- `.claude/rules/orchestrator-rules.md` — the six rules you must obey
- `.claude/rules/governed-artifact-reads.md` — how to read `build/**/*.json` (only via consumer binaries)

## How to execute

Work through `tasks.md` **phase by phase, in order**. There are eight phases:

- Phase 0 — Foundations
- Phase 1 — Schema migration
- Phase 2 — Stagecraft API rewrite
- Phase 3 — Duplex envelopes
- Phase 4 — Stagecraft web org Agents surface
- Phase 5 — Stagecraft web project Agents tab rewrite
- Phase 6 — OPC desktop cache rebind
- Phase 7 — Factory engine `agent_resolver`
- Phase 8 — Closure (spec 119 amendment + status flip)

For each phase:

1. **Read the phase tasks** in `tasks.md`. Identify which tasks are `[P]` (can run in parallel) versus sequential.
2. **Use specialised agents** when appropriate:
   - `architect` — when you need to plan a phase's internal sequencing or validate an approach against the spec.
   - `explorer` — when you need to trace existing code (e.g. all callers of `verifyProjectInOrg` before Phase 2's API rewrite).
   - `implementer` — for focused code changes once the plan for the phase is clear.
   - `reviewer` — after a phase's code is written, before the checkpoint commit.
   - `encore-expert` — for Encore.ts framework questions in `platform/services/stagecraft/`.
3. **Execute each task**, marking it complete in your task tracker as you go (use the `TaskCreate` / `TaskUpdate` tool).
4. **Hit the phase checkpoint**:
   - `npm run typecheck` (and `npm run build` where the phase touches web) in `platform/services/stagecraft` and `platform/services/stagecraft/web` as applicable.
   - `cargo check` at the workspace root (and `cargo test` for crates touched in Phases 6–7).
   - The phase-specific tests called out in `tasks.md` must pass.
   - **No lint warnings, no type errors, no failing tests.**
5. **Commit** with the phase's commit message from `tasks.md` (look for "Commit message:" at the end of each phase's checkpoint section).
6. **Move to the next phase.**

## Halt-on-failure rule

If any checkpoint fails — typecheck error, test failure, lint warning, missing dependency, ambiguous spec interpretation, or any unexpected state — **stop immediately**. Surface the error in detail (file paths, line numbers, exact failing command output) and ask the user how to proceed. Do not silently work around the problem. Do not skip the phase. Do not merge phases.

## Specific gotchas

- **Migration `30` is destructive.** Phase 1 drops `project_id` columns and dedupes catalog rows. Test the migration against a fresh dev DB first (Tasks T019, T020). Do not run it against any database the user might care about without explicit confirmation.
- **Cross-project name divergence aborts the migration.** Task T015 detects same-name agents with different `content_hash` across projects in one org. The migration MUST abort, not silently win. If it aborts in your dev DB, surface the conflict list and stop.
- **Envelope schema bump is a clean break.** Phase 3 bumps `agent.catalog.*` from `v: 1` to `v: 2`. There is no compat path. Compile-time `const` enforcement (Task T044) is the gate.
- **Project agent endpoint repurpose.** `GET /api/projects/:projectId/agents` changes from "list agents" to "list bindings". Phase 2 (T032) deletes the old shape entirely. Internal callers must be migrated in the same PR (T033).
- **Spec 119 amendment edits land in Phase 8, not Phase 0.** Do not touch `specs/119-project-as-unit-of-governance/spec.md` until the closure phase.
- **The frontmatter flip is the LAST thing.** Spec 123's `status: draft → approved` and `implementation: pending → complete` happens in Task T091, after all earlier phases are committed and the acceptance criteria (A-1..A-11) are individually verified.
- **Run governed reads through consumer binaries.** Per `.claude/rules/governed-artifact-reads.md`, you MUST NOT parse `build/**/*.json` with `python`, `jq`, `awk`, or `sed`. Use `./tools/registry-consumer/target/release/registry-consumer` and `./tools/codebase-indexer/target/release/codebase-indexer`.

## Acceptance criteria

The spec defines A-1 through A-11 in §11. Phase 8 (Tasks T094, T095, T096) verifies each one with a concrete artifact (test path, grep output, registry query, etc.). Do not flip the spec to `approved` until every acceptance criterion has a verifiable green check.

## Tool and permission posture

- You may freely run local read-only commands (`git status`, `cargo check`, `npm run typecheck`, `./tools/.../target/release/...`).
- You may freely apply the migration to the **local dev** Postgres (the one stagecraft uses in `make dev-platform`).
- You may NOT push to remote, force-push, drop production tables, or run any destructive operation against shared infrastructure.
- For risky operations the user did not explicitly authorize (anything beyond local dev), pause and ask.

## What to do at the end

When Phase 8 is complete and all acceptance criteria are green:

1. Confirm spec 123 frontmatter shows `status: approved` and `implementation: complete`.
2. Confirm spec 119 frontmatter shows `amended: "2026-05-01"` and `amendment_record: "123"`.
3. Confirm `make ci` is green.
4. Print a final summary: total commits made, total tasks completed, any open questions you had to resolve, and any acceptance-criterion artifacts the user should review (e.g. screenshots of the new top-nav surface).
5. Stop. Do not open a PR — the user will review the branch and decide.

## What you must NOT do

- Do not enter plan mode. The command is the plan; `tasks.md` is the execution checklist.
- Do not skip, reorder, or merge phases.
- Do not proceed past a failing checkpoint.
- Do not modify the spec mid-flight to make a task easier. If the spec is wrong, halt and surface it.
- Do not amend prior commits. Each phase produces a new commit.
- Do not push to remote.
- Do not flip `status: approved` until every acceptance criterion is verified.

## You can ask the user

- For clarification on an open question (OQ-1..OQ-5 in spec §12) when it blocks a task.
- For a decision when a task discovers a precondition the spec did not anticipate (e.g. an existing dev DB state that breaks dedup).
- For permission for any operation outside local dev (remote push, shared infra, prod-like data).

You can NOT ask the user to make routine implementation decisions that the spec already covers. If the spec covers it, follow the spec.

## Begin

Start by running `/init`. Then read the three spec files. Then begin Phase 0.

# END HANDOFF
