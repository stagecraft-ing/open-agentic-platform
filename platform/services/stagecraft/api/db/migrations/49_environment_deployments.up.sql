-- Spec 215 FR-003: environment_deployments, stagecraft's durable record of
-- every deploy dispatch (UI trigger or PR webhook), keyed to deployd-api's
-- release id (`rel_<uuid>`). It is the env page's source of truth (FR-004)
-- and the destroy path's lookup (FR-005 replaces the prior constructed-string
-- destroy with a read of release_id from here). Every dispatch path MUST
-- write through this table (the create proxy and the PR webhook).
CREATE TABLE environment_deployments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    environment_id  UUID NOT NULL,
    project_id      UUID NOT NULL,
    -- deployd-api's `rel_<uuid>`; nullable until the create response arrives
    -- (a REQUEST_FAILED row never gets one).
    release_id      TEXT,
    release_sha     TEXT NOT NULL,
    artifact_ref    TEXT NOT NULL,
    -- spec 214 FR-009 deploy unit; the default variant for preview/dev is public.
    variant         TEXT NOT NULL DEFAULT 'public'
        CONSTRAINT environment_deployments_variant_chk
        CHECK (variant IN ('root', 'public', 'internal')),
    status          TEXT NOT NULL
        CONSTRAINT environment_deployments_status_chk
        CHECK (status IN ('REQUESTED', 'PENDING', 'ROLLED_OUT', 'FAILED', 'REQUEST_FAILED', 'DESTROYED')),
    -- live endpoint URLs returned by deployd-api at create (JSON array).
    endpoints       JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- triggering org member's user id, or the literal 'webhook' for PR deploys.
    dispatched_by   TEXT NOT NULL,
    -- verbatim helm / connection diagnostic on FAILED or REQUEST_FAILED.
    diagnostic      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Latest-per-environment lookup (FR-004 env page, FR-008 lazy reconcile).
CREATE INDEX idx_environment_deployments_env_created
    ON environment_deployments (environment_id, created_at DESC);

-- Destroy-path lookup by deployd release id (FR-005).
CREATE INDEX idx_environment_deployments_release_id
    ON environment_deployments (release_id)
    WHERE release_id IS NOT NULL;
