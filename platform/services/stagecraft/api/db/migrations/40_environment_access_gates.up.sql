-- Spec 137 Phase 1 — environment access gate schema.
--
-- Per-environment passwordless OIDC gate descriptors and their email/domain
-- allowlists. Two sibling tables, both anchored to `environments(id)`.
--
-- Decision lineage:
-- - clarifications-resolved.md §Decision 2 — dedicated sibling tables (not
--   JSONB on `environments`); rationale: row-level FKs + per-entry audit
--   shape + efficient indexing on the post-auth allowlist check.
-- - clarifications-resolved.md §Decision 3 — Rauthy 0.35 smoke; the
--   `rauthy_client_ref` column stores the Rauthy client id returned by
--   `POST /auth/v1/clients`. There is no parallel `password_login_enabled`
--   field on the Rauthy client record (Rauthy 0.35 controls this via
--   `flows_enabled`); the gate's password-free property is enforced when
--   stagecraft provisions the client, not by a column here.
-- - `feedback_hetzner_postgres_fips` — cluster Postgres is FIPS-mode and
--   rejects `md5()`. No hashing in this migration; the allowlist uniqueness
--   uses `lower(value)` directly.

BEGIN;

-- ---------------------------------------------------------------------------
-- environment_access_gates
-- ---------------------------------------------------------------------------
-- 1:1 with `environments`. `environment_id` is both PK and FK — exactly one
-- gate descriptor per environment, deleted when the environment is deleted.
--
-- CHECK constraint: an enabled gate MUST have a Rauthy client ref. The ref
-- is allowed NULL when `enabled = false` (admin toggles a future gate's
-- shape before flipping enabled).
CREATE TABLE environment_access_gates (
    environment_id                              UUID PRIMARY KEY
        REFERENCES environments(id) ON DELETE CASCADE,
    enabled                                     BOOLEAN NOT NULL DEFAULT false,
    rauthy_client_ref                           TEXT,
    login_method_magic_link                     BOOLEAN NOT NULL DEFAULT true,
    login_method_federated_provider             TEXT,
    login_method_federated_provider_client_ref  TEXT,
    created_at                                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT environment_access_gates_enabled_requires_ref
        CHECK (enabled = false OR rauthy_client_ref IS NOT NULL),
    CONSTRAINT environment_access_gates_federated_provider_values
        CHECK (
            login_method_federated_provider IS NULL OR
            login_method_federated_provider IN ('google', 'microsoft', 'github', 'generic_oidc')
        ),
    CONSTRAINT environment_access_gates_federated_pair_consistent
        CHECK (
            (login_method_federated_provider IS NULL
             AND login_method_federated_provider_client_ref IS NULL)
            OR
            (login_method_federated_provider IS NOT NULL
             AND login_method_federated_provider_client_ref IS NOT NULL)
        )
);

-- ---------------------------------------------------------------------------
-- environment_access_gate_allowlist_emails
-- ---------------------------------------------------------------------------
-- One row per allowlist entry. `kind` distinguishes literal emails from
-- domain suffixes (e.g. `example.com` matches `*@example.com`).
--
-- Unique on `(environment_id, kind, lower(value))` — case-insensitive
-- without `citext` (citext is FIPS-incompatible per
-- `reference_hetzner_postgres_fips`). The API layer normalises to lowercase
-- before insert/lookup for consistency.
--
-- Indexed for the post-auth callback check: oauth2-proxy will query
-- `(environment_id, kind)` to fetch all entries for an env, then the API
-- layer matches against the post-auth email.
CREATE TABLE environment_access_gate_allowlist_emails (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    environment_id  UUID NOT NULL
        REFERENCES environments(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL,
    value           TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT environment_access_gate_allowlist_emails_kind_values
        CHECK (kind IN ('email', 'domain'))
);

CREATE UNIQUE INDEX environment_access_gate_allowlist_emails_unique
    ON environment_access_gate_allowlist_emails (environment_id, kind, lower(value));

CREATE INDEX environment_access_gate_allowlist_emails_env_idx
    ON environment_access_gate_allowlist_emails (environment_id, kind);

COMMIT;
