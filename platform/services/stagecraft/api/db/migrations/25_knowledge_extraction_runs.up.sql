-- Spec 115 — Knowledge Extraction Pipeline.
--
-- Replaces the manual click-walk on `imported → extracting → extracted`
-- (knowledge.ts:436-501) with an automatic, agent-aware pipeline. The
-- run row drives a Topic + Subscription worker that claims a pending row,
-- CAS-transitions to running, dispatches to a deterministic or agent
-- extractor by mime type + workspace policy, and writes a typed
-- extraction_output. Mirrors the spec 114 project_clone_runs shape so
-- redelivery, staleness sweeping, and forensic audit all work the same way.
--
-- knowledge_objects also gains last_extraction_error (FR-025): when a
-- run fails the worker reverts the object to `imported` and stamps this
-- column so the dashboard can surface a Retry banner without joining
-- against the run history.

-- ---------------------------------------------------------------------------
-- Enum
-- ---------------------------------------------------------------------------

CREATE TYPE knowledge_extraction_run_status AS ENUM (
    'pending', 'running', 'completed', 'failed', 'abandoned'
);

-- ---------------------------------------------------------------------------
-- knowledge_extraction_runs — one row per extraction attempt
-- ---------------------------------------------------------------------------
--
-- Idempotency key (workspace_id, content_hash, extractor_version) is enforced
-- in application code (extractionCore.ts), not as a SQL UNIQUE constraint —
-- the same key is allowed to recur after a 24h window so retries against the
-- same content with the same extractor version remain possible without
-- reusing a stale run row.
--
-- agent_run, token_spend, error are JSONB so the typed shapes can evolve
-- with extractor versions without further migrations. cost_usd is NUMERIC(10,6)
-- to preserve sub-cent precision against per-million-token Anthropic pricing.

CREATE TABLE knowledge_extraction_runs (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    knowledge_object_id  UUID NOT NULL REFERENCES knowledge_objects(id) ON DELETE CASCADE,
    workspace_id         UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    status               knowledge_extraction_run_status NOT NULL DEFAULT 'pending',
    extractor_kind       TEXT,
    extractor_version    TEXT,
    agent_run            JSONB,
    token_spend          JSONB,
    cost_usd             NUMERIC(10, 6),
    error                JSONB,
    attempts             INTEGER NOT NULL DEFAULT 0,
    queued_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    running_at           TIMESTAMPTZ,
    completed_at         TIMESTAMPTZ,
    duration_ms          INTEGER
);

-- Sweeper picks up stale `running` rows by (workspace_id, status); also
-- powers per-workspace dashboard counters of pending / running / failed runs.
CREATE INDEX idx_knowledge_extraction_runs_workspace_status
    ON knowledge_extraction_runs (workspace_id, status);

-- "Latest run for this object" — list/detail endpoints denormalise this
-- field (FR-030) and need an indexed point lookup.
CREATE INDEX idx_knowledge_extraction_runs_object_queued
    ON knowledge_extraction_runs (knowledge_object_id, queued_at DESC);

-- Day-aggregate cost query (FR-019) — `SUM(cost_usd) WHERE workspace_id = ?
-- AND completed_at >= today_utc_midnight`. Indexed for cheap repeat lookup
-- on every agent extractor pre-flight.
CREATE INDEX idx_knowledge_extraction_runs_workspace_completed
    ON knowledge_extraction_runs (workspace_id, completed_at DESC)
    WHERE completed_at IS NOT NULL;

-- ---------------------------------------------------------------------------
-- knowledge_objects.last_extraction_error (FR-025)
-- ---------------------------------------------------------------------------
--
-- Shape: { code, message, extractorKind, attemptedAt } (validated in TS).
-- NULL means the object has not failed since its last successful extraction
-- (or has never been attempted). The Retry endpoint refuses with `not_failed`
-- when this column is NULL.

ALTER TABLE knowledge_objects
    ADD COLUMN last_extraction_error JSONB;
