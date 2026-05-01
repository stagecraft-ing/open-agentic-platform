// Spec 115 FR-016 — typed contract for `knowledge_objects.extraction_output`.
//
// Postgres JSONB cannot enforce this shape cheaply, so the worker validates
// at write time. A malformed payload fails the run with
// `extractor_returned_malformed_output` rather than silently corrupting the
// workspace's knowledge.

// zod 4 named imports. The package exposes `z` as a re-exported
// namespace alias (`import * as z from "..."; export { z }`) which
// Encore.ts's TS parser cannot resolve — `import { z } from "zod"`
// fails with `error: object not found: z` and `import * as z from
// "zod"` fails with `error: unsupported member on type never` during
// `encore build` codegen. Direct named imports go through the package's
// top-level `export *` re-exports cleanly. This is also the idiomatic,
// tree-shakable v4 form.
import {
  array,
  number,
  object,
  record,
  string,
  unknown,
  type ZodIssue,
  type infer as zInfer,
} from "zod";

// Spec 120 FR-002 — shared schema version, mirrored verbatim by
// `KNOWLEDGE_SCHEMA_VERSION` in `crates/factory-contracts/src/knowledge.rs`.
// Drift is a CI failure (see `tools/schema-parity-check`).
export const KNOWLEDGE_SCHEMA_VERSION = "1.0.0" as const;

// Spec 120 FR-016(d) — minimum schema version the external
// `extraction-output` endpoint accepts. OPC sets the
// `X-Knowledge-Schema-Version` request header from its compile-time
// `KNOWLEDGE_SCHEMA_VERSION` const; bodies below this minimum are rejected
// with `failed_precondition` / `schema_version_too_old`.
export const MINIMUM_KNOWLEDGE_SCHEMA_VERSION = "1.0.0" as const;
export const KNOWLEDGE_SCHEMA_VERSION_HEADER = "x-knowledge-schema-version";

// ---------------------------------------------------------------------------
// Zod schema
// ---------------------------------------------------------------------------

const tokenSpendSchema = object({
  input: number().int().nonnegative(),
  output: number().int().nonnegative(),
  cacheRead: number().int().nonnegative().optional(),
  cacheWrite: number().int().nonnegative().optional(),
});

const agentRunSchema = object({
  modelId: string().min(1),
  // sha256 hex of the prompt template + key params; reproducible across runs.
  promptFingerprint: string().regex(/^[a-f0-9]{64}$/),
  durationMs: number().int().nonnegative(),
  tokenSpend: tokenSpendSchema,
  costUsd: number().nonnegative(),
  attempts: number().int().positive(),
});

const pageSchema = object({
  index: number().int().nonnegative(),
  text: string(),
  bbox: unknown().optional(),
});

const outlineEntrySchema = object({
  level: number().int().positive(),
  text: string().min(1),
  pageIndex: number().int().nonnegative().optional(),
});

const extractorMetaSchema = object({
  kind: string().min(1),
  version: string().min(1),
  agentRun: agentRunSchema.optional(),
});

export const extractionOutputSchema = object({
  text: string(),
  pages: array(pageSchema).optional(),
  // ISO 639-1 (e.g. "en", "fr"). Optional — many short payloads are unsafe
  // to language-detect.
  language: string().min(2).max(8).optional(),
  outline: array(outlineEntrySchema).optional(),
  metadata: record(string(), unknown()),
  extractor: extractorMetaSchema,
});

// ---------------------------------------------------------------------------
// Public types (inferred from schema so drift is impossible)
// ---------------------------------------------------------------------------

export type TokenSpend = zInfer<typeof tokenSpendSchema>;
export type AgentRun = zInfer<typeof agentRunSchema>;
export type ExtractionPage = zInfer<typeof pageSchema>;
export type OutlineEntry = zInfer<typeof outlineEntrySchema>;
export type ExtractionOutput = zInfer<typeof extractionOutputSchema>;

// ---------------------------------------------------------------------------
// Validation helper
// ---------------------------------------------------------------------------

export class ExtractorReturnedMalformedOutputError extends Error {
  readonly code = "extractor_returned_malformed_output";
  readonly issues: ZodIssue[];
  constructor(issues: ZodIssue[]) {
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
