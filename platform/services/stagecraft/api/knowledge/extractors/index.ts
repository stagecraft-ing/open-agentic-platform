// Spec 115 — extractor registry assembly.
//
// Importing this module registers every deterministic extractor (and, in
// Phase 2, every policy-gated agent extractor) into the dispatcher in
// cost-ascending order. The worker imports this once at startup so the
// registry is populated before any message can arrive.

import { registerExtractor } from "./dispatch";
import { deterministicTextExtractor } from "./deterministic-text";
import { deterministicPdfEmbeddedExtractor } from "./deterministic-pdf-embedded";
import { deterministicDocxExtractor } from "./deterministic-docx";

let registered = false;

export function registerDeterministicExtractors(): void {
  if (registered) return;
  // Register cheapest → most expensive. Order matters: the dispatcher
  // returns the first match.
  registerExtractor(deterministicTextExtractor);
  registerExtractor(deterministicPdfEmbeddedExtractor);
  registerExtractor(deterministicDocxExtractor);
  registered = true;
}

// Side-effect registration on first import — the worker module imports
// this file once, which is enough to populate the registry for all
// subsequent dispatches in the process.
registerDeterministicExtractors();
