// Spec 227 Stage 1 (catalog derivation): serve the create-project feature
// module catalog as a projection of the adapter's own module manifests, not a
// hand-mirrored constant. The module `manifest.json` files sync into the
// substrate under `adapters/<name>/modules/<id>/manifest.json` (the factory
// source mirrors `adapters/**`, translator.ts); they land in the catch-all
// `reference-data` kind, so this filters by PATH rather than kind. Admission
// gated via browse.ts's shared `loadOrgView`/`servableRows` so an org whose
// factory origin is not admitted gets no catalog (fail-closed), the same
// posture as the adapter/contract/process browsers (spec 198 FR-001).

import { api } from "encore.dev/api";
import log from "encore.dev/log";
import { getAuthData } from "~encore/auth";
import { loadOrgView, servableRows } from "./browse";
import type { SubstrateRowDraft } from "./translator";
import {
  deriveModuleCatalog,
  isModuleManifestPath,
  type ModuleDescriptor,
  type RawModuleManifest,
} from "../projects/scaffold/moduleCatalog";
import { getCachedModuleCatalog } from "./moduleCatalogCache";

/** Substrate rows that are an adapter module's `manifest.json`. */
export function moduleManifestRows(
  rows: SubstrateRowDraft[]
): SubstrateRowDraft[] {
  return rows.filter((r) => isModuleManifestPath(r.path));
}

/**
 * Derive the org's feature-module catalog from its admitted substrate. Parses
 * each module `manifest.json` body as JSON; a body that fails to parse is
 * skipped with a warning rather than failing the whole catalog.
 */
async function loadModuleCatalogUncached(
  orgId: string
): Promise<ModuleDescriptor[]> {
  const view = await loadOrgView(orgId);
  const manifests: RawModuleManifest[] = [];
  for (const row of moduleManifestRows(servableRows(view))) {
    try {
      const parsed = JSON.parse(row.upstreamBody) as RawModuleManifest;
      if (parsed && typeof parsed.name === "string") {
        manifests.push(parsed);
      }
    } catch (err) {
      log.warn("loadModuleCatalogForOrg: unparseable module manifest skipped", {
        orgId,
        path: row.path,
        cause: err instanceof Error ? err.message : String(err),
      });
    }
  }
  return deriveModuleCatalog(manifests);
}

/**
 * Cached front door for the org's feature-module catalog. The catalog changes
 * only when the org's factory origin re-syncs, so a short-TTL per-org cache
 * (see moduleCatalogCache) lets create/read paths reuse a recent derivation
 * instead of a fresh substrate load + admission check on every call (ai-review
 * on #533; spec 227 Stage 2 optimization).
 */
export async function loadModuleCatalogForOrg(
  orgId: string
): Promise<ModuleDescriptor[]> {
  return getCachedModuleCatalog(orgId, () => loadModuleCatalogUncached(orgId));
}

export const getModuleCatalog = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/factory/module-catalog",
  },
  async (): Promise<{ modules: ModuleDescriptor[] }> => {
    const auth = getAuthData()!;
    return { modules: await loadModuleCatalogForOrg(auth.orgId) };
  },
);
