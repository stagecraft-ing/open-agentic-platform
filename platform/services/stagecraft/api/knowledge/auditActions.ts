// Spec 115 FR-026 — audit action constants for the extraction pipeline.
//
// `audit_log.action` is a free-text column (api/db/schema.ts:275). Existing
// knowledge actions ("knowledge.upload_requested" etc) are inline string
// literals at the call sites. This module centralises the spec-115 actions
// so renames stay grep-able and so dashboards keying on these strings
// can't drift from the writers.

export const KNOWLEDGE_EXTRACTED = "knowledge.extracted" as const;
export const KNOWLEDGE_EXTRACTION_FAILED =
  "knowledge.extraction_failed" as const;
export const KNOWLEDGE_EXTRACTION_RETRY_REQUESTED =
  "knowledge.extraction_retry_requested" as const;
// Spec 120 FR-020 — resolver decision audit when multiple extraction
// records exist for the same `(object_id, content_hash)`.
export const KNOWLEDGE_EXTRACTION_RESOLVED =
  "knowledge.extraction_resolved" as const;

export type KnowledgeExtractionAuditAction =
  | typeof KNOWLEDGE_EXTRACTED
  | typeof KNOWLEDGE_EXTRACTION_FAILED
  | typeof KNOWLEDGE_EXTRACTION_RETRY_REQUESTED
  | typeof KNOWLEDGE_EXTRACTION_RESOLVED;
