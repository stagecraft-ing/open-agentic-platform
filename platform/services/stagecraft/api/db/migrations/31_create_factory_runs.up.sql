-- Migration 31 — Spec 124: factory_runs table.
--
-- Closes the spec 108 §7.4 deferral by persisting OPC factory runs on the
-- platform side (one row per run; lifecycle driven by duplex events from
-- the desktop). Migration ordering: spec 123's `30_agent_catalog_org_rescope`
-- MUST have applied first — `factory_runs` indirectly depends on the
-- rescoped `agent_catalog.org_id` column for the `source_shas.agents[]`
-- triple captured at reservation time. The pre-flight guard below aborts
-- loud if that column is missing.

-- ===========================================================================
-- 0. Migration ordering guard (T013).
--    Aborts before any DDL if spec 123's migration 30 has not run. The
--    failure surface is structured (RAISE EXCEPTION with a clear message)
--    rather than a column-missing error during INSERT later.
-- ===========================================================================

DO $migration_31_preflight$
DECLARE
    has_org_id BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1
          FROM information_schema.columns
         WHERE table_name = 'agent_catalog'
           AND column_name = 'org_id'
    ) INTO has_org_id;

    IF NOT has_org_id THEN
        RAISE EXCEPTION
            'Migration 31 aborted: agent_catalog.org_id is missing — spec 123 migration 30 (agent_catalog_org_rescope) has not been applied. Run migration 30 first; spec 124 depends on it for source_shas.agents[] (org_agent_id, version, content_hash) triples.';
    END IF;
END $migration_31_preflight$;

-- ===========================================================================
-- 1. factory_runs table (T010).
--    Spec 124 §3 — one row per OPC factory run. Mutations after creation
--    flow exclusively via the duplex bus (`factory.run.*` envelopes); the
--    only mutation REST surface is `POST /api/factory/runs` (reservation),
--    keyed by `client_run_id` for idempotent retries.
-- ===========================================================================

CREATE TABLE factory_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Org ownership (joined to authenticated identity on every API/duplex
    -- read; cross-org reads are rejected — spec 124 §3 §6).
    org_id UUID NOT NULL REFERENCES organizations(id),
    -- Optional project binding. NULL for ad-hoc runs (no project_agent_bindings
    -- consulted; ResolvedAgent.content_hash taken directly from the catalog
    -- row at reservation time per spec 124 §3). ON DELETE CASCADE so that
    -- deleting a project tombstones its run history rather than blocking
    -- the deletion (T016 expects this).
    project_id UUID REFERENCES projects(id) ON DELETE CASCADE,
    -- User who triggered the run (or initiated cancellation; the sweeper
    -- writes failed rows under the spec 119 system user).
    triggered_by UUID NOT NULL REFERENCES users(id),
    -- Adapter / process resolved at reservation time. Reference the row id,
    -- NOT the (org, name) tuple — these tables are spec 108 / 109 owned and
    -- their (org, name) constraints might shift over time.
    adapter_id UUID NOT NULL REFERENCES factory_adapters(id),
    process_id UUID NOT NULL REFERENCES factory_processes(id),
    -- Idempotency key supplied by the desktop on `POST /api/factory/runs`.
    -- Repeated POSTs with the same `(org_id, client_run_id)` pair return
    -- the existing row (spec 124 §4, T021 / T023). NOT NULL so the
    -- partial unique index below has every row eligible.
    client_run_id TEXT NOT NULL,
    status TEXT NOT NULL,
    -- Per-stage progress trail. Each entry:
    --   { stage_id: string, status: 'running'|'ok'|'failed'|'skipped',
    --     started_at: string, completed_at?: string,
    --     agent_ref?: { org_agent_id, version, content_hash } }
    -- Appended by the duplex stage_started handler; updated in place by
    -- stage_completed. Out-of-order delivery synthesises an entry rather
    -- than fail (T032).
    stage_progress JSONB NOT NULL DEFAULT '[]',
    -- Token spend rolled up across stages on terminal `factory.run.completed`.
    -- Shape: { input: number, output: number, total: number }.
    token_spend JSONB,
    -- Free-form error message on terminal `factory.run.failed`. The sweeper
    -- writes a structured "sweeper: no events for <duration>" string.
    error TEXT,
    -- Content-addressed identity of inputs at reservation time. Shape:
    --   { adapter: sha, process: sha, contracts: { name: sha, ... },
    --     agents: [{ org_agent_id, version, content_hash }, ...] }
    -- The agents[] triple is the spec 122 Stage CD comparator key and is
    -- mandatory — even ad-hoc runs without a project_id populate it from
    -- the catalog row's content_hash directly.
    source_shas JSONB NOT NULL,
    -- Wall-clock from the reservation POST; `completed_at` is the desktop
    -- emit time on terminal events. The sweeper's staleness window is
    -- relative to `last_event_at`, not `started_at`, so a long-running
    -- stage that emits regularly is not falsely failed.
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    -- Updated on every duplex event for this run. Drives the sweeper's
    -- staleness query (T061). Defaults to `now()` so a freshly-reserved
    -- row has a well-defined last-event marker before any stage event
    -- arrives.
    last_event_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ===========================================================================
-- 2. status CHECK (T011).
-- ===========================================================================

ALTER TABLE factory_runs
    ADD CONSTRAINT factory_runs_status_check
    CHECK (status IN ('queued', 'running', 'ok', 'failed', 'cancelled'));

-- ===========================================================================
-- 3. Indexes (T012).
--    Run-history view: `(org_id, started_at DESC)` for the default Runs
--    tab listing.
--    In-flight view: partial index on status IN ('queued','running') for
--    the live updates path.
--    Idempotency: partial unique on `(org_id, client_run_id)` so the
--    reservation POST can use ON CONFLICT semantics.
-- ===========================================================================

CREATE INDEX factory_runs_org_started_at_idx
    ON factory_runs (org_id, started_at DESC);

CREATE INDEX factory_runs_org_in_flight_idx
    ON factory_runs (org_id)
    WHERE status IN ('queued', 'running');

CREATE UNIQUE INDEX factory_runs_org_client_run_id_uniq
    ON factory_runs (org_id, client_run_id);

-- Sweeper query: `WHERE status IN ('queued','running') AND last_event_at < cutoff`.
-- The partial in-flight index above already narrows the candidate set; a
-- secondary `(last_event_at)` index would help only if scans against the
-- partial set become hot enough to matter — re-evaluate with metrics.
