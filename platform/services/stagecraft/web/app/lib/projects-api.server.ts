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
