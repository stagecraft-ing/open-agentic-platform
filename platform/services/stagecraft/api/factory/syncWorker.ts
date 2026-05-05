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
import { runScaffoldWarmup } from "../projects/scaffold/scheduler";

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

  // Spec 139 generalised factory_upstreams to N-per-org. The legacy
  // singleton row (which carried both factory + template sources) is
  // tagged source_id='legacy-mixed' / role='mixed' (migration 32). The
  // sync worker keeps reading the legacy shape through Phase 1; Phase 4
  // drops the legacy columns when consumers migrate.
  const [upstreamRow] = await db
    .select()
    .from(factoryUpstreams)
    .where(
      and(
        eq(factoryUpstreams.orgId, req.orgId),
        eq(factoryUpstreams.sourceId, "legacy-mixed"),
      ),
    )
    .limit(1);

  if (!upstreamRow) {
    await failRun(req, "No factory upstream configured for org");
    return;
  }

  // Spec 139 made the legacy columns nullable to permit the dual-write
  // transition. The legacy-mixed row must still carry all four for the
  // sync worker to do meaningful work — surface the gap clearly.
  if (
    !upstreamRow.factorySource ||
    !upstreamRow.factoryRef ||
    !upstreamRow.templateSource ||
    !upstreamRow.templateRef
  ) {
    await failRun(
      req,
      "legacy-mixed upstream row is missing factorySource/factoryRef/templateSource/templateRef; reconfigure via POST /api/factory/upstreams",
    );
    return;
  }
  const factorySource = upstreamRow.factorySource;
  const factoryRef = upstreamRow.factoryRef;
  const templateSource = upstreamRow.templateSource;
  const templateRef = upstreamRow.templateRef;

  await db
    .update(factoryUpstreams)
    .set({
      lastSyncStatus: "running",
      lastSyncError: null,
      updatedAt: startedAt,
    })
    .where(
      and(
        eq(factoryUpstreams.orgId, req.orgId),
        eq(factoryUpstreams.sourceId, "legacy-mixed"),
      ),
    );

  try {
    const resolved = await resolveFactoryUpstreamToken(req.orgId);
    const result = await runSyncPipeline({
      orgId: req.orgId,
      factorySource,
      factoryRef,
      templateSource,
      templateRef,
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

    // Spec 138 §2.1 — a successful /factory-sync may have just stamped
    // template_remote on previously-unmanaged adapter rows. Kick the
    // scaffold warmup immediately so the Create form unlocks without
    // waiting for the next 30-min cron tick.
    void runScaffoldWarmup().catch((err) => {
      log.warn("factory sync worker: post-sync warmup trigger failed", {
        syncRunId: req.syncRunId,
        orgId: req.orgId,
        err: err instanceof Error ? err.message : String(err),
      });
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
    .where(
      and(
        eq(factoryUpstreams.orgId, req.orgId),
        eq(factoryUpstreams.sourceId, "legacy-mixed"),
      ),
    );

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
