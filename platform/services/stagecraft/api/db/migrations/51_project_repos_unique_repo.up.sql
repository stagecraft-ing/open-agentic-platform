-- Spec 213 FR-009: one GitHub repo maps to exactly one project.
--
-- A build event's owning project is derived from project_repos
-- (FR-006 findRepoRow). When the same (github_org, repo_name) is linked by
-- more than one project, the build is attributed to an arbitrary project and
-- the deploy path (spec 215) resolves the wrong image. This migration makes
-- that edge structurally 1:1.
--
-- Pre-flight resolves any pre-existing duplicate links deterministically by
-- keeping the primary link and dropping the non-primary duplicates. If a
-- duplicate group has zero or multiple primaries the migration fails loud so
-- a human resolves it, rather than the migration guessing which project owns
-- the repo.
DO $$
DECLARE
  bad RECORD;
BEGIN
  FOR bad IN
    SELECT github_org,
           repo_name,
           count(*) AS n,
           count(*) FILTER (WHERE is_primary) AS primaries
    FROM project_repos
    GROUP BY github_org, repo_name
    HAVING count(*) > 1
  LOOP
    IF bad.primaries <> 1 THEN
      RAISE EXCEPTION
        'project_repos has % rows for %/% with % primary link(s); cannot auto-resolve to one-repo-one-project (FR-009). Resolve manually before applying this migration.',
        bad.n, bad.github_org, bad.repo_name, bad.primaries;
    END IF;
  END LOOP;

  -- Every duplicate group has exactly one primary: drop the non-primary
  -- duplicate links so the unique index below applies cleanly.
  DELETE FROM project_repos pr
  USING (
    SELECT github_org, repo_name
    FROM project_repos
    GROUP BY github_org, repo_name
    HAVING count(*) > 1
  ) dup
  WHERE pr.github_org = dup.github_org
    AND pr.repo_name = dup.repo_name
    AND pr.is_primary = false;
END $$;

CREATE UNIQUE INDEX project_repos_org_repo_uniq
    ON project_repos (github_org, repo_name);
