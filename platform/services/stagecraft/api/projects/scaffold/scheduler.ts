// Spec 112 §5.3 — scaffold warmup scheduler.
//
// Two Encore-native pieces wire the warmup machinery into stagecraft:
//   1. `runScaffoldWarmup` — internal endpoint that resolves a warmup
//      context (templateRemote + branch + PAT) and runs the cache + four
//      prebuilds. Called at boot via fire-and-forget at module load AND
//      by a 30-min cron for upstream-SHA refresh.
//   2. `_scaffoldRefresher` — Encore CronJob that fires `runScaffoldWarmup`
//      every 30 minutes. Replaces template-distributor's setInterval.
//
// Multi-tenancy: stagecraft today resolves a single warmup context (the
// first org with both an adapter declaring `template_remote` and a
// configured upstream PAT). Cache contents are public template files +
// npm node_modules — neither org-sensitive — so a shared cache is safe
// for MVP. Per-org caching can layer on later by keying off
// `factory_adapters.source_sha`.

import { api } from "encore.dev/api";
import { CronJob } from "encore.dev/cron";
import log from "encore.dev/log";
import { db } from "../../db/drizzle";
import { factoryAdapters } from "../../db/schema";
import { loadFactoryUpstreamPatToken } from "../../factory/upstreamPat";
import {
  defaultWorkspaceDir,
  runWarmup,
  startBackgroundRefresher,
  type WarmupContext,
} from "./templateCache";

interface ResolvedWarmupContext extends WarmupContext {
  /** Surfaced for log lines / readiness reports. */
  orgId: string;
}

async function resolveWarmupContext(): Promise<ResolvedWarmupContext | null> {
  const rows = await db
    .select({
      orgId: factoryAdapters.orgId,
      manifest: factoryAdapters.manifest,
    })
    .from(factoryAdapters);

  for (const row of rows) {
    const manifest = row.manifest as {
      template_remote?: string;
      template_default_branch?: string;
    };
    if (!manifest?.template_remote) continue;

    const pat = await loadFactoryUpstreamPatToken(row.orgId).catch(() => null);
    if (!pat) continue;

    const orgId = row.orgId;
    return {
      orgId,
      workspaceDir: defaultWorkspaceDir(),
      templateRemote: manifest.template_remote,
      defaultBranch: manifest.template_default_branch ?? "main",
      patResolver: () => loadFactoryUpstreamPatToken(orgId),
    };
  }
  return null;
}

export const runScaffoldWarmup = api(
  { expose: false, method: "POST", path: "/internal/scaffold/warmup" },
  async (): Promise<void> => {
    const ctx = await resolveWarmupContext();
    if (!ctx) {
      log.info(
        "scaffold warmup: no eligible org — need an adapter with template_remote and a configured factory upstream PAT"
      );
      return;
    }
    log.info("scaffold warmup: starting", {
      orgId: ctx.orgId,
      templateRemote: ctx.templateRemote,
      branch: ctx.defaultBranch,
    });
    await runWarmup(ctx);
    // Idempotent — first call wires the in-process refresher; subsequent
    // calls are no-ops. Belt-and-braces with the CronJob below: the
    // setInterval keeps refreshing if the cron is unavailable in dev.
    startBackgroundRefresher(ctx);
  }
);

// Fire-and-forget initial warmup at module load. Encore handlers must
// remain responsive while warmup runs; the CronJob's first fire is up to
// 30 min out, which is too slow for the post-deploy "first user creates
// a project" flow.
void (async () => {
  try {
    const ctx = await resolveWarmupContext();
    if (!ctx) return;
    log.info("scaffold warmup: kicking off at module load", {
      orgId: ctx.orgId,
    });
    await runWarmup(ctx);
    startBackgroundRefresher(ctx);
  } catch (err) {
    log.warn("initial scaffold warmup failed", {
      error: err instanceof Error ? err.message : String(err),
    });
  }
})();

const _scaffoldRefresher = new CronJob("scaffold-warmup-refresher", {
  title: "Scaffold Warmup Refresher",
  every: "30m",
  endpoint: runScaffoldWarmup,
});
void _scaffoldRefresher;
