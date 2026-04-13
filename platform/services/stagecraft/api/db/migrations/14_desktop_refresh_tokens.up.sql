-- Desktop refresh tokens for OPC PKCE auth flow (spec 080 Phase 1)
CREATE TABLE desktop_refresh_tokens (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash      TEXT NOT NULL UNIQUE,
    user_id         UUID NOT NULL REFERENCES users(id),
    org_id          UUID NOT NULL REFERENCES organizations(id),
    workspace_id    TEXT NOT NULL DEFAULT '',
    org_slug        TEXT NOT NULL DEFAULT '',
    github_login    TEXT NOT NULL,
    platform_role   TEXT NOT NULL DEFAULT 'member',
    rauthy_user_id  TEXT NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_desktop_refresh_tokens_user ON desktop_refresh_tokens(user_id);
CREATE INDEX idx_desktop_refresh_tokens_expires ON desktop_refresh_tokens(expires_at);
