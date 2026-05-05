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

// Spec 113 §FR-039 — `hasPrimaryRepo` is computed server-side via an EXISTS
// subquery so the projects index can hide the Clone affordance for projects
// without a primary repo without a second round-trip. `primaryRepoName`
// drives the dialog's `<sourceRepoName>-clone` pre-fill.
export interface ProjectListEntry {
  id: string;
  orgId: string;
  name: string;
  slug: string;
  description: string;
  factoryAdapterId: string | null;
  createdBy: string | null;
  createdAt: string;
  updatedAt: string;
  hasPrimaryRepo: boolean;
  primaryRepoName: string | null;
}

export async function listProjects(request: Request) {
  return apiFetch(request, "/api/projects") as Promise<{
    projects: ProjectListEntry[];
    destinationGithubOrgLogin: string | null;
  }>;
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

// Spec 113 — Clone Project (availability + submit).

export type CloneAvailabilityState =
  | "available"
  | "unavailable"
  | "invalid"
  | "unverifiable";

export type CloneAvailabilityReason =
  | "format"
  | "exists"
  | "rate_limited"
  | "no_installation"
  | "transient_error";

export interface CloneAvailabilityVerdict {
  value: string;
  state: CloneAvailabilityState;
  reason?: CloneAvailabilityReason;
  retryAfterSec?: number;
}

export interface CloneAvailabilityResponse {
  repoName?: CloneAvailabilityVerdict;
  slug?: CloneAvailabilityVerdict;
}

export async function checkCloneAvailability(
  request: Request,
  params: { repoName?: string; slug?: string }
) {
  const qs = new URLSearchParams();
  if (params.repoName !== undefined) qs.set("repoName", params.repoName);
  if (params.slug !== undefined) qs.set("slug", params.slug);
  return apiFetch(
    request,
    `/api/projects/clone/check-availability?${qs.toString()}`
  ) as Promise<CloneAvailabilityResponse>;
}

/**
 * Spec 114 §5.1 — the submit endpoint now queues and returns a job id.
 * The dialog polls `getCloneRunStatus` until the run is terminal.
 */
export interface CloneJobAccepted {
  cloneJobId: string;
  status: "queued";
}

export interface CloneRunStatus {
  cloneJobId: string;
  status: "pending" | "running" | "ok" | "failed";
  sourceProjectId: string;
  queuedAt: string;
  startedAt: string | null;
  completedAt: string | null;
  projectId: string | null;
  finalName: string | null;
  finalSlug: string | null;
  repoFullName: string | null;
  defaultBranch: string | null;
  opcDeepLink: string | null;
  rawArtifactsCopied: number | null;
  rawArtifactsSkipped: number | null;
  durationMs: number | null;
  error: string | null;
  errorDetail: string | null;
}

export async function cloneProject(
  request: Request,
  sourceProjectId: string,
  body: { name?: string; slug?: string; repoName?: string }
) {
  return apiFetch(request, `/api/projects/${sourceProjectId}/clone`, {
    method: "POST",
    body: JSON.stringify({ ...body, sourceProjectId }),
  }) as Promise<CloneJobAccepted>;
}

export async function getCloneRunStatus(
  request: Request,
  cloneJobId: string
) {
  return apiFetch(
    request,
    `/api/projects/clone/runs/${encodeURIComponent(cloneJobId)}`
  ) as Promise<CloneRunStatus>;
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
    modules?: string[];
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
    opcDeepLink: string;
    scaffoldJobId: string;
    factoryAdapterId: string;
    devEnvironmentId: string;
    profile: string;
  }>;
}

// Spec 112 Phase 5 — scaffold-readiness gate for the Create form.
// Spec 139 Phase 2 (T056) — added per-adapter eligibility verdicts and the
// `scaffoldSourceResolved` blocker.
export interface AdapterReadinessVerdict {
  id: string;
  name: string;
  declaresScaffoldSource: boolean;
  scaffoldSourceResolved: boolean;
  hasTemplateRemote: boolean;
  createEligible: boolean;
}

export interface ScaffoldReadiness {
  ready: boolean;
  step: string;
  progress: number;
  error?: string;
  hasFactoryAdapter: boolean;
  hasUpstreamPat: boolean;
  hasTemplateRemote: boolean;
  scaffoldSourceResolved: boolean;
  adapters: AdapterReadinessVerdict[];
  canCreate: boolean;
  blocker?:
    | "warming-up"
    | "warmup-error"
    | "no-factory-adapter"
    | "stale-adapter-manifest"
    | "no-scaffold-source-resolved"
    | "no-upstream-pat";
}

export async function getScaffoldReadiness(
  request: Request
): Promise<ScaffoldReadiness> {
  return apiFetch(
    request,
    "/api/projects/scaffold-readiness"
  ) as Promise<ScaffoldReadiness>;
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

// Spec 112 §6.1 — App-installation picker for the Stagecraft Import UI.
export interface ImportInstallationRepo {
  owner: string;
  name: string;
  fullName: string;
  htmlUrl: string;
  cloneUrl: string;
  defaultBranch: string;
  isPrivate: boolean;
}

export interface ImportInstallationEntry {
  installationId: number;
  githubOrgLogin: string;
  error: string | null;
  repos: ImportInstallationRepo[];
}

export async function listImportInstallations(request: Request) {
  return apiFetch(
    request,
    "/api/projects/factory-import/installations"
  ) as Promise<{ installations: ImportInstallationEntry[] }>;
}

export async function importFactoryProject(
  request: Request,
  data: {
    repoUrl: string;
    name?: string;
    slug?: string;
    description?: string;
    previewOnly?: boolean;
    githubPat?: string;
    skipPullRequest?: boolean;
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
    opcDeepLink: string | null;
    translatorVersion: string | null;
    translatedPreview?: Record<string, unknown>;
    previewOnly: boolean;
    rawArtifacts: ImportedRawArtifact[];
    rawArtifactsSkipped: number;
    /** L1 only — URL of the translation PR opened on the source repo. */
    pullRequestUrl?: string | null;
    /** L1 only — message when PR opening failed after registration. */
    pullRequestError?: string;
  }>;
}

// Spec 112 §6.3 — Open-in-OPC bundle for a project.
export interface OpcBundle {
  project: {
    id: string;
    name: string;
    slug: string;
    orgId: string;
  };
  repo: {
    cloneUrl: string;
    githubOrg: string;
    repoName: string;
    defaultBranch: string;
  } | null;
  deepLink: string | null;
  adapter: {
    id: string;
    name: string;
    version: string;
    sourceSha: string;
    syncedAt: string;
    manifest: unknown;
  } | null;
  contracts: Array<{
    name: string;
    version: string;
    sourceSha: string;
    syncedAt: string;
    schema: unknown;
  }>;
  processes: Array<{
    name: string;
    version: string;
    sourceSha: string;
    syncedAt: string;
    definition: unknown;
  }>;
  agents: Array<{
    id: string;
    name: string;
    version: number;
    status: "published";
    contentHash: string;
    frontmatter: unknown;
    bodyMarkdown: string;
  }>;
}

export async function getProjectOpcBundle(request: Request, projectId: string) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/opc-bundle`
  ) as Promise<OpcBundle>;
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

export async function setPrimaryProjectRepo(
  request: Request,
  projectId: string,
  repoId: string
) {
  return apiFetch(
    request,
    `/api/projects/${projectId}/repos/${repoId}/primary`,
    { method: "POST" }
  ) as Promise<{ repo: any }>;
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
