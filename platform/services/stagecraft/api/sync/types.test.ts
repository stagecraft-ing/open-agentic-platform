import { describe, expect, test } from "vitest";
import { isClientEnvelope } from "./types";

describe("isClientEnvelope", () => {
  test("accepts a well-formed execution.status envelope", () => {
    expect(
      isClientEnvelope({
        kind: "execution.status",
        meta: { v: 1, eventId: "e1", sentAt: "2026-04-20T00:00:00Z" },
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
        meta: { v: 1, eventId: "e2", sentAt: "2026-04-20T00:00:00Z" },
      }),
    ).toBe(true);
  });

  test("rejects unknown kinds", () => {
    expect(
      isClientEnvelope({
        kind: "server.secret.leak",
        meta: { v: 1, eventId: "e", sentAt: "x" },
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
        meta: { v: 1, eventId: 123, sentAt: "now" },
      }),
    ).toBe(false);
  });

  // Spec 087 §5.3 FR-SYNC-003: strict schema-version equality.
  test("rejects envelopes missing schema version", () => {
    expect(
      isClientEnvelope({
        kind: "sync.heartbeat",
        meta: { eventId: "e", sentAt: "t" },
      }),
    ).toBe(false);
  });

  test("rejects envelopes with a different schema version", () => {
    expect(
      isClientEnvelope({
        kind: "sync.heartbeat",
        meta: { v: 2, eventId: "e", sentAt: "t" },
      }),
    ).toBe(false);
  });

  test("rejects envelopes with non-numeric schema version", () => {
    expect(
      isClientEnvelope({
        kind: "sync.heartbeat",
        meta: { v: "1", eventId: "e", sentAt: "t" },
      }),
    ).toBe(false);
  });

  // spec 110 §2.2 — factory.run.ack recognition
  test("accepts a well-formed factory.run.ack envelope", () => {
    expect(
      isClientEnvelope({
        kind: "factory.run.ack",
        meta: { v: 1, eventId: "e-ack", sentAt: "2026-04-21T00:00:00Z" },
        pipelineId: "pl-1",
        sessionId: "sess-1",
        opcInstanceId: "opc-1",
        accepted: true,
        observedAt: "2026-04-21T00:00:01Z",
      }),
    ).toBe(true);
  });

  test("accepts a factory.run.ack that declines the request", () => {
    expect(
      isClientEnvelope({
        kind: "factory.run.ack",
        meta: { v: 1, eventId: "e-ack-2", sentAt: "2026-04-21T00:00:00Z" },
        pipelineId: "pl-2",
        sessionId: "sess-2",
        opcInstanceId: "opc-1",
        accepted: false,
        declineReason: "policy_denied",
        observedAt: "2026-04-21T00:00:01Z",
      }),
    ).toBe(true);
  });

  // spec 110 §5: directives are server→client only; a desktop MUST NOT be
  // able to synthesise a factory.run.request and have it slip through the
  // client-inbox guard as a valid ClientEnvelope.
  test("rejects factory.run.request arriving on the client inbox", () => {
    expect(
      isClientEnvelope({
        kind: "factory.run.request",
        meta: { v: 1, eventId: "e-forged", sentAt: "2026-04-21T00:00:00Z" },
        projectId: "p1",
        pipelineId: "pl-x",
      }),
    ).toBe(false);
  });
});
