-- Spec 163 FR-004 — reverse migration. Drops the cosmetic-name table.

BEGIN;

DROP TABLE IF EXISTS project_spec_group_names;

COMMIT;
