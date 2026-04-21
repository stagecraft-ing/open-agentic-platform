/**
 * Factory API helpers (spec 108). Mirrors projects-api.server.ts — manual
 * fetch so we can forward the SSR session cookie; the Encore generated
 * client does not support that.
 */

const DEFAULT_API_BASE = "http://localhost:4000";

function getBaseUrl(): string {
  return process.env.ENCORE_API_BASE_URL ?? DEFAULT_API_BASE;
}

async function apiFetch(request: Request, path: string, init?: RequestInit) {
  const base = getBaseUrl();
  const cookie = request.headers.get("Cookie") ?? "";
  const res = await fetch(`${base}${path}`, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(cookie && { Cookie: cookie }),
      ...init?.headers,
    },
  });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(body || `API error: ${res.status}`);
  }
  return res.json();
}

export type FactoryUpstream = {
  orgId: string;
  factorySource: string;
  factoryRef: string;
  templateSource: string;
  templateRef: string;
  lastSyncedAt: string | null;
  lastSyncSha: { factory?: string; template?: string } | null;
  lastSyncStatus: string | null;
  lastSyncError: string | null;
  createdAt: string;
  updatedAt: string;
};

export type FactoryUpstreamCounts = {
  adapters: number;
  contracts: number;
  processes: number;
};

export async function getFactoryUpstreams(request: Request) {
  return apiFetch(request, "/api/factory/upstreams") as Promise<{
    upstream: FactoryUpstream | null;
    counts: FactoryUpstreamCounts;
  }>;
}

export async function upsertFactoryUpstreams(
  request: Request,
  data: {
    factorySource: string;
    factoryRef?: string;
    templateSource: string;
    templateRef?: string;
  }
) {
  return apiFetch(request, "/api/factory/upstreams", {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{ upstream: FactoryUpstream }>;
}

export type FactorySyncTriggerResponse = {
  syncRunId: string;
  status: "pending" | "running";
  queuedAt: string;
};

export type FactorySyncRunStatus = "pending" | "running" | "ok" | "failed";

export type FactorySyncRun = {
  id: string;
  status: FactorySyncRunStatus;
  triggeredBy: string;
  factorySha: string | null;
  templateSha: string | null;
  counts: FactoryUpstreamCounts | null;
  error: string | null;
  queuedAt: string;
  startedAt: string | null;
  completedAt: string | null;
};

export async function syncFactoryUpstreams(
  request: Request
): Promise<FactorySyncTriggerResponse> {
  return apiFetch(request, "/api/factory/upstreams/sync", {
    method: "POST",
    body: "{}",
  }) as Promise<FactorySyncTriggerResponse>;
}

export async function listFactorySyncRuns(request: Request) {
  return apiFetch(request, "/api/factory/upstreams/sync") as Promise<{
    runs: FactorySyncRun[];
  }>;
}

export async function getFactorySyncRun(
  request: Request,
  id: string
): Promise<FactorySyncRun> {
  return apiFetch(
    request,
    `/api/factory/upstreams/sync/${encodeURIComponent(id)}`
  ) as Promise<FactorySyncRun>;
}

// ---------------------------------------------------------------------------
// Factory upstream PAT (spec 109 §6)
// ---------------------------------------------------------------------------

export type FactoryUpstreamPatMetadata = {
  exists: boolean;
  tokenPrefix?: string;
  isFineGrained?: boolean;
  scopes?: string[];
  githubLogin?: string | null;
  lastUsedAt?: string | null;
  lastCheckedAt?: string;
  createdAt?: string;
};

export type FactoryUpstreamPatValidation = {
  ok: boolean;
  tokenPrefix: string;
  isFineGrained: boolean;
  scopes: string[];
  lastCheckedAt: string;
  githubLogin?: string;
  reason?: "pat_invalid" | "pat_rate_limited" | "pat_saml_not_authorized";
};

export async function getFactoryUpstreamPat(request: Request) {
  return apiFetch(
    request,
    "/api/factory/upstreams/pat"
  ) as Promise<FactoryUpstreamPatMetadata>;
}

export async function storeFactoryUpstreamPat(
  request: Request,
  token: string
): Promise<FactoryUpstreamPatValidation> {
  return apiFetch(request, "/api/factory/upstreams/pat", {
    method: "POST",
    body: JSON.stringify({ token }),
  }) as Promise<FactoryUpstreamPatValidation>;
}

export async function revokeFactoryUpstreamPat(request: Request) {
  return apiFetch(request, "/api/factory/upstreams/pat", {
    method: "DELETE",
  }) as Promise<{ revoked: boolean }>;
}

export async function validateFactoryUpstreamPat(
  request: Request
): Promise<FactoryUpstreamPatValidation> {
  return apiFetch(request, "/api/factory/upstreams/pat/validate", {
    method: "POST",
    body: "{}",
  }) as Promise<FactoryUpstreamPatValidation>;
}

// ---------------------------------------------------------------------------
// Project PAT (spec 109 §6)
// ---------------------------------------------------------------------------

export type ProjectPatMetadata = FactoryUpstreamPatMetadata;
export type ProjectPatValidation = FactoryUpstreamPatValidation;

export async function getProjectPat(request: Request, projectId: string) {
  return apiFetch(
    request,
    `/api/projects/${encodeURIComponent(projectId)}/pat`
  ) as Promise<ProjectPatMetadata>;
}

export async function storeProjectPat(
  request: Request,
  projectId: string,
  token: string
): Promise<ProjectPatValidation> {
  return apiFetch(
    request,
    `/api/projects/${encodeURIComponent(projectId)}/pat`,
    {
      method: "POST",
      body: JSON.stringify({ token }),
    }
  ) as Promise<ProjectPatValidation>;
}

export async function revokeProjectPat(request: Request, projectId: string) {
  return apiFetch(
    request,
    `/api/projects/${encodeURIComponent(projectId)}/pat`,
    { method: "DELETE" }
  ) as Promise<{ revoked: boolean }>;
}

export async function validateProjectPat(
  request: Request,
  projectId: string
): Promise<ProjectPatValidation> {
  return apiFetch(
    request,
    `/api/projects/${encodeURIComponent(projectId)}/pat/validate`,
    { method: "POST", body: "{}" }
  ) as Promise<ProjectPatValidation>;
}

// ---------------------------------------------------------------------------
// Spec 108 Phase 4 — read-only browsers.
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

export async function listFactoryAdapters(request: Request) {
  return apiFetch(request, "/api/factory/adapters") as Promise<{
    adapters: FactoryResourceSummary[];
  }>;
}

export async function getFactoryAdapter(request: Request, name: string) {
  return apiFetch(
    request,
    `/api/factory/adapters/${encodeURIComponent(name)}`
  ) as Promise<FactoryAdapterDetail>;
}

export async function listFactoryContracts(request: Request) {
  return apiFetch(request, "/api/factory/contracts") as Promise<{
    contracts: FactoryResourceSummary[];
  }>;
}

export async function getFactoryContract(request: Request, name: string) {
  return apiFetch(
    request,
    `/api/factory/contracts/${encodeURIComponent(name)}`
  ) as Promise<FactoryContractDetail>;
}

export async function listFactoryProcesses(request: Request) {
  return apiFetch(request, "/api/factory/processes") as Promise<{
    processes: FactoryResourceSummary[];
  }>;
}

export async function getFactoryProcess(request: Request, name: string) {
  return apiFetch(
    request,
    `/api/factory/processes/${encodeURIComponent(name)}`
  ) as Promise<FactoryProcessDetail>;
}
