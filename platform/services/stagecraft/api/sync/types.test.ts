import { describe, expect, test } from "vitest";
import { isClientEnvelope } from "./types";

describe("isClientEnvelope", () => {
  test("accepts a well-formed execution.status envelope", () => {
    expect(
      isClientEnvelope({
        kind: "execution.status",
        meta: { eventId: "e1", sentAt: "2026-04-20T00:00:00Z" },
        projectId: "p1",
        executionId: "x1",
        status: "started",
      }),
    ).toBe(true);
  });

  test("accepts sync.heartbeat", () => {
    expect(
      isClientEnvelope({
        kind: "sync.heartbeat",
        meta: { eventId: "e2", sentAt: "2026-04-20T00:00:00Z" },
      }),
    ).toBe(true);
  });

  test("rejects unknown kinds", () => {
    expect(
      isClientEnvelope({
        kind: "server.secret.leak",
        meta: { eventId: "e", sentAt: "x" },
      }),
    ).toBe(false);
  });

  test("rejects envelopes without meta", () => {
    expect(isClientEnvelope({ kind: "sync.heartbeat" })).toBe(false);
  });

  test("rejects non-objects", () => {
    expect(isClientEnvelope(null)).toBe(false);
    expect(isClientEnvelope("hello")).toBe(false);
    expect(isClientEnvelope(42)).toBe(false);
  });

  test("rejects envelopes with non-string eventId", () => {
    expect(
      isClientEnvelope({
        kind: "sync.heartbeat",
        meta: { eventId: 123, sentAt: "now" },
      }),
    ).toBe(false);
  });
});
