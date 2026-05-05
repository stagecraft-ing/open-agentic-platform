// Spec 139 Phase 2 — T054 + T055: OAP-native adapter ingestion.
//
// Ingests the four OAP-controlled adapters (`next-prisma`, `rust-axum`,
// `encore-react`, `aim-vue-node`) from on-disk source directories into
// the substrate under `origin='oap-self'`. Applies the D-4 sanitisation
// locked at Phase 0 + the T055 manifest extension:
//
//   1. Bump `stack.runtime` to `node-24` for `next-prisma` (was node-22)
//      and `encore-react` (was node-20). `rust-axum` keeps `native`.
//      `aim-vue-node` keeps its existing runtime.
//   2. Inject `orchestration_source_id`, `scaffold_source_id`,
//      `scaffold_runtime` keys on the manifest at ingest time.
//   3. Drop the duplicate `validation:` block from the manifest;
//      `validation/invariants.yaml` is canonical.
//   4. Auto-generate minimal frontmatter on `patterns/**/*.md` so each
//      pattern is substrate-addressable
//      (`id: <adapter>-pattern-<rel-kebab>`, `adapter: <name>`,
//      `category: <api|data|page-types|ui>`).
//
// Idempotent: safe to re-run. Each adapter's ingest is a no-op on
// repeat invocation thanks to `ON CONFLICT (org_id, origin, path,
// version) DO NOTHING`.

import { readFile, readdir, stat } from "node:fs/promises";
import { basename, join, relative } from "node:path";
import { db } from "../db/drizzle";
import {
  factoryArtifactSubstrate,
  factoryArtifactSubstrateAudit,
} from "../db/schema";
import { sha256Hex } from "./substrate";
import { classifyArtifactKind, extractFrontmatter } from "./translator";
import {
  OAP_NATIVE_ADAPTERS,
  sanitiseForIngest,
  type OapNativeAdapterConfig,
} from "./oapNativeSanitise";

// Re-export for callers that need the config table.
export {
  OAP_NATIVE_ADAPTERS,
  sanitiseForIngest,
  type OapNativeAdapterConfig,
} from "./oapNativeSanitise";

// ---------------------------------------------------------------------------
// Per-adapter ingest config (T055 + D-4) — see `./oapNativeSanitise.ts`
// ---------------------------------------------------------------------------

export const OAP_NATIVE_ORIGIN = "oap-self";

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

export type IngestOptions = {
  orgId: string;
  /** Default `oap-self`. */
  origin?: string;
  /** Default the OAP-native adapter id (e.g. `oap-next-prisma`). */
  upstreamSha?: string;
  actorUserId?: string | null;
};

export type IngestSummary = {
  adapter: string;
  rowsInserted: number;
  rowsSkipped: number;
  filesWalked: number;
};

/**
 * Ingest a single OAP-native adapter source tree into the substrate.
 *
 * @param adapterDir Absolute path to the adapter source directory (the
 *   one that contains `manifest.yaml`, `agents/`, `patterns/`, etc.).
 */
export async function ingestOapNativeAdapter(
  adapterDir: string,
  opts: IngestOptions,
): Promise<IngestSummary> {
  const adapterName = basename(adapterDir);
  const config = OAP_NATIVE_ADAPTERS[adapterName];
  if (!config) {
    throw new Error(`unknown OAP-native adapter: ${adapterName}`);
  }

  const dirStat = await stat(adapterDir).catch(() => null);
  if (!dirStat || !dirStat.isDirectory()) {
    throw new Error(`adapter source is not a directory: ${adapterDir}`);
  }

  const origin = opts.origin ?? OAP_NATIVE_ORIGIN;
  const upstreamSha =
    opts.upstreamSha ?? `oap-self/${adapterName}/${shortHash(adapterDir)}`;

  let rowsInserted = 0;
  let rowsSkipped = 0;
  let filesWalked = 0;

  await db.transaction(async (tx) => {
    for await (const file of walkAdapter(adapterDir)) {
      filesWalked += 1;
      const result = await ingestOneFile(tx, {
        orgId: opts.orgId,
        origin,
        adapterName,
        adapterDir,
        rel: file.rel,
        abs: file.abs,
        config,
        upstreamSha,
        actorUserId: opts.actorUserId ?? null,
      });
      if (result === "inserted") rowsInserted += 1;
      else rowsSkipped += 1;
    }
  });

  return {
    adapter: adapterName,
    rowsInserted,
    rowsSkipped,
    filesWalked,
  };
}

