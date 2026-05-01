/**
 * Agent catalog SSR API helpers.
 *
 * Two families of helpers:
 *
 * 1. **Project-scoped** (legacy, spec 111 + 119): list/create take an
 *    explicit `:projectId`. Phase 5 of spec 123 will delete/rewrite these.
 *    DO NOT remove them yet — the project agent routes still reference them.
 *
 * 2. **Org-scoped** (spec 123 §5.1): `listOrgAgents`, `getOrgAgent`,
 *    `createOrgAgent`, `patchOrgAgent`, `publishOrgAgent`, `retireOrgAgent`,
 *    `forkOrgAgent`, `listOrgAgentAudit` — hit `/api/orgs/:orgId/agents…`.
 *    The `OrgCatalogAgent` type carries `org_id` (not `project_id`).
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

// ---------------------------------------------------------------------------
// Project-scoped types (spec 111/119 — Phase 5 will rewrite these)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Org-scoped types (spec 123 §5.1)
// ---------------------------------------------------------------------------

/** Wire shape for org-scoped agent catalog rows (spec 123). */
export type OrgCatalogAgent = {
  id: string;
  org_id: string;
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

export type OrgCatalogAuditEntry = {
  id: string;
  agent_id: string;
  org_id: string;
  action: AgentCatalogAuditAction;
  actor_user_id: string;
  before: Record<string, unknown> | null;
  after: Record<string, unknown> | null;
  created_at: string;
};

// ---------------------------------------------------------------------------
// Project-scoped helpers (legacy — Phase 5 deletes/rewrites)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Org-scoped helpers (spec 123 §5.1)
// ---------------------------------------------------------------------------

export async function listOrgAgents(
  request: Request,
  orgId: string,
  status?: AgentCatalogStatus,
) {
  const qs = status ? `?status=${encodeURIComponent(status)}` : "";
  return apiFetch(
    request,
    `/api/orgs/${orgId}/agents${qs}`,
  ) as Promise<{ agents: OrgCatalogAgent[] }>;
}

export async function getOrgAgent(
  request: Request,
  orgId: string,
  id: string,
) {
  return apiFetch(
    request,
    `/api/orgs/${orgId}/agents/${id}`,
  ) as Promise<{ agent: OrgCatalogAgent }>;
}

export async function createOrgAgent(
  request: Request,
  orgId: string,
  data: {
    name: string;
    frontmatter: CatalogFrontmatter;
    body_markdown: string;
  },
) {
  return apiFetch(request, `/api/orgs/${orgId}/agents`, {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{ agent: OrgCatalogAgent }>;
}

export async function patchOrgAgent(
  request: Request,
  orgId: string,
  id: string,
  data: {
    frontmatter?: CatalogFrontmatter;
    body_markdown?: string;
    expected_content_hash?: string;
  },
) {
  return apiFetch(request, `/api/orgs/${orgId}/agents/${id}`, {
    method: "PATCH",
    body: JSON.stringify(data),
  }) as Promise<{ agent: OrgCatalogAgent }>;
}

export async function publishOrgAgent(
  request: Request,
  orgId: string,
  id: string,
) {
  return apiFetch(request, `/api/orgs/${orgId}/agents/${id}/publish`, {
    method: "POST",
    body: "{}",
  }) as Promise<{ agent: OrgCatalogAgent; retired?: OrgCatalogAgent }>;
}

export async function retireOrgAgent(
  request: Request,
  orgId: string,
  id: string,
) {
  return apiFetch(request, `/api/orgs/${orgId}/agents/${id}/retire`, {
    method: "POST",
    body: "{}",
  }) as Promise<{ agent: OrgCatalogAgent }>;
}

export async function forkOrgAgent(
  request: Request,
  orgId: string,
  id: string,
  newName: string,
) {
  return apiFetch(request, `/api/orgs/${orgId}/agents/${id}/fork`, {
    method: "POST",
    body: JSON.stringify({ new_name: newName }),
  }) as Promise<{ agent: OrgCatalogAgent }>;
}

export async function listOrgAgentAudit(
  request: Request,
  orgId: string,
  id: string,
) {
  return apiFetch(
    request,
    `/api/orgs/${orgId}/agents/${id}/audit`,
  ) as Promise<{ entries: OrgCatalogAuditEntry[] }>;
}
