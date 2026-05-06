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
import { and, eq, sql } from "drizzle-orm";
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
import {
  LEGACY_SINGLETON_SOURCE_ID,
  LEGACY_TEMPLATE_SOURCE_ID,
} from "./upstreams";

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

  // Spec 139 Phase 4b — factory_upstreams is N-per-org. The legacy
  // singleton wire shape composes from two rows: `legacy-mixed`
  // (factory side, role='mixed') and `legacy-template-mixed` (template
  // side, role='scaffold'). The four legacy per-side columns are
  // dropped in migration 35 — repo_url + ref are the canonical fields.
  const sideRows = await db
    .select()
    .from(factoryUpstreams)
    .where(
      and(
        eq(factoryUpstreams.orgId, req.orgId),
        sql`${factoryUpstreams.sourceId} IN (${LEGACY_SINGLETON_SOURCE_ID}, ${LEGACY_TEMPLATE_SOURCE_ID})`,
      ),
    );
  const factoryRow = sideRows.find(
    (r) => r.sourceId === LEGACY_SINGLETON_SOURCE_ID,
  );
  const templateRow = sideRows.find(
    (r) => r.sourceId === LEGACY_TEMPLATE_SOURCE_ID,
  );

  if (!factoryRow || !templateRow) {
    await failRun(
      req,
      "factory upstream not configured for org; reconfigure via POST /api/factory/upstreams",
    );
    return;
  }

  const factorySource = factoryRow.repoUrl;
  const factoryRef = factoryRow.ref;
  const templateSource = templateRow.repoUrl;
  const templateRef = templateRow.ref;

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
        eq(factoryUpstreams.sourceId, LEGACY_SINGLETON_SOURCE_ID),
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

    // Spec 140 §2.1 — a successful /factory-sync may have just produced
    // adapter rows whose projected manifest carries `scaffold_source_id`
    // resolving to a `factory_upstreams` row. Kick the scaffold warmup
    // immediately so the Create form unlocks without waiting for the
    // next 30-min cron tick.
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
        eq(factoryUpstreams.sourceId, LEGACY_SINGLETON_SOURCE_ID),
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
