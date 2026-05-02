---
id: "124-opc-factory-run-platform-integration"
slug: opc-factory-run-platform-integration
title: OPC Factory-Run Platform Integration
status: approved
implementation: complete
owner: bart
created: "2026-05-01"
approved: "2026-05-01"
risk: high
summary: >
  Closes the spec 108 §7.1 punt and §7.4 deferral. OPC stops reading
  `factory/` from a developer's local checkout and instead pulls adapter
  manifests, contract schemas, and process definitions from the platform's
  `/api/factory/*` endpoints; runs are persisted as `factory_runs` rows and
  streamed back over the existing duplex channel so a run can be observed
  from the web UI in real time. Replaces the current
  `resolve_factory_root()` walk-up-from-CARGO_MANIFEST_DIR with an
  authenticated platform fetch + on-disk run cache.
depends_on:
  - "087"  # unified-workspace-architecture (duplex substrate)
  - "106"  # rauthy-native-oidc-and-membership (desktop OIDC)
  - "107"  # rauthy-client-redirect-convergence
  - "108"  # factory-as-platform-feature (provides /api/factory/*)
  - "109"  # factory-pat-and-pubsub-sync (the platform-side sync model)
  - "123"  # agent-catalog-org-rescope (agent_resolver + binding-aware run identity)
implements:
  - path: apps/desktop/src-tauri/src/commands/factory.rs
  - path: platform/services/stagecraft/api/factory/runs.ts
  - path: platform/services/stagecraft/api/db/migrations/31_create_factory_runs.up.sql
---

# 124 — OPC Factory-Run Platform Integration

## 1. Problem Statement

Spec 108 made Factory a first-class platform feature: adapters, contracts,
and processes live in `factory_adapters`, `factory_contracts`,
`factory_processes`, surfaced over `/api/factory/*`. The §8 deletion of the
in-tree `factory/` directory landed on 2026-05-01.

Two pieces of OPC's local execution path were explicitly **deferred** by
spec 108 and remain unaddressed:

1. **§7.1 punt.** `apps/desktop/src-tauri/src/commands/factory.rs` still
   resolves `factory_root` by walking up from `CARGO_MANIFEST_DIR` to find
   an in-tree `factory/adapters/`. After §8 that path no longer exists in
   the repo; the desktop's factory-run command therefore requires a
   developer to keep a separate clone of `goa-software-factory` somewhere
   on disk. This is brittle, undocumented, and inverts the spec 108
   ownership story — OPC executes runs but the platform is supposed to
   be the source of truth for the definitions.

2. **§7.4 deferral.** Spec 108 §7 declares OPC will stream run events
   back over `api/sync/duplex.ts` into a new `factory_runs` table.
   Neither the table nor the streaming wiring shipped. Today an OPC
   factory run produces local `state.json` files only; the platform UI
   has no view of an in-flight or completed run, and the `/app/factory`
   tab cannot show "what's been run" from the org's vantage point.

The mental model from spec 108 only closes once both of these are done:
*platform is the orchestration tier, OPC is the execution tier, and the
contract between them is `/api/factory/*` (definitions in) +
`factory_runs` (results out)*.

## 2. Decision

Two coupled deliverables, sized to land in one spec because the persistence
layer (factory_runs) is meaningful only when paired with the API-fetch
migration that produces the runs.

1. **Replace `resolve_factory_root()` with an authenticated platform fetch.**
   OPC's factory-run command authenticates against stagecraft using the
   existing desktop OIDC flow (specs 106/107), GETs the adapter / contract /
   process bodies it needs for the run, materialises them into a per-run
   cache directory, and points the existing `factory-engine` crate at that
   cache via its already-configurable `factory_root`. No change to
   `factory-engine`'s API surface; only the materialisation source
   changes. The legacy `resolve_factory_root` in-tree fallback is removed.

2. **Persist runs in `factory_runs` and stream over the duplex channel.**
   A new `factory_runs` table on the platform side captures one row per
   run with status, started_by, project_id (optional), adapter, process,
   stage progress (`{stage_id, status, started_at, completed_at}[]`),
   token spend, and an error column. OPC publishes lifecycle events
   (`factory.run.queued`, `factory.run.stage_started`,
   `factory.run.stage_completed`, `factory.run.completed`,
   `factory.run.failed`) over `api/sync/duplex.ts`; a stagecraft handler
   maps each event to a row update so the web UI's Factory tab can show
   live progress and run history without OPC needing a separate webhook
   surface.

