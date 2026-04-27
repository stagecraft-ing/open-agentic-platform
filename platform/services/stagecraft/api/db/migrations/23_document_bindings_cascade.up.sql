-- Add ON DELETE CASCADE to document_bindings.project_id.
--
-- Migration 10 created the FK without CASCADE, which broke project delete
-- for any imported / created project that had knowledge bindings (every
-- factory-import project gets bindings via registerRawArtifactsFromRepo).
-- The other six project-child tables (project_repos, environments,
-- project_members, factory_pipelines, factory_policy_bundles,
-- project_github_pats) all cascade — this row was the lone gap.

ALTER TABLE document_bindings
    DROP CONSTRAINT document_bindings_project_id_fkey;

ALTER TABLE document_bindings
    ADD CONSTRAINT document_bindings_project_id_fkey
        FOREIGN KEY (project_id)
        REFERENCES projects(id)
        ON DELETE CASCADE;
