-- Sync run history: tracks each connector sync execution (spec 087 Phase 4)
CREATE TYPE sync_run_status AS ENUM ('running', 'completed', 'failed');

CREATE TABLE sync_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    connector_id UUID NOT NULL REFERENCES source_connectors(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    status sync_run_status NOT NULL DEFAULT 'running',
    objects_created INTEGER NOT NULL DEFAULT 0,
    objects_updated INTEGER NOT NULL DEFAULT 0,
    objects_skipped INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    delta_token TEXT,              -- opaque cursor for incremental sync (e.g. Microsoft Graph delta link)
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_sync_runs_connector ON sync_runs(connector_id);
CREATE INDEX idx_sync_runs_workspace ON sync_runs(workspace_id);
