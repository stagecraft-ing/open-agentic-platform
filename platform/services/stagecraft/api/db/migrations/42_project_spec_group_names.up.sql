-- Spec 163 FR-004 — cosmetic display names for derived spec-spine groups.
--
-- The Requirements view renders specs grouped by an algorithmic
-- projection (`by-category`, `by-establishment-chain`,
-- `by-supersession-chain`). The group identities are computed at
-- render time and have no persistent storage in the spec spine
-- itself. A project owner may attach a cosmetic display name to any
-- derived group — pure presentation metadata that does not change
-- ownership, authority, or any gate behaviour (FR-004 §2.3).
--
-- The (project_id, group_id) pair is the natural key. `group_id` is
-- the projection-prefixed stable id assigned by the grouping
-- function — for example `by-category:lifecycle` or
-- `by-supersession-chain:042-old-spec`. The column is TEXT (no
-- foreign key) because derived groups are not first-class entities
-- elsewhere in the schema; they are emergent over the spec
-- registry.

BEGIN;

CREATE TABLE project_spec_group_names (
    project_id    UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    group_id      TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    created_by    UUID,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, group_id),
    CONSTRAINT project_spec_group_names_display_name_nonempty
        CHECK (length(trim(display_name)) > 0),
    CONSTRAINT project_spec_group_names_display_name_max_len
        CHECK (length(display_name) <= 200)
);

CREATE INDEX project_spec_group_names_by_project
    ON project_spec_group_names (project_id);

COMMIT;
