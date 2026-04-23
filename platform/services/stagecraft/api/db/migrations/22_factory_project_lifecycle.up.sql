-- Spec 112 Phase 5: Factory Project Lifecycle — Stagecraft Create.
--
-- Adds two pieces of state:
--
-- 1. `projects.factory_adapter_id` — nullable link from a project to the
--    adapter it was created/imported with. Nullable because non-factory
--    projects predate this spec; newly-created factory projects MUST set
--    it (enforced at the API layer, not the DB, to keep backwards
--    compatibility).
--
-- 2. `scaffold_jobs` — durable, concurrency-safe replacement for the
--    retired `template-distributor` service's in-memory job map. Every
--    Create request inserts a row, which carries the lifecycle state of
--    the server-side scaffold work (clone → run adapter entry point →
--    push). Retained for audit even after success.

ALTER TABLE projects
    ADD COLUMN IF NOT EXISTS factory_adapter_id UUID REFERENCES factory_adapters(id);

CREATE INDEX IF NOT EXISTS idx_projects_factory_adapter_id
    ON projects(factory_adapter_id);

CREATE TABLE IF NOT EXISTS scaffold_jobs (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id           UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    workspace_id     UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    project_id       UUID REFERENCES projects(id) ON DELETE SET NULL,
    factory_adapter_id UUID NOT NULL REFERENCES factory_adapters(id),
    requested_by     UUID NOT NULL REFERENCES users(id),
    variant          TEXT NOT NULL,
    profile_name     TEXT,
    status           TEXT NOT NULL DEFAULT 'pending',
    step             TEXT,
    error_message    TEXT,
    github_org       TEXT,
    repo_name        TEXT,
    clone_url        TEXT,
    commit_sha       TEXT,
    metadata         JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_scaffold_jobs_workspace_status
    ON scaffold_jobs(workspace_id, status);

CREATE INDEX IF NOT EXISTS idx_scaffold_jobs_project
    ON scaffold_jobs(project_id);
