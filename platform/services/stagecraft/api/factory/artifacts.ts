// Spec 139 Phase 1 — `/api/factory/artifacts/*` endpoints.
//
// Kind-filtered listing, path-addressed read, by-id read, and the
// override/clear-override mutators. All endpoints are org-scoped via
// `getAuthData()`. Mutators require the `factory:configure` permission
// (mirrors spec 108 / spec 109 write-side discipline).
//
// Audit writes for `artifact.overridden` and `artifact.override_cleared`
// land inside the same transaction as the row mutation (T031).

import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { and, asc, desc, eq, sql } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  factoryArtifactSubstrate,
  factoryArtifactSubstrateAudit,
  type ArtifactKind,
} from "../db/schema";
import { hasOrgPermission } from "../auth/membership";
import { sha256Hex, type SubstrateRow } from "./substrate";

// ---------------------------------------------------------------------------
// Wire types — what the HTTP API exposes.
// ---------------------------------------------------------------------------

export type ArtifactSummary = {
  id: string;
  orgId: string;
  origin: string;
  path: string;
  kind: ArtifactKind;
  bundleId: string | null;
  version: number;
  status: "active" | "retired";
  contentHash: string;
  conflictState: "ok" | "diverged" | null;
  hasOverride: boolean;
  syncedAt: string;
};

export type ArtifactDetail = ArtifactSummary & {
  upstreamSha: string | null;
  upstreamBody: string | null;
  userBody: string | null;
  effectiveBody: string;
  frontmatter: Record<string, unknown> | null;
  conflictUpstreamSha: string | null;
  userModifiedAt: string | null;
  userModifiedBy: string | null;
};

export type ListArtifactsRequest = {
  kind?: ArtifactKind;
  origin?: string;
  status?: "active" | "retired";
  page?: number;
  pageSize?: number;
};

export type ListArtifactsResponse = {
  artifacts: ArtifactSummary[];
  total: number;
  page: number;
  pageSize: number;
};

export interface ArtifactAuth {
  orgId: string;
  userID: string;
}

// ---------------------------------------------------------------------------
// Core (auth-explicit; tests call these directly).
// ---------------------------------------------------------------------------

export async function listArtifactsCore(
  auth: ArtifactAuth,
  req: ListArtifactsRequest,
): Promise<ListArtifactsResponse> {
  const page = Math.max(1, Math.floor(req.page ?? 1));
  const pageSize = Math.min(200, Math.max(1, Math.floor(req.pageSize ?? 50)));

  const filters = [eq(factoryArtifactSubstrate.orgId, auth.orgId)];
  if (req.kind) filters.push(eq(factoryArtifactSubstrate.kind, req.kind));
  if (req.origin) filters.push(eq(factoryArtifactSubstrate.origin, req.origin));
  if (req.status) filters.push(eq(factoryArtifactSubstrate.status, req.status));

  // `and(...filters)` returns `SQL | undefined` in drizzle-orm types because
  // a zero-arg call would be undefined. We always have at least the orgId
  // filter; assert the result is defined.
  const where = and(...filters)!;

  const [{ count }] = await db
    .select({ count: sql<number>`count(*)::int` })
    .from(factoryArtifactSubstrate)
    .where(where);

  const rows = await db
    .select()
    .from(factoryArtifactSubstrate)
    .where(where)
    .orderBy(asc(factoryArtifactSubstrate.path), desc(factoryArtifactSubstrate.version))
    .limit(pageSize)
    .offset((page - 1) * pageSize);

  return {
    artifacts: rows.map(toSummary),
    total: count,
    page,
    pageSize,
  };
}

export async function getArtifactByPathCore(
  auth: ArtifactAuth,
  req: { origin: string; path: string },
): Promise<ArtifactDetail> {
  const rows = await db
    .select()
    .from(factoryArtifactSubstrate)
    .where(
      and(
        eq(factoryArtifactSubstrate.orgId, auth.orgId),
        eq(factoryArtifactSubstrate.origin, req.origin),
        eq(factoryArtifactSubstrate.path, req.path),
      ),
    )
    .orderBy(desc(factoryArtifactSubstrate.version))
    .limit(1);
  if (!rows[0]) {
    throw APIError.notFound(
      `artifact not found at (origin=${req.origin}, path=${req.path})`,
    );
  }
  return toDetail(rows[0]);
}

export async function getArtifactByIdCore(
  auth: ArtifactAuth,
  req: { id: string },
): Promise<ArtifactDetail> {
  const rows = await db
    .select()
    .from(factoryArtifactSubstrate)
    .where(
      and(
        eq(factoryArtifactSubstrate.orgId, auth.orgId),
        eq(factoryArtifactSubstrate.id, req.id),
      ),
    )
    .limit(1);
  if (!rows[0]) {
    throw APIError.notFound(`artifact ${req.id} not found`);
  }
  return toDetail(rows[0]);
}

