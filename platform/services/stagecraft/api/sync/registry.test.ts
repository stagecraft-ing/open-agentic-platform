import { beforeEach, describe, expect, test, vi } from "vitest";
import type { ServerEnvelope, SyncStream } from "./types";
import * as registry from "./registry";
import type { Session } from "./registry";

function makeStream(opts: { failSend?: boolean } = {}): {
  stream: SyncStream;
  sent: ServerEnvelope[];
  closed: boolean;
} {
  const sent: ServerEnvelope[] = [];
  let closed = false;
  const stream = {
    send: vi.fn(async (msg: ServerEnvelope) => {
      if (opts.failSend) throw new Error("boom");
      sent.push(msg);
    }),
    close: vi.fn(async () => {
      closed = true;
    }),
    recv: vi.fn(async () => {
      throw new Error("not used in registry tests");
    }),
    [Symbol.asyncIterator]: async function* () {
      // no messages
    },
  } as unknown as SyncStream;
  return {
    stream,
    sent,
    get closed() {
      return closed;
    },
  };
}

function makeSession(workspaceId: string, clientId: string, stream: SyncStream): Session {
  return {
    meta: {
      workspaceId,
      clientId,
      clientKind: "desktop-opc",
      userId: "u1",
      orgId: "o1",
      connectedAt: new Date(),
      lastHeartbeatAt: new Date(),
    },
    stream,
  };
}

function sampleEvent(
  workspaceId: string,
  cursor = "0000000000000000001",
): ServerEnvelope {
  return {
    kind: "sync.heartbeat",
    meta: {
      eventId: "evt-1",
      sentAt: "2026-04-20T00:00:00Z",
      workspaceId,
      workspaceCursor: cursor,
    },
  };
}

describe("sync registry", () => {
  beforeEach(() => registry.__resetForTests());

  test("registers and counts sessions scoped by workspace", () => {
    const a = makeStream();
    const b = makeStream();
    registry.register(makeSession("ws1", "c1", a.stream));
    registry.register(makeSession("ws1", "c2", b.stream));
    registry.register(makeSession("ws2", "c3", makeStream().stream));

    expect(registry.sessionCount("ws1")).toBe(2);
    expect(registry.sessionCount("ws2")).toBe(1);
    expect(registry.sessionCount()).toBe(3);
    expect(registry.workspaceCount()).toBe(2);
  });

  test("replacing a session closes the prior stream", () => {
    const first = makeStream();
    const second = makeStream();
    registry.register(makeSession("ws1", "c1", first.stream));
    registry.register(makeSession("ws1", "c1", second.stream));
    expect(first.stream.close).toHaveBeenCalled();
    expect(registry.sessionCount("ws1")).toBe(1);
  });

  test("unregister removes the session and cleans the workspace", () => {
    const a = makeStream();
    registry.register(makeSession("ws1", "c1", a.stream));
    registry.unregister("ws1", "c1");
    expect(registry.sessionCount("ws1")).toBe(0);
    expect(registry.workspaceCount()).toBe(0);
  });

  test("sendTo delivers to the target and updates lastSentCursor", async () => {
    const a = makeStream();
    const session = makeSession("ws1", "c1", a.stream);
    registry.register(session);

    const evt = sampleEvent("ws1", "0000000000000000007");
    const ok = await registry.sendTo("ws1", "c1", evt);

    expect(ok).toBe(true);
    expect(a.sent).toEqual([evt]);
    expect(session.meta.lastSentCursor).toBe("0000000000000000007");
  });

  test("sendTo prunes the session on send failure", async () => {
    const a = makeStream({ failSend: true });
    registry.register(makeSession("ws1", "c1", a.stream));

    const ok = await registry.sendTo("ws1", "c1", sampleEvent("ws1"));
    expect(ok).toBe(false);
    expect(registry.sessionCount("ws1")).toBe(0);
  });

  test("broadcastWorkspace does not leak across workspaces", async () => {
    const a = makeStream();
    const b = makeStream();
    const other = makeStream();
    registry.register(makeSession("ws1", "c1", a.stream));
    registry.register(makeSession("ws1", "c2", b.stream));
    registry.register(makeSession("ws2", "x1", other.stream));

    const evt = sampleEvent("ws1");
    const result = await registry.broadcastWorkspace("ws1", evt);

    expect(result.sent).toBe(2);
    expect(a.sent).toHaveLength(1);
    expect(b.sent).toHaveLength(1);
    expect(other.sent).toHaveLength(0);
  });

  test("broadcastWorkspace honours excludeClientId", async () => {
    const a = makeStream();
    const b = makeStream();
    registry.register(makeSession("ws1", "c1", a.stream));
    registry.register(makeSession("ws1", "c2", b.stream));

    const result = await registry.broadcastWorkspace("ws1", sampleEvent("ws1"), {
      excludeClientId: "c1",
    });
    expect(result.sent).toBe(1);
    expect(a.sent).toHaveLength(0);
    expect(b.sent).toHaveLength(1);
  });

  test("broadcastWorkspace prunes dead streams", async () => {
    const dead = makeStream({ failSend: true });
    const alive = makeStream();
    registry.register(makeSession("ws1", "c-dead", dead.stream));
    registry.register(makeSession("ws1", "c-alive", alive.stream));

    const result = await registry.broadcastWorkspace("ws1", sampleEvent("ws1"));
    expect(result.sent).toBe(1);
    expect(result.pruned).toBe(1);
    expect(registry.sessionCount("ws1")).toBe(1);
    expect(registry.get("ws1", "c-dead")).toBeUndefined();
  });
});
