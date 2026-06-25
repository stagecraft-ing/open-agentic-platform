-- Spec 208 FR-001: drop the org-wide kill-switch quarantine record. The table
-- is introduced fresh by 53_org_halts.up.sql (no pre-existing table is
-- widened), so the inverse is a plain drop; the partial liveness index falls
-- with the table.
DROP TABLE IF EXISTS org_halts;
