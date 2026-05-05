-- Spec 139 Phase 1 — rollback for the factory artifact substrate.
--
-- Drops the three new tables and reverts factory_upstreams to its
-- spec 108 singleton-row shape. Phase 2/3/4 migrations stack on top of
-- this; rolling back beyond Phase 1 requires their own .down scripts.

DROP TABLE IF EXISTS factory_bindings;
DROP TABLE IF EXISTS factory_artifact_substrate_audit;
DROP TABLE IF EXISTS factory_artifact_substrate;

-- Restore factory_upstreams to (org_id) PK with NOT NULL legacy columns.
-- The 'legacy-mixed' singleton row's data carries over verbatim — we only
-- need to drop the new columns and swap PK back.

ALTER TABLE factory_upstreams DROP CONSTRAINT factory_upstreams_pkey;
ALTER TABLE factory_upstreams
    ADD CONSTRAINT factory_upstreams_pkey PRIMARY KEY (org_id);

ALTER TABLE factory_upstreams
    ALTER COLUMN factory_source  SET NOT NULL,
    ALTER COLUMN template_source SET NOT NULL;

ALTER TABLE factory_upstreams
    DROP COLUMN IF EXISTS source_id,
    DROP COLUMN IF EXISTS role,
    DROP COLUMN IF EXISTS subpath,
    DROP COLUMN IF EXISTS repo_url,
    DROP COLUMN IF EXISTS ref;
