-- Spec 119 follow-up — add ON DELETE CASCADE to the three project_id FKs
-- that migration 27 created without a cascade clause. The deleteProject
-- endpoint assumed CASCADE would clean up child rows; in practice the
-- delete failed with a foreign-key violation as soon as the project
-- owned any knowledge_objects / source_connectors / sync_runs.
--
-- Postgres can't `ALTER` an existing FK in place — the constraint must
-- be dropped and re-added. The constraint names are the Postgres defaults
-- emitted by migration 27 (`<table>_project_id_fkey`).

ALTER TABLE knowledge_objects
    DROP CONSTRAINT knowledge_objects_project_id_fkey,
    ADD CONSTRAINT knowledge_objects_project_id_fkey
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE;

ALTER TABLE source_connectors
    DROP CONSTRAINT source_connectors_project_id_fkey,
    ADD CONSTRAINT source_connectors_project_id_fkey
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE;

ALTER TABLE sync_runs
    DROP CONSTRAINT sync_runs_project_id_fkey,
    ADD CONSTRAINT sync_runs_project_id_fkey
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE;
