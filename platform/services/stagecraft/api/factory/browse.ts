import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { and, asc, desc, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  factoryAdapters,
  factoryContracts,
  factoryProcesses,
} from "../db/schema";

// ---------------------------------------------------------------------------
// Spec 108 Phase 4 — read-only browsers for adapters / contracts / processes.
//
// List endpoints return summary rows (no JSON body) so the UI can render a
// table without pulling every manifest. Detail endpoints return the full
// body plus source_sha + synced_at for provenance. Everything is org-scoped
// via getAuthData(); writes happen only via the sync worker.
// ---------------------------------------------------------------------------

export type FactoryResourceSummary = {
  name: string;
  version: string;
  sourceSha: string;
  syncedAt: string;
};

export type FactoryAdapterDetail = FactoryResourceSummary & {
  manifest: unknown;
};

export type FactoryContractDetail = FactoryResourceSummary & {
  schema: unknown;
};

export type FactoryProcessDetail = FactoryResourceSummary & {
  definition: unknown;
};

// ---------------------------------------------------------------------------
// Adapters
// ---------------------------------------------------------------------------

export const listAdapters = api(
  { expose: true, auth: true, method: "GET", path: "/api/factory/adapters" },
  async (): Promise<{ adapters: FactoryResourceSummary[] }> => {
    const auth = getAuthData()!;
    const rows = await db
      .select({
        name: factoryAdapters.name,
        version: factoryAdapters.version,
        sourceSha: factoryAdapters.sourceSha,
        syncedAt: factoryAdapters.syncedAt,
      })
      .from(factoryAdapters)
      .where(eq(factoryAdapters.orgId, auth.orgId))
      .orderBy(asc(factoryAdapters.name));

    return {
      adapters: rows.map((r) => ({
        name: r.name,
        version: r.version,
        sourceSha: r.sourceSha,
        syncedAt: r.syncedAt.toISOString(),
      })),
    };
  }
);

export const getAdapter = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/factory/adapters/:name",
  },
  async (req: { name: string }): Promise<FactoryAdapterDetail> => {
    const auth = getAuthData()!;
    const [row] = await db
      .select()
      .from(factoryAdapters)
      .where(
        and(
          eq(factoryAdapters.orgId, auth.orgId),
          eq(factoryAdapters.name, req.name)
        )
      )
      .limit(1);

    if (!row) {
      throw APIError.notFound(`adapter "${req.name}" not found`);
    }
    return {
      name: row.name,
      version: row.version,
      sourceSha: row.sourceSha,
      syncedAt: row.syncedAt.toISOString(),
      manifest: row.manifest,
    };
  }
);

// ---------------------------------------------------------------------------
// Contracts
//
// factory_contracts is keyed on (orgId, name, version). The Phase 3 sync
// worker currently writes one row per name, but we still resolve the detail
// endpoint by picking the most recently synced row so future versioning
// doesn't break this contract.
// ---------------------------------------------------------------------------

export const listContracts = api(
  { expose: true, auth: true, method: "GET", path: "/api/factory/contracts" },
  async (): Promise<{ contracts: FactoryResourceSummary[] }> => {
    const auth = getAuthData()!;
    const rows = await db
      .select({
        name: factoryContracts.name,
        version: factoryContracts.version,
        sourceSha: factoryContracts.sourceSha,
        syncedAt: factoryContracts.syncedAt,
      })
      .from(factoryContracts)
      .where(eq(factoryContracts.orgId, auth.orgId))
      .orderBy(asc(factoryContracts.name));

    return {
      contracts: rows.map((r) => ({
        name: r.name,
        version: r.version,
        sourceSha: r.sourceSha,
        syncedAt: r.syncedAt.toISOString(),
      })),
    };
  }
);

export const getContract = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/factory/contracts/:name",
  },
  async (req: { name: string }): Promise<FactoryContractDetail> => {
    const auth = getAuthData()!;
    const [row] = await db
      .select()
      .from(factoryContracts)
      .where(
        and(
          eq(factoryContracts.orgId, auth.orgId),
          eq(factoryContracts.name, req.name)
        )
      )
      .orderBy(desc(factoryContracts.syncedAt))
      .limit(1);

    if (!row) {
      throw APIError.notFound(`contract "${req.name}" not found`);
    }
    return {
      name: row.name,
      version: row.version,
      sourceSha: row.sourceSha,
      syncedAt: row.syncedAt.toISOString(),
      schema: row.schema,
    };
  }
);

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

export const listProcesses = api(
  { expose: true, auth: true, method: "GET", path: "/api/factory/processes" },
  async (): Promise<{ processes: FactoryResourceSummary[] }> => {
    const auth = getAuthData()!;
    const rows = await db
      .select({
        name: factoryProcesses.name,
        version: factoryProcesses.version,
        sourceSha: factoryProcesses.sourceSha,
        syncedAt: factoryProcesses.syncedAt,
      })
      .from(factoryProcesses)
      .where(eq(factoryProcesses.orgId, auth.orgId))
      .orderBy(asc(factoryProcesses.name));

    return {
      processes: rows.map((r) => ({
        name: r.name,
        version: r.version,
        sourceSha: r.sourceSha,
        syncedAt: r.syncedAt.toISOString(),
      })),
    };
  }
);

export const getProcess = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/factory/processes/:name",
  },
  async (req: { name: string }): Promise<FactoryProcessDetail> => {
    const auth = getAuthData()!;
    const [row] = await db
      .select()
      .from(factoryProcesses)
      .where(
        and(
          eq(factoryProcesses.orgId, auth.orgId),
          eq(factoryProcesses.name, req.name)
        )
      )
      .orderBy(desc(factoryProcesses.syncedAt))
      .limit(1);

    if (!row) {
      throw APIError.notFound(`process "${req.name}" not found`);
    }
    return {
      name: row.name,
      version: row.version,
      sourceSha: row.sourceSha,
      syncedAt: row.syncedAt.toISOString(),
      definition: row.definition,
    };
  }
);