/**
 * Convenience batch ingest — applies all four adapters from a single
 * parent directory (typically `_tmp/factory/adapters/` per research §3).
 */
export async function ingestAllOapNativeAdapters(
  adaptersParentDir: string,
  opts: IngestOptions,
): Promise<IngestSummary[]> {
  const summaries: IngestSummary[] = [];
  for (const adapterName of Object.keys(OAP_NATIVE_ADAPTERS)) {
    const adapterDir = join(adaptersParentDir, adapterName);
    const exists = await stat(adapterDir).catch(() => null);
    if (!exists) continue;
    summaries.push(await ingestOapNativeAdapter(adapterDir, opts));
  }
  return summaries;
}

// ---------------------------------------------------------------------------
// Internal — file walk + classify + insert
// ---------------------------------------------------------------------------

const INGEST_EXCLUDES = [
  /(^|\/)\.git(\/|$)/,
  /(^|\/)\.DS_Store$/,
  /(^|\/)node_modules(\/|$)/,
];

async function* walkAdapter(
  root: string,
): AsyncGenerator<{ rel: string; abs: string }> {
  async function* recurse(
    dir: string,
  ): AsyncGenerator<{ rel: string; abs: string }> {
    const entries = await readdir(dir, { withFileTypes: true });
    entries.sort((a, b) => a.name.localeCompare(b.name));
    for (const entry of entries) {
      const abs = join(dir, entry.name);
      const rel = relative(root, abs).split(/\\|\//).join("/");
      if (INGEST_EXCLUDES.some((re) => re.test(rel))) continue;
      if (entry.isDirectory()) {
        yield* recurse(abs);
      } else if (entry.isFile()) {
        yield { rel, abs };
      }
    }
  }
  yield* recurse(root);
}

type IngestFileArgs = {
  orgId: string;
  origin: string;
  adapterName: string;
  adapterDir: string;
  rel: string;
  abs: string;
  config: OapNativeAdapterConfig;
  upstreamSha: string;
  actorUserId: string | null;
};

type Tx = Parameters<Parameters<typeof db.transaction>[0]>[0];

async function ingestOneFile(
  tx: Tx,
  args: IngestFileArgs,
): Promise<"inserted" | "skipped"> {
  // Substrate path includes the adapter prefix so multiple adapters
  // coexist under the same origin/source.
  const substratePath = `adapters/${args.adapterName}/${args.rel}`;

  let bodyText = await readFile(args.abs, "utf8");
  let frontmatter: Record<string, unknown> | null = null;

  // Per-file sanitisation passes (D-4 + T055).
  const sanitised = await sanitiseForIngest({
    rel: args.rel,
    body: bodyText,
    adapterName: args.adapterName,
    config: args.config,
  });
  bodyText = sanitised.body;
  frontmatter = sanitised.frontmatter;

  if (frontmatter === null && /\.md$/i.test(args.rel)) {
    // Re-parse if sanitiser didn't return one (e.g. non-pattern .md).
    frontmatter = extractFrontmatter(bodyText).frontmatter;
  }

  const kind = classifyArtifactKind(substratePath, frontmatter);
  const contentHash = sha256Hex(bodyText);

  const inserted = await tx
    .insert(factoryArtifactSubstrate)
    .values({
      orgId: args.orgId,
      origin: args.origin,
      path: substratePath,
      kind,
      version: 1,
      status: "active",
      upstreamSha: args.upstreamSha,
      upstreamBody: bodyText,
      contentHash,
      frontmatter,
      conflictState: "ok",
    })
    .onConflictDoNothing({
      target: [
        factoryArtifactSubstrate.orgId,
        factoryArtifactSubstrate.origin,
        factoryArtifactSubstrate.path,
        factoryArtifactSubstrate.version,
      ],
    })
    .returning({ id: factoryArtifactSubstrate.id });

  if (inserted.length === 0) return "skipped";

  await tx.insert(factoryArtifactSubstrateAudit).values({
    artifactId: inserted[0].id,
    orgId: args.orgId,
    action: "artifact.synced",
    actorUserId: args.actorUserId,
    before: null,
    after: { origin: args.origin, path: substratePath, kind },
  });

  return "inserted";
}

function shortHash(input: string): string {
  return sha256Hex(input).slice(0, 12);
}
