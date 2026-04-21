/**
 * Event relay — subscribes to internal PubSub topics and dispatches the
 * corresponding ServerEnvelope variants through the outbox path.
 *
 * Adding a new server-originated event type means:
 *   1. Define the envelope variant in `types.ts`.
 *   2. Subscribe to the producing topic here and call `dispatchServerEvent`.
 */
import { Subscription } from "encore.dev/pubsub";
import log from "encore.dev/log";
import { eq } from "drizzle-orm";
import { FactoryEventTopic } from "../factory/events";
import { db } from "../db/drizzle";
import { projects } from "../db/schema";
import { dispatchServerEvent } from "./service";

// ---------------------------------------------------------------------------
// project_id -> workspace_id cache (TTL 5 min)
// ---------------------------------------------------------------------------

const PROJECT_WORKSPACE_TTL_MS = 5 * 60_000;
const projectWorkspaceCache = new Map<
  string,
  { workspaceId: string | null; fetchedAt: number }
>();

async function resolveWorkspaceId(projectId: string): Promise<string | null> {
  const now = Date.now();
  const cached = projectWorkspaceCache.get(projectId);
  if (cached && now - cached.fetchedAt < PROJECT_WORKSPACE_TTL_MS) {
    return cached.workspaceId;
  }

  const [row] = await db
    .select({ workspaceId: projects.workspaceId })
    .from(projects)
    .where(eq(projects.id, projectId))
    .limit(1);

  const workspaceId = row?.workspaceId ?? null;
  projectWorkspaceCache.set(projectId, { workspaceId, fetchedAt: now });
  return workspaceId;
}

// ---------------------------------------------------------------------------
// Factory pipeline events
// ---------------------------------------------------------------------------

const _factorySub = new Subscription(FactoryEventTopic, "sync-outbox-relay", {
  handler: async (event) => {
    const workspaceId = await resolveWorkspaceId(event.project_id);
    if (!workspaceId) {
      log.warn("sync.relay: factory event with unknown project — skipping", {
        projectId: event.project_id,
        pipelineId: event.pipeline_id,
      });
      return;
    }

    await dispatchServerEvent(workspaceId, {
      kind: "factory.event",
      pipelineId: event.pipeline_id,
      projectId: event.project_id,
      eventType: event.event_type,
      stageId: event.stage_id,
      actor: event.actor,
      details: event.details,
    });
  },
});
