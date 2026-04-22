-- Spec 111 Phase 1: Org-managed Agent Catalog.
--
-- Makes the authoritative agent catalog a stagecraft workspace entity. Each
-- agent is a (name, version) pair scoped to one workspace. Drafts mutate in
-- place (bumping only content_hash); publication bumps `version` monotonically
-- per (workspace_id, name). Retirement is a status flip, not a delete — the
-- audit trail outlives every row it describes.
--
-- Phase 1 scope: CRUD only. Duplex envelopes that push published/retired state
-- to connected OPCs land in Phase 3 (see spec 111 §7 Rollout).

-- ---------------------------------------------------------------------------
-- agent_catalog: authoritative per-workspace agent definitions.
-- ---------------------------------------------------------------------------
--
-- name is a kebab-case catalog key unique per workspace across all versions
-- and statuses. `UNIQUE (workspace_id, name, version)` pins the version
-- monotonicity invariant at the DB boundary.
--
-- frontmatter holds the serialised UnifiedFrontmatter (spec 054). The
-- shared TS mirror (Phase 2) round-trips against this column.
--
-- content_hash is sha-256 over the canonical JSON of frontmatter + body;
-- the editor uses it for optimistic locking and the duplex snapshot
-- directory (Phase 3) uses it to drive lazy body fetches.

CREATE TABLE agent_catalog (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    version         INTEGER NOT NULL DEFAULT 1,
    status          TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'published', 'retired')),
    frontmatter     JSONB NOT NULL,
    body_markdown   TEXT NOT NULL,
    content_hash    TEXT NOT NULL,
    created_by      UUID NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, name, version)
);

CREATE INDEX agent_catalog_ws_name_idx ON agent_catalog (workspace_id, name);
CREATE INDEX agent_catalog_ws_status_idx ON agent_catalog (workspace_id, status);

-- ---------------------------------------------------------------------------
-- agent_catalog_audit: versioned audit of catalog mutations.
-- ---------------------------------------------------------------------------
--
-- `before`/`after` capture the full row snapshot (minus body_markdown, which
-- can be reconstructed from the catalog history when needed) so that replay,
-- diffing, and compliance review do not require a cross-query join against
-- `agent_catalog` at the time of audit.

CREATE TABLE agent_catalog_audit (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id        UUID NOT NULL REFERENCES agent_catalog(id) ON DELETE CASCADE,
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    action          TEXT NOT NULL
        CHECK (action IN ('create', 'edit', 'publish', 'retire', 'fork')),
    actor_user_id   UUID NOT NULL REFERENCES users(id),
    before          JSONB,
    after           JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX agent_catalog_audit_agent_idx ON agent_catalog_audit (agent_id);
CREATE INDEX agent_catalog_audit_workspace_idx ON agent_catalog_audit (workspace_id, created_at DESC);
