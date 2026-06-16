-- Spec 213 FR-006: project_artifacts, the record of a successfully
-- published OCI image for a project commit. Populated by the GitHub
-- webhook on `workflow_run.completed` (workflow `oap-build`, conclusion
-- `success`) for repos present in `project_repos`. A missed event degrades
-- to the deterministic ref derivation (FR-005), never to a hard failure.
-- This is the deploy path's first lookup for "what image does this commit
-- have?" (spec 213 User Story 3).
CREATE TABLE project_artifacts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id      UUID NOT NULL,
    release_sha     TEXT NOT NULL,
    -- `root` for single-variant trees; `public`/`internal` for dual-profile
    -- trees (spec 214 FR-009).
    variant         TEXT NOT NULL DEFAULT 'root'
        CONSTRAINT project_artifacts_variant_chk
        CHECK (variant IN ('root', 'public', 'internal')),
    image_ref       TEXT NOT NULL,
    workflow_run_id BIGINT,
    built_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Idempotent upsert key (FR-006; re-run / two-build race on the same SHA
    -- upserts rather than duplicating).
    CONSTRAINT project_artifacts_project_sha_variant_uniq
        UNIQUE (project_id, release_sha, variant)
);

-- Deploy-path lookup: known images for a (project, sha) across variants.
CREATE INDEX idx_project_artifacts_project_sha
    ON project_artifacts (project_id, release_sha);
