/**
 * Sync service (spec 087 Phase 3).
 *
 * Provides:
 *  1. WebSocket relay (streamOut) — pushes workspace-scoped events to connected
 *     web and OPC clients.
 *  2. OPC event ingestion (HTTP POST) — receives desktop→web events.
 *  3. PubSub subscriber on FactoryEventTopic — fans out pipeline events to
 *     connected workspace streams.
 *
 * All streams are keyed by workspaceId. A connecting client must be
 * authenticated and carry a workspace context in their auth token.
 */

import { api, APIError, type StreamOut } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { Subscription } from "encore.dev/pubsub";
import log from "encore.dev/log";
import { FactoryEventTopic } from "../factory/events";
import { db } from "../db/drizzle";
import { auditLog } from "../db/schema";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Events pushed from Stagecraft to connected clients (web UI, OPC). */
interface SyncEvent {
  type: string;
  workspaceId: string;
  timestamp: string;
  payload: Record<string, unknown>;
}

/** Events pushed from OPC to Stagecraft. */
interface OpcInboundEvent {
  type: string;
  workspaceId: string;
  projectId: string;
  timestamp: string;
  payload: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// In-memory stream registry (workspace → connected clients)
// ---------------------------------------------------------------------------

// TODO: Process-local — for horizontal scale-out, replace with Redis pub/sub
// or Encore's built-in pub/sub fan-out to relay across instances.
const workspaceStreams = new Map<string, Set<StreamOut<SyncEvent>>>();

function registerStream(wsId: string, stream: StreamOut<SyncEvent>) {
  if (!workspaceStreams.has(wsId)) {
    workspaceStreams.set(wsId, new Set());
  }
  workspaceStreams.get(wsId)!.add(stream);
  log.info("stream registered", {
    workspaceId: wsId,
    activeStreams: workspaceStreams.get(wsId)!.size,
  });
}

function unregisterStream(wsId: string, stream: StreamOut<SyncEvent>) {
  const set = workspaceStreams.get(wsId);
  if (set) {
    set.delete(stream);
    if (set.size === 0) workspaceStreams.delete(wsId);
  }
  log.info("stream unregistered", {
    workspaceId: wsId,
    activeStreams: set?.size ?? 0,
  });
}

/** Broadcast an event to all streams connected to a workspace. */
export function broadcastToWorkspace(
  workspaceId: string,
  event: SyncEvent
): void {
  const streams = workspaceStreams.get(workspaceId);
  if (!streams || streams.size === 0) return;

  for (const stream of streams) {
    stream.send(event).catch(() => {
      // Dead stream — remove it; heartbeat loop will also clean up
      streams.delete(stream);
    });
  }
}

// ---------------------------------------------------------------------------
// WebSocket relay — workspace-scoped event stream (NF-005)
// ---------------------------------------------------------------------------

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export const workspaceEventStream = api.streamOut<SyncEvent>(
  { path: "/api/sync/events", expose: true, auth: true },
  async (stream) => {
    const auth = getAuthData()!;
    const wsId = auth.workspaceId;

    if (!wsId) {
      await stream.send({
        type: "error",
        workspaceId: "",
        timestamp: new Date().toISOString(),
        payload: { message: "workspace context required" },
      });
      return;
    }

    registerStream(wsId, stream);

    try {
      // Keep the handler alive with a heartbeat loop.
      // When the client disconnects, stream.send() will throw,
      // breaking the loop and triggering cleanup.
      while (true) {
        await sleep(30_000);
        await stream.send({
          type: "heartbeat",
          workspaceId: wsId,
          timestamp: new Date().toISOString(),
          payload: {},
        });
      }
    } catch {
      // Client disconnected — expected
    } finally {
      unregisterStream(wsId, stream);
    }
  }
);

// ---------------------------------------------------------------------------
// OPC event ingestion — desktop → web (HTTP POST, fire-and-forget)
// ---------------------------------------------------------------------------

export const ingestOpcEvent = api(
  { expose: true, auth: true, method: "POST", path: "/api/sync/opc-events" },
  async (req: OpcInboundEvent): Promise<{ accepted: boolean }> => {
    const auth = getAuthData()!;

    if (!auth.workspaceId) {
      throw APIError.invalidArgument("workspace context required");
    }

    // Verify the event targets the authenticated workspace
    if (req.workspaceId !== auth.workspaceId) {
      throw APIError.permissionDenied("workspace mismatch");
    }

    // Record audit events to the audit log
    if (req.type === "audit_event") {
      await db.insert(auditLog).values({
        actorUserId: auth.userID,
        action: `opc.${req.payload.action ?? "event"}`,
        targetType: String(req.payload.targetType ?? "unknown"),
        targetId: String(req.payload.targetId ?? ""),
        metadata: req.payload,
      });
    }

    // Broadcast to any connected web UI clients watching this workspace
    broadcastToWorkspace(req.workspaceId, {
      type: req.type,
      workspaceId: req.workspaceId,
      timestamp: req.timestamp || new Date().toISOString(),
      payload: {
        ...req.payload,
        projectId: req.projectId,
        source: "opc",
      },
    });

    log.info("opc event ingested", {
      type: req.type,
      workspaceId: req.workspaceId,
      projectId: req.projectId,
    });

    return { accepted: true };
  }
);

// ---------------------------------------------------------------------------
// PubSub subscriber — fan out factory pipeline events to WebSocket clients
// ---------------------------------------------------------------------------

const _ = new Subscription(FactoryEventTopic, "sync-relay", {
  handler: async (event) => {
    // Resolve workspace ID from the project — factory events carry project_id
    // but not workspace_id. For efficiency, we broadcast to all workspaces
    // that have active streams and let the client filter. In practice, a
    // project belongs to exactly one workspace.
    //
    // Look up all connected workspaces and check if the project matches.
    // For now, broadcast the event with the project_id and let clients
    // that care about this pipeline pick it up.
    const syncEvent: SyncEvent = {
      type: "pipeline_event",
      workspaceId: "", // filled per-stream below
      timestamp: new Date().toISOString(),
      payload: {
        pipelineId: event.pipeline_id,
        projectId: event.project_id,
        eventType: event.event_type,
        stageId: event.stage_id,
        actor: event.actor,
        details: event.details,
      },
    };

    // Fan out to all connected workspaces — each stream is scoped and
    // the client can filter by projectId. In a scaled deployment this
    // would use a project→workspace lookup cache.
    for (const [wsId, streams] of workspaceStreams) {
      const wsEvent = { ...syncEvent, workspaceId: wsId };
      for (const stream of streams) {
        stream.send(wsEvent).catch(() => {
          streams.delete(stream);
        });
      }
    }

    log.info("factory event relayed", {
      pipelineId: event.pipeline_id,
      eventType: event.event_type,
      streamCount: workspaceStreams.size,
    });
  },
});
