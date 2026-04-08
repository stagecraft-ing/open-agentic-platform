-- Spec 082 Phase 3: Cross-run artifact persistence

-- Link pipeline runs for incremental execution
ALTER TABLE factory_pipelines
  ADD COLUMN previous_pipeline_id UUID REFERENCES factory_pipelines(id);

CREATE INDEX idx_factory_pipelines_previous ON factory_pipelines(previous_pipeline_id);

-- Content-addressable artifact registry
CREATE TABLE factory_artifacts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pipeline_id UUID NOT NULL REFERENCES factory_pipelines(id) ON DELETE CASCADE,
  stage_id VARCHAR(50) NOT NULL,
  artifact_type VARCHAR(100) NOT NULL,
  content_hash VARCHAR(64) NOT NULL,
  storage_path TEXT NOT NULL,
  size_bytes BIGINT NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_factory_artifacts_pipeline ON factory_artifacts(pipeline_id, stage_id);
CREATE INDEX idx_factory_artifacts_hash ON factory_artifacts(content_hash);
