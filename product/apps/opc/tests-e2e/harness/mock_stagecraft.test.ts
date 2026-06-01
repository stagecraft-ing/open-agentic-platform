import { describe, it, expect, afterEach } from "vitest";
import { WebSocket } from "ws";
import { MockStagecraft, ENVELOPE_SCHEMA_VERSION } from "./mock_stagecraft";

// Spec 187 FR-T2 + 187 AC-6. A real ws client connects to the mock and asserts
// the observable behaviour of each of the four modes. The handshake query
// string mirrors the Encore streamInOut convention the desktop client uses.

let server: MockStagecraft | undefined;
afterEach(async () => {
  await server?.stop();
  server = undefined;
});

const HANDSHAKE = "?clientId=test-client&clientKind=desktop-opc";

function firstMessage(ws: WebSocket): Promise<Record<string, unknown>> {
  return new Promise((resolve, reject) => {
    ws.once("message", (data) => resolve(JSON.parse(data.toString())));
    ws.once("error", reject);
    setTimeout(() => reject(new Error("no message within 3s")), 3000);
  });
}

function observeUntilClose(
  ws: WebSocket,
): Promise<{ opened: boolean; gotMessage: boolean }> {
  return new Promise((resolve, reject) => {
    let opened = false;
    let gotMessage = false;
    ws.on("open", () => (opened = true));
    ws.on("message", () => (gotMessage = true));
    ws.on("error", () => undefined); // a clean server close should not error
    ws.on("close", () => resolve({ opened, gotMessage }));
    setTimeout(() => reject(new Error("no close within 3s")), 3000);
  });
}

function waitForClose(ws: WebSocket): Promise<void> {
  return new Promise((resolve, reject) => {
    ws.once("close", () => resolve());
    ws.on("error", () => undefined);
    setTimeout(() => reject(new Error("no close within 3s")), 3000);
  });
}

function waitForError(ws: WebSocket): Promise<Error> {
  return new Promise((resolve, reject) => {
    ws.once("error", (err) => resolve(err));
    setTimeout(() => reject(new Error("no transport error within 3s")), 3000);
  });
}

describe("FR-T2 mock-stagecraft duplex server", () => {
  it("healthy: emits a v2 sync.hello after handshake and keeps the socket open", async () => {
    server = new MockStagecraft({ mode: "healthy", orgId: "org_test" });
    const { url } = await server.start();
    const ws = new WebSocket(`${url}${HANDSHAKE}`, {
      headers: { Authorization: "Bearer test" },
    });
    const hello = await firstMessage(ws);
    expect(hello.kind).toBe("sync.hello");
    expect((hello.meta as { v: number }).v).toBe(ENVELOPE_SCHEMA_VERSION);
    expect((hello.meta as { v: number }).v).toBe(2);
    expect((hello.meta as { orgId: string }).orgId).toBe("org_test");
    expect(typeof hello.sessionId).toBe("string");
    expect(ws.readyState).toBe(WebSocket.OPEN);
    ws.close();
  });

  it("handshake-rejects: accepts the connection then closes before any sync.hello", async () => {
    server = new MockStagecraft({ mode: "handshake-rejects" });
    const { url } = await server.start();
    const ws = new WebSocket(`${url}${HANDSHAKE}`);
    const outcome = await observeUntilClose(ws);
    expect(outcome.opened).toBe(true);
    expect(outcome.gotMessage).toBe(false);
  });

  it("mid-session-drop: emits sync.hello then drops the connection", async () => {
    server = new MockStagecraft({ mode: "mid-session-drop", orgId: "org_drop" });
    const { url } = await server.start();
    const ws = new WebSocket(`${url}${HANDSHAKE}`);
    const hello = await firstMessage(ws);
    expect(hello.kind).toBe("sync.hello");
    await waitForClose(ws);
    expect(ws.readyState).toBe(WebSocket.CLOSED);
  });

  it("network-unreachable: refuses the connection at the transport layer", async () => {
    server = new MockStagecraft({ mode: "network-unreachable" });
    const { url } = await server.start();
    const ws = new WebSocket(`${url}${HANDSHAKE}`);
    await expect(waitForError(ws)).resolves.toBeInstanceOf(Error);
  });

  it("exposes a ws:// url on the canonical duplex path", async () => {
    server = new MockStagecraft({ mode: "healthy" });
    const { url, port } = await server.start();
    expect(url).toMatch(/^ws:\/\/127\.0\.0\.1:\d+\/api\/sync\/duplex$/);
    expect(port).toBeGreaterThan(0);
  });
});
