// Spec 112 Phase 5 — scaffold-readiness endpoint.
// Spec 139 Phase 2 (T056) — extended with `scaffold_source_resolved` per
// adapter so the silent-reject path described in spec 112 §5.4 becomes an
// explicit blocker.
//
// Public read: returns the warmup state plus the per-org "do you have what
// you need to Create" preconditions (factory adapter on file, upstream PAT
// configured, scaffold source resolved per spec 139 §7.2). The
// /app/projects/new loader calls this on every render so the form can
// banner clearly when something is missing instead of surfacing the
// generic "an internal error occurred" 500.

import { api } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { and, eq, inArray } from "drizzle-orm";
import { db } from "../db/drizzle";
import { factoryUpstreams } from "../db/schema";
import { loadFactoryUpstreamPatToken } from "../factory/upstreamPat";
import { loadSubstrateForOrg } from "../factory/substrateBrowser";
import { projectSubstrateToLegacy } from "../factory/projection";
import { getInitStatus } from "./scaffold/templateCache";

export type AdapterReadinessVerdict = {
  /** Adapter row id (matches factory_adapters.id). */
  id: string;
  /** Adapter name (e.g. `aim-vue-node`, `next-prisma`). */
  name: string;
  /** True iff the manifest declares `scaffold_source_id`. */
  declaresScaffoldSource: boolean;
  /** True iff `scaffold_source_id` resolves to a `factory_upstreams` row. */
  scaffoldSourceResolved: boolean;
  /**
   * Spec 138 legacy: true iff `template_remote` is set. Combined with
   * `scaffoldSourceResolved` to evaluate Create-eligibility — adapters
   * synced before spec 139 carry `template_remote` instead of
   * `scaffold_source_id`; both satisfy the spec 139 §7.2 contract during
   * the transition window.
   */
  hasTemplateRemote: boolean;
  /** True iff this adapter alone is Create-eligible. */
  createEligible: boolean;
};

export interface ScaffoldReadinessResponse {
  ready: boolean;
  step: string;
  progress: number;
  error?: string;
  hasFactoryAdapter: boolean;
  hasUpstreamPat: boolean;
  /**
   * True iff at least one of the org's factory_adapters carries
   * `template_remote` in its manifest. Existing adapter rows synced
   * before spec 138 §2.1 lack the field; surfacing this distinct from
   * `hasFactoryAdapter` lets the UI banner say "re-run /factory-sync"
   * instead of "no adapter".
   */
  hasTemplateRemote: boolean;
  /**
   * Spec 139 §7.2 — true iff at least one adapter has its
   * `scaffold_source_id` resolved to a `factory_upstreams` row in the
   * caller's org. Adapters that still rely on legacy `template_remote`
   * (e.g. aim-vue-node pre-Phase 2) are counted as resolved when
   * `template_remote` is set so the existing flow doesn't regress.
   */
  scaffoldSourceResolved: boolean;
  /**
   * Per-adapter eligibility verdicts. The web Create form renders one
   * row per adapter so users see WHY a particular pick is greyed out.
   */
  adapters: AdapterReadinessVerdict[];
  /**
   * Convenience flag — true iff every gate is green: warmup ready, at
   * least one adapter is Create-eligible, PAT configured. The UI uses
   * this to enable the submit button without AND-ing the three fields
   * itself.
   */
  canCreate: boolean;
  /** First missing precondition, in resolution order — purely for banner copy. */
  blocker?:
    | "warming-up"
    | "warmup-error"
    | "no-factory-adapter"
    | "stale-adapter-manifest"
    | "no-scaffold-source-resolved"
    | "no-upstream-pat";
}

type AdapterManifest = {
  template_remote?: unknown;
  scaffold_source_id?: unknown;
} & Record<string, unknown>;

/** Spec 139 Phase 4 — must match `browse.ts::synthesiseId`. */
function synthesiseAdapterId(orgId: string, name: string): string {
  return `synthetic-adapter-${orgId.slice(0, 8)}-${name}`;
}

function readManifestStringField(
  manifest: AdapterManifest | null,
  field: "template_remote" | "scaffold_source_id",
): string | null {
  const v = manifest?.[field];
  return typeof v === "string" && v.length > 0 ? v : null;
}

