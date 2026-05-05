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
import { eq } from "drizzle-orm";
import { db } from "../../db/drizzle";
import { factoryArtifactSubstrate } from "../../db/schema";
import { loadFactoryUpstreamPatToken } from "../../factory/upstreamPat";
import { loadSubstrateForOrg } from "../../factory/substrateBrowser";
import { projectSubstrateToLegacy } from "../../factory/projection";
import {
  defaultWorkspaceDir,
  runWarmup,
  setInitErrorFromContext,
  startBackgroundRefresher,
  type WarmupContext,
} from "./templateCache";

interface ResolvedWarmupContext extends WarmupContext {
  /** Surfaced for log lines / readiness reports. */
  orgId: string;
}

type WarmupResolution =
  | { kind: "ok"; ctx: ResolvedWarmupContext }
  | { kind: "no-adapters" }
  | { kind: "no-template-remote" }
  | { kind: "no-pat" };

async function resolveWarmupContext(): Promise<WarmupResolution> {
  // Spec 139 Phase 4 (T091): adapter manifests project from
  // `factory_artifact_substrate`. The warmup iterates all orgs that
  // have any substrate row (i.e. completed at least one sync) and for
  // each projects the latest adapter manifest set.
  const orgRows = await db
    .selectDistinctOn([factoryArtifactSubstrate.orgId], {
      orgId: factoryArtifactSubstrate.orgId,
    })
    .from(factoryArtifactSubstrate)
    .where(eq(factoryArtifactSubstrate.status, "active"));

  if (orgRows.length === 0) return { kind: "no-adapters" };

  let sawTemplateRemote = false;
  for (const { orgId } of orgRows) {
    const substrate = await loadSubstrateForOrg(orgId);
    const projection = projectSubstrateToLegacy(substrate);
    if (projection.adapters.length === 0) continue;
    for (const adapter of projection.adapters) {
      const manifest = (adapter.manifest ?? {}) as {
        template_remote?: string;
        template_default_branch?: string;
      };
      if (!manifest.template_remote) continue;
      sawTemplateRemote = true;

      const pat = await loadFactoryUpstreamPatToken(orgId).catch(() => null);
      if (!pat) continue;

      return {
        kind: "ok",
        ctx: {
          orgId,
          workspaceDir: defaultWorkspaceDir(),
          templateRemote: manifest.template_remote,
          defaultBranch: manifest.template_default_branch ?? "main",
          patResolver: () => loadFactoryUpstreamPatToken(orgId),
        },
      };
    }
  }
  return sawTemplateRemote ? { kind: "no-pat" } : { kind: "no-template-remote" };
}

function reportUnresolvable(resolution: WarmupResolution): void {
  if (resolution.kind === "ok") return;
  const messages: Record<Exclude<WarmupResolution["kind"], "ok">, string> = {
    "no-adapters":
      "scaffold warmup: no factory_adapters rows for any org — run /factory-sync to populate",
    "no-template-remote":
      "scaffold warmup: factory_adapters rows are present but none declare template_remote — re-run /factory-sync (translator was upgraded in spec 138 §2.1; existing rows predate the change)",
    "no-pat":
      "scaffold warmup: factory_adapters carry template_remote but no factory_upstream_pats row — configure a PAT at /app/admin/factory/pat",
  };
  const reason = messages[resolution.kind];
  log.info(reason);
  setInitErrorFromContext(reason);
}

export const runScaffoldWarmup = api(
  { expose: false, method: "POST", path: "/internal/scaffold/warmup" },
  async (): Promise<void> => {
    const resolution = await resolveWarmupContext();
    if (resolution.kind !== "ok") {
      reportUnresolvable(resolution);
      return;
    }
    const ctx = resolution.ctx;
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
    const resolution = await resolveWarmupContext();
    if (resolution.kind !== "ok") {
      reportUnresolvable(resolution);
      return;
    }
    const ctx = resolution.ctx;
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
