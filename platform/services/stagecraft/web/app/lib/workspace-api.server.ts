/**
 * Server-side API helpers for workspace, knowledge, and factory endpoints.
 *
 * Uses direct fetch with cookie forwarding — the Encore-generated client
 * does not support forwarding cookies from incoming SSR requests, which is
 * required for React Router v7 server-side loaders.
 */

const DEFAULT_API_BASE = "http://localhost:4000";

function getBaseUrl(request: Request): string {
  try {
    const url = new URL(request.url);
    if (url.hostname === "origin") {
      return process.env.ENCORE_API_BASE_URL ?? DEFAULT_API_BASE;
    }
    return url.origin;
  } catch {
    return process.env.ENCORE_API_BASE_URL ?? DEFAULT_API_BASE;
  }
}

async function apiFetch(request: Request, path: string, init?: RequestInit) {
  const base = getBaseUrl(request);
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

// =========================================================================
// Workspaces
// =========================================================================

export async function listWorkspaces(request: Request) {
  return apiFetch(request, "/api/workspaces") as Promise<{
    workspaces: WorkspaceRow[];
  }>;
}

export async function getWorkspace(request: Request, id: string) {
  return apiFetch(request, `/api/workspaces/${id}`) as Promise<{
    workspace: WorkspaceRow;
  }>;
}

export async function getDefaultWorkspace(request: Request) {
  return apiFetch(request, "/api/workspaces/by-org/default") as Promise<{
    workspace: WorkspaceRow;
  }>;
}

export async function createWorkspace(
  request: Request,
  data: { name: string; slug: string }
) {
  return apiFetch(request, "/api/workspaces", {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{ workspace: WorkspaceRow }>;
}

// =========================================================================
// Knowledge Objects
// =========================================================================

export async function listKnowledgeObjects(
  request: Request,
  state?: string
) {
  const qs = state ? `?state=${encodeURIComponent(state)}` : "";
  return apiFetch(request, `/api/knowledge/objects${qs}`) as Promise<{
    objects: KnowledgeObjectRow[];
  }>;
}

export async function getKnowledgeObject(request: Request, id: string) {
  return apiFetch(request, `/api/knowledge/objects/${id}`) as Promise<{
    object: KnowledgeObjectRow;
  }>;
}

export async function requestUpload(
  request: Request,
  data: { filename: string; mimeType: string; contentHash: string }
) {
  return apiFetch(request, "/api/knowledge/upload", {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{ objectId: string; uploadUrl: string; storageKey: string }>;
}

export async function confirmUpload(request: Request, objectId: string) {
  return apiFetch(request, `/api/knowledge/objects/${objectId}/confirm`, {
    method: "POST",
    body: "{}",
  }) as Promise<{ object: KnowledgeObjectRow }>;
}

export async function getDownloadUrl(request: Request, objectId: string) {
  return apiFetch(
    request,
    `/api/knowledge/objects/${objectId}/download`
  ) as Promise<{ downloadUrl: string }>;
}

export async function transitionKnowledgeState(
  request: Request,
  objectId: string,
  data: {
    targetState: string;
    extractionOutput?: Record<string, unknown>;
    classification?: string[];
  }
) {
  return apiFetch(request, `/api/knowledge/objects/${objectId}/transition`, {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{ object: KnowledgeObjectRow }>;
}

export async function deleteKnowledgeObject(
  request: Request,
  objectId: string
) {
  return apiFetch(request, `/api/knowledge/objects/${objectId}`, {
    method: "DELETE",
  }) as Promise<{ deleted: boolean }>;
}

// =========================================================================
// Source Connectors
// =========================================================================

export async function listConnectors(request: Request) {
  return apiFetch(request, "/api/knowledge/connectors") as Promise<{
    connectors: SourceConnectorRow[];
  }>;
}

export async function createConnector(
  request: Request,
  data: { type: string; name: string; config?: Record<string, unknown>; syncSchedule?: string }
) {
  return apiFetch(request, "/api/knowledge/connectors", {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{ connector: SourceConnectorRow }>;
}

// =========================================================================
// Document Bindings
// =========================================================================

export async function listBindings(request: Request, projectId: string) {
  return apiFetch(
    request,
    `/api/knowledge/bindings/${projectId}`
  ) as Promise<{ bindings: DocumentBindingRow[] }>;
}

export async function bindToProject(
  request: Request,
  projectId: string,
  knowledgeObjectIds: string[]
) {
  return apiFetch(request, `/api/knowledge/bindings/${projectId}`, {
    method: "POST",
    body: JSON.stringify({ knowledgeObjectIds }),
  }) as Promise<{ bindings: DocumentBindingRow[] }>;
}

// =========================================================================
// Factory Pipelines
// =========================================================================

export async function getFactoryStatus(request: Request, projectId: string) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/factory/status`
  ) as Promise<{ pipeline: PipelineStatusRow | null }>;
}

export async function listFactoryAudit(request: Request, projectId: string) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/factory/audit`
  ) as Promise<{ entries: FactoryAuditEntry[] }>;
}

export async function confirmFactoryStage(
  request: Request,
  projectId: string,
  stageId: string,
  actorUserId: string,
  notes?: string
) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/factory/stage/${stageId}/confirm`,
    { method: "POST", body: JSON.stringify({ actorUserId, notes }) }
  ) as Promise<{ stage: FactoryStageRow }>;
}

export async function rejectFactoryStage(
  request: Request,
  projectId: string,
  stageId: string,
  actorUserId: string,
  reason: string
) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/factory/stage/${stageId}/reject`,
    { method: "POST", body: JSON.stringify({ actorUserId, reason }) }
  ) as Promise<{ stage: FactoryStageRow }>;
}

export async function cancelPipeline(
  request: Request,
  projectId: string,
  actorUserId: string,
  reason?: string
) {
  return apiFetch(request, `/api/projects/${projectId}/factory/cancel`, {
    method: "POST",
    body: JSON.stringify({ actorUserId, reason: reason ?? "Cancelled from web UI" }),
  }) as Promise<{ pipeline: PipelineStatusRow }>;
}

// =========================================================================
// Deploy
// =========================================================================

export async function listEnvironments(request: Request, projectId: string) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/envs`
  ) as Promise<{ environments: EnvironmentRow[] }>;
}

// =========================================================================
// Types (server-side, matching API responses)
// =========================================================================

export type WorkspaceRow = {
  id: string;
  orgId: string;
  name: string;
  slug: string;
  objectStoreBucket: string;
  createdAt: string;
  updatedAt: string;
};

export type KnowledgeObjectRow = {
  id: string;
  workspaceId: string;
  connectorId: string | null;
  storageKey: string;
  filename: string;
  mimeType: string;
  sizeBytes: number;
  contentHash: string;
  state: string;
  extractionOutput: unknown;
  classification: unknown;
  provenance: {
    sourceType: string;
    sourceUri: string;
    importedAt: string;
    lastSyncedAt?: string;
    versionId?: string;
  };
  createdAt: string;
  updatedAt: string;
};

export type SourceConnectorRow = {
  id: string;
  workspaceId: string;
  type: string;
  name: string;
  syncSchedule: string | null;
  status: string;
  lastSyncedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type DocumentBindingRow = {
  id: string;
  projectId: string;
  knowledgeObjectId: string;
  boundBy: string;
  boundAt: string;
};

export type PipelineStatusRow = {
  id: string;
  projectId: string;
  adapterName: string;
  status: string;
  policyBundleId: string | null;
  buildSpecHash: string | null;
  startedAt: string | null;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
  stages?: FactoryStageRow[];
};

export type FactoryStageRow = {
  id: string;
  pipelineId: string;
  stageIndex: number;
  stageId: string;
  status: string;
  output: unknown;
  startedAt: string | null;
  completedAt: string | null;
};

export type FactoryAuditEntry = {
  id: string;
  pipelineId: string;
  timestamp: string;
  event: string;
  actor: string | null;
  stageId: string | null;
  featureId: string | null;
  details: unknown;
};

export type EnvironmentRow = {
  id: string;
  projectId: string;
  name: string;
  kind: string;
  k8sNamespace: string | null;
  autoDeployBranch: string | null;
  requiresApproval: boolean;
  createdAt: string;
  updatedAt: string;
};
