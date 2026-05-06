// Spec 112 §5.3 — scaffold warmup scheduler.
//
// Two Encore-native pieces wire the warmup machinery into stagecraft:
//   1. `runScaffoldWarmup` — internal endpoint that resolves a warmup
//      context (scaffoldRepoUrl + scaffoldRef + PAT) and runs the cache
//      + four prebuilds. Called at boot via fire-and-forget at module
//      load AND by a 30-min cron for upstream-SHA refresh.
//   2. `_scaffoldRefresher` — Encore CronJob that fires `runScaffoldWarmup`
//      every 30 minutes. Replaces template-distributor's setInterval.
//
// Multi-tenancy: stagecraft today resolves a single warmup context (the
// first org with both an adapter declaring `scaffold_source_id` resolving
// to a `factory_upstreams` row and a configured upstream PAT). Cache
// contents are public template files + npm node_modules — neither
// org-sensitive — so a shared cache is safe for MVP.
//
// Spec 140 §2.2 — the warmup resolver reads `manifest.scaffold_source_id`
// off each projected adapter and looks up `factory_upstreams (org_id,
// source_id)` for the actual `(repo_url, ref)`. URLs no longer ride on
// the manifest itself.

import { api } from "encore.dev/api";
import { CronJob } from "encore.dev/cron";
import log from "encore.dev/log";
import { and, eq } from "drizzle-orm";
import { db } from "../../db/drizzle";
import {
  factoryArtifactSubstrate,
  factoryUpstreams,
} from "../../db/schema";
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

export type WarmupResolution =
  | { kind: "ok"; ctx: ResolvedWarmupContext }
  | { kind: "no-adapters" }
  | { kind: "no-scaffold-source-id" }
  | { kind: "no-scaffold-source-resolved" }
  | { kind: "no-pat" };

const SHA40 = /^[0-9a-f]{40}$/i;
function refToBranch(ref: string): string {
  return SHA40.test(ref) ? "main" : ref;
}

/**
 * Spec 140 §2.2 — read `manifest.scaffold_source_id` and translate it
 * into a `factory_upstreams` row for the same org. Exported so the
 * scaffoldReadiness endpoint and tests can share the resolver.
 */
export async function resolveScaffoldUpstream(
  orgId: string,
  scaffoldSourceId: string,
): Promise<{ repoUrl: string; ref: string } | null> {
  const rows = await db
    .select({
      repoUrl: factoryUpstreams.repoUrl,
      ref: factoryUpstreams.ref,
    })
    .from(factoryUpstreams)
    .where(
      and(
        eq(factoryUpstreams.orgId, orgId),
        eq(factoryUpstreams.sourceId, scaffoldSourceId),
      ),
    )
    .limit(1);
  if (rows.length === 0) return null;
  const { repoUrl, ref } = rows[0];
  if (!repoUrl) return null;
  return { repoUrl, ref };
}

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

  let sawScaffoldSourceId = false;
  let sawResolvedSource = false;
  for (const { orgId } of orgRows) {
    const substrate = await loadSubstrateForOrg(orgId);
    const projection = projectSubstrateToLegacy(substrate);
    if (projection.adapters.length === 0) continue;
    for (const adapter of projection.adapters) {
      const manifest = (adapter.manifest ?? {}) as {
        scaffold_source_id?: unknown;
      };
      const scaffoldSourceId =
        typeof manifest.scaffold_source_id === "string"
          ? manifest.scaffold_source_id
          : null;
      if (!scaffoldSourceId) continue;
      sawScaffoldSourceId = true;

      const upstream = await resolveScaffoldUpstream(orgId, scaffoldSourceId);
      if (!upstream) continue;
      sawResolvedSource = true;

      const pat = await loadFactoryUpstreamPatToken(orgId).catch(() => null);
      if (!pat) continue;

      return {
        kind: "ok",
        ctx: {
          orgId,
          workspaceDir: defaultWorkspaceDir(),
          scaffoldRepoUrl: upstream.repoUrl,
          scaffoldRef: refToBranch(upstream.ref),
          patResolver: () => loadFactoryUpstreamPatToken(orgId),
        },
      };
    }
  }
  if (!sawScaffoldSourceId) return { kind: "no-scaffold-source-id" };
  if (!sawResolvedSource) return { kind: "no-scaffold-source-resolved" };
  return { kind: "no-pat" };
}

function reportUnresolvable(resolution: WarmupResolution): void {
  if (resolution.kind === "ok") return;
  const messages: Record<Exclude<WarmupResolution["kind"], "ok">, string> = {
    "no-adapters":
      "scaffold warmup: no factory adapter rows for any org — run /factory-sync to populate (spec 139 §7.2)",
    "no-scaffold-source-id":
      "scaffold warmup: factory adapters are present but none declare scaffold_source_id — re-run /factory-sync (spec 139 §7.2 / spec 140 §2.1)",
    "no-scaffold-source-resolved":
      "scaffold warmup: an adapter declares scaffold_source_id but no factory_upstreams row matches it — register the upstream at /app/factory/upstreams (spec 139 §7.2)",
    "no-pat":
      "scaffold warmup: an adapter and matching upstream are configured but no factory_upstream_pats row exists — configure a PAT at /app/admin/factory/pat",
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
      scaffoldRepoUrl: ctx.scaffoldRepoUrl,
      scaffoldRef: ctx.scaffoldRef,
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
