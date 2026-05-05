import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { and, asc, eq, sql } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  factoryUpstreams,
  factoryArtifactSubstrate,
  auditLog,
} from "../db/schema";
import { hasOrgPermission } from "../auth/membership";
import { loadSubstrateForOrg } from "./substrateBrowser";
import { projectSubstrateToLegacy } from "./projection";

// ---------------------------------------------------------------------------
// Spec 108 + spec 139 — Factory upstream configuration.
//
// Spec 108 introduced one row per organisation with fixed factory+template
// fields. Spec 139 generalises the table to N-per-org keyed on
// (org_id, source_id) with role/subpath columns. The legacy singleton row
// migrates to source_id='legacy-mixed' / role='mixed'; spec 108's API
// surface continues to serve through Phase 1's legacy endpoints below.
//
// The new N-per-org endpoints are exposed alongside the legacy singleton
// shape so consumers can opt into role-aware sourcing for the OAP-native
// adapters (Phase 2 lights up `oap-next-prisma`, `oap-rust-axum`,
// `oap-encore-react` source rows).
// ---------------------------------------------------------------------------

export const LEGACY_SINGLETON_SOURCE_ID = "legacy-mixed";

export type FactoryUpstreamRow = {
  orgId: string;
  factorySource: string;
  factoryRef: string;
  templateSource: string;
  templateRef: string;
  lastSyncedAt: Date | null;
  lastSyncSha: { factory?: string; template?: string } | null;
  lastSyncStatus: string | null;
  lastSyncError: string | null;
  createdAt: Date;
  updatedAt: Date;
};

export type FactoryUpstreamSource = {
  orgId: string;
  sourceId: string;
  role: string;
  repoUrl: string;
  ref: string;
  subpath: string | null;
  lastSyncedAt: Date | null;
  lastSyncStatus: string | null;
  createdAt: Date;
  updatedAt: Date;
};

export type FactoryUpstreamCounts = {
  adapters: number;
  contracts: number;
  processes: number;
  /** Spec 139 — total active substrate rows. */
  artifacts: number;
};

// Matches the "owner/repo" shape GitHub uses. Permissive on character classes
// — the sync worker does the authoritative validation when it tries to clone.
const REPO_PATTERN = /^[A-Za-z0-9][A-Za-z0-9_.-]*\/[A-Za-z0-9][A-Za-z0-9_.-]*$/;

function validateRepo(value: string, field: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    throw APIError.invalidArgument(`${field} is required`);
  }
  if (!REPO_PATTERN.test(trimmed)) {
    throw APIError.invalidArgument(
      `${field} must be in the form "owner/repo" (got "${trimmed}")`,
    );
  }
  return trimmed;
}

function validateRef(value: string | undefined, field: string): string {
  const trimmed = (value ?? "").trim();
  if (!trimmed) return "main";
  if (trimmed.length > 255) {
    throw APIError.invalidArgument(`${field} must be ≤ 255 characters`);
  }
  return trimmed;
}

const ALLOWED_ROLES = new Set([
  "orchestration",
  "scaffold",
  "mixed",
  "oap-self",
]);

function validateRole(value: string): string {
  const trimmed = value.trim();
  if (!ALLOWED_ROLES.has(trimmed)) {
    throw APIError.invalidArgument(
      `role must be one of: orchestration, scaffold, mixed, oap-self`,
    );
  }
  return trimmed;
}

function validateSourceId(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    throw APIError.invalidArgument("sourceId is required");
  }
  if (trimmed.length > 100) {
    throw APIError.invalidArgument("sourceId must be ≤ 100 characters");
  }
  if (!/^[a-z0-9][a-z0-9-]*$/.test(trimmed)) {
    throw APIError.invalidArgument(
      "sourceId must be lowercase kebab-case (a-z, 0-9, -)",
    );
  }
  return trimmed;
}

// ---------------------------------------------------------------------------
// Legacy singleton row helpers — read/write the 'legacy-mixed' row that
// spec 108's external API surface depends on.
// ---------------------------------------------------------------------------