// ---------------------------------------------------------------------------
// Override mutators — write `user_body`, audit, return updated row.
// ---------------------------------------------------------------------------

export type ApplyOverrideArgs = {
  orgId: string;
  userId: string;
  artifactId: string;
  userBody: string;
};

export async function applyOverrideCore(
  args: ApplyOverrideArgs,
): Promise<SubstrateRow> {
  return db.transaction(async (tx) => {
    const existingRows = await tx
      .select()
      .from(factoryArtifactSubstrate)
      .where(
        and(
          eq(factoryArtifactSubstrate.orgId, args.orgId),
          eq(factoryArtifactSubstrate.id, args.artifactId),
        ),
      )
      .limit(1);
    const existing = existingRows[0];
    if (!existing) {
      throw APIError.notFound(`artifact ${args.artifactId} not found`);
    }
    if (existing.status === "retired") {
      throw APIError.failedPrecondition(
        `artifact ${args.artifactId} is retired and cannot be overridden`,
      );
    }
    const now = new Date();
    const updatedRows = await tx
      .update(factoryArtifactSubstrate)
      .set({
        userBody: args.userBody,
        userModifiedAt: now,
        userModifiedBy: args.userId,
        contentHash: sha256Hex(args.userBody),
        // Pin the upstream sha at the moment of override — used by sync's
        // divergence detection.
        conflictUpstreamSha: existing.upstreamSha,
        conflictState: "ok",
        updatedAt: now,
      })
      .where(eq(factoryArtifactSubstrate.id, existing.id))
      .returning();
    const row = mapStoredRowToSubstrate(updatedRows[0]);
    await tx.insert(factoryArtifactSubstrateAudit).values({
      artifactId: row.id,
      orgId: args.orgId,
      action: "artifact.overridden",
      actorUserId: args.userId,
      before: { userBody: existing.userBody },
      after: { userBody: args.userBody },
    });
    return row;
  });
}

export type ClearOverrideArgs = {
  orgId: string;
  userId: string;
  artifactId: string;
};

export async function clearOverrideCore(
  args: ClearOverrideArgs,
): Promise<SubstrateRow> {
  return db.transaction(async (tx) => {
    const existingRows = await tx
      .select()
      .from(factoryArtifactSubstrate)
      .where(
        and(
          eq(factoryArtifactSubstrate.orgId, args.orgId),
          eq(factoryArtifactSubstrate.id, args.artifactId),
        ),
      )
      .limit(1);
    const existing = existingRows[0];
    if (!existing) {
      throw APIError.notFound(`artifact ${args.artifactId} not found`);
    }
    if (existing.userBody === null) {
      // No-op; return as-is. Avoid emitting an audit row with nothing
      // to record.
      return mapStoredRowToSubstrate(existing);
    }
    const now = new Date();
    const updatedRows = await tx
      .update(factoryArtifactSubstrate)
      .set({
        userBody: null,
        userModifiedAt: now,
        userModifiedBy: args.userId,
        contentHash: sha256Hex(existing.upstreamBody ?? ""),
        conflictUpstreamSha: null,
        conflictState: "ok",
        updatedAt: now,
      })
      .where(eq(factoryArtifactSubstrate.id, existing.id))
      .returning();
    const row = mapStoredRowToSubstrate(updatedRows[0]);
    await tx.insert(factoryArtifactSubstrateAudit).values({
      artifactId: row.id,
      orgId: args.orgId,
      action: "artifact.override_cleared",
      actorUserId: args.userId,
      before: { userBody: existing.userBody },
      after: { userBody: null },
    });
    return row;
  });
}

// ---------------------------------------------------------------------------
// HTTP handlers — thin wrappers over the Core functions.
// ---------------------------------------------------------------------------

export const listArtifacts = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/factory/artifacts",
  },
  async (req: ListArtifactsRequest): Promise<ListArtifactsResponse> => {
    const auth = getAuthData()!;
    return listArtifactsCore(
      { orgId: auth.orgId, userID: auth.userID },
      req,
    );
  },
);

export const getArtifactByPath = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/factory/artifacts/by-path",
  },
  async (req: {
    origin: string;
    path: string;
  }): Promise<ArtifactDetail> => {
    const auth = getAuthData()!;
    return getArtifactByPathCore(
      { orgId: auth.orgId, userID: auth.userID },
      req,
    );
  },
);

