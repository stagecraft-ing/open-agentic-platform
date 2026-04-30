/**
 * Agent catalog SSR API helpers.
 *
 * Shape mirrors the Encore.ts endpoints in `api/agents/catalog.ts`. The
 * catalog is project-scoped: list/create take an explicit `:projectId`,
 * while detail endpoints (`/api/agents/:id`) resolve the project from the
 * agent row and verify it belongs to the caller's org.
 */

import type {
  AgentCatalogAuditAction,
  AgentCatalogStatus,
} from "../../../api/db/schema";
import type { CatalogFrontmatter } from "../../../api/agents/frontmatter";

export type {
  AgentCatalogAuditAction,
  AgentCatalogStatus,
};
export type { CatalogFrontmatter };

const DEFAULT_API_BASE = "http://localhost:4000";

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

export type CatalogAgent = {
  id: string;
  project_id: string;
  name: string;
  version: number;
  status: AgentCatalogStatus;
  frontmatter: CatalogFrontmatter;
  body_markdown: string;
  content_hash: string;
  created_by: string;
  created_at: string;
  updated_at: string;
};

export type CatalogAuditEntry = {
  id: string;
  agent_id: string;
  project_id: string;
  action: AgentCatalogAuditAction;
  actor_user_id: string;
  before: Record<string, unknown> | null;
  after: Record<string, unknown> | null;
  created_at: string;
};

export async function listAgents(
  request: Request,
  projectId: string,
  status?: AgentCatalogStatus,
) {
  const qs = status ? `?status=${encodeURIComponent(status)}` : "";
  return apiFetch(
    request,
    `/api/projects/${projectId}/agents${qs}`,
  ) as Promise<{ agents: CatalogAgent[] }>;
}

export async function getAgent(request: Request, id: string) {
  return apiFetch(request, `/api/agents/${id}`) as Promise<{
    agent: CatalogAgent;
  }>;
}

export async function createAgent(
  request: Request,
  projectId: string,
  data: {
    name: string;
    frontmatter: CatalogFrontmatter;
    body_markdown: string;
  },
) {
  return apiFetch(request, `/api/projects/${projectId}/agents`, {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{ agent: CatalogAgent }>;
}

export async function patchAgent(
  request: Request,
  id: string,
  data: {
    frontmatter?: CatalogFrontmatter;
    body_markdown?: string;
    expected_content_hash?: string;
  },
) {
  return apiFetch(request, `/api/agents/${id}`, {
    method: "PATCH",
    body: JSON.stringify(data),
  }) as Promise<{ agent: CatalogAgent }>;
}

export async function publishAgent(request: Request, id: string) {
  return apiFetch(request, `/api/agents/${id}/publish`, {
    method: "POST",
    body: "{}",
  }) as Promise<{ agent: CatalogAgent; retired?: CatalogAgent }>;
}

export async function retireAgent(request: Request, id: string) {
  return apiFetch(request, `/api/agents/${id}/retire`, {
    method: "POST",
    body: "{}",
  }) as Promise<{ agent: CatalogAgent }>;
}

export async function forkAgent(
  request: Request,
  id: string,
  newName: string,
) {
  return apiFetch(request, `/api/agents/${id}/fork`, {
    method: "POST",
    body: JSON.stringify({ new_name: newName }),
  }) as Promise<{ agent: CatalogAgent }>;
}

export async function listAgentAudit(request: Request, id: string) {
  return apiFetch(request, `/api/agents/${id}/audit`) as Promise<{
    entries: CatalogAuditEntry[];
  }>;
}
