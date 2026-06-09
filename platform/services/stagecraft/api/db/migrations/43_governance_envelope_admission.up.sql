-- Spec 198 — governance envelope admission gate (FR-001/FR-003) and
-- containment (FR-010): admission records, revocations keyed on the
-- admission graph, and the `governance-envelope` artifact kind.

-- 1. New substrate kind for the process-layer envelope file
--    (`process/governance-envelope.yaml`). The CHECK list mirrors the
--    closed set in api/db/schema.ts::ArtifactKind.
ALTER TABLE factory_artifact_substrate
    DROP CONSTRAINT factory_artifact_substrate_kind_chk;
ALTER TABLE factory_artifact_substrate
    ADD CONSTRAINT factory_artifact_substrate_kind_chk
        CHECK (kind IN (
            'agent',
            'skill',
            'process-stage',
            'adapter-manifest',
            'contract-schema',
            'pattern',
            'page-type-reference',
            'sample-html',
            'reference-data',
            'invariant',
            'pipeline-orchestrator',
            'governance-envelope'
        ));

-- 2. Admission records (spec 198 FR-001/FR-003). One row per evaluation;
--    the latest row per (org_id, origin) is the standing admission state.
--    `status='refused'` rows are kept — refusals are attributable evidence,
--    not transient errors.
CREATE TABLE factory_admissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL,
    origin          TEXT NOT NULL,
    status          TEXT NOT NULL,
    -- SHA-256 of the process envelope body as filed (content-addressed
    -- identity; FR-009 pinning).
    envelope_hash   TEXT,
    -- The composed admitted envelope: process envelope ⊕ adapter
    -- sub-envelope(s) ⊕ constituent digests ⊕ resolved scaffold sources
    -- (never flattened — FR-004 composition is preserved in the JSON shape).
    composed        JSONB,
    -- Attributable refusal reasons (empty array when admitted).
    violations      JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- Spec 199 FR-009 / D-5: adapter name → resolved scaffold source
    -- ({ source_id, repo_url, ref }) recorded at admission time.
    scaffold_resolutions JSONB NOT NULL DEFAULT '{}'::jsonb,
    factory_sha     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT factory_admissions_status_chk
        CHECK (status IN ('admitted', 'refused'))
);

CREATE INDEX factory_admissions_org_origin_idx
    ON factory_admissions (org_id, origin, created_at DESC);

-- 3. Revocations on the admission graph (spec 198 FR-010). One mechanism,
--    four keys; org-scoped (org_id NULL = a global, OAP-published advisory).
--    Checked fail-closed at serve, bind, and run-grant issuance/renewal.
CREATE TABLE factory_revocations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID,
    scope_kind      TEXT NOT NULL,
    -- factory → origin id; adapter → adapter name; agent → agent id;
    -- content-hash → the pinned artifact hash.
    key             TEXT NOT NULL,
    mode            TEXT NOT NULL,
    reason          TEXT NOT NULL,
    actor           UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Reintegration from quarantine (ASI10 m7) clears by setting lifted_at
    -- after fresh validation + human approval; lifted rows stop matching.
    lifted_at       TIMESTAMPTZ,
    lifted_by       UUID,

    CONSTRAINT factory_revocations_scope_chk
        CHECK (scope_kind IN ('factory', 'adapter', 'agent', 'content-hash')),
    CONSTRAINT factory_revocations_mode_chk
        CHECK (mode IN ('revoked', 'quarantined'))
);

CREATE INDEX factory_revocations_lookup_idx
    ON factory_revocations (scope_kind, key)
    WHERE lifted_at IS NULL;