The adapter / contract / process payloads themselves are content-addressed
by `source_sha`; OPC keeps a small content-addressed cache under
`$XDG_CACHE_HOME/oap-factory/<sha>/` so repeat runs against the same
definitions don't refetch.

## 3. Data Model

Added to `platform/services/stagecraft/api/db/schema.ts` via migration
`31_create_factory_runs.up.sql`. Spec 123 reserves migration `30` for
`agent_catalog_org_rescope`; this spec MUST take `31` and MUST NOT
re-use a slot below it.

```ts
factory_runs (
  id           uuid pk default gen_random_uuid(),
  org_id       uuid not null references organizations(id),
  project_id   uuid references projects(id),    // nullable: ad-hoc runs
  triggered_by uuid not null references users(id),
  adapter_id   uuid not null references factory_adapters(id),
  process_id   uuid not null references factory_processes(id),
  status       text not null,    // queued | running | ok | failed | cancelled
  stage_progress jsonb not null default '[]',  // [{stage_id, status, started_at, completed_at, agent_ref?}]
  token_spend  jsonb,             // { input, output, total } per stage rolled up
  error        text,
  source_shas  jsonb not null,    // { adapter, process, contracts:{name:sha}, agents:[{org_agent_id, version, content_hash}] }
  started_at   timestamptz not null default now(),
  completed_at timestamptz,
  created_at   timestamptz not null default now()
)
```

Indexes on `(org_id, started_at desc)` for the run-history view and
`(org_id, status) where status in ('queued', 'running')` for the
in-flight view. All rows are org-scoped; the duplex handler validates
that the OPC connection's auth identity matches `org_id` before
accepting an update.

`source_shas.agents[]` carries the spec-123 `(org_agent_id, version,
content_hash)` triple per stage so a Stage CD comparator (spec 122) run
across two projects can attest whether the same agent definition
participated. `pinned_content_hash` is taken from
`project_agent_bindings` at run-start time (spec 123 §4.4 invariant
I-B2); for ad-hoc runs without a project binding, the resolver records
the catalog row's `content_hash` directly.

## 4. Encore APIs

New endpoints under `api/factory/runs.ts`:

| Method | Path | Purpose |
|--------|------|---------|
| GET    | `/api/factory/runs` | List recent runs for the caller's org (limit 50, queryparam `status?`). |
| GET    | `/api/factory/runs/:id` | Single run with full stage progress. |
| POST   | `/api/factory/runs` | Reserve a run id (returns `{run_id, source_shas}`). Called by OPC right before it starts cloning artefacts so the row exists before any stage event arrives. Idempotent: passing an existing `client_run_id` returns the existing row. |

Read endpoints require `requireUser`; the create endpoint requires the
caller to belong to the org. No mutation surface exists for editing
stage state — that path is duplex-only.

Duplex events handled by `api/sync/duplex.ts`:

| Event | Effect on factory_runs |
|-------|------------------------|
| `factory.run.stage_started` | Append `{stage_id, status: "running", started_at}` to `stage_progress`; flip `status` to `running`. |
| `factory.run.stage_completed` | Update the matching `stage_progress` entry's `status` and `completed_at`. |
| `factory.run.completed` | Set `status = 'ok'`, `completed_at`, `token_spend`. |
| `factory.run.failed` | Set `status = 'failed'`, `completed_at`, `error`, partial `stage_progress`. |
| `factory.run.cancelled` | Same shape as failed but `status = 'cancelled'`, no `error` required. |

Each event is keyed by the `run_id` from the POST `/runs` reservation;
the handler rejects events for runs the caller does not own.

### 4.1 Agent definitions are out of band

`/api/factory/*` returns adapter / contract / process bodies. Agents,
per spec 123, live in `agent_catalog` and reach OPC via the duplex
catalog cache (spec 123 §7.1 envelopes), not via this spec's API. OPC's
materialiser MUST resolve agent references through
`crates/factory-engine/src/agent_resolver.rs` (spec 123 §8.2) against
the desktop's local catalog cache + the active project's
`project_agent_bindings`. The resolver returns the
`(content_hash, body_markdown, frontmatter)` triple for each
stage-referenced agent; OPC writes the resolved content into the
per-run cache root under `process/agents/<role>.md` so the in-tree
shape `factory-engine` already expects (spec 108 §6 §7) keeps working.

