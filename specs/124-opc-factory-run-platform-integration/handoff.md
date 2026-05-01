# Handoff Prompt — Implement Spec 124

Paste this entire prompt into a fresh Claude Code session at the repo root (`/Users/bart/Dev2/open-agentic-platform`). The session will execute spec 124 phase-by-phase, committing after each phase, until the spec frontmatter is flipped to `status: approved` / `implementation: complete`.

---

# BEGIN HANDOFF

You are implementing spec **`124-opc-factory-run-platform-integration`** in the `open-agentic-platform` repository. This is an autonomous, multi-phase implementation. Follow the orchestrator behavioral rules in `.claude/rules/orchestrator-rules.md` exactly.

## Mandatory first action

Run `/init` to load session context (memory, codebase index, spec registry, recent activity, agent and command list). The init protocol is defined in `AGENTS.md`.

## What you are implementing

Spec 124 closes two punts left by spec 108:

1. **§7.1 — OPC desktop migration off the in-tree `factory/`.** The desktop's `apps/desktop/src-tauri/src/commands/factory.rs` currently calls `resolve_factory_root()` which walks up from `CARGO_MANIFEST_DIR` to find `factory/adapters/`. Spec 108 §8 deleted that directory. Spec 124 replaces the walk-up with an authenticated `/api/factory/*` fetch into a per-run, content-addressed cache directory (`$XDG_CACHE_HOME/oap-factory/<sha>/`). The `factory-engine` crate is untouched — only the materialisation source changes; the on-disk shape is preserved so the engine's `factory_root` config keeps working.

2. **§7.4 — `factory_runs` persistence.** Spec 108 §7 declared OPC would stream run events back over `api/sync/duplex.ts` into a new `factory_runs` table. Neither the table nor the streaming wiring shipped. Spec 124 adds the table on the platform side and `factory.run.*` envelopes on the duplex bus; the desktop's run lifecycle (queued → running → ok/failed/cancelled) becomes visible at `/app/factory/runs` with live updates while a run is in flight.

Agent definitions for each stage flow through spec 123's `agent_resolver` (catalog cache + project bindings), **not** through `/api/factory/*`. This spec's API surfaces adapter / contract / process bodies only. See spec §4.1.

Read these three files in full before starting Phase 0:

1. `specs/124-opc-factory-run-platform-integration/spec.md` — the contract you are implementing
2. `specs/124-opc-factory-run-platform-integration/plan.md` — sequencing rationale, decisions, risks
3. `specs/124-opc-factory-run-platform-integration/tasks.md` — per-phase task list with task IDs (T001, T010, …) and commit messages

Also read these as needed (do not pre-load all):

- `specs/108-factory-as-platform-feature/spec.md` — the spec whose §7.1 / §7.4 punts this closes; §7.1 carries the cross-reference to 124
- `specs/123-agent-catalog-org-rescope/spec.md` — owns `AgentReference`, `AgentResolver`, `ResolvedAgent`, `CatalogClient`; spec 124 consumes all four
- `specs/109-factory-pat-and-pubsub-sync/spec.md` — the platform-side sync model spec 124 mirrors for run state
- `specs/115-knowledge-extraction-pipeline/spec.md` — `extraction-staleness-sweeper` cron is the model for the `factory-runs` sweeper
- `.claude/rules/orchestrator-rules.md` — the six rules you must obey
- `.claude/rules/governed-artifact-reads.md` — how to read `build/**/*.json` (only via consumer binaries)
- `platform/CLAUDE.md` — platform-layer conventions
- `platform/services/stagecraft/CLAUDE.md` — Encore.ts conventions; the framework specialist agent (`encore-expert`) is available

## How to execute

Work through `tasks.md` **phase by phase, in order**. There are nine phases:

- Phase 0 — Foundations (envelope types, audit actions, cache-root helper)
- Phase 1 — Schema migration `31_create_factory_runs.up.sql`
- Phase 2 — Stagecraft API: `/api/factory/runs` reservation + read endpoints
- Phase 3 — Duplex handlers for `factory.run.*` envelopes
- Phase 4 — Platform client crate (`crates/factory-platform-client`)
- Phase 5 — OPC migration: rewrite `commands/factory.rs`, delete `resolve_factory_root`
- Phase 6 — Sweeper for stuck `running` rows
- Phase 7 — UI: Runs tab + detail at `/app/factory/runs`
- Phase 8 — Closure (acceptance verification + status flip)

For each phase:

1. **Read the phase tasks** in `tasks.md`. Identify which tasks are `[P]` (parallel-safe) versus sequential.
2. **Use specialised agents** when appropriate:
   - `architect` — when you need to plan a phase's internal sequencing or validate an approach against the spec
   - `explorer` — when tracing existing code (e.g. all callers of `resolve_factory_root` before Phase 5; how `extraction-staleness-sweeper` is wired before Phase 6)
   - `implementer` — for focused code changes once the phase plan is clear
   - `reviewer` — after a phase's code is written, before the checkpoint commit
   - `encore-expert` — for Encore.ts framework questions in Phases 1–3, 6
3. **Execute each task**, marking it complete in your task tracker as you go (use `TaskCreate` / `TaskUpdate`).
4. **Hit the phase checkpoint:**
   - `cd platform/services/stagecraft && npx tsc --noEmit && npm test` (Phases 1–3, 6, 7)
   - `cargo check` and `cargo test` for any crate touched (Phases 0, 4, 5)
   - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` (Phase 5)
   - `cd platform/services/stagecraft && npm run build:frontend` (Phase 7)
   - The phase-specific tests called out in `tasks.md` must pass
   - **No lint warnings, no type errors, no failing tests.**
5. **Commit** with the phase's commit message from `tasks.md` (look for "Commit:" at the end of each phase's checkpoint).
6. **Move to the next phase.**

## Halt-on-failure rule

If any checkpoint fails — typecheck error, test failure, lint warning, missing dependency, ambiguous spec interpretation, or any unexpected state — **stop immediately**. Surface the error in detail (file paths, line numbers, exact failing command output) and ask the user how to proceed. Do not silently work around the problem. Do not skip the phase. Do not merge phases.

## Specific gotchas

- **Migration ordering.** Spec 123 reserved migration `30`. Spec 124 takes `31_create_factory_runs.up.sql`. Task T013 adds a SQL guard that raises if `agent_catalog.org_id` doesn't exist (i.e. spec 123 hasn't been applied). Do NOT bump the number; the ordering is a hard invariant.
- **Agent definitions are NOT in this spec's API.** §4.1 of the spec is non-negotiable. The materialiser (T043) calls `factory_engine::agent_resolver::AgentResolver::resolve(reference)` for each agent in the process; do NOT add an `/api/factory/agents/*` surface. The spec-123 catalog + bindings is the only path.
- **`AgentReference` and `ResolvedAgent` are spec 123's, not yours.** Import them; don't redeclare. The TS-side projection is `Pick<ResolvedAgent, "org_agent_id" | "version" | "content_hash">` — keep field names exactly aligned (T088 grep gate).
- **Reservation idempotency.** `POST /api/factory/runs` keys on `client_run_id` — repeated calls with the same id MUST return the same `run_id`. Tests in T021 / T023 cover this.
- **Duplex handler is idempotent.** Re-delivery of the same `(run_id, stage_id, status)` tuple is a no-op. Out-of-order delivery (stage_completed before stage_started) appends a synthesised entry rather than fail — at-least-once semantics demand this. Task T036 covers it.
- **Envelope schema-version constant is a build error if mismatched.** `FACTORY_RUN_ENVELOPE_VERSION = 1` lives in shared TS + Rust types; desktop / platform skew is a compile failure, not a runtime parse error. Per the `feedback_schema_compile_time` convention.
- **Local replay queue has a 1000-event cap.** T053. If a run produces more than 1000 buffered events while the duplex link is down, mark the run failed locally and surface to the user; do NOT silently grow the queue.
- **Sweeper bias toward false-positive failure.** Default timeout is `max_stage_duration × 2`. A legitimately-slow run gets marked failed; the user re-runs. Do NOT raise the default to "never fail" — the dead-row case is worse than the re-run case.
- **`source_shas.agents[]` is mandatory, not optional.** Spec 122's Stage CD comparator depends on it. Even ad-hoc runs without a `project_id` populate it (resolver records the catalog row's `content_hash` directly).
- **`resolve_factory_root` deletion is in Phase 5, not earlier.** Phase 5 (T051) deletes the function and the `// TODO(spec-108-§7-punt)` marker together. Earlier phases must not touch it; downstream phases assume it's still there until 5.
- **The Frontmatter flip is the LAST thing.** Spec 124's `status: draft → approved` and `implementation: pending → complete` happens in Task T089, after all earlier phases are committed and the acceptance criteria (A-1..A-9) are individually verified.
- **Run governed reads through consumer binaries.** Per `.claude/rules/governed-artifact-reads.md`, you MUST NOT parse `build/**/*.json` with `python`, `jq`, `awk`, or `sed`. Use `./tools/registry-consumer/target/release/registry-consumer` and `./tools/codebase-indexer/target/release/codebase-indexer`.
- **Stagecraft is npm, not pnpm.** Do not invoke `pnpm` in `platform/services/stagecraft/`; the directory is excluded from the root pnpm workspace.

## Pre-existing CI break (carry-over from spec 108)

`make ci-schema-parity` is currently red on `main` due to commit `b6859d3 fix(stagecraft): hand-roll API validators, drop zod from Encore parse path` removing zod from `extractionOutput.ts`. This is **not** a spec 124 regression and is tracked under draft spec 125. **Do not attempt to fix it as part of spec 124.** Acceptance gate A-5 explicitly notes that ci-schema-parity going green requires spec 125 to land first; T091 (final `make ci`) treats the parity failure as carry-over until 125 is implemented.

## Acceptance criteria

The spec defines A-1 through A-9 in §10. Phase 8 (Tasks T080..T088) verifies each one with a concrete artifact (test path, grep output, registry query, etc.). Do not flip the spec to `approved` until every acceptance criterion has a verifiable green check, with the documented exception for A-5 above.

## Tool and permission posture

- You may freely run local read-only commands (`git status`, `cargo check`, `cargo test`, `npx tsc --noEmit`, `npm test`, `./tools/.../target/release/...`).
- You may freely apply the migration to the **local dev** Postgres (the one stagecraft uses in `make dev-platform`).
- You may NOT push to remote, force-push, drop production tables, or run any destructive operation against shared infrastructure.
- You may NOT modify spec 108, 123, 125, or 126 to make a spec 124 task easier. If a spec referenced by 124 is wrong, halt and surface it.
- For risky operations the user did not explicitly authorize (anything beyond local dev), pause and ask.

## What to do at the end

When Phase 8 is complete and all acceptance criteria are green (with A-5 carry-over noted):

1. Confirm spec 124 frontmatter shows `status: approved` and `implementation: complete`.
2. Confirm spec 108 §7.1 and §7.4 already reference spec 124 (they were updated when 124 was authored — verify, don't re-edit).
3. Confirm `rg "factory/(adapters|contracts|process|upstream-map)" apps/ crates/` returns only the test-fixture skips documented under spec 108 §7. The desktop's `commands/factory.rs` should have no hits.
4. Confirm `rg "resolve_factory_root" apps/desktop` returns zero hits.
5. Run `make registry` — clean.
6. Run `make ci` — every gate green except `ci-schema-parity` (carry-over from spec 125).
7. Print a final summary: total commits made, total tasks completed, any open questions you had to resolve, the per-phase commit SHA range, and any acceptance-criterion artifacts the user should review (e.g. screenshots of the new Runs tab).
8. Stop. Do not open a PR — the user will review the branch and decide.

## What you must NOT do

- Do not enter plan mode. The command is the plan; `tasks.md` is the execution checklist.
- Do not skip, reorder, or merge phases.
- Do not proceed past a failing checkpoint.
- Do not modify the spec mid-flight to make a task easier. If the spec is wrong, halt and surface it.
- Do not amend prior commits. Each phase produces a new commit.
- Do not push to remote.
- Do not flip `status: approved` until every acceptance criterion is verified.
- Do not attempt to fix `ci-schema-parity` — it's spec 125's domain.
- Do not add an `/api/factory/agents/*` endpoint — agents are spec 123's surface.
- Do not redeclare `AgentReference` or `ResolvedAgent` — import them from spec 123's modules.

## You can ask the user

- For clarification on an open question (OQ-1..OQ-3 in spec §9) when it blocks a task.
- For a decision when a task discovers a precondition the spec did not anticipate (e.g. an existing dev DB state that breaks the migration ordering guard).
- For permission for any operation outside local dev (remote push, shared infra, prod-like data).
- For confirmation before committing if any phase's diff exceeds 500 lines (CONST-004 warning policy).

You can NOT ask the user to make routine implementation decisions that the spec already covers. If the spec covers it, follow the spec.

## Spec inter-dependencies (quick reference)

```
spec 108 (factory as platform feature) ─── §7.1 / §7.4 punts ──┐
                                                                │
spec 123 (agent catalog org-rescope) ──── AgentReference,      │
                                          AgentResolver,        ▼
                                          ResolvedAgent,    spec 124
                                          CatalogClient   ────────────┐
                                                                       │
spec 109 (PubSub sync) ──── pattern source for run state mgmt ────────┤
                                                                       │
spec 115 (extraction pipeline) ──── pattern source for sweeper ───────┤
                                                                       │
spec 087 §5.3 (duplex) ──── transport for factory.run.* events ───────┘
```

Spec 125 (schema-parity) and spec 126 (AgentPicker UI) are sibling drafts — they do NOT block spec 124 implementation. Spec 126's primary consumer is spec 124's Phase 7 run-trigger UI; if you reach Phase 7 and find the picker missing, that's expected (the picker can be added later by spec 126).

## Begin

Start by running `/init`. Then read the three spec files (spec, plan, tasks). Then begin Phase 0.

# END HANDOFF
