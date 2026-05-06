// Spec 139 Phase 4 (T091) — substrate-direct browser.
//
// Loads `factory_artifact_substrate` rows for an org and projects them to
// the spec 108 `TranslationResult` wire shape. The wire shape stays
// byte-stable — `browse.ts` calls into here and serves the projected
// result; legacy `factory_adapters` / `factory_contracts` /
// `factory_processes` tables are no longer touched.
//
// The projection helper itself (`projectSubstrateToLegacy`) lives in
// `projection.ts` so the round-trip parity test (T010) keeps using it
// as the in-memory projector. This module just bridges DB rows into the
// `SubstrateTranslation` shape that projector accepts.

import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import { factoryArtifactSubstrate } from "../db/schema";
import {
  DEFAULT_FACTORY_ORIGIN,
  DEFAULT_TEMPLATE_ORIGIN,
  type SubstrateRowDraft,
  type SubstrateTranslation,
} from "./translator";

export type LoadSubstrateForOrgOptions = {
  /** Default `goa-software-factory`. */
  factoryOriginId?: string;
  /** Default `aim-vue-node-template`. */
  templateOriginId?: string;
  /** Optional `template_remote` injected onto adapter manifests (matches Phase 1 syncPipeline). */
  templateRemote?: string;
  templateDefaultBranch?: string;
};

/**
 * Read every active substrate row for `(org, factoryOrigin)` and
 * `(org, templateOrigin)` and shape them into a [`SubstrateTranslation`]
 * the existing `projectSubstrateToLegacy` helper can project.
 *
 * **Performance:** loads all rows for the org's factory + template
 * origins. The current corpus (~122 rows) is comfortably small. Scale
 * past 10,000 rows would warrant a kind-filtered query path.
 */
export async function loadSubstrateForOrg(
  orgId: string,
  options: LoadSubstrateForOrgOptions = {},
): Promise<SubstrateTranslation> {
  const factoryOriginId = options.factoryOriginId ?? DEFAULT_FACTORY_ORIGIN;
  const templateOriginId = options.templateOriginId ?? DEFAULT_TEMPLATE_ORIGIN;

  const rows = await db
    .select({
      origin: factoryArtifactSubstrate.origin,
      path: factoryArtifactSubstrate.path,
      kind: factoryArtifactSubstrate.kind,
      bundleId: factoryArtifactSubstrate.bundleId,
      effectiveBody: factoryArtifactSubstrate.effectiveBody,
      contentHash: factoryArtifactSubstrate.contentHash,
      frontmatter: factoryArtifactSubstrate.frontmatter,
      upstreamSha: factoryArtifactSubstrate.upstreamSha,
      version: factoryArtifactSubstrate.version,
      status: factoryArtifactSubstrate.status,
    })
    .from(factoryArtifactSubstrate)
    .where(
      and(
        eq(factoryArtifactSubstrate.orgId, orgId),
        eq(factoryArtifactSubstrate.status, "active"),
      ),
    );

  // The projection consumes rows from the factory + template origins
  // (for adapters/processes/contracts) and from `oap-self` (for OAP-owned
  // contract schemas under `crates/factory-contracts/schemas/`).
  // Filter in TS (rather than constructing a complex SQL OR) so the
  // query plan stays one indexed lookup per row.
  const drafts: SubstrateRowDraft[] = [];
  let factorySha = "";
  let templateSha = "";
  for (const row of rows) {
    if (
      row.origin !== factoryOriginId &&
      row.origin !== templateOriginId &&
      row.origin !== "oap-self"
    ) {
      continue;
    }
    // Track the latest upstream sha per origin for the projection's
    // `factorySourceSha` / `templateSourceSha` fields. The projection
    // uses these for the `version` slug on the legacy adapter / process
    // rows; any row's sha for the origin is acceptable since spec 108
    // rotated all rows on the same sync sha.
    if (row.origin === factoryOriginId && row.upstreamSha) {
      factorySha = row.upstreamSha;
    }
    if (row.origin === templateOriginId && row.upstreamSha) {
      templateSha = row.upstreamSha;
    }
    drafts.push({
      origin: row.origin,
      path: row.path,
      kind: row.kind,
      bundleId: row.bundleId,
      upstreamSha: row.upstreamSha ?? "",
      upstreamBody: row.effectiveBody,
      contentHash: row.contentHash,
      frontmatter:
        (row.frontmatter as Record<string, unknown> | null) ?? null,
    });
  }

  // Deterministic ordering matches translator.ts so the projection's
  // output (in particular the legacy `manifest.skills` Object key
  // ordering) is stable across calls.
  drafts.sort((a, b) => {
    if (a.origin !== b.origin) return a.origin.localeCompare(b.origin);
    return a.path.localeCompare(b.path);
  });

  return {
    rows: drafts,
    factorySourceSha: factorySha,
    templateSourceSha: templateSha,
    factoryOriginId,
    templateOriginId,
    templateRemote: options.templateRemote,
    templateDefaultBranch: options.templateDefaultBranch,
  };
}