If an agent reference cannot be resolved (binding missing, retired
upstream with no replacement) the run is rejected client-side BEFORE
reservation. No `factory_runs` row is created in that case; the desktop
shows the resolver error to the user with a deep-link to the project's
binding management UI (spec 123 §6.2).

## 5. OPC Migration — `commands/factory.rs`

Today (post-spec-108):

```rust
fn resolve_factory_root() -> Result<PathBuf, String> {
    // walks up from CARGO_MANIFEST_DIR — broken without an in-tree checkout.
}
```

After spec 124:

```rust
async fn materialise_run_root(
    api: &PlatformClient,
    adapter_name: &str,
    process_name: &str,
) -> Result<RunRoot, FactoryError>;
```

`materialise_run_root` returns a `RunRoot { path, source_shas }` where
`path` is a per-run temp directory hydrated from the platform API:

```
$XDG_CACHE_HOME/oap-factory/sha-<short>/
├── adapters/<adapter_name>/manifest.yaml
├── adapters/<adapter_name>/agents/...
├── adapters/<adapter_name>/patterns/...
├── process/agents/...
├── process/stages/...
└── contract/<name>.schema.json
```

The directory layout matches the legacy in-tree shape so
`factory-engine` and `factory-contracts` keep working unchanged. The
on-disk cache is content-addressed by the `source_shas` returned from
the API (one sha per artefact), so identical inputs hit a warm cache
and skip the fetch.

`resolve_factory_root` is **deleted**, not retained as a fallback. The
`// TODO(spec-108-§7-punt)` marker is removed at the same time.

## 6. Run Reservation + Streaming

Order of operations for an OPC factory run:

1. User picks adapter + process + (optional) project in the desktop UI.
2. OPC calls `POST /api/factory/runs` with `{adapter_name, process_name,
   client_run_id}`; receives `{run_id, source_shas}`.
3. OPC materialises the cache root under `oap-factory/<source_shas.run>/`.
4. OPC starts `FactoryEngine::new({factory_root: cache_root, ...})` and
   begins the pipeline.
5. For each stage: emits `factory.run.stage_started`, runs, emits
   `factory.run.stage_completed`. Events are queued locally and replayed
   if the duplex connection drops; the platform-side handler is
   idempotent against duplicates by `(run_id, stage_id, status)` tuple.
6. On terminal state: emits `factory.run.completed` or
   `factory.run.failed`. The platform row's `completed_at` matches OPC's
   wall clock at emission time.

Crash semantics: if OPC dies mid-run, the row remains in `running` until a
sweeper marks it `failed` after a configurable timeout (default: max stage
duration × 2). The sweeper lives in `api/factory/runsScheduler.ts`,
modelled on the existing `extraction-staleness-sweeper` cron (spec 115).

### 6.1 Duplex envelope versioning

`factory.run.*` envelopes are independent of spec 123's `agent.catalog.*`
(`v: 2`) and `project.agent_binding.*` (`v: 1`) envelopes; they ride the
same duplex bus but carry their own compile-time schema constant
starting at `v: 1`:

```ts
interface FactoryRunStageStarted {
  v: 1;
  kind: "factory.run.stage_started";
  event_id: string;
  org_id: string;
  run_id: string;
  stage_id: string;
  agent_ref: { org_agent_id: string; version: number; content_hash: string };
  started_at: string;
}
// ... stage_completed, completed, failed, cancelled mirror this shape
```

`api/sync/duplex.ts` registers the `factory.run.*` envelope kinds
alongside the spec-123 catalog/binding kinds. The desktop and platform
honour the embedded-schema-version convention (compile-time const,
build-error on mismatch) the same way spec 123 §7.3 describes.

## 7. UI Surface

Adds two views to `/app/factory`:

- **Runs** — a new tab listing the org's recent `factory_runs`,
  defaulting to the last 14 days. Filterable by status and adapter.
- **Run detail** — drawer / route opened from a row, showing per-stage
  progress, token spend, and any error. Live-updates via the duplex
  stream while `status in ('queued', 'running')`.

Phase 1 (this spec) ships the data path and a minimal Runs tab + detail
drawer. Detailed stage-level visualisation (artifact previews,
diff viewers) is out of scope and will land alongside the existing
StageCdReview surface in a later spec if required.

