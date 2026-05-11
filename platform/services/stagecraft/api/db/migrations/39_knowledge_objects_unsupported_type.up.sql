-- Spec 143 §12 FU-019 — `unsupported_type` status taxonomy.
--
-- Adds a terminal informational state to `knowledge_object_state` for rows
-- whose MIME type has no registered extractor under any policy. Distinct
-- from the existing `lastExtractionError` codes `policy_pending` /
-- `policy_denied` / `extractor_not_implemented`, which all currently
-- surface as red on the dashboard. Examples in the wild: pptx, xlsx, zip.

ALTER TYPE knowledge_object_state ADD VALUE IF NOT EXISTS 'unsupported_type';
