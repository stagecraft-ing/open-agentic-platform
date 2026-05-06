// Spec 112 Phase 5 — scaffold-readiness endpoint.
// Spec 139 Phase 2 (T056) — extended with `scaffold_source_resolved`
//   per adapter so the silent-reject path described in spec 112 §5.4
//   becomes an explicit blocker.
// Spec 140 Phase 3 (§2.3 / T060) — legacy `template_remote` fallback
//   removed. Create-eligibility is now purely a function of
//   `scaffold_source_id` resolving to a `factory_upstreams` row.
//
// Public read: returns the warmup state plus the per-org "do you have
// what you need to Create" preconditions (factory adapter on file,
// upstream PAT configured, scaffold source resolved per spec 139 §7.2).
// The /app/projects/new loader calls this on every render so the form
// can banner clearly when something is missing instead of surfacing the
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
import {
  resolveBlocker,
  type ScaffoldReadinessBlocker,
} from "./scaffoldReadinessBlocker";

export type {
  BlockerInputs,
  ScaffoldReadinessBlocker,
} from "./scaffoldReadinessBlocker";
export { resolveBlocker } from "./scaffoldReadinessBlocker";

export type AdapterReadinessVerdict = {
  /** Adapter row id (matches factory_adapters.id). */
  id: string;
  /** Adapter name (e.g. `aim-vue-node`, `next-prisma`). */
  name: string;
  /** True iff the manifest declares `scaffold_source_id`. */
  declaresScaffoldSource: boolean;
  /** True iff `scaffold_source_id` resolves to a `factory_upstreams` row. */
  scaffoldSourceResolved: boolean;
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
   * Spec 139 §7.2 — true iff at least one adapter has its
   * `scaffold_source_id` resolved to a `factory_upstreams` row in the
   * caller's org.
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
  blocker?: ScaffoldReadinessBlocker;
}

type AdapterManifest = {
  scaffold_source_id?: unknown;
} & Record<string, unknown>;

/** Spec 139 Phase 4 — must match `browse.ts::synthesiseId`. */
function synthesiseAdapterId(orgId: string, name: string): string {
  return `synthetic-adapter-${orgId.slice(0, 8)}-${name}`;
}

function readScaffoldSourceId(
  manifest: AdapterManifest | null,
): string | null {
  const v = manifest?.scaffold_source_id;
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
      const sid = readScaffoldSourceId(row.manifest);
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
      const scaffoldSourceId = readScaffoldSourceId(row.manifest);
      const declaresScaffoldSource = scaffoldSourceId !== null;
      const scaffoldSourceResolved =
        scaffoldSourceId !== null && resolvedSourceIds.has(scaffoldSourceId);
      // Spec 140 §2.3 — Create-eligibility is purely
      // `scaffoldSourceResolved`. The legacy `template_remote` fallback
      // is gone.
      const createEligible = scaffoldSourceResolved;
      return {
        id: row.id,
        name: row.name,
        declaresScaffoldSource,
        scaffoldSourceResolved,
        createEligible,
      };
    });

    const scaffoldSourceResolved = adapters.some(
      (a) => a.scaffoldSourceResolved,
    );
    const anyEligibleAdapter = adapters.some((a) => a.createEligible);

    const pat = await loadFactoryUpstreamPatToken(auth.orgId).catch(
      () => null,
    );
    const hasUpstreamPat = Boolean(pat);

    const blocker = resolveBlocker({
      hasFactoryAdapter,
      anyDeclaresScaffoldSource: adapters.some((a) => a.declaresScaffoldSource),
      scaffoldSourceResolved,
      hasUpstreamPat,
      warmupReady: status.ready,
      warmupError: status.error,
    });

    return {
      ready: status.ready,
      step: status.step,
      progress: status.progress,
      error: status.error,
      hasFactoryAdapter,
      hasUpstreamPat,
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
