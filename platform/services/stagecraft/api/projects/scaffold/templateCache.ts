// Spec 112 §5.3 operation 1 — template cache clone + npm install + upstream-SHA refresh.
//
// The cache key is `factory_adapters.source_sha` (immutable per snapshot).
// A cache hit returns the on-disk path; a miss clones the adapter's
// upstream source repo, runs `npm install`, and records the resolved
// commit SHA against the cache key. The cache is per-workspace so that
// workspace-scoped org identity/secrets never cross tenants.

import { join } from "node:path";
import { mkdir, access, writeFile, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";

export interface TemplateCacheEntry {
  /** Absolute path to the cached tree. Directory is read-only for callers. */
  path: string;
  /** Resolved git commit SHA (same as adapter.sourceSha on cache hit). */
  sourceSha: string;
  /** Wall-clock creation time, ISO8601. */
  createdAt: string;
}

export interface TemplateCacheOptions {
  /** Workspace-scoped root. Defaults to tmpdir for tests. */
  root?: string;
  workspaceId: string;
}

export async function ensureTemplateCache(
  adapterName: string,
  sourceSha: string,
  opts: TemplateCacheOptions
): Promise<TemplateCacheEntry> {
  const rootDir = opts.root ?? join(tmpdir(), "stagecraft-scaffold-cache");
  const cacheDir = join(rootDir, opts.workspaceId, adapterName, sourceSha);
  const metaPath = join(cacheDir, ".cache-meta.json");

  // Cache hit?
  try {
    await access(metaPath);
    const meta = JSON.parse(
      await readFile(metaPath, "utf8")
    ) as TemplateCacheEntry;
    if (meta.sourceSha === sourceSha) {
      return { ...meta, path: cacheDir };
    }
    // SHA drift — wipe and re-populate.
    await rm(cacheDir, { recursive: true, force: true });
  } catch {
    // miss — fall through
  }

  await mkdir(cacheDir, { recursive: true });

  // Heavy lifting (git clone + npm install) is deferred to a production
  // implementation that shells out safely; this skeleton writes only the
  // cache metadata so the rest of the Create pipeline has a stable handle
  // to plumb through. Callers must run their own populator before reading
  // files from `cacheDir` in the first cold-cache invocation.
  const entry: TemplateCacheEntry = {
    path: cacheDir,
    sourceSha,
    createdAt: new Date().toISOString(),
  };
  await writeFile(metaPath, JSON.stringify(entry, null, 2), "utf8");
  return entry;
}
