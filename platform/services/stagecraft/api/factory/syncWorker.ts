/**
 * Factory sync PubSub worker (spec 109 §5).
 *
 * Consumes FactorySyncRequestTopic. For each request:
 *   1. CAS-transition factory_sync_runs.status from 'pending' -> 'running'.
 *      If the row is already 'running' / 'ok' / 'failed' we skip: the
 *      worker is idempotent under at-least-once redelivery.
 *   2. Load the upstream config and resolve a token (PAT -> installation
 *      -> anonymous).
 *   3. Run the shared sync pipeline (clone + translate + upsert).
 *   4. Update the run row with status + shas + counts (or error).
 *   5. Mirror the final status onto factory_upstreams so the Overview page
 *      can keep reading the denormalised "current state" columns.
 */

import { Subscription } from "encore.dev/pubsub";
import log from "encore.dev/log";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  auditLog,
  factorySyncRuns,
  factoryUpstreams,
} from "../db/schema";
import { FactorySyncRequestTopic, type FactorySyncRequest } from "./events";
import { resolveFactoryUpstreamToken } from "./tokenResolver";
import { runSyncPipeline } from "./syncPipeline";

async function handleSyncRequest(req: FactorySyncRequest): Promise<void> {
  const startedAt = new Date();

  const claimed = await db
    .update(factorySyncRuns)
    .set({ status: "running", startedAt })
    .where(
      and(
        eq(factorySyncRuns.id, req.syncRunId),
        eq(factorySyncRuns.status, "pending")
      )
    )
    .returning({ id: factorySyncRuns.id });

  if (claimed.length === 0) {
    log.info("factory sync worker: run already claimed, skipping", {
      syncRunId: req.syncRunId,
      orgId: req.orgId,
    });
    return;
  }

  const [upstreamRow] = await db
    .select()
    .from(factoryUpstreams)
    .where(eq(factoryUpstreams.orgId, req.orgId))
    .limit(1);

  if (!upstreamRow) {
    await failRun(req, "No factory upstream configured for org");
    return;
  }

  await db
    .update(factoryUpstreams)
    .set({
      lastSyncStatus: "running",
      lastSyncError: null,
      updatedAt: startedAt,
    })
    .where(eq(factoryUpstreams.orgId, req.orgId));

  try {
    const resolved = await resolveFactoryUpstreamToken(req.orgId);
    const result = await runSyncPipeline({
      orgId: req.orgId,
      factorySource: upstreamRow.factorySource,
      factoryRef: upstreamRow.factoryRef,
      templateSource: upstreamRow.templateSource,
      templateRef: upstreamRow.templateRef,
      token: resolved?.token,
    });

    const completedAt = new Date();
    await db
      .update(factorySyncRuns)
      .set({
        status: "ok",
        factorySha: result.factorySha,
        templateSha: result.templateSha,
        counts: result.counts,
        completedAt,
      })
      .where(eq(factorySyncRuns.id, req.syncRunId));

    await db.insert(auditLog).values({
      actorUserId: req.triggeredBy,
      action: "factory.upstreams.sync_ok",
      targetType: "factory_sync_runs",
      targetId: req.syncRunId,
      metadata: {
        orgId: req.orgId,
        factorySha: result.factorySha,
        templateSha: result.templateSha,
        counts: result.counts,
        tokenSource: resolved?.source ?? "anonymous",
      },
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    log.error("factory sync worker: pipeline failed", {
      syncRunId: req.syncRunId,
      orgId: req.orgId,
      err: message,
    });
    await failRun(req, message);
  }
}

async function failRun(
  req: FactorySyncRequest,
  message: string
): Promise<void> {
  const completedAt = new Date();
  await db
    .update(factorySyncRuns)
    .set({ status: "failed", error: message, completedAt })
    .where(eq(factorySyncRuns.id, req.syncRunId));

  await db
    .update(factoryUpstreams)
    .set({
      lastSyncStatus: "failed",
      lastSyncError: message,
      updatedAt: completedAt,
    })
    .where(eq(factoryUpstreams.orgId, req.orgId));

  await db.insert(auditLog).values({
    actorUserId: req.triggeredBy,
    action: "factory.upstreams.sync_failed",
    targetType: "factory_sync_runs",
    targetId: req.syncRunId,
    metadata: { orgId: req.orgId, error: message },
  });
}

const _syncWorker = new Subscription(FactorySyncRequestTopic, "factory-sync-worker", {
  handler: handleSyncRequest,
});
void _syncWorker;
