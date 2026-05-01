// Spec 124 §6 / Phase 6 — `factory_runs` staleness sweeper.
//
// Mirrors `api/knowledge/scheduler.ts::runExtractionStalenessSweep`
// (spec 115 FR-006). When a desktop dies mid-run the row stays in
// `running` (or `queued`, if the desktop never followed through with a
// stage_started) — the sweeper is the recovery loop that flips those
// rows to `failed` so the UI does not show a permanently-stuck run.
//
// Staleness signal: `factory_runs.last_event_at` is bumped on every
// `factory.run.*` envelope handled by the duplex handlers (Phase 3). A
// row whose `last_event_at` is older than
// `STAGECRAFT_FACTORY_RUN_STALE_AFTER_SEC` (default 1800s, i.e. 30 min)
// is presumed dead.
//
// Bias: prefer false-positive failure to false-positive aliveness. A
// legitimately-slow run that exceeds the timeout is recorded `failed` —
// the user can re-run; a permanent `running` row would be misinformation
// every time the operator opened the Runs tab.

import { api } from "encore.dev/api";
import { CronJob } from "encore.dev/cron";
import log from "encore.dev/log";
import { and, eq, inArray, sql } from "drizzle-orm";
import { db } from "../db/drizzle";
import { factoryRuns, auditLog } from "../db/schema";
import { FACTORY_RUN_SWEPT } from "./auditActions";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Default staleness window. The desktop emits a stage_started or
 *  stage_completed at least once per stage; a run that's been silent for
 *  half an hour is overwhelmingly likely to be the desktop having died. */
const DEFAULT_STALE_AFTER_SEC = 30 * 60;

/** Override via `STAGECRAFT_FACTORY_RUN_STALE_AFTER_SEC=<seconds>`. Used
 *  by integration tests that compress the window down to one-digit
 *  seconds; documented in stagecraft/CLAUDE.md alongside spec 115's
 *  knobs. */
const ENV_STALE_AFTER_SEC = "STAGECRAFT_FACTORY_RUN_STALE_AFTER_SEC";

/** spec 119 §2 seed migration `2_seed_system_user`. The sweeper is a
 *  server-side cron, not a user action — every audit row it emits MUST
 *  be authored by the system user. Hardcoding the UUID here keeps the
 *  sweeper independent of the seed-state of arbitrary test fixtures. */
const SYSTEM_USER_ID = "00000000-0000-0000-0000-000000000000";

function staleAfterSeconds(): number {
  const raw = process.env[ENV_STALE_AFTER_SEC];
  if (!raw) return DEFAULT_STALE_AFTER_SEC;
  const n = Number.parseInt(raw, 10);
  return Number.isFinite(n) && n > 0 ? n : DEFAULT_STALE_AFTER_SEC;
}

// ---------------------------------------------------------------------------
// Core — exported for direct test coverage (T062). The cron endpoint
// below is the production caller.
// ---------------------------------------------------------------------------

export interface SweepResult {
  swept: number;
  /** Per-row IDs that were flipped to `failed`. Mainly useful for tests
   *  asserting "this specific run got swept" without re-querying the DB. */
  ids: string[];
}

/**
 * Find every `factory_runs` row in `(queued, running)` whose
 * `last_event_at` is older than the staleness cutoff and flip it to
 * `failed`. Emits a `factory.run.swept` audit row per swept run under
 * the system user.
 *
 * `now` is injectable so tests can drive deterministic windows without
 * waiting for wall clock.
 */
export async function sweepStaleFactoryRuns(
  now: Date = new Date(),
): Promise<SweepResult> {
  const staleAfter = staleAfterSeconds();
  const cutoff = new Date(now.getTime() - staleAfter * 1000);

  const stale = await db
    .select({
      id: factoryRuns.id,
      orgId: factoryRuns.orgId,
      adapterId: factoryRuns.adapterId,
      processId: factoryRuns.processId,
      status: factoryRuns.status,
      clientRunId: factoryRuns.clientRunId,
    })
    .from(factoryRuns)
    .where(
      and(
        inArray(factoryRuns.status, ["queued", "running"]),
        sql`${factoryRuns.lastEventAt} < ${cutoff}`,
      ),
    );

  if (stale.length === 0) {
    return { swept: 0, ids: [] };
  }

  const errorMessage = `sweeper: no events for ${staleAfter}s`;
  const ids: string[] = [];

  // One transaction per row — keeps the sweep loop interruptible
  // (a transient DB hiccup loses one row, not the whole batch) and
  // mirrors the spec 115 sweeper's row-at-a-time discipline.
  for (const row of stale) {
    try {
      await db.transaction(async (tx) => {
        await tx
          .update(factoryRuns)
          .set({
            status: "failed",
            error: errorMessage,
            completedAt: now,
            lastEventAt: now,
          })
          .where(
            and(
              eq(factoryRuns.id, row.id),
              // Re-check status under the transaction so we don't race a
              // duplex handler that just landed a terminal-state event.
              inArray(factoryRuns.status, ["queued", "running"]),
            ),
          );

        await tx.insert(auditLog).values({
          actorUserId: SYSTEM_USER_ID,
          action: FACTORY_RUN_SWEPT,
          targetType: "factory_runs",
          targetId: row.id,
          metadata: {
            orgId: row.orgId,
            adapterId: row.adapterId,
            processId: row.processId,
            clientRunId: row.clientRunId,
            statusBeforeSweep: row.status,
            staleAfterSec: staleAfter,
            sweptAt: now.toISOString(),
          },
        });
      });
      ids.push(row.id);
    } catch (err) {
      // Do NOT abort the loop on a per-row failure — the next cron tick
      // will retry. Surface the error in logs so operators can spot a
      // systemic issue (e.g. an audit_log FK violation) early.
      log.error("sweepStaleFactoryRuns: per-row sweep failed", {
        runId: row.id,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  log.warn("sweepStaleFactoryRuns: recovered runs", {
    swept: ids.length,
    staleAfterSec: staleAfter,
  });
  return { swept: ids.length, ids };
}

// ---------------------------------------------------------------------------
// Encore endpoint — required for `CronJob.endpoint` (must be an `api()`
// handler). Internal-only (`expose: false`).
// ---------------------------------------------------------------------------

export const runFactoryRunsStalenessSweep = api(
  {
    expose: false,
    method: "POST",
    path: "/internal/factory/runs-staleness-sweep",
  },
  async (): Promise<void> => {
    try {
      const result = await sweepStaleFactoryRuns();
      if (result.swept > 0) {
        log.info("factory.runs staleness sweep: rows recovered", {
          swept: result.swept,
        });
      }
    } catch (err) {
      log.error("factory.runs staleness sweep failed", {
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },
);

// ---------------------------------------------------------------------------
// Cron registration — runs every minute, mirrors spec 115's cadence.
// ---------------------------------------------------------------------------

const _factoryRunsSweeper = new CronJob("factory-runs-staleness-sweeper", {
  title: "Factory Runs Staleness Sweeper",
  every: "1m",
  endpoint: runFactoryRunsStalenessSweep,
});
void _factoryRunsSweeper;
