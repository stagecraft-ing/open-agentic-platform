-- Reverse of 51_project_repos_unique_repo.up.sql. Drops the one-repo-one-
-- project unique index (FR-009). The deduplication of pre-existing duplicate
-- links is not reversible: removed non-primary duplicate links are not
-- restored.
DROP INDEX IF EXISTS project_repos_org_repo_uniq;
