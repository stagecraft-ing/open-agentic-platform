-- Spec 097: Promotion-Grade Platform Mirror — promotions table

CREATE TYPE promotion_status AS ENUM ('promoted', 'revoked');

CREATE TABLE promotions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL,
    pipeline_id UUID NOT NULL,
    workflow_id TEXT NOT NULL,
    status promotion_status NOT NULL DEFAULT 'promoted',
    promoted_by TEXT,
    evidence JSONB NOT NULL DEFAULT '{}',
    promoted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_promotions_workspace ON promotions(workspace_id);
CREATE INDEX idx_promotions_pipeline ON promotions(pipeline_id);
