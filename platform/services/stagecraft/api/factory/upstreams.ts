import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  factoryUpstreams,
  factoryAdapters,
  factoryContracts,
  factoryProcesses,
  auditLog,
} from "../db/schema";
import { hasOrgPermission } from "../auth/membership";

// ---------------------------------------------------------------------------
// Spec 108 — Factory upstream configuration.
//
// One row per organisation. Replaces the repo-rooted upstream-map.yaml; the
// derived adapters/contracts/processes tables are produced by the sync worker
// (Phase 3) and surfaced here only as counts.
// ---------------------------------------------------------------------------

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

export type FactoryUpstreamCounts = {
  adapters: number;
  contracts: number;
  processes: number;
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
      `${field} must be in the form "owner/repo" (got "${trimmed}")`
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

async function loadUpstream(orgId: string): Promise<FactoryUpstreamRow | null> {
  const rows = await db
    .select()
    .from(factoryUpstreams)
    .where(eq(factoryUpstreams.orgId, orgId))
    .limit(1);

  const row = rows[0];
  if (!row) return null;

  return {
    orgId: row.orgId,
    factorySource: row.factorySource,
    factoryRef: row.factoryRef,
    templateSource: row.templateSource,
    templateRef: row.templateRef,
    lastSyncedAt: row.lastSyncedAt,
    lastSyncSha: (row.lastSyncSha as FactoryUpstreamRow["lastSyncSha"]) ?? null,
    lastSyncStatus: row.lastSyncStatus,
    lastSyncError: row.lastSyncError,
    createdAt: row.createdAt,
    updatedAt: row.updatedAt,
  };
}

async function loadCounts(orgId: string): Promise<FactoryUpstreamCounts> {
  const [adapters, contracts, processes] = await Promise.all([
    db.select().from(factoryAdapters).where(eq(factoryAdapters.orgId, orgId)),
    db.select().from(factoryContracts).where(eq(factoryContracts.orgId, orgId)),
    db.select().from(factoryProcesses).where(eq(factoryProcesses.orgId, orgId)),
  ]);
  return {
    adapters: adapters.length,
    contracts: contracts.length,
    processes: processes.length,
  };
}

// ---------------------------------------------------------------------------
// GET /api/factory/upstreams — fetch current org config (or null).
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
  }
);

// ---------------------------------------------------------------------------
// POST /api/factory/upstreams — create or update. Idempotent.
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
    req: UpsertUpstreamRequest
  ): Promise<{ upstream: FactoryUpstreamRow }> => {
    const auth = getAuthData()!;

    if (!hasOrgPermission(auth.platformRole, "factory:configure")) {
      throw APIError.permissionDenied(
        "Only org admins can configure factory upstreams"
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
          updatedAt: new Date(),
        })
        .where(eq(factoryUpstreams.orgId, auth.orgId));
    } else {
      await db.insert(factoryUpstreams).values({
        orgId: auth.orgId,
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
  }
);
