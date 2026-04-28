// Spec 115 FR-016 — `extraction_output` typed-contract validation tests.
//
// The Drizzle write path is wrapped with `validateExtractionOutput`; a
// malformed payload fails the run with `extractor_returned_malformed_output`.
// These tests pin every required field so a regression to the contract
// cannot ship without a corresponding test failure.

import { describe, expect, test } from "vitest";
import {
  ExtractorReturnedMalformedOutputError,
  validateExtractionOutput,
} from "./extractionOutput";

const MIN_VALID = {
  text: "hello world",
  metadata: {},
  extractor: { kind: "deterministic-text", version: "1" },
};

const FINGERPRINT = "a".repeat(64);

describe("validateExtractionOutput", () => {
  test("accepts the minimum valid shape", () => {
    const result = validateExtractionOutput(MIN_VALID);
    expect(result.text).toBe("hello world");
    expect(result.extractor.kind).toBe("deterministic-text");
  });

  test("accepts an agent-extracted payload with full agentRun", () => {
    const result = validateExtractionOutput({
      ...MIN_VALID,
      pages: [{ index: 0, text: "page 0" }],
      language: "en",
      outline: [{ level: 1, text: "Intro", pageIndex: 0 }],
      extractor: {
        kind: "agent-pdf-vision",
        version: "1",
        agentRun: {
          modelId: "claude-sonnet-4-6",
          promptFingerprint: FINGERPRINT,
          durationMs: 1200,
          tokenSpend: { input: 100, output: 200 },
          costUsd: 0.0123,
          attempts: 1,
        },
      },
    });
    expect(result.extractor.agentRun?.modelId).toBe("claude-sonnet-4-6");
  });

  test("rejects missing text", () => {
    expect(() =>
      validateExtractionOutput({
        metadata: {},
        extractor: { kind: "x", version: "1" },
      }),
    ).toThrow(ExtractorReturnedMalformedOutputError);
  });

  test("rejects missing extractor.kind", () => {
    expect(() =>
      validateExtractionOutput({
        text: "ok",
        metadata: {},
        extractor: { version: "1" },
      }),
    ).toThrow(ExtractorReturnedMalformedOutputError);
  });

  test("rejects malformed promptFingerprint", () => {
    expect(() =>
      validateExtractionOutput({
        ...MIN_VALID,
        extractor: {
          kind: "agent-pdf-vision",
          version: "1",
          agentRun: {
            modelId: "x",
            promptFingerprint: "not-hex",
            durationMs: 1,
            tokenSpend: { input: 1, output: 1 },
            costUsd: 0.001,
            attempts: 1,
          },
        },
      }),
    ).toThrow(ExtractorReturnedMalformedOutputError);
  });

  test("rejects negative token counts", () => {
    expect(() =>
      validateExtractionOutput({
        ...MIN_VALID,
        extractor: {
          kind: "agent-pdf-vision",
          version: "1",
          agentRun: {
            modelId: "x",
            promptFingerprint: FINGERPRINT,
            durationMs: 1,
            tokenSpend: { input: -1, output: 1 },
            costUsd: 0.001,
            attempts: 1,
          },
        },
      }),
    ).toThrow(ExtractorReturnedMalformedOutputError);
  });
});