async function loadUpstream(orgId: string): Promise<FactoryUpstreamRow | null> {
  const rows = await db
    .select()
    .from(factoryUpstreams)
    .where(
      and(
        eq(factoryUpstreams.orgId, orgId),
        eq(factoryUpstreams.sourceId, LEGACY_SINGLETON_SOURCE_ID),
      ),
    )
    .limit(1);
  const row = rows[0];
  if (!row) return null;
  if (!row.factorySource || !row.templateSource) {
    // Phase 4 drops the legacy columns. Until then the singleton row
    // always carries them; missing values mean the row was never
    // populated through the legacy endpoint.
    return null;
  }
  return {
    orgId: row.orgId,
    factorySource: row.factorySource,
    factoryRef: row.factoryRef ?? "main",
    templateSource: row.templateSource,
    templateRef: row.templateRef ?? "main",
    lastSyncedAt: row.lastSyncedAt,
    lastSyncSha: (row.lastSyncSha as FactoryUpstreamRow["lastSyncSha"]) ?? null,
    lastSyncStatus: row.lastSyncStatus,
    lastSyncError: row.lastSyncError,
    createdAt: row.createdAt,
    updatedAt: row.updatedAt,
  };
}

async function loadCounts(orgId: string): Promise<FactoryUpstreamCounts> {
  // Spec 139 Phase 4 (T091): adapter / contract / process counts come
  // from the substrate via the same projection used by browse.ts. The
  // substrate-row count is a separate kind-agnostic head-count.
  const [substrateForOrg, artifactsRow] = await Promise.all([
    loadSubstrateForOrg(orgId),
    db
      .select({ count: sql<number>`count(*)::int` })
      .from(factoryArtifactSubstrate)
      .where(
        and(
          eq(factoryArtifactSubstrate.orgId, orgId),
          eq(factoryArtifactSubstrate.status, "active"),
        ),
      ),
  ]);
  const projection = projectSubstrateToLegacy(substrateForOrg);
  return {
    adapters: projection.adapters.length,
    contracts: projection.contracts.length,
    processes: projection.processes.length,
    artifacts: artifactsRow[0]?.count ?? 0,
  };
}

// ---------------------------------------------------------------------------
// GET /api/factory/upstreams — fetch current org config (or null).
// Singleton-shaped; backed by the 'legacy-mixed' row.
// ---------------------------------------------------------------------------

export const getUpstreams = api(
  { expose: true, auth: true, method: "GET", path: "/api/factory/upstreams" },
  async (): Promise<{
    upstream: FactoryUpstreamRow | null;
    counts: FactoryUpstreamCounts;
  }> => {
    const auth = getAuthData()!;
    const [upstream, counts] = await Promise.all([
      loadUpstream(auth.orgId),
      loadCounts(auth.orgId),
    ]);
    return { upstream, counts };
  },
);

// ---------------------------------------------------------------------------
// POST /api/factory/upstreams — create or update the legacy singleton.
// Idempotent. Writes both the legacy fixed columns and the new spec 139
// (sourceId, role, repoUrl, ref) columns so syncWorker.ts can keep
// reading legacy fields while the new schema is authoritative.
// ---------------------------------------------------------------------------

type UpsertUpstreamRequest = {
  factorySource: string;
  factoryRef?: string;
  templateSource: string;
  templateRef?: string;
};

