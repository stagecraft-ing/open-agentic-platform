/**
 * Typed HTTP client for the Stagecraft Platform API (spec 087).
 *
 * Provides workspace CRUD, knowledge intake, connector management,
 * and sync event posting. Consumed by OPC desktop and the web UI.
 */

import type {
  Workspace,
  CreateWorkspaceRequest,
  UpdateWorkspaceRequest,
  ListWorkspacesResponse,
  GetWorkspaceResponse,
} from "./workspace";

import type {
  KnowledgeObject,
  SourceConnector,
  SyncRun,
  DocumentBinding,
  ConnectorType,
  ConnectorConfig,
  SyncSchedule,
} from "./knowledge";

import type { OpcEvent } from "./sync";

// ---------------------------------------------------------------------------
// Client factory
// ---------------------------------------------------------------------------

export interface StagecraftClientOptions {
  baseUrl: string;
  token: string;
}

export interface StagecraftClient {
  // -- Workspaces --
  listWorkspaces(): Promise<ListWorkspacesResponse>;
  getWorkspace(id: string): Promise<GetWorkspaceResponse>;
  createWorkspace(req: CreateWorkspaceRequest): Promise<Workspace>;
  updateWorkspace(id: string, req: UpdateWorkspaceRequest): Promise<Workspace>;

  // -- Knowledge objects --
  listKnowledgeObjects(workspaceId: string): Promise<KnowledgeObject[]>;
  getKnowledgeObject(workspaceId: string, id: string): Promise<KnowledgeObject>;
  deleteKnowledgeObject(workspaceId: string, id: string): Promise<void>;
  requestUpload(workspaceId: string, filename: string, mimeType: string): Promise<{ uploadUrl: string; objectId: string }>;
  confirmUpload(workspaceId: string, objectId: string): Promise<KnowledgeObject>;

  // -- Connectors --
  listConnectors(workspaceId: string): Promise<SourceConnector[]>;
  getConnector(workspaceId: string, id: string): Promise<SourceConnector>;
  createConnector(workspaceId: string, req: CreateConnectorRequest): Promise<SourceConnector>;
  deleteConnector(workspaceId: string, id: string): Promise<void>;
  testConnection(workspaceId: string, id: string): Promise<{ ok: boolean; error?: string }>;
  triggerSync(workspaceId: string, connectorId: string): Promise<SyncRun>;
  listSyncRuns(workspaceId: string, connectorId: string): Promise<SyncRun[]>;

  // -- Bindings --
  listBindings(projectId: string): Promise<DocumentBinding[]>;
  bindToProject(projectId: string, knowledgeObjectId: string): Promise<DocumentBinding>;
  unbindFromProject(projectId: string, bindingId: string): Promise<void>;

  // -- Sync (OPC → Stagecraft) --
  postOpcEvent(event: OpcEvent): Promise<void>;
}

export interface CreateConnectorRequest {
  type: ConnectorType;
  name: string;
  config: ConnectorConfig;
  syncSchedule?: SyncSchedule;
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

export function createStagecraftClient(opts: StagecraftClientOptions): StagecraftClient {
  const { baseUrl, token } = opts;

  async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = `${baseUrl.replace(/\/$/, "")}${path}`;
    const res = await fetch(url, {
      method,
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!res.ok) {
      const text = await res.text().catch(() => "");
      throw new Error(`Stagecraft API ${method} ${path} returned ${res.status}: ${text}`);
    }
    if (res.status === 204) return undefined as T;
    return res.json() as Promise<T>;
  }

  return {
    // -- Workspaces --
    listWorkspaces: () => request<ListWorkspacesResponse>("GET", "/api/workspaces"),
    getWorkspace: (id) => request<GetWorkspaceResponse>("GET", `/api/workspaces/${id}`),
    createWorkspace: (req) => request<Workspace>("POST", "/api/workspaces", req),
    updateWorkspace: (id, req) => request<Workspace>("PUT", `/api/workspaces/${id}`, req),

    // -- Knowledge objects --
    listKnowledgeObjects: (wsId) =>
      request<KnowledgeObject[]>("GET", `/api/workspaces/${wsId}/knowledge`),
    getKnowledgeObject: (wsId, id) =>
      request<KnowledgeObject>("GET", `/api/workspaces/${wsId}/knowledge/${id}`),
    deleteKnowledgeObject: (wsId, id) =>
      request<void>("DELETE", `/api/workspaces/${wsId}/knowledge/${id}`),
    requestUpload: (wsId, filename, mimeType) =>
      request("POST", `/api/workspaces/${wsId}/knowledge/upload`, { filename, mimeType }),
    confirmUpload: (wsId, objectId) =>
      request<KnowledgeObject>("POST", `/api/workspaces/${wsId}/knowledge/${objectId}/confirm`),

    // -- Connectors --
    listConnectors: (wsId) =>
      request<SourceConnector[]>("GET", `/api/workspaces/${wsId}/connectors`),
    getConnector: (wsId, id) =>
      request<SourceConnector>("GET", `/api/workspaces/${wsId}/connectors/${id}`),
    createConnector: (wsId, req) =>
      request<SourceConnector>("POST", `/api/workspaces/${wsId}/connectors`, req),
    deleteConnector: (wsId, id) =>
      request<void>("DELETE", `/api/workspaces/${wsId}/connectors/${id}`),
    testConnection: (wsId, id) =>
      request("POST", `/api/workspaces/${wsId}/connectors/${id}/test`),
    triggerSync: (wsId, connectorId) =>
      request<SyncRun>("POST", `/api/workspaces/${wsId}/connectors/${connectorId}/sync`),
    listSyncRuns: (wsId, connectorId) =>
      request<SyncRun[]>("GET", `/api/workspaces/${wsId}/connectors/${connectorId}/runs`),

    // -- Bindings --
    listBindings: (projectId) =>
      request<DocumentBinding[]>("GET", `/api/projects/${projectId}/bindings`),
    bindToProject: (projectId, knowledgeObjectId) =>
      request<DocumentBinding>("POST", `/api/projects/${projectId}/bindings`, { knowledgeObjectId }),
    unbindFromProject: (projectId, bindingId) =>
      request<void>("DELETE", `/api/projects/${projectId}/bindings/${bindingId}`),

    // -- Sync --
    postOpcEvent: (event) => request<void>("POST", "/api/sync/opc-events", event),
  };
}