## 8. Non-Goals

- Editing or replaying runs from the web UI. Runs are immutable once
  recorded; a re-run produces a new row.
- Cross-org visibility of runs. Even read-only.
- OPC-side "claim" semantics for resuming runs across desktop hosts. A
  run is owned by the desktop instance that created it; if that instance
  dies, the run is failed by the sweeper.
- Versioning of `factory_runs` rows (no soft-delete, no schema lineage
  tracking beyond the `source_shas` snapshot).

## 9. Open Questions

- Should the per-run cache be torn down on success, or kept for
  post-mortem inspection? Default: keep, with a configurable retention
  window (`OAP_FACTORY_CACHE_RETAIN_DAYS`, default 7).
- Do ad-hoc runs (no `project_id`) appear in the project-scoped UI?
  Default: no — they only show in the org-level Runs tab.
- Does this spec subsume the Phase 1 content of a future
  "factory observability" spec, or does that work get a separate spec?
  Leaning towards subsume; revisit if event volume forces separation.

## 10. Acceptance

A-1. `apps/desktop/src-tauri/src/commands/factory.rs` no longer references
     `resolve_factory_root`; the `// TODO(spec-108-§7-punt)` marker is
     removed.
A-2. `rg "factory/(adapters|contracts|process|upstream-map)" apps/ crates/`
     returns only the test-fixture probes already documented under spec 108
     §7.
A-3. `factory_runs` migration applied; the four duplex event handlers
     covered by integration tests against a real Postgres.
A-4. A factory run started from the desktop produces a `factory_runs` row
     visible at `/app/factory` (Runs tab) before the run's first stage
     completes.
A-5. The schema-parity check (spec 125) treats `factory_runs` as a normal
     stagecraft table — no special-casing.
A-6. Spec 108 §7.1 and §7.4 are updated to reference 124 rather than
     "deferred to a follow-up spec".
A-7. Migration is named `31_create_factory_runs.up.sql` and applies
     cleanly on top of spec 123's migration `30`.
A-8. `factory_runs.source_shas.agents[]` carries the spec-123 triple
     per stage; a Stage CD comparator integration test verifies two
     runs of the same pipeline against two projects record identical
     `agents[].content_hash` values.
A-9. The desktop's `materialise_run_root` materialises agent bodies by
     calling `agent_resolver` (spec 123 §8.2) — `rg "agent_catalog"
     apps/desktop/src-tauri/src/commands/factory.rs` returns zero hits
     because the resolver is consumed via the factory-engine crate, not
     directly.

## 11. Implementation Notes

Shipped 2026-05-01 across nine commits. Per-phase summary:

- **Phase 0 — Foundations** (`81ccb33`): `factory.run.*` envelope kinds with
  compile-time `FACTORY_RUN_ENVELOPE_VERSION = 1`; audit-action strings
  (`factory.run.{reserved,completed,failed,cancelled,swept}`); pure
  cache-root helper (`cache_root_for(source_shas)`); platform-client
  scaffold; OIDC token-getter wrapper.
- **Phase 1 — Schema migration** (`2b23212`): migration
  `31_create_factory_runs.up.sql`, FK `CASCADE` on `project_id` and
  `RESTRICT` on adapter/process; partial index on
  `(org_id) WHERE status IN ('queued','running')`; SQL `DO $$` ordering
  guard that aborts loud with a referenced error message
  (`Migration 31 aborted: agent_catalog.org_id is missing — spec 123
  migration 30 ... has not been applied`) when migration 30 is missing;
  Drizzle definitions mirror the SQL.
- **Phase 2 — Stagecraft API** (`0e00b3a`):
  `POST /api/factory/runs` reservation + `GET /api/factory/runs` list +
  `GET /api/factory/runs/:id` detail; idempotency on
  `(org_id, client_run_id)`; pure agent-refs walker
  (`runAgentRefs.ts`) with unit tests; integration tests against a real
  Postgres for happy path, idempotent replay, hot-loop concurrency,
  retired-binding rejection, foreign-org reject, list pagination /
  filtering, invalid cursor.
- **Phase 3 — Duplex handlers** (`0f99f80`): five
  `factory.run.*` envelope kinds registered on `api/sync/duplex.ts`;
  idempotent on `(run_id, stage_id, status)`; out-of-order delivery
  tolerated (synthesised `stage_started` if `stage_completed` arrives
  first); foreign-org and envelope-version mismatches rejected before
  any DB write; integration test suite covers in-order, out-of-order,
  duplicate, and reject paths.
