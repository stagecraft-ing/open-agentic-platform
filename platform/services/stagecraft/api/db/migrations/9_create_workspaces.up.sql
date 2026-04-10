-- Spec 087 Phase 1: Workspace Foundation
-- The workspace is the unit of identity, governance, collaboration,
-- knowledge intake, and factory execution.

-- ---------------------------------------------------------------------------
-- Workspaces
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS workspaces (
  id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id             UUID NOT NULL REFERENCES organizations(id),
  name               TEXT NOT NULL,
  slug               TEXT NOT NULL,
  object_store_bucket TEXT NOT NULL,  -- S3-compatible bucket for this workspace
  created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(org_id, slug)
);

-- Seed a default workspace for the default org so existing data migrates cleanly.
INSERT INTO workspaces (id, org_id, name, slug, object_store_bucket)
VALUES (
  '00000000-0000-0000-0000-000000000002',
  '00000000-0000-0000-0000-000000000001',
  'Default',
  'default',
  'oap-default-workspace'
);

-- ---------------------------------------------------------------------------
-- Add workspace_id to projects (nullable initially for migration safety)
-- ---------------------------------------------------------------------------

ALTER TABLE projects
  ADD COLUMN IF NOT EXISTS workspace_id UUID REFERENCES workspaces(id);

-- Backfill: assign all existing projects to the default workspace
UPDATE projects
SET workspace_id = '00000000-0000-0000-0000-000000000002'
WHERE workspace_id IS NULL;

-- Now make it NOT NULL
ALTER TABLE projects ALTER COLUMN workspace_id SET NOT NULL;

-- Update unique constraint: projects are unique per workspace, not per org
-- Drop old constraint first, then add new one
ALTER TABLE projects DROP CONSTRAINT IF EXISTS projects_org_id_slug_key;
ALTER TABLE projects ADD CONSTRAINT projects_workspace_id_slug_key UNIQUE (workspace_id, slug);