export const scaffoldReadiness = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/projects/scaffold-readiness",
  },
  async (): Promise<ScaffoldReadinessResponse> => {
    const auth = getAuthData()!;
    const status = getInitStatus();

    // Spec 139 Phase 4 (T091): adapter manifests project from substrate.
    const substrate = await loadSubstrateForOrg(auth.orgId);
    const projection = projectSubstrateToLegacy(substrate);
    const adapterRows = projection.adapters.map((a) => ({
      id: synthesiseAdapterId(auth.orgId, a.name),
      name: a.name,
      manifest: a.manifest as AdapterManifest | null,
    }));
    const hasFactoryAdapter = adapterRows.length > 0;

    // Collect all distinct scaffold_source_ids declared across the org's
    // adapters, then resolve them in one round-trip against
    // `factory_upstreams` (composite-PK on (org_id, source_id) post-spec 139).
    const declaredSourceIds = new Set<string>();
    for (const row of adapterRows) {
      const sid = readManifestStringField(
        row.manifest as AdapterManifest | null,
        "scaffold_source_id",
      );
      if (sid) declaredSourceIds.add(sid);
    }
    const resolvedSourceIds = new Set<string>();
    if (declaredSourceIds.size > 0) {
      const resolvedRows = await db
        .select({ sourceId: factoryUpstreams.sourceId })
        .from(factoryUpstreams)
        .where(
          and(
            eq(factoryUpstreams.orgId, auth.orgId),
            inArray(factoryUpstreams.sourceId, [...declaredSourceIds]),
          ),
        );
      for (const r of resolvedRows) {
        resolvedSourceIds.add(r.sourceId);
      }
    }

    const adapters: AdapterReadinessVerdict[] = adapterRows.map((row) => {
      const manifest = row.manifest as AdapterManifest | null;
      const scaffoldSourceId = readManifestStringField(
        manifest,
        "scaffold_source_id",
      );
      const declaresScaffoldSource = scaffoldSourceId !== null;
      const scaffoldSourceResolved =
        scaffoldSourceId !== null && resolvedSourceIds.has(scaffoldSourceId);
      const hasTemplateRemote =
        readManifestStringField(manifest, "template_remote") !== null;
      // Create-eligible iff EITHER the legacy `template_remote` is set
      // (transition window) OR the new `scaffold_source_id` resolves to a
      // factory_upstreams row.
      const createEligible = hasTemplateRemote || scaffoldSourceResolved;
      return {
        id: row.id,
        name: row.name,
        declaresScaffoldSource,
        scaffoldSourceResolved,
        hasTemplateRemote,
        createEligible,
      };
    });

    const hasTemplateRemote = adapters.some((a) => a.hasTemplateRemote);
    const scaffoldSourceResolved = adapters.some(
      (a) => a.scaffoldSourceResolved,
    );
    const anyEligibleAdapter = adapters.some((a) => a.createEligible);

    const pat = await loadFactoryUpstreamPatToken(auth.orgId).catch(
      () => null,
    );
    const hasUpstreamPat = Boolean(pat);

    let blocker: ScaffoldReadinessResponse["blocker"];
    if (!hasFactoryAdapter) blocker = "no-factory-adapter";
    else if (!hasTemplateRemote && !scaffoldSourceResolved) {
      // Spec 138 stale manifest is the legacy blocker; spec 139's
      // scaffold-source-resolved is the new one. If neither is satisfied
      // by any adapter, surface the more informative message based on
      // whether any adapter declares scaffold_source_id at all.
      blocker = adapters.some((a) => a.declaresScaffoldSource)
        ? "no-scaffold-source-resolved"
        : "stale-adapter-manifest";
    } else if (!anyEligibleAdapter) {
      blocker = "no-scaffold-source-resolved";
    } else if (!hasUpstreamPat) blocker = "no-upstream-pat";
    else if (status.error) blocker = "warmup-error";
    else if (!status.ready) blocker = "warming-up";

    return {
      ready: status.ready,
      step: status.step,
      progress: status.progress,
      error: status.error,
      hasFactoryAdapter,
      hasUpstreamPat,
      hasTemplateRemote,
      scaffoldSourceResolved,
      adapters,
      canCreate:
        status.ready &&
        hasFactoryAdapter &&
        anyEligibleAdapter &&
        hasUpstreamPat,
      blocker,
    };
  },
);