export const upsertUpstreams = api(
  { expose: true, auth: true, method: "POST", path: "/api/factory/upstreams" },
  async (
    req: UpsertUpstreamRequest,
  ): Promise<{ upstream: FactoryUpstreamRow }> => {
    const auth = getAuthData()!;

    if (!hasOrgPermission(auth.platformRole, "factory:configure")) {
      throw APIError.permissionDenied(
        "Only org admins can configure factory upstreams",
      );
    }

    const factorySource = validateRepo(req.factorySource, "factorySource");
    const templateSource = validateRepo(req.templateSource, "templateSource");
    const factoryRef = validateRef(req.factoryRef, "factoryRef");
    const templateRef = validateRef(req.templateRef, "templateRef");

    const existing = await loadUpstream(auth.orgId);

    if (existing) {
      await db
        .update(factoryUpstreams)
        .set({
          factorySource,
          factoryRef,
          templateSource,
          templateRef,
          // Keep the spec 139 columns in sync with the legacy singleton
          // shape — the sync worker reads either one.
          repoUrl: factorySource,
          ref: factoryRef,
          role: "mixed",
          updatedAt: new Date(),
        })
        .where(
          and(
            eq(factoryUpstreams.orgId, auth.orgId),
            eq(factoryUpstreams.sourceId, LEGACY_SINGLETON_SOURCE_ID),
          ),
        );
    } else {
      await db.insert(factoryUpstreams).values({
        orgId: auth.orgId,
        sourceId: LEGACY_SINGLETON_SOURCE_ID,
        role: "mixed",
        repoUrl: factorySource,
        ref: factoryRef,
        factorySource,
        factoryRef,
        templateSource,
        templateRef,
      });
    }

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: existing ? "factory.upstreams.update" : "factory.upstreams.create",
      targetType: "factory_upstreams",
      targetId: auth.orgId,
      metadata: {
        factorySource,
        factoryRef,
        templateSource,
        templateRef,
      },
    });

    const upstream = await loadUpstream(auth.orgId);
    if (!upstream) {
      // Should never happen — we just inserted/updated it.
      throw APIError.internal("failed to read back factory_upstreams row");
    }
    return { upstream };
  },
);

// ---------------------------------------------------------------------------
// Spec 139 — N-per-org source endpoints.
//
// These let an org declare separate orchestration / scaffold sources for
// the substrate (Phase 2 wires them per-adapter). The legacy singleton
// row continues to surface in `getUpstreams` above.
// ---------------------------------------------------------------------------

export interface UpstreamsAuth {
  orgId: string;
  userID: string;
}

export type ListUpstreamSourcesResponse = {
  sources: FactoryUpstreamSource[];
};

export async function listUpstreamSourcesCore(
  auth: UpstreamsAuth,
): Promise<ListUpstreamSourcesResponse> {
  const rows = await db
    .select()
    .from(factoryUpstreams)
    .where(eq(factoryUpstreams.orgId, auth.orgId))
    .orderBy(asc(factoryUpstreams.sourceId));
  return {
    sources: rows.map(rowToSource),
  };
}

export type UpsertUpstreamSourceRequest = {
  sourceId: string;
  role: string;
  repoUrl: string;
  ref?: string;
  subpath?: string;
};

export async function upsertUpstreamSourceCore(
  auth: UpstreamsAuth,
  req: UpsertUpstreamSourceRequest,
): Promise<FactoryUpstreamSource> {
  const sourceId = validateSourceId(req.sourceId);
  const role = validateRole(req.role);
  const repoUrl = req.repoUrl.trim();
  if (!repoUrl) {
    throw APIError.invalidArgument("repoUrl is required");
  }
  const ref = validateRef(req.ref, "ref");
  const subpath = req.subpath?.trim() || null;

  const existing = await db
    .select()
    .from(factoryUpstreams)
    .where(
      and(
        eq(factoryUpstreams.orgId, auth.orgId),
        eq(factoryUpstreams.sourceId, sourceId),
      ),
    )
    .limit(1);

  if (existing[0]) {
    await db
      .update(factoryUpstreams)
      .set({ role, repoUrl, ref, subpath, updatedAt: new Date() })
      .where(
        and(
          eq(factoryUpstreams.orgId, auth.orgId),
          eq(factoryUpstreams.sourceId, sourceId),
        ),
      );
  } else {
    await db.insert(factoryUpstreams).values({
      orgId: auth.orgId,
      sourceId,
      role,
      repoUrl,
      ref,
      subpath,
    });
  }

  await db.insert(auditLog).values({
    actorUserId: auth.userID,
    action: existing[0] ? "factory.source.updated" : "factory.source.created",
    targetType: "factory_upstreams",
    targetId: `${auth.orgId}:${sourceId}`,
    metadata: { sourceId, role, repoUrl, ref, subpath },
  });

  const reread = await db
    .select()
    .from(factoryUpstreams)
    .where(
      and(
        eq(factoryUpstreams.orgId, auth.orgId),
        eq(factoryUpstreams.sourceId, sourceId),
      ),
    )
    .limit(1);
  if (!reread[0]) throw APIError.internal("failed to read back source row");
  return rowToSource(reread[0]);
}

