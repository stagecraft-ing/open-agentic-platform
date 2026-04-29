-- Persist the underlying failure message from the clone worker so the
-- Clone dialog can show *why* a run failed instead of only the typed
-- code. The existing `error` column keeps the typed code; `error_detail`
-- carries the free-form stderr / exception message captured at the
-- failure site (e.g. the `git push --mirror` stderr behind a
-- `mirror_push_failed` code).

ALTER TABLE project_clone_runs
    ADD COLUMN error_detail TEXT;
