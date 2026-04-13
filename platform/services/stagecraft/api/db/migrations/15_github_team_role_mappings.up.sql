-- Spec 080 Phase 3 (FR-009): GitHub Team to OAP Role Mapping
CREATE TYPE target_scope AS ENUM ('org', 'project');

CREATE TABLE github_team_role_mappings (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id          UUID NOT NULL REFERENCES organizations(id),
  github_team_slug TEXT NOT NULL,
  github_team_id  BIGINT NOT NULL,
  target_scope    target_scope NOT NULL,
  target_id       UUID,            -- NULL for org-level, project_id for project-level
  role            TEXT NOT NULL,    -- platform_role or project_member_role
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Unique constraint for project-scoped mappings (target_id IS NOT NULL)
CREATE UNIQUE INDEX idx_team_mappings_project_unique
  ON github_team_role_mappings(org_id, github_team_slug, target_scope, target_id)
  WHERE target_id IS NOT NULL;

-- Unique constraint for org-scoped mappings (target_id IS NULL)
CREATE UNIQUE INDEX idx_team_mappings_org_unique
  ON github_team_role_mappings(org_id, github_team_slug, target_scope)
  WHERE target_id IS NULL;

CREATE INDEX idx_team_mappings_org ON github_team_role_mappings(org_id);
CREATE INDEX idx_team_mappings_org_team ON github_team_role_mappings(org_id, github_team_slug);
