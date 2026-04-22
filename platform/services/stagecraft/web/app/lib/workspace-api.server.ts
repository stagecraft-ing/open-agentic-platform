/**
 * Server-side API helpers for workspace, knowledge, and factory endpoints.
 *
 * Uses direct fetch with cookie forwarding — the Encore-generated client
 * does not support forwarding cookies from incoming SSR requests, which is
 * required for React Router v7 server-side loaders.
 */

const DEFAULT_API_BASE = "http://localhost:4000";

// SSR runs in the same pod as the Encore API. Calling back via the public
// hostname is a pointless trip through Cloudflare + ingress and drops the
// Cookie header when HTTP is upgraded to HTTPS mid-flight. Loop back via
// localhost:4000 instead.
function getBaseUrl(_request: Request): string {
  return process.env.ENCORE_API_BASE_URL ?? DEFAULT_API_BASE;
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

export async function getConnector(request: Request, id: string) {
  return apiFetch(request, `/api/knowledge/connectors/${id}`) as Promise<{
    connector: SourceConnectorRow;
  }>;
}

export async function updateConnector(
  request: Request,
  id: string,
  data: { name?: string; config?: Record<string, unknown>; syncSchedule?: string | null; status?: string }
) {
  return apiFetch(request, `/api/knowledge/connectors/${id}`, {
    method: "PATCH",
    body: JSON.stringify(data),
  }) as Promise<{ connector: SourceConnectorRow }>;
}

export async function deleteConnector(request: Request, id: string) {
  return apiFetch(request, `/api/knowledge/connectors/${id}`, {
    method: "DELETE",
  }) as Promise<{ deleted: boolean }>;
}

export async function testConnectorConnection(request: Request, id: string) {
  return apiFetch(request, `/api/knowledge/connectors/${id}/test`, {
    method: "POST",
    body: "{}",
  }) as Promise<{ success: boolean; error?: string }>;
}

export async function triggerSync(request: Request, id: string) {
  return apiFetch(request, `/api/knowledge/connectors/${id}/sync`, {
    method: "POST",
    body: "{}",
  }) as Promise<{ syncRunId: string }>;
}

export async function listSyncRuns(request: Request, connectorId: string) {
  return apiFetch(
    request,
    `/api/knowledge/connectors/${connectorId}/sync-runs`
  ) as Promise<{ runs: SyncRunRow[] }>;
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

export async function getFactoryStatus(
  request: Request,
  projectId: string,
  workspaceId: string
) {
  const qs = `?workspaceId=${encodeURIComponent(workspaceId)}`;
  return apiFetch(
    request,
    `/api/projects/${projectId}/factory/status${qs}`
  ) as Promise<{ pipeline: PipelineStatusRow | null }>;
}

export async function listFactoryAudit(
  request: Request,
  projectId: string,
  workspaceId: string
) {
  const qs = `?workspaceId=${encodeURIComponent(workspaceId)}`;
  return apiFetch(
    request,
    `/api/projects/${projectId}/factory/audit${qs}`
  ) as Promise<{ entries: FactoryAuditEntry[] }>;
}

export async function initFactoryPipeline(
  request: Request,
  projectId: string,
  data: {
    adapter: string;
    actorUserId: string;
    workspaceId: string;
    business_docs?: Array<{ name: string; storage_ref: string }>;
    knowledge_object_ids?: string[];
    policy_overrides?: Record<string, unknown>;
  }
) {
  return apiFetch(request, `/api/projects/${projectId}/factory/init`, {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{
    pipeline_id: string;
    adapter: string;
    policy_bundle_id: string;
    status: string;
    created_at: string;
  }>;
}

export async function confirmFactoryStage(
  request: Request,
  projectId: string,
  stageId: string,
  actorUserId: string,
  workspaceId: string,
  notes?: string
) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/factory/stage/${stageId}/confirm`,
    { method: "POST", body: JSON.stringify({ actorUserId, notes, workspaceId }) }
  ) as Promise<{ stage: FactoryStageRow }>;
}

export async function rejectFactoryStage(
  request: Request,
  projectId: string,
  stageId: string,
  actorUserId: string,
  workspaceId: string,
  feedback: string
) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/factory/stage/${stageId}/reject`,
    { method: "POST", body: JSON.stringify({ actorUserId, feedback, workspaceId }) }
  ) as Promise<{ stage: FactoryStageRow }>;
}

export async function cancelPipeline(
  request: Request,
  projectId: string,
  actorUserId: string,
  workspaceId: string,
  reason?: string
) {
  return apiFetch(request, `/api/projects/${projectId}/factory/cancel`, {
    method: "POST",
    body: JSON.stringify({
      actorUserId,
      workspaceId,
      reason: reason ?? "Cancelled from web UI",
    }),
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

export type SyncRunRow = {
  id: string;
  connectorId: string;
  workspaceId: string;
  status: string;
  objectsCreated: number;
  objectsUpdated: number;
  objectsSkipped: number;
  error: string | null;
  deltaToken: string | null;
  startedAt: string;
  completedAt: string | null;
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