- **Phase 4 — `factory-platform-client` crate** (`6b98bc3`): new
  `crates/factory-platform-client`; `PlatformClient` implements both
  the typed REST surface and spec 123's `CatalogClient` trait so a
  single instance threads into `AgentResolver`; warm/cold cache via
  atomic-rename so partial materialisation can never leave a half-built
  cache visible; cross-checks the resolver's
  `(org_agent_id, version, content_hash)` triple against
  `reservation.source_shas.agents[]` and aborts on drift; mock-server
  tests cover both cache paths and partial-failure cleanup.
- **Phase 5 — OPC migration** (`1fc8498`): `commands/factory.rs`
  rewritten to call `materialise_run_root`; `resolve_factory_root`
  (and the `// TODO(spec-108-§7-punt)` marker) deleted entirely; new
  `commands/factory_platform.rs` (886 lines) carries the platform
  pipeline logic; local replay queue at
  `$XDG_DATA_HOME/oap/factory-run-events/<run_id>.ndjson` capped at
  1000 events with terminal-fail-out on overflow; `list_factory_runs`
  command rebound to `GET /api/factory/runs`.
- **Phase 6 — Sweeper** (`8af0515`): `api/factory/runsScheduler.ts`
  cron (`factory-runs-staleness-sweeper`, 60 s); flips `(queued,
  running)` rows older than `STAGECRAFT_FACTORY_RUN_STALE_AFTER_SEC`
  (default 1800 s = 30 min) to `failed` with
  `error = 'sweeper: no events for <duration>'` and a
  `factory.run.swept` audit row; tests cover stale-running, fresh-row
  noop, and stale-queued cases.
- **Phase 7 — Runs UI** (`7abee42`): `/app/factory/runs` (list) and
  `/app/factory/runs/:runId` (detail) routes; loaders read
  `lib/factory-api.server.ts` helpers (`listFactoryRuns`,
  `getFactoryRun`); status / adapter / date-range filters; cursor
  pagination on `started_at`; per-stage progress with
  `agent_ref` short-hash hover; live-updates via 3 s polling while
  status is `(queued, running)`; vitest fixture renders detail with
  mocked event stream.
- **Phase 8 — Closure** (this commit): A-1..A-9 verified; cross-project
  comparator test (A-8) added to `runs.test.ts` as a single-test
  extension; spec frontmatter flipped to `approved` + `complete`;
  registry + codebase index refreshed.

### Decisions and follow-ups

- **Two distinct AgentReference enums.** `factory_engine::agent_resolver`
  and `factory_contracts::agent_reference` each carry their own enum
  shape; Phase 5 imports the engine variant as `EngineAgentReference`
  to disambiguate. Worth a future cleanup spec to collapse to one
  canonical type — neither owner of this duplication is changing
  shape in the near term.
- **`XDG_CACHE_HOME` race in `cache_root` tests.** Pre-existing race
  surfaced once the new tests started writing under the same cache
  root in parallel; fixed in-place with a `Mutex` around
  `XDG_CACHE_HOME`-mutating tests rather than rewriting the helper.
- **Phase 7 date-range filter is client-side.** The chosen option in
  the user's session-3 review; the loader pulls a 14-day window and
  the React component filters in-memory. Easy to flip to a server-side
  `started_at` query if/when the active-runs window grows.
- **Live updates via 3 s polling, not WebSocket.** The merge reducer
  in `app.factory.runs.$runId.tsx` keeps the data path swappable; the
  duplex bus already carries every `factory.run.*` envelope, so a
  future spec can replace the polling with a duplex subscription
  without touching the view tree.
- **A-5 carry-over.** `make ci-schema-parity` fails on
  `extractionOutput.ts is missing exports` — a regression in the
  knowledge-extraction Zod surface owned by spec 125. `factory_runs`
  is treated as a normal stagecraft table by the parity walker (no
  special-casing); the carry-over is informational and does not block
  spec 124 closure.
- **A-4 (live desktop → web smoke) deferred to user smoke.** The full
  Encore + Postgres + desktop stack is non-trivial to spin up inside
  a closure session; manual steps are documented in the Phase 7
  checkpoint and should be exercised before merging the branch.