export async function deleteUpstreamSourceCore(
  auth: UpstreamsAuth,
  req: { sourceId: string },
): Promise<void> {
  const sourceId = validateSourceId(req.sourceId);
  if (sourceId === LEGACY_SINGLETON_SOURCE_ID) {
    throw APIError.failedPrecondition(
      `cannot delete the legacy-mixed source row; it backs spec 108's API surface`,
    );
  }
  const deleted = await db
    .delete(factoryUpstreams)
    .where(
      and(
        eq(factoryUpstreams.orgId, auth.orgId),
        eq(factoryUpstreams.sourceId, sourceId),
      ),
    )
    .returning({ sourceId: factoryUpstreams.sourceId });
  if (deleted.length === 0) {
    throw APIError.notFound(`source ${sourceId} not found`);
  }
  await db.insert(auditLog).values({
    actorUserId: auth.userID,
    action: "factory.source.deleted",
    targetType: "factory_upstreams",
    targetId: `${auth.orgId}:${sourceId}`,
    metadata: { sourceId },
  });
}

// HTTP handlers for N-per-org sources

export const listUpstreamSources = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/factory/upstreams/sources",
  },
  async (): Promise<ListUpstreamSourcesResponse> => {
    const auth = getAuthData()!;
    return listUpstreamSourcesCore({ orgId: auth.orgId, userID: auth.userID });
  },
);

export const upsertUpstreamSource = api(
  {
    expose: true,
    auth: true,
    method: "PUT",
    path: "/api/factory/upstreams/sources/:sourceId",
  },
  async (
    req: { sourceId: string } & Omit<UpsertUpstreamSourceRequest, "sourceId">,
  ): Promise<{ source: FactoryUpstreamSource }> => {
    const auth = getAuthData()!;
    if (!hasOrgPermission(auth.platformRole, "factory:configure")) {
      throw APIError.permissionDenied(
        "factory:configure permission required to configure upstream sources",
      );
    }
    const source = await upsertUpstreamSourceCore(
      { orgId: auth.orgId, userID: auth.userID },
      { ...req, sourceId: req.sourceId },
    );
    return { source };
  },
);

export const deleteUpstreamSource = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/api/factory/upstreams/sources/:sourceId",
  },
  async (req: { sourceId: string }): Promise<{ ok: true }> => {
    const auth = getAuthData()!;
    if (!hasOrgPermission(auth.platformRole, "factory:configure")) {
      throw APIError.permissionDenied(
        "factory:configure permission required to delete upstream sources",
      );
    }
    await deleteUpstreamSourceCore(
      { orgId: auth.orgId, userID: auth.userID },
      req,
    );
    return { ok: true };
  },
);

// ---------------------------------------------------------------------------
// Mappers
// ---------------------------------------------------------------------------

type StoredUpstreamRow = typeof factoryUpstreams.$inferSelect;

function rowToSource(row: StoredUpstreamRow): FactoryUpstreamSource {
  return {
    orgId: row.orgId,
    sourceId: row.sourceId,
    role: row.role,
    repoUrl: row.repoUrl,
    ref: row.ref,
    subpath: row.subpath,
    lastSyncedAt: row.lastSyncedAt,
    lastSyncStatus: row.lastSyncStatus,
    createdAt: row.createdAt,
    updatedAt: row.updatedAt,
  };
}
