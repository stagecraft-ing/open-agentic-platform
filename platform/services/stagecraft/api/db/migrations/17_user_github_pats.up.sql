-- User GitHub Personal Access Tokens (spec 106 FR-005/FR-006).
--
-- Spec 106 makes PAT the documented fallback when the GitHub App installation
-- token cannot resolve a user's org memberships (e.g. the org refuses to
-- install the stagecraft app). Tokens are encrypted at rest with AES-256-GCM
-- using a per-row nonce; the key lives in the PAT_ENCRYPTION_KEY secret.
-- A partial unique index enforces at most one active PAT per user; revoked
-- rows are kept for audit.

CREATE TABLE user_github_pats (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_enc        BYTEA NOT NULL,
    token_nonce      BYTEA NOT NULL,
    token_prefix     TEXT NOT NULL,
    scopes           TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    is_fine_grained  BOOLEAN NOT NULL DEFAULT FALSE,
    last_used_at     TIMESTAMPTZ,
    last_checked_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at       TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_user_github_pats_active_one_per_user
    ON user_github_pats(user_id)
    WHERE revoked_at IS NULL;

CREATE INDEX idx_user_github_pats_user
    ON user_github_pats(user_id);

CREATE INDEX idx_user_github_pats_last_checked
    ON user_github_pats(last_checked_at)
    WHERE revoked_at IS NULL;
