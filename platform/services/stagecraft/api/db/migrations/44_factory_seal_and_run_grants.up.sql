-- Spec 198 phase 4 — signing authority (FR-014) + run-grants (FR-005).

-- 1. Admission seal: platform compact-JWS over the admitted composition's
--    content-addressed constituents (envelope hash, adapter manifest hashes,
--    agent digests, scaffold resolutions). `kid` lives in the JWS header.
--    NULL on rows that predate the signing authority or on deployments
--    without keys — such admissions are unsealed and the OPC engine refuses
--    them fail-closed (PD-5; a re-sync re-admits with a seal).
ALTER TABLE factory_admissions ADD COLUMN seal_jws TEXT;
ALTER TABLE factory_admissions ADD COLUMN sealed_at TIMESTAMPTZ;

-- 2. Run-grants — the signed intent capsule realised (ASI01 m5): issued at
--    run start (seq 0) and renewed at every stage boundary (seq 1..N).
--    Refusals are recorded with a reason — goal-shift and revocation
--    refusals are attributable evidence (ASI01 m4/m7), not transient errors.
--    The issued sequence is what the emission countersign reconciles
--    against (FR-014: stagecraft verifies the engine's chain matches the
--    grants it issued before sealing the certificate).
CREATE TABLE factory_run_grants (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL,
    run_id          UUID NOT NULL,
    project_id      UUID,
    goal_id         TEXT NOT NULL,
    capsule_hash    TEXT NOT NULL,
    envelope_hash   TEXT NOT NULL,
    build_spec_hash TEXT,
    seq             INTEGER NOT NULL,
    status          TEXT NOT NULL,
    refused_reason  TEXT,
    grant_jws       TEXT,
    kid             TEXT,
    issued_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ,

    CONSTRAINT factory_run_grants_status_chk
        CHECK (status IN ('issued', 'refused'))
);

CREATE INDEX factory_run_grants_run_idx
    ON factory_run_grants (org_id, run_id, seq);

-- One issued grant per (run, seq); refused attempts may repeat a seq.
CREATE UNIQUE INDEX factory_run_grants_issued_seq_uq
    ON factory_run_grants (run_id, seq)
    WHERE status = 'issued';

-- 3. Emission countersign evidence on the run row (FR-014): the certificate
--    hash the engine reported at completion and when the platform
--    countersigned it.
ALTER TABLE factory_runs ADD COLUMN certificate_sha256 TEXT;
ALTER TABLE factory_runs ADD COLUMN countersigned_at TIMESTAMPTZ;
