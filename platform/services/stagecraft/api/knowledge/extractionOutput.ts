// Spec 115 FR-016 — typed contract for `knowledge_objects.extraction_output`.
//
// Postgres JSONB cannot enforce this shape cheaply, so the worker validates
// at write time. A malformed payload fails the run with
// `extractor_returned_malformed_output` rather than silently corrupting the
// workspace's knowledge.

import { z } from "zod";

// Spec 120 FR-002 — shared schema version, mirrored verbatim by
// `KNOWLEDGE_SCHEMA_VERSION` in `crates/factory-contracts/src/knowledge.rs`.
// Drift is a CI failure (see `tools/schema-parity-check`).
export const KNOWLEDGE_SCHEMA_VERSION = "1.0.0" as const;

// ---------------------------------------------------------------------------
// Zod schema
// ---------------------------------------------------------------------------

const tokenSpendSchema = z.object({
  input: z.number().int().nonnegative(),
  output: z.number().int().nonnegative(),
  cacheRead: z.number().int().nonnegative().optional(),
  cacheWrite: z.number().int().nonnegative().optional(),
});

const agentRunSchema = z.object({
  modelId: z.string().min(1),
  // sha256 hex of the prompt template + key params; reproducible across runs.
  promptFingerprint: z.string().regex(/^[a-f0-9]{64}$/),
  durationMs: z.number().int().nonnegative(),
  tokenSpend: tokenSpendSchema,
  costUsd: z.number().nonnegative(),
  attempts: z.number().int().positive(),
});

const pageSchema = z.object({
  index: z.number().int().nonnegative(),
  text: z.string(),
  bbox: z.unknown().optional(),
});

const outlineEntrySchema = z.object({
  level: z.number().int().positive(),
  text: z.string().min(1),
  pageIndex: z.number().int().nonnegative().optional(),
});

const extractorMetaSchema = z.object({
  kind: z.string().min(1),
  version: z.string().min(1),
  agentRun: agentRunSchema.optional(),
});

export const extractionOutputSchema = z.object({
  text: z.string(),
  pages: z.array(pageSchema).optional(),
  // ISO 639-1 (e.g. "en", "fr"). Optional — many short payloads are unsafe
  // to language-detect.
  language: z.string().min(2).max(8).optional(),
  outline: z.array(outlineEntrySchema).optional(),
  metadata: z.record(z.string(), z.unknown()),
  extractor: extractorMetaSchema,
});

// ---------------------------------------------------------------------------
// Public types (inferred from schema so drift is impossible)
// ---------------------------------------------------------------------------

export type TokenSpend = z.infer<typeof tokenSpendSchema>;
export type AgentRun = z.infer<typeof agentRunSchema>;
export type ExtractionPage = z.infer<typeof pageSchema>;
export type OutlineEntry = z.infer<typeof outlineEntrySchema>;
export type ExtractionOutput = z.infer<typeof extractionOutputSchema>;

// ---------------------------------------------------------------------------
// Validation helper
// ---------------------------------------------------------------------------

export class ExtractorReturnedMalformedOutputError extends Error {
  readonly code = "extractor_returned_malformed_output";
  readonly issues: z.ZodIssue[];
  constructor(issues: z.ZodIssue[]) {
    super(
      `extractor returned malformed output: ${issues
        .map((i) => `${i.path.join(".") || "<root>"}: ${i.message}`)
        .join("; ")}`,
    );
    this.issues = issues;
  }
}

export function validateExtractionOutput(value: unknown): ExtractionOutput {
  const result = extractionOutputSchema.safeParse(value);
  if (!result.success) {
    throw new ExtractorReturnedMalformedOutputError(result.error.issues);
  }
  return result.data;
}
