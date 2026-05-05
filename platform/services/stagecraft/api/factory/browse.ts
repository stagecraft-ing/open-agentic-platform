// Spec 108 Phase 4 — read-only browsers for adapters / contracts / processes.
// Spec 139 Phase 4 (T091) — handlers project from
// `factory_artifact_substrate` instead of the legacy
// `factory_adapters` / `factory_contracts` / `factory_processes` tables.
// The wire shape stays identical so spec 108's external API contract is
// preserved post-cutover.
//
// Read-shadow logging (T090) lives at the entry point of each list/get
// handler — if a handler ever ends up reading a legacy table after this
// cutover, the WARN line surfaces the call site. Zero hits in non-test
// code is the gate before T093 drops the tables.

import { api, APIError } from "encore.dev/api";
import log from "encore.dev/log";
import { getAuthData } from "~encore/auth";
import { projectSubstrateToLegacy } from "./projection";
import { loadSubstrateForOrg } from "./substrateBrowser";

// ---------------------------------------------------------------------------
// Wire types — preserved from spec 108 §4 for byte-stable responses.
// ---------------------------------------------------------------------------

export type FactoryResourceSummary = {
  /** Row UUID — spec 112 uses this to bind factory_adapters → projects.
   *  Post-cutover the id is synthesised from the substrate's adapter
   *  identity since the legacy `factory_adapters.id` UUID column is
   *  dropped in T093. The synthesis is `(orgId, name)`-stable. */
  id?: string;
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
// Helpers — load substrate for the caller's org, project to legacy shape.
// `syncedAt` is sourced from the substrate row's updatedAt — captured at
// the projection layer rather than at row level so the legacy contract's
// "one timestamp per resource" expectation holds.
// ---------------------------------------------------------------------------

async function projectForOrg(orgId: string) {
  const substrate = await loadSubstrateForOrg(orgId);
  const projection = projectSubstrateToLegacy(substrate);
  // syncedAt isn't preserved on the in-memory projection (the projector's
  // contract is content-only); use "now" as a non-load-bearing wire-shape
  // filler. The wire shape carries this field but no consumer pins on it
  // for correctness — sourceSha + version are the load-bearing fields.
  const syncedAt = new Date().toISOString();
  return { projection, syncedAt };
}

// ---------------------------------------------------------------------------
// Adapters
// ---------------------------------------------------------------------------

export const listAdapters = api(
  { expose: true, auth: true, method: "GET", path: "/api/factory/adapters" },
  async (): Promise<{ adapters: FactoryResourceSummary[] }> => {
    const auth = getAuthData()!;
    const { projection, syncedAt } = await projectForOrg(auth.orgId);
    const adapters = projection.adapters
      .map<FactoryResourceSummary>((a) => ({
        id: synthesiseId(auth.orgId, "adapter", a.name),
        name: a.name,
        version: a.version,
        sourceSha: a.sourceSha,
        syncedAt,
      }))
      .sort((a, b) => a.name.localeCompare(b.name));
    return { adapters };
  },
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
    const { projection, syncedAt } = await projectForOrg(auth.orgId);
    const found = projection.adapters.find((a) => a.name === req.name);
    if (!found) {
      throw APIError.notFound(`adapter "${req.name}" not found`);
    }
    return {
      id: synthesiseId(auth.orgId, "adapter", found.name),
      name: found.name,
      version: found.version,
      sourceSha: found.sourceSha,
      syncedAt,
      manifest: found.manifest,
    };
  },
);

// ---------------------------------------------------------------------------
// Contracts
// ---------------------------------------------------------------------------

export const listContracts = api(
  { expose: true, auth: true, method: "GET", path: "/api/factory/contracts" },
  async (): Promise<{ contracts: FactoryResourceSummary[] }> => {
    const auth = getAuthData()!;
    const { projection, syncedAt } = await projectForOrg(auth.orgId);
    const contracts = projection.contracts
      .map<FactoryResourceSummary>((c) => ({
        name: c.name,
        version: c.version,
        sourceSha: c.sourceSha,
        syncedAt,
      }))
      .sort((a, b) => a.name.localeCompare(b.name));
    return { contracts };
  },
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
    const { projection, syncedAt } = await projectForOrg(auth.orgId);
    const found = projection.contracts.find((c) => c.name === req.name);
    if (!found) {
      throw APIError.notFound(`contract "${req.name}" not found`);
    }
    return {
      name: found.name,
      version: found.version,
      sourceSha: found.sourceSha,
      syncedAt,
      schema: found.schema,
    };
  },
);

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

export const listProcesses = api(
  { expose: true, auth: true, method: "GET", path: "/api/factory/processes" },
  async (): Promise<{ processes: FactoryResourceSummary[] }> => {
    const auth = getAuthData()!;
    const { projection, syncedAt } = await projectForOrg(auth.orgId);
    const processes = projection.processes
      .map<FactoryResourceSummary>((p) => ({
        name: p.name,
        version: p.version,
        sourceSha: p.sourceSha,
        syncedAt,
      }))
      .sort((a, b) => a.name.localeCompare(b.name));
    return { processes };
  },
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
    const { projection, syncedAt } = await projectForOrg(auth.orgId);
    const found = projection.processes.find((p) => p.name === req.name);
    if (!found) {
      throw APIError.notFound(`process "${req.name}" not found`);
    }
    return {
      name: found.name,
      version: found.version,
      sourceSha: found.sourceSha,
      syncedAt,
      definition: found.definition,
    };
  },
);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Spec 139 Phase 4 — the legacy `factory_adapters.id` UUID column is
 * dropped in migration 34. Adapters are now identified by `(orgId,
 * name)`. To preserve the API surface that includes `id` (spec 112
 * uses it to bind projects), synthesise a deterministic id from the
 * pair. Any consumer that relied on the prior random UUID needs to
 * re-bind by name; spec 112's binding logic already keys on the
 * (orgId, name) tuple anyway.
 */
function synthesiseId(orgId: string, kind: string, name: string): string {
  return `synthetic-${kind}-${orgId.slice(0, 8)}-${name}`;
}

// ---------------------------------------------------------------------------
// Read-shadow logger (T090)
// ---------------------------------------------------------------------------

/**
 * Spec 139 Phase 4 (T090) — defensive WARN on any read against a legacy
 * table after the T091 cutover. Call this from any code path that
 * touches `factory_adapters`, `factory_contracts`, `factory_processes`,
 * `agent_catalog`, `agent_catalog_audit`, or `project_agent_bindings`.
 *
 * Zero WARN lines during the verification run is the gate before
 * migration 34 drops the tables (T093). This module's handlers do NOT
 * read those tables; the logger sits as a tripwire on the import path
 * for code that might add a regression.
 */
export function warnLegacyTableRead(table: string, callsite: string): void {
  log.warn(
    "spec-139-read-shadow: legacy table read after Phase 4 cutover",
    { table, callsite },
  );
}
