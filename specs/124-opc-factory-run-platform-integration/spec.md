---
id: "124-opc-factory-run-platform-integration"
slug: opc-factory-run-platform-integration
title: OPC Factory-Run Platform Integration
status: draft
implementation: pending
owner: bart
created: "2026-05-01"
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
implements:
  - path: apps/desktop/src-tauri/src/commands/factory.rs
  - path: platform/services/stagecraft/api/factory/runs.ts
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

Added to `platform/services/stagecraft/api/db/schema.ts`:

```ts
factory_runs (
  id           uuid pk default gen_random_uuid(),
  org_id       uuid not null references organizations(id),
  project_id   uuid references projects(id),    // nullable: ad-hoc runs
  triggered_by uuid not null references users(id),
  adapter_id   uuid not null references factory_adapters(id),
  process_id   uuid not null references factory_processes(id),
  status       text not null,    // queued | running | ok | failed | cancelled
  stage_progress jsonb not null default '[]',  // [{stage_id, status, started_at, completed_at}]
  token_spend  jsonb,             // { input, output, total } per stage rolled up
  error        text,
  source_shas  jsonb not null,    // { adapter, process, contracts: {name: sha} }
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
