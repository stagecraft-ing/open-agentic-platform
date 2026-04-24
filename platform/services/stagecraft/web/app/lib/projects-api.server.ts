/**
 * Projects API helpers using direct fetch.
 * The Encore generated client now includes the projects service
 * (see client.ts projects.ServiceClient), but these helpers are kept
 * because they forward cookies and derive the base URL from the incoming
 * request — behavior the generated client does not support.
 * Decision: Keep manual fetch. The Encore generated client does not support
 * forwarding cookies from incoming SSR requests, which is required for
 * server-side loaders in React Router v7 to proxy the user's session cookie.
 */

const DEFAULT_API_BASE = "http://localhost:4000";

/**
 * Resolve the base URL for the SSR → Encore API hop.
 *
 * In production, the RR SSR runs in the same pod as the Encore API. Routing
 * the call back out through the public hostname means a pointless trip
 * through Cloudflare + ingress, and — if `x-forwarded-proto` is missing —
 * reconstructs `request.url` as `http://…`, which Cloudflare then redirects
 * to HTTPS, causing undici to drop the Cookie header on the scheme change.
 * The result is a 401 for the inner call even though the outer request was
 * perfectly authenticated.
 *
 * Prefer `ENCORE_API_BASE_URL` when set, otherwise always loop back via
 * localhost:4000 (same pod). `request` is accepted for future use.
 */
function getBaseUrl(_request: Request): string {
  return process.env.ENCORE_API_BASE_URL ?? DEFAULT_API_BASE;
}

async function apiFetch(request: Request, path: string, init?: RequestInit) {
  const base = getBaseUrl(request);
  const cookie = request.headers.get("Cookie") ?? "";
  const fullUrl = `${base}${path}`;
  const method = init?.method ?? "GET";
  const hasSessionCookie = cookie.includes("__session=");
  console.log("apiFetch outbound", {
    method,
    url: fullUrl,
    requestUrl: request.url,
    hasSessionCookie,
    cookieLen: cookie.length,
    envBase: process.env.ENCORE_API_BASE_URL ?? null,
  });
  let res: Response;
  try {
    res = await fetch(fullUrl, {
      ...init,
      headers: {
        "Content-Type": "application/json",
        ...(cookie && { Cookie: cookie }),
        ...init?.headers,
      },
    });
  } catch (err) {
    const cause = err instanceof Error ? err.message : String(err);
    console.error("apiFetch network error", { method, url: fullUrl, cause });
    throw new Error(`Network error calling ${path}: ${cause}`);
  }
  if (!res.ok) {
    const body = await res.text();
    console.error("apiFetch non-ok response", {
      method,
      url: fullUrl,
      status: res.status,
      bodyPreview: body.slice(0, 300),
    });
    throw new Error(body || `API error: ${res.status}`);
  }
  return res.json();
}

// Projects

export async function listProjects(request: Request) {
  return apiFetch(request, "/api/projects") as Promise<{ projects: any[] }>;
}

export async function getProject(request: Request, id: string) {
  return apiFetch(request, `/api/projects/${id}`) as Promise<{
    project: any;
  }>;
}

