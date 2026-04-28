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

const DEFAULT_MAX_CONCURRENCY = 8;

function maxConcurrency(): number {
  const v = process.env.STAGECRAFT_EXTRACT_WORKER_CONCURRENCY;
  if (!v) return DEFAULT_MAX_CONCURRENCY;
  const n = Number.parseInt(v, 10);
  return Number.isFinite(n) && n > 0 ? n : DEFAULT_MAX_CONCURRENCY;
}

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

const _extractionWorker = new Subscription(
  KnowledgeExtractionRequestTopic,
  "knowledge-extraction-worker",
  {
    handler: handleExtractionRequest,
    maxConcurrency: maxConcurrency(),
  },
);
void _extractionWorker;
