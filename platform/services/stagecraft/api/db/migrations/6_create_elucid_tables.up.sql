-- Elucid pipeline lifecycle tables (spec 077)

-- Pipeline status enum
CREATE TYPE elucid_pipeline_status AS ENUM (
  'initialized', 'running', 'paused', 'completed', 'failed'
);

-- Stage status enum
CREATE TYPE elucid_stage_status AS ENUM (
  'pending', 'in_progress', 'completed', 'confirmed', 'rejected'
);

-- Scaffold feature status enum
CREATE TYPE elucid_scaffold_status AS ENUM (
  'pending', 'in_progress', 'completed', 'failed'
);

-- Scaffold feature category enum
CREATE TYPE elucid_scaffold_category AS ENUM (
  'data', 'api', 'ui', 'configure', 'trim', 'validate'
);

-- Elucid pipeline tracking
CREATE TABLE elucid_pipelines (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  adapter_name VARCHAR(100) NOT NULL,
  status elucid_pipeline_status NOT NULL DEFAULT 'initialized',
  policy_bundle_id UUID,
  build_spec_hash VARCHAR(64),
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_elucid_pipelines_project ON elucid_pipelines(project_id);

-- Business document references
CREATE TABLE elucid_business_docs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pipeline_id UUID NOT NULL REFERENCES elucid_pipelines(id) ON DELETE CASCADE,
  name VARCHAR(255) NOT NULL,
  storage_ref TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_elucid_business_docs_pipeline ON elucid_business_docs(pipeline_id);

-- Stage progress tracking (synced from OPC)
CREATE TABLE elucid_stages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pipeline_id UUID NOT NULL REFERENCES elucid_pipelines(id) ON DELETE CASCADE,
  stage_id VARCHAR(50) NOT NULL,
  status elucid_stage_status NOT NULL DEFAULT 'pending',
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  confirmed_by VARCHAR(255),
  confirmed_at TIMESTAMPTZ,
  rejected_by VARCHAR(255),
  rejected_at TIMESTAMPTZ,
  rejection_feedback TEXT,
  prompt_tokens INTEGER DEFAULT 0,
  completion_tokens INTEGER DEFAULT 0,
  model VARCHAR(50),
  UNIQUE(pipeline_id, stage_id)
);
CREATE INDEX idx_elucid_stages_pipeline ON elucid_stages(pipeline_id);

-- Scaffolding feature tracking
CREATE TABLE elucid_scaffold_features (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pipeline_id UUID NOT NULL REFERENCES elucid_pipelines(id) ON DELETE CASCADE,
  feature_id VARCHAR(100) NOT NULL,
  category elucid_scaffold_category NOT NULL,
  status elucid_scaffold_status NOT NULL DEFAULT 'pending',
  retry_count INTEGER DEFAULT 0,
  last_error TEXT,
  files_created TEXT[],
  prompt_tokens INTEGER DEFAULT 0,
  completion_tokens INTEGER DEFAULT 0,
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  UNIQUE(pipeline_id, feature_id)
);
CREATE INDEX idx_elucid_scaffold_pipeline ON elucid_scaffold_features(pipeline_id);

-- Immutable audit log
CREATE TABLE elucid_audit_log (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pipeline_id UUID NOT NULL REFERENCES elucid_pipelines(id) ON DELETE CASCADE,
  timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  event VARCHAR(50) NOT NULL,
  actor VARCHAR(255),
  stage_id VARCHAR(50),
  feature_id VARCHAR(100),
  details JSONB NOT NULL DEFAULT '{}'
);
CREATE INDEX idx_elucid_audit_pipeline ON elucid_audit_log(pipeline_id, timestamp);

-- Policy bundles
CREATE TABLE elucid_policy_bundles (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  adapter_name VARCHAR(100) NOT NULL,
  rules JSONB NOT NULL,
  compiled_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_elucid_policy_bundles_project ON elucid_policy_bundles(project_id);