export async function createProject(
  request: Request,
  data: {
    name: string;
    slug: string;
    description?: string;
    actorUserId: string;
  }
) {
  return apiFetch(request, "/api/projects", {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{ project: any }>;
}

export async function deleteProject(
  request: Request,
  id: string,
  actorUserId: string
) {
  return apiFetch(
    request,
    `/api/projects/${id}?actorUserId=${encodeURIComponent(actorUserId)}`,
    { method: "DELETE" }
  ) as Promise<{ ok: true }>;
}

// Self-service project creation (spec 080 Phase 2)

export async function createProjectWithRepo(
  request: Request,
  data: {
    name: string;
    slug: string;
    description?: string;
    adapter: string;
    repoName: string;
    isPrivate?: boolean;
  }
) {
  return apiFetch(request, "/api/projects/with-repo", {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{
    project: any;
    repo: any;
    environments: any[];
    githubRepoUrl: string;
  }>;
}

// Spec 112 §5.2 — ACP-native factory project creation.
export async function createFactoryProject(
  request: Request,
  data: {
    name: string;
    slug: string;
    description?: string;
    adapterId: string;
    variant: "single-public" | "single-internal" | "dual";
    profileName?: string;
    repoName: string;
    isPrivate?: boolean;
  }
) {
  return apiFetch(request, "/api/projects/factory-create", {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{
    projectId: string;
    repoUrl: string;
    cloneUrl: string;
    oapDeepLink: string;
    scaffoldJobId: string;
    factoryAdapterId: string;
  }>;
}

export async function listFactoryAdapters(request: Request) {
  return apiFetch(request, "/api/factory/adapters") as Promise<{
    adapters: Array<{
      id: string;
      name: string;
      version: string;
      sourceSha: string;
    }>;
  }>;
}

// Spec 112 §6.2 — factory project import.
export interface ImportedRawArtifact {
  objectId: string;
  filename: string;
  relativePath: string;
  contentHash: string;
  sizeBytes: number;
}

export async function importFactoryProject(
  request: Request,
  data: {
    repoUrl: string;
    name?: string;
    slug?: string;
    description?: string;
    previewOnly?: boolean;
  }
) {
  return apiFetch(request, "/api/projects/factory-import", {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{
    projectId: string | null;
    detectionLevel:
      | "not_factory"
      | "scaffold_only"
      | "legacy_produced"
      | "acp_produced";
    repoUrl: string;
    cloneUrl: string;
    oapDeepLink: string | null;
    translatorVersion: string | null;
    translatedPreview?: Record<string, unknown>;
    previewOnly: boolean;
    rawArtifacts: ImportedRawArtifact[];
    rawArtifactsSkipped: number;
  }>;
}

// Spec 087 Phase 2 + spec 112 §6 — per-project knowledge object views.
export interface ProjectKnowledgeObject {
  id: string;
  filename: string;
  mimeType: string;
  sizeBytes: number;
  contentHash: string;
  state:
    | "imported"
    | "extracting"
    | "extracted"
    | "classified"
    | "available";
  storageKey: string;
  extractedStorageKey: string | null;
  provenance: Record<string, unknown>;
  boundAt: string;
  updatedAt: string;
}

export async function listProjectKnowledge(
  request: Request,
  projectId: string
) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/knowledge`
  ) as Promise<{ objects: ProjectKnowledgeObject[] }>;
}

export async function advanceKnowledgeToExtracted(
  request: Request,
  projectId: string,
  objectId: string
) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/knowledge/${objectId}/advance-extracted`,
    { method: "POST" }
  ) as Promise<{
    objectId: string;
    state: "extracted";
    extractedStorageKey: string;
    summary: {
      ok: number;
      cached: number;
      error: number;
      skip_unsupported: number;
    };
    extractorMessage: string;
  }>;
}

// Repos

export async function listProjectRepos(request: Request, projectId: string) {
  return apiFetch(request, `/api/projects/${projectId}/repos`) as Promise<{
    repos: any[];
  }>;
}

export async function addProjectRepo(
  request: Request,
  projectId: string,
  data: {
    githubOrg: string;
    repoName: string;
    defaultBranch?: string;
    isPrimary?: boolean;
    actorUserId: string;
  }
) {
  return apiFetch(request, `/api/projects/${projectId}/repos`, {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{ repo: any }>;
}

export async function removeProjectRepo(
  request: Request,
  projectId: string,
  repoId: string,
  actorUserId: string
) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/repos/${repoId}?actorUserId=${encodeURIComponent(actorUserId)}`,
    { method: "DELETE" }
  ) as Promise<{ ok: true }>;
}

// Environments

export async function listEnvironments(request: Request, projectId: string) {
  return apiFetch(request, `/api/projects/${projectId}/envs`) as Promise<{
    environments: any[];
  }>;
}

export async function createEnvironment(
  request: Request,
  projectId: string,
  data: {
    name: string;
    kind?: string;
    autoDeployBranch?: string;
    requiresApproval?: boolean;
    actorUserId: string;
  }
) {
  return apiFetch(request, `/api/projects/${projectId}/envs`, {
    method: "POST",
    body: JSON.stringify(data),
  }) as Promise<{ environment: any }>;
}

export async function deleteEnvironment(
  request: Request,
  projectId: string,
  envId: string,
  actorUserId: string
) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/envs/${envId}?actorUserId=${encodeURIComponent(actorUserId)}`,
    { method: "DELETE" }
  ) as Promise<{ ok: true }>;
}

// Members

export async function listProjectMembers(
  request: Request,
  projectId: string
) {
  return apiFetch(request, `/api/projects/${projectId}/members`) as Promise<{
    members: any[];
  }>;
}
