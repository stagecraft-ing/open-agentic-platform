-- Spec 094: Unified Artifact Store — workspace scoping and provenance

ALTER TABLE factory_artifacts
  ADD COLUMN workspace_id UUID,
  ADD COLUMN producer_agent VARCHAR(100);

CREATE INDEX idx_factory_artifacts_workspace ON factory_artifacts(workspace_id);
CREATE INDEX idx_factory_artifacts_workspace_hash ON factory_artifacts(workspace_id, content_hash);
