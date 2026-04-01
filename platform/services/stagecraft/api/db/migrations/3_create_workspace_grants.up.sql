-- Seam C: workspace-scoped permission grants for OPC governance.
CREATE TABLE IF NOT EXISTS workspace_grants (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id),
  workspace_id TEXT NOT NULL,
  enable_file_read BOOLEAN NOT NULL DEFAULT true,
  enable_file_write BOOLEAN NOT NULL DEFAULT true,
  enable_network BOOLEAN NOT NULL DEFAULT true,
  max_tier INTEGER NOT NULL DEFAULT 2,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (user_id, workspace_id)
);
