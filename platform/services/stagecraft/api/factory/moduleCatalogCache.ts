// Spec 227 (interim, post-Stage 1): short-TTL per-org cache for the derived
// module catalog. `loadModuleCatalogForOrg` runs a full substrate load plus an
// admission check; the catalog only changes when the org's factory origin
// re-syncs, so create/read paths should not pay that round-trip on every call
// (ai-review flagged the per-request substrate load on #533; spec 227 Stage 2
// optimization, pulled forward).
//
// Pure module (no Encore imports) so the TTL semantics are unit-testable
// without infra; the clock is injectable for the same reason. Mirrors the
// per-project cache idiom in `api/knowledge/extractionPolicy.ts`.

import type { ModuleDescriptor } from "../projects/scaffold/moduleCatalog";

export const MODULE_CATALOG_CACHE_TTL_MS = 60_000;

interface CacheEntry {
  catalog: ModuleDescriptor[];
  loadedAt: number;
}

const cache = new Map<string, CacheEntry>();

/**
 * Return the org's module catalog, memoized per org for a short TTL. Sequential
 * calls within the window reuse the cached derivation; overlapping cache misses
 * are not coalesced (each calls `load`), which is acceptable on this
 * low-frequency create/read path. The returned array is the shared cached
 * reference, so callers must treat it as read-only. `now` is injectable for
 * tests (defaults to `Date.now`).
 */
export async function getCachedModuleCatalog(
  orgId: string,
  load: () => Promise<ModuleDescriptor[]>,
  now: () => number = Date.now
): Promise<ModuleDescriptor[]> {
  const t = now();
  const hit = cache.get(orgId);
  if (hit && t - hit.loadedAt < MODULE_CATALOG_CACHE_TTL_MS) {
    return hit.catalog;
  }
  const catalog = await load();
  cache.set(orgId, { catalog, loadedAt: t });
  return catalog;
}

/** Visible for tests. */
export function clearModuleCatalogCache(): void {
  cache.clear();
}
