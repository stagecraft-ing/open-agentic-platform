// Spec 112 Phase 5 — scaffold-readiness endpoint.
//
// Public read: returns the warmup state plus the per-org "do you have what
// you need to Create" preconditions (factory adapter on file, upstream PAT
// configured). The /app/projects/new loader calls this on every render so
// the form can banner clearly when something is missing instead of
// surfacing the generic "an internal error occurred" 500.

import { api } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import { factoryAdapters } from "../db/schema";
import { loadFactoryUpstreamPatToken } from "../factory/upstreamPat";
import { getInitStatus } from "./scaffold/templateCache";

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
   * Convenience flag — true iff every gate is green: warmup ready, adapter
   * available, PAT configured. The UI uses this to enable the submit button
   * without having to AND the three fields itself.
   */
  canCreate: boolean;
  /** First missing precondition, in resolution order — purely for banner copy. */
  blocker?:
    | "warming-up"
    | "warmup-error"
    | "no-factory-adapter"
    | "stale-adapter-manifest"
    | "no-upstream-pat";
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

    const adapterRows = await db
      .select({
        id: factoryAdapters.id,
        manifest: factoryAdapters.manifest,
      })
      .from(factoryAdapters)
      .where(eq(factoryAdapters.orgId, auth.orgId));
    const hasFactoryAdapter = adapterRows.length > 0;
    const hasTemplateRemote = adapterRows.some((row) => {
      const m = row.manifest as { template_remote?: unknown } | null;
      return typeof m?.template_remote === "string" && m.template_remote.length > 0;
    });

    const pat = await loadFactoryUpstreamPatToken(auth.orgId).catch(
      () => null
    );
    const hasUpstreamPat = Boolean(pat);

    let blocker: ScaffoldReadinessResponse["blocker"];
    if (!hasFactoryAdapter) blocker = "no-factory-adapter";
    else if (!hasTemplateRemote) blocker = "stale-adapter-manifest";
    else if (!hasUpstreamPat) blocker = "no-upstream-pat";
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
      canCreate:
        status.ready &&
        hasFactoryAdapter &&
        hasTemplateRemote &&
        hasUpstreamPat,
      blocker,
    };
  }
);
