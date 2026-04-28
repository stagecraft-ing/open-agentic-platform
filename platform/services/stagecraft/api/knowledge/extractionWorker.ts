// Spec 115 §5 / FR-004 — PubSub subscriber that drives the extraction worker.
//
// At-least-once delivery: the handler MUST be idempotent. CAS on
// `pending → running` happens inside `runExtractionWork`, so a redelivered
// message that finds the row already terminal is a no-op (FR-005).

import { Subscription } from "encore.dev/pubsub";
import log from "encore.dev/log";
import { runExtractionWork } from "./extractionCore";
import {
  KnowledgeExtractionRequestTopic,
  type KnowledgeExtractionRequest,
} from "./extractionEvents";
// Side-effect import: registers every deterministic / agent extractor
// into the dispatch table before any message can arrive.
import "./extractors";

async function handleExtractionRequest(
  req: KnowledgeExtractionRequest,
): Promise<void> {
  try {
    await runExtractionWork({ extractionRunId: req.extractionRunId });
  } catch (err) {
    // runExtractionWork already converts known throws into `failed` rows.
    // Anything that escapes is a bug — log loudly so the operator can
    // investigate, and re-throw so PubSub redelivery can retry once.
    // The auto-retry cap inside runExtractionWork stops the loop.
    log.error("extractionWorker: unhandled exception in handler", {
      runId: req.extractionRunId,
      err: err instanceof Error ? err.message : String(err),
    });
    throw err;
  }
}

// Encore parses Subscription config at build time and only accepts literal
// integers for `maxConcurrency`. We omit it here and rely on Encore's
// default fan-out; runtime back-pressure is handled by the per-run CAS +
// the day-aggregate cost gate inside `runExtractionWork`.
const _extractionWorker = new Subscription(
  KnowledgeExtractionRequestTopic,
  "knowledge-extraction-worker",
  {
    handler: handleExtractionRequest,
  },
);
void _extractionWorker;
