-- Spec 109: Factory PAT Broker + PubSub Sync Worker.
--
-- Adds three tables:
--   * factory_upstream_pats — one PAT per org, authenticates the Factory
--     sync worker against the configured factory_source / template_source
--     repositories. Takes precedence over the org's github_installations
--     row when present.
--   * project_github_pats — one PAT per project, authenticates project_repos
--     operations against repos that live outside the platform's GitHub org
--     (installation tokens can't reach those).
--   * factory_sync_runs — lifecycle row per sync trigger. Feeds the new
--     PubSub sync worker: the HTTP endpoint inserts a 'pending' row and
--     publishes an event; the subscription transitions it through
--     running → ok|failed.
--
-- Both PAT tables store ciphertext + nonce produced by api/auth/patCrypto.ts
-- (AES-256-GCM, PAT_ENCRYPTION_KEY Encore secret). Revoke is hard-delete —
-- unlike user_github_pats these are operational credentials, not identity
-- artifacts, so no audit history row is retained on the table itself (it
-- goes to audit_log).

CREATE TABLE factory_upstream_pats (
    org_id          UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
    token_enc       BYTEA NOT NULL,
    token_nonce     BYTEA NOT NULL,
    token_prefix    TEXT NOT NULL,
    scopes          TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    is_fine_grained BOOLEAN NOT NULL DEFAULT FALSE,
    github_login    TEXT,
    last_used_at    TIMESTAMPTZ,
    last_checked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by      UUID NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE project_github_pats (
    project_id      UUID PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    token_enc       BYTEA NOT NULL,
    token_nonce     BYTEA NOT NULL,
    token_prefix    TEXT NOT NULL,
    scopes          TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    is_fine_grained BOOLEAN NOT NULL DEFAULT FALSE,
    github_login    TEXT,
    last_used_at    TIMESTAMPTZ,
    last_checked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by      UUID NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TYPE factory_sync_run_status AS ENUM (
    'pending', 'running', 'ok', 'failed'
);

CREATE TABLE factory_sync_runs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    status          factory_sync_run_status NOT NULL DEFAULT 'pending',
    triggered_by    UUID NOT NULL REFERENCES users(id),
    factory_sha     TEXT,
    template_sha    TEXT,
    counts          JSONB,
    error           TEXT,
    queued_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ
);

CREATE INDEX idx_factory_sync_runs_org_queued
    ON factory_sync_runs (org_id, queued_at DESC);
