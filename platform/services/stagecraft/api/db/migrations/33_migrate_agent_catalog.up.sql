-- Spec 139 Phase 2 — backfill spec 111/123 content into the substrate.
--
-- Lands T050 (agent_catalog → factory_artifact_substrate),
-- T051 (agent_catalog_audit → factory_artifact_substrate_audit), and
-- T052 (project_agent_bindings → factory_bindings) atomically. The
-- ordering matters: T050 must complete before T052 because
-- factory_bindings has a FK to factory_artifact_substrate(id), and we
-- preserve `agent_catalog.id` as `factory_artifact_substrate.id` so
-- existing `project_agent_bindings.org_agent_id` resolves directly to
-- the new `artifact_id`.
--
-- All three steps are idempotent — running this migration twice is a
-- no-op.
--
-- **Halt condition (per tasks.md):** ≥ 1% data divergence between source
-- `agent_catalog` rows and target `factory_artifact_substrate` rows must
-- halt the migration. The DB-bound dry-run test
-- (`api/factory/agentCatalogMigration.test.ts`, gated to `encore test`)
-- asserts content_hash + frontmatter byte-equality before this lands in
-- production.

BEGIN;

-- ---------------------------------------------------------------------
-- §1. T050 — agent_catalog → factory_artifact_substrate.
--
-- Preserves `agent_catalog.id` as `factory_artifact_substrate.id` so
-- `project_agent_bindings.org_agent_id` continues to resolve in §3 below.
-- `body_markdown` lands in `user_body` (per spec 139 §9 Phase 2 narrative);
-- `upstream_*` columns stay NULL because user-authored content has no
-- upstream. `effective_body` is generated stored — we do NOT write it.
-- ---------------------------------------------------------------------

INSERT INTO factory_artifact_substrate (
    id,
    org_id,
    origin,
    path,
    kind,
    version,
    status,
    user_body,
    user_modified_at,
    user_modified_by,
    content_hash,
    frontmatter,
    conflict_state,
    created_at,
    updated_at
)
SELECT
    ac.id,
    ac.org_id,
    'user-authored',
    'user-authored/' || ac.name || '.md',
    'agent',
    ac.version,
    -- Status mapping (T040 §pure): draft + published → active; retired → retired.
    CASE WHEN ac.status = 'retired' THEN 'retired' ELSE 'active' END,
    ac.body_markdown,
    ac.updated_at,
    ac.created_by,
    ac.content_hash,
    ac.frontmatter,
    'ok',
    ac.created_at,
    ac.updated_at
FROM agent_catalog ac
ON CONFLICT (org_id, origin, path, version) DO NOTHING;

-- ---------------------------------------------------------------------
-- §2. T051 — agent_catalog_audit → factory_artifact_substrate_audit.
--
-- OQ-1 action mapping (locked Phase 2 directive):
--
--     agent_catalog_audit.action  →  factory_artifact_substrate_audit.action
--     ----------------------------------------------------------------------
--     create                      →  artifact.synced
--     edit                        →  artifact.overridden
--     publish                     →  artifact.synced  (status transition
--                                    encoded in before/after JSONB)
--     retire                      →  artifact.retired
--     fork                        →  artifact.forked  (NEW; spec 139 §6.4
--                                    extended to absorb spec 111
--                                    `agent_catalog_audit.action='fork'`)
--
-- Mirrors `mapAgentCatalogAuditAction` in
-- `api/factory/agentCatalogMigration.ts` byte-for-byte. The pure-functional
-- test in `agentCatalogMigration.test.ts` pins the mapping; if this CASE
-- drifts, the test breaks before the migration lands.
-- ---------------------------------------------------------------------

INSERT INTO factory_artifact_substrate_audit (
    id,
    artifact_id,
    org_id,
    action,
    actor_user_id,
    before,
    after,
    created_at
)
SELECT
    aca.id,
    aca.agent_id,
    aca.org_id,
    CASE aca.action
        WHEN 'create'  THEN 'artifact.synced'
        WHEN 'edit'    THEN 'artifact.overridden'
        WHEN 'publish' THEN 'artifact.synced'
        WHEN 'retire'  THEN 'artifact.retired'
        WHEN 'fork'    THEN 'artifact.forked'
    END,
    aca.actor_user_id,
    aca.before,
    aca.after,
    aca.created_at
FROM agent_catalog_audit aca
WHERE aca.action IN ('create', 'edit', 'publish', 'retire', 'fork')
  AND EXISTS (
      -- Skip orphans defensively. Per spec 111 invariants the audit row
      -- always points at a live agent_catalog row, but a defence-in-depth
      -- WHERE keeps the migration loud rather than throwing on the FK
      -- shape.
      SELECT 1 FROM factory_artifact_substrate fas
       WHERE fas.id = aca.agent_id
  )
ON CONFLICT (id) DO NOTHING;

-- ---------------------------------------------------------------------
-- §3. T052 — project_agent_bindings → factory_bindings.
--
-- `org_agent_id` was preserved verbatim by §1 (id == id), so the FK
-- target on `factory_bindings.artifact_id` resolves cleanly.
-- ---------------------------------------------------------------------

INSERT INTO factory_bindings (
    id,
    project_id,
    artifact_id,
    pinned_version,
    pinned_content_hash,
    bound_by,
    bound_at
)
SELECT
    pab.id,
    pab.project_id,
    pab.org_agent_id,
    pab.pinned_version,
    pab.pinned_content_hash,
    pab.bound_by,
    pab.bound_at
FROM project_agent_bindings pab
WHERE EXISTS (
    SELECT 1 FROM factory_artifact_substrate fas
     WHERE fas.id = pab.org_agent_id
)
ON CONFLICT (project_id, artifact_id) DO NOTHING;

COMMIT;
