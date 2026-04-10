/**
 * Connector sync scheduler (spec 087 Phase 4).
 *
 * Encore cron job that runs every 15 minutes and dispatches sync runs
 * for connectors that have a sync_schedule set and are due for a sync.
 *
 * The scheduler checks each active connector's sync_schedule (a simple
 * interval string) against its last_synced_at timestamp. If a connector
 * is due, it dispatches an async sync run via executeSyncRun().
 *
 * Supported sync_schedule values:
 *   "15m", "30m", "1h", "6h", "12h", "24h"
 */

import { api } from "encore.dev/api";
import { CronJob } from "encore.dev/cron";
import log from "encore.dev/log";
import { db } from "../db/drizzle";
import { sourceConnectors, syncRuns, workspaces } from "../db/schema";
import { and, eq, desc, isNotNull } from "drizzle-orm";
import { executeSyncRun } from "./knowledge";

// ---------------------------------------------------------------------------
// Schedule interval parser
// ---------------------------------------------------------------------------

const INTERVALS: Record<string, number> = {
  "15m": 15 * 60_000,
  "30m": 30 * 60_000,
  "1h": 60 * 60_000,
  "6h": 6 * 60 * 60_000,
  "12h": 12 * 60 * 60_000,
  "24h": 24 * 60 * 60_000,
};

function parseIntervalMs(schedule: string): number | null {
  return INTERVALS[schedule] ?? null;
}

// System user ID for scheduler-initiated syncs (not user-triggered)
const SYSTEM_USER_ID = "00000000-0000-0000-0000-000000000000";

// ---------------------------------------------------------------------------
// Cron endpoint — must be an api() endpoint for Encore CronJob
// ---------------------------------------------------------------------------

export const runScheduledSyncs = api(
  { expose: false, method: "POST", path: "/internal/knowledge/scheduled-sync" },
  async (): Promise<void> => {
    const now = Date.now();

    const connectors = await db
      .select()
      .from(sourceConnectors)
      .where(
        and(
          eq(sourceConnectors.status, "active"),
          isNotNull(sourceConnectors.syncSchedule)
        )
      );

    let dispatched = 0;

    for (const conn of connectors) {
      if (!conn.syncSchedule || conn.type === "upload") continue;

      const intervalMs = parseIntervalMs(conn.syncSchedule);
      if (!intervalMs) {
        log.warn("unknown sync_schedule", {
          connectorId: conn.id,
          schedule: conn.syncSchedule,
        });
        continue;
      }

      // Check if enough time has passed since the last sync
      const lastSync = conn.lastSyncedAt
        ? new Date(conn.lastSyncedAt).getTime()
        : 0;

      if (now - lastSync < intervalMs) continue;

      // Check if there's already a running sync for this connector
      const [running] = await db
        .select({ id: syncRuns.id })
        .from(syncRuns)
        .where(
          and(
            eq(syncRuns.connectorId, conn.id),
            eq(syncRuns.status, "running")
          )
        )
        .limit(1);

      if (running) continue;

      // Resolve workspace bucket
      const [ws] = await db
        .select({ objectStoreBucket: workspaces.objectStoreBucket })
        .from(workspaces)
        .where(eq(workspaces.id, conn.workspaceId))
        .limit(1);

      if (!ws) continue;

      // Get the last successful delta token
      const [lastRun] = await db
        .select({ deltaToken: syncRuns.deltaToken })
        .from(syncRuns)
        .where(
          and(
            eq(syncRuns.connectorId, conn.id),
            eq(syncRuns.status, "completed")
          )
        )
        .orderBy(desc(syncRuns.completedAt))
        .limit(1);

      try {
        await executeSyncRun(
          conn.id,
          conn.workspaceId,
          ws.objectStoreBucket,
          conn.type,
          (conn.configEncrypted as Record<string, unknown>) ?? {},
          lastRun?.deltaToken ?? null,
          SYSTEM_USER_ID
        );
        dispatched++;
      } catch (err) {
        log.error("failed to dispatch scheduled sync", {
          connectorId: conn.id,
          error: err instanceof Error ? err.message : String(err),
        });
      }
    }

    if (dispatched > 0) {
      log.info("scheduled syncs dispatched", { count: dispatched });
    }
  }
);

// ---------------------------------------------------------------------------
// Register the cron job — runs every 15 minutes
// ---------------------------------------------------------------------------

const _ = new CronJob("connector-sync-scheduler", {
  title: "Connector Sync Scheduler",
  every: "15m",
  endpoint: runScheduledSyncs,
});
