/**
 * OAP-owned contract schemas (spec 108 §3.2).
 *
 * Contract schemas are not owned by upstream repos. Per the Factory §3.2
 * architecture note, the canonical home is OAP-internal — today at
 * `factory/contract/schemas/`, tomorrow at `crates/factory-contracts/schemas/`
 * (spec 108 Phase 2).
 *
 * With upstream-map v2.0.0 pointing at `GovAlta-Pronghorn/goa-software-factory`
 * and `GovAlta-Pronghorn/template`, neither upstream carries `*.schema.*`
 * files in the main tree. Without this loader, `factory_contracts` stays
 * empty after every sync and the /app/factory/contracts browser reports
 * "No contracts yet".
 *
 * The loader is called by the sync pipeline alongside the upstream
 * translators. It reads schemas from a directory resolved in this order:
 *   1. `OAP_FACTORY_SCHEMAS_DIR` env var (explicit override — set this in
 *      production container images)
 *   2. Walk up from `__dirname` looking for `factory/contract/schemas/`
 *      (dev / monorepo-local execution)
 *   3. Return empty (no crash — browser just stays empty)
 */

import { readdir, readFile, stat } from "node:fs/promises";
import { dirname, join, relative, basename } from "node:path";
import { fileURLToPath } from "node:url";
import log from "encore.dev/log";

import type { ContractTranslation } from "./translator";

const MODULE_DIR = dirname(fileURLToPath(import.meta.url));

// Walk up from a starting directory looking for `factory/contract/schemas/`.
// Bounded (walks at most 8 levels) so it fails fast if run outside the
// monorepo.
async function findSchemasDirByWalkUp(start: string): Promise<string | null> {
  let current = start;
  for (let i = 0; i < 8; i += 1) {
    const candidate = join(current, "factory", "contract", "schemas");
    const s = await stat(candidate).catch(() => null);
    if (s && s.isDirectory()) return candidate;
    const parent = dirname(current);
    if (parent === current) return null;
    current = parent;
  }
  return null;
}

async function resolveSchemasDir(): Promise<string | null> {
  const override = process.env.OAP_FACTORY_SCHEMAS_DIR;
  if (override) {
    const s = await stat(override).catch(() => null);
    if (s && s.isDirectory()) return override;
    log.warn("OAP_FACTORY_SCHEMAS_DIR set but not a directory", {
      path: override,
    });
  }
  return findSchemasDirByWalkUp(MODULE_DIR);
}

async function* walkSchemas(root: string): AsyncGenerator<{ rel: string; abs: string }> {
  async function* recurse(dir: string): AsyncGenerator<{ rel: string; abs: string }> {
    const entries = await readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const abs = join(dir, entry.name);
      if (entry.isDirectory()) {
        yield* recurse(abs);
      } else if (entry.isFile() && /\.schema\.(json|ya?ml)$/.test(entry.name)) {
        const rel = relative(root, abs).split(/\\|\//).join("/");
        yield { rel, abs };
      }
    }
  }
  yield* recurse(root);
}

function deriveContractName(relPath: string): string {
  // Use the basename minus .schema.{json,yaml,yml} for top-level schemas.
  // For `stage-outputs/<name>.schema.json` keep a `stage-outputs.<name>`
  // prefix so operators can tell them apart from top-level schemas in the
  // browser's name column.
  const base = basename(relPath).replace(/\.schema\.(json|ya?ml)$/, "");
  if (relPath.startsWith("stage-outputs/")) {
    return `stage-outputs.${base}`;
  }
  return base;
}

/**
 * Load OAP-owned contract schemas as ContractTranslation rows.
 *
 * `version` is the caller-supplied identifier — typically the sync run's
 * factory SHA so a sync is stamped with a consistent version across all
 * rows. `sourceSha` uses the same value: OAP schemas are not keyed off a
 * per-file SHA yet (that's a spec 108 Phase 2 concern once schemas move
 * into the factory-contracts crate with compile-time const SCHEMA_VERSION).
 */
export async function loadOapOwnedContracts(
  version: string,
  sourceSha: string
): Promise<ContractTranslation[]> {
  const dir = await resolveSchemasDir();
  if (!dir) {
    log.warn(
      "OAP-owned contract schemas not found; factory_contracts will not include them",
      { searched: [process.env.OAP_FACTORY_SCHEMAS_DIR ?? "(no env)", MODULE_DIR] }
    );
    return [];
  }

  const rows: ContractTranslation[] = [];
  for await (const { rel, abs } of walkSchemas(dir)) {
    const body = await readFile(abs, "utf8");
    rows.push({
      name: deriveContractName(rel),
      version,
      sourceSha,
      schema: { path: `factory/contract/schemas/${rel}`, body },
    });
  }
  rows.sort((a, b) => a.name.localeCompare(b.name));
  log.info("loaded OAP-owned contract schemas", {
    dir,
    count: rows.length,
  });
  return rows;
}
