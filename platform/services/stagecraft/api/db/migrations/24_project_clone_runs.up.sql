-- Spec 114 — Async Project Clone Pipeline.
--
-- Splits spec 113's `POST /api/projects/:id/clone` into a thin sync
-- queue step + a PubSub worker. The run row holds the full lifecycle
-- so the dialog can poll until terminal state, and so a redelivered
-- worker message can recognise an already-claimed run and reclaim
-- partial state instead of producing a second clone.

CREATE TYPE project_clone_run_status AS ENUM (
    'pending', 'running', 'ok', 'failed'
);

CREATE TABLE project_clone_runs (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_project_id     UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    workspace_id          UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    org_id                UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    triggered_by          UUID NOT NULL REFERENCES users(id),
    status                project_clone_run_status NOT NULL DEFAULT 'pending',
    requested_name        TEXT,
    requested_slug        TEXT,
    requested_repo_name   TEXT,
    final_name            TEXT,
    final_slug            TEXT,
    final_repo_name       TEXT,
    default_branch        TEXT,
    dest_repo_full_name   TEXT,
    project_id            UUID REFERENCES projects(id) ON DELETE SET NULL,
    opc_deep_link         TEXT,
    raw_artifacts_copied  INTEGER,
    raw_artifacts_skipped INTEGER,
    duration_ms           INTEGER,
    error                 TEXT,
    queued_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at            TIMESTAMPTZ,
    completed_at          TIMESTAMPTZ
);

CREATE INDEX idx_project_clone_runs_workspace_queued
    ON project_clone_runs (workspace_id, queued_at DESC);
