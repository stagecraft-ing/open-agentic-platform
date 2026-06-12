-- Spec 200 FR-001 — durable scan intent for the substrate override async
-- scanner. One row per accepted `user_body` revision per scanner version;
-- inserted INSIDE the write transaction (deterministic bookkeeping), the
-- PubSub publish happens after commit, and the staleness sweeper re-drives
-- queued rows whose publish was lost.
CREATE TABLE factory_override_scan_runs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL,
    artifact_id     UUID NOT NULL,
    content_hash    TEXT NOT NULL,
    scanner_version TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'queued'
        CONSTRAINT factory_override_scan_runs_status_chk
        CHECK (status IN ('queued', 'running', 'completed', 'failed', 'skipped')),
    -- FR-007 — the model selects between exactly two outcomes. NULL until
    -- a completed run records one.
    verdict         TEXT
        CONSTRAINT factory_override_scan_runs_verdict_chk
        CHECK (verdict IN ('clean', 'flagged')),
    -- Untrusted model output, stored as provenance-attached evidence only —
    -- never parsed into further actions (FR-007).
    rationale       TEXT,
    cost_usd        NUMERIC(10, 6),
    attempts        INTEGER NOT NULL DEFAULT 0,
    -- Structured skip/failure detail ({code, message, modelId,
    -- promptFingerprint, ...}); the audited reason for `skipped` rows.
    detail          JSONB,
    queued_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    running_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    last_event_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Idempotency probe (FR-001): existing queued|running|completed run for
-- (org, artifact, content_hash, scanner_version) within the dedupe window
-- absorbs re-enqueues.
CREATE INDEX idx_override_scan_runs_dedupe
    ON factory_override_scan_runs (org_id, artifact_id, content_hash, scanner_version, queued_at);

-- Staleness sweeper probe: queued rows to re-drive, running rows to fail.
CREATE INDEX idx_override_scan_runs_status_event
    ON factory_override_scan_runs (status, last_event_at);

-- Day-aggregate cost gate (FR-005): committed costs only, org-scoped.
CREATE INDEX idx_override_scan_runs_org_completed
    ON factory_override_scan_runs (org_id, completed_at);

-- Spec 200 FR-008 — scan-outcome audit vocabulary. The migration-46 lesson:
-- the check constraint must widen in the SAME migration that introduces the
-- writers, or every scan-outcome audit INSERT violates
-- `factory_artifact_substrate_audit_action_chk` on a fresh database.
ALTER TABLE factory_artifact_substrate_audit
    DROP CONSTRAINT factory_artifact_substrate_audit_action_chk;
ALTER TABLE factory_artifact_substrate_audit
    ADD CONSTRAINT factory_artifact_substrate_audit_action_chk
        CHECK (action IN (
            'artifact.synced',
            'artifact.retired',
            'artifact.overridden',
            'artifact.override_cleared',
            'artifact.conflict_detected',
            'artifact.conflict_resolved',
            'artifact.forked',
            'artifact.override_gate_rejected',
            'artifact.override_verified',
            'artifact.scan_flagged',
            'artifact.scan_clean',
            'artifact.scan_skipped',
            'artifact.scan_failed'
        ));
