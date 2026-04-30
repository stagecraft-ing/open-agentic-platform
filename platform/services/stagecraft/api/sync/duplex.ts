/**
 * Authenticated duplex sync endpoint.
 *
 *   WebSocket /api/sync/duplex?clientId=…&clientKind=…&lastServerCursor=…
 *
 * The handshake carries the caller-chosen clientId and clientKind. The
 * authenticated orgId is taken from the Rauthy JWT — NOT from the
 * handshake — so a client cannot subscribe to an org it does not own.
 *
 * Spec 119: scope key is `orgId`.
 */
import { api } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import log from "encore.dev/log";
import { randomUUID } from "node:crypto";
import type {
  SyncHandshake,
  ServerHello,
  ClientEnvelopeWire,
  ServerEnvelopeWire,
} from "./types";
import { ENVELOPE_SCHEMA_VERSION } from "./types";
import type { SessionMeta } from "./registry";
import * as registry from "./registry";
import {
  handleInbound,
  publishAck,
  publishNack,
  type InboundContext,
} from "./service";
import { cursors } from "./store";
import { sendAgentCatalogSnapshot } from "../agents/relay";
import { sendProjectCatalogSnapshot } from "./projectCatalogRelay";

const HEARTBEAT_INTERVAL_MS = 30_000;
const SERVER_STARTED_AT = new Date().toISOString();

export const duplex = api.streamInOut<
  SyncHandshake,
  ClientEnvelopeWire,
  ServerEnvelopeWire
>(
  { expose: true, auth: true, path: "/api/sync/duplex" },
  async (handshake, stream) => {
    const auth = getAuthData()!;
    const orgId = auth.orgId;

    if (!orgId) {
      log.warn("sync: handshake rejected — no org in auth context", {
        userId: auth.userID,
        clientId: handshake.clientId,
      });
      await stream
        .send({
          kind: "sync.nack",
          meta: {
            v: ENVELOPE_SCHEMA_VERSION,
            eventId: randomUUID(),
            sentAt: new Date().toISOString(),
            orgId: "",
            orgCursor: "",
          },
          clientEventId: "",
          reason: "unauthorized",
          detail: "no org context in auth token",
        })
        .catch(() => undefined);
      await stream.close();
      return;
    }

    if (!handshake.clientId || typeof handshake.clientId !== "string") {
      await stream
        .send({
          kind: "sync.nack",
          meta: {
            v: ENVELOPE_SCHEMA_VERSION,
            eventId: randomUUID(),
            sentAt: new Date().toISOString(),
            orgId,
            orgCursor: "",
          },
          clientEventId: "",
          reason: "invalid",
          detail: "handshake.clientId required",
        })
        .catch(() => undefined);
      await stream.close();
      return;
    }

    const sessionMeta: SessionMeta = {
      orgId,
      clientId: handshake.clientId,
      clientKind: handshake.clientKind ?? "unknown",
      userId: auth.userID,
      connectedAt: new Date(),
      lastHeartbeatAt: new Date(),
    };
    registry.register({ meta: sessionMeta, stream });

    const ctx: InboundContext = {
      orgId,
      clientId: handshake.clientId,
      userId: auth.userID,
    };

    // Greet the client with a ServerHello so it sees the current cursor and
    // session ID before any other traffic.
    const lastCursor = cursors.peek(orgId);
    const cursorGap =
      handshake.lastServerCursor !== undefined &&
      lastCursor !== undefined &&
      handshake.lastServerCursor !== lastCursor;

    const hello: ServerHello = {
      kind: "sync.hello",
      meta: {
        v: ENVELOPE_SCHEMA_VERSION,
        eventId: randomUUID(),
        sentAt: new Date().toISOString(),
        orgId,
        orgCursor: lastCursor ?? cursors.next(orgId),
      },
      sessionId: `${orgId}:${handshake.clientId}`,
      serverStartedAt: SERVER_STARTED_AT,
      cursorGap,
    };
    await stream.send(hello).catch(() => undefined);

    if (cursorGap) {
      await stream
        .send({
          kind: "sync.resync_required",
          meta: {
            v: ENVELOPE_SCHEMA_VERSION,
            eventId: randomUUID(),
            sentAt: new Date().toISOString(),
            orgId,
            orgCursor: cursors.next(orgId),
          },
          reason: "cursor_gap",
        })
        .catch(() => undefined);
    }

    // Spec 111 §2.3 Phase 3 (amended by spec 119) — post-handshake catalog
    // directory spans every project in the session's org. Sent to every
    // connecting OPC so a desktop that missed incremental updates can diff
    // hashes against its local cache and pull only what changed.
    // Fire-and-log: a DB hiccup here must not stop the duplex session.
    void sendAgentCatalogSnapshot(orgId, handshake.clientId).catch((err) => {
      log.warn("sync: agent.catalog.snapshot post-handshake send failed", {
        orgId,
        clientId: handshake.clientId,
        err: err instanceof Error ? err.message : String(err),
      });
    });

    // Spec 112 Phase 8 (amended by spec 119) — post-handshake project list,
    // one upsert per row across the org so the OPC's Projects panel renders
    // without a follow-up round-trip. Same fire-and-log posture as the
    // agent snapshot above.
    void sendProjectCatalogSnapshot(orgId, handshake.clientId).catch((err) => {
      log.warn("sync: project.catalog snapshot post-handshake send failed", {
        orgId,
        clientId: handshake.clientId,
        err: err instanceof Error ? err.message : String(err),
      });
    });

    // Start a heartbeat so idle connections surface half-open sockets.
    let heartbeatAlive = true;
    const heartbeatTimer = setInterval(() => {
      registry
        .sendTo(orgId, handshake.clientId, {
          kind: "sync.heartbeat",
          meta: {
            v: ENVELOPE_SCHEMA_VERSION,
            eventId: randomUUID(),
            sentAt: new Date().toISOString(),
            orgId,
            orgCursor: cursors.peek(orgId) ?? "",
          },
        })
        .then((ok) => {
          if (!ok) heartbeatAlive = false;
        })
        .catch(() => {
          heartbeatAlive = false;
        });
    }, HEARTBEAT_INTERVAL_MS);

    try {
      for await (const msg of stream) {
        if (!heartbeatAlive) break;

        const result = await handleInbound(ctx, msg);

        const clientEventId =
          msg && typeof msg === "object" && "meta" in msg
            ? (msg as { meta?: { eventId?: string } }).meta?.eventId ?? ""
            : "";

        if (result.ok) {
          // Heartbeats and ACKs don't need their own ACK response.
          const silent =
            msg &&
            typeof msg === "object" &&
            "kind" in msg &&
            ((msg as { kind?: string }).kind === "sync.heartbeat" ||
              (msg as { kind?: string }).kind === "sync.ack");
          if (!silent && clientEventId) {
            await publishAck(ctx, clientEventId);
          }
        } else {
          await publishNack(ctx, clientEventId, result.reason, result.detail);
        }
      }
    } catch (err) {
      log.warn("sync: duplex stream error", {
        orgId,
        clientId: handshake.clientId,
        err: err instanceof Error ? err.message : String(err),
      });
    } finally {
      clearInterval(heartbeatTimer);
      registry.unregister(orgId, handshake.clientId);
    }
  },
);