export const getArtifactById = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/factory/artifacts/:id",
  },
  async (req: { id: string }): Promise<ArtifactDetail> => {
    const auth = getAuthData()!;
    return getArtifactByIdCore(
      { orgId: auth.orgId, userID: auth.userID },
      req,
    );
  },
);

export const applyOverride = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/factory/artifacts/:id/override",
  },
  async (req: { id: string; userBody: string }): Promise<ArtifactDetail> => {
    const auth = getAuthData()!;
    if (!hasOrgPermission(auth.platformRole, "factory:configure")) {
      throw APIError.permissionDenied(
        "factory:configure permission required to override an artifact",
      );
    }
    if (typeof req.userBody !== "string") {
      throw APIError.invalidArgument("userBody must be a string");
    }
    const row = await applyOverrideCore({
      orgId: auth.orgId,
      userId: auth.userID,
      artifactId: req.id,
      userBody: req.userBody,
    });
    return substrateRowToDetail(row);
  },
);

export const clearOverride = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/api/factory/artifacts/:id/override",
  },
  async (req: { id: string }): Promise<ArtifactDetail> => {
    const auth = getAuthData()!;
    if (!hasOrgPermission(auth.platformRole, "factory:configure")) {
      throw APIError.permissionDenied(
        "factory:configure permission required to clear an override",
      );
    }
    const row = await clearOverrideCore({
      orgId: auth.orgId,
      userId: auth.userID,
      artifactId: req.id,
    });
    return substrateRowToDetail(row);
  },
);

// ---------------------------------------------------------------------------
// Mappers
// ---------------------------------------------------------------------------

type StoredArtifactRow = typeof factoryArtifactSubstrate.$inferSelect;

function toSummary(row: StoredArtifactRow): ArtifactSummary {
  return {
    id: row.id,
    orgId: row.orgId,
    origin: row.origin,
    path: row.path,
    kind: row.kind,
    bundleId: row.bundleId,
    version: row.version,
    status: row.status,
    contentHash: row.contentHash,
    conflictState: row.conflictState ?? null,
    hasOverride: row.userBody !== null,
    syncedAt: row.updatedAt.toISOString(),
  };
}

function toDetail(row: StoredArtifactRow): ArtifactDetail {
  return {
    ...toSummary(row),
    upstreamSha: row.upstreamSha,
    upstreamBody: row.upstreamBody,
    userBody: row.userBody,
    effectiveBody: row.effectiveBody,
    frontmatter: (row.frontmatter as Record<string, unknown> | null) ?? null,
    conflictUpstreamSha: row.conflictUpstreamSha,
    userModifiedAt: row.userModifiedAt
      ? row.userModifiedAt.toISOString()
      : null,
    userModifiedBy: row.userModifiedBy,
  };
}

function mapStoredRowToSubstrate(row: StoredArtifactRow): SubstrateRow {
  return {
    id: row.id,
    orgId: row.orgId,
    origin: row.origin,
    path: row.path,
    kind: row.kind,
    bundleId: row.bundleId,
    version: row.version,
    status: row.status,
    upstreamSha: row.upstreamSha,
    upstreamBody: row.upstreamBody,
    userBody: row.userBody,
    userModifiedAt: row.userModifiedAt,
    userModifiedBy: row.userModifiedBy,
    effectiveBody: row.effectiveBody,
    contentHash: row.contentHash,
    frontmatter: (row.frontmatter as Record<string, unknown> | null) ?? null,
    conflictState: row.conflictState ?? null,
    conflictUpstreamSha: row.conflictUpstreamSha,
    conflictResolvedAt: row.conflictResolvedAt,
    conflictResolvedBy: row.conflictResolvedBy,
    createdAt: row.createdAt,
    updatedAt: row.updatedAt,
  };
}

function substrateRowToDetail(row: SubstrateRow): ArtifactDetail {
  return {
    id: row.id,
    orgId: row.orgId,
    origin: row.origin,
    path: row.path,
    kind: row.kind,
    bundleId: row.bundleId,
    version: row.version,
    status: row.status,
    contentHash: row.contentHash,
    conflictState: row.conflictState,
    hasOverride: row.userBody !== null,
    syncedAt: row.updatedAt.toISOString(),
    upstreamSha: row.upstreamSha,
    upstreamBody: row.upstreamBody,
    userBody: row.userBody,
    effectiveBody: row.effectiveBody,
    frontmatter: row.frontmatter,
    conflictUpstreamSha: row.conflictUpstreamSha,
    userModifiedAt: row.userModifiedAt
      ? row.userModifiedAt.toISOString()
      : null,
    userModifiedBy: row.userModifiedBy,
  };
}
