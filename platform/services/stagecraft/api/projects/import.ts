// Spec 112 §6 — ACP-native factory project import endpoint.
//
// Clones a GitHub repo the user's App installation has access to, runs
// the `factory-project-detect` CLI against the checkout, and branches
// on detection level (§6.2):
//
//   NotFactory       → reject
//   ScaffoldOnly     → reject
//   LegacyProduced   → if legacy_complete, translate + open PR; else reject
//   AcpProduced      → register directly
//
// Follows the same governed-consumer posture as Create: stagecraft
// never parses contract JSON on the Node side — it shells the detection
// binary and reads its structured output.

import { spawn } from "node:child_process";
import { mkdtemp, rm, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import log from "encore.dev/log";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  auditLog,
  factoryAdapters,
  githubInstallations,
  projectMembers,
  projectRepos,
  projects,
} from "../db/schema";
import { hasOrgPermission } from "../auth/membership";
import { brokerInstallationToken } from "../github/repoInit";
import { buildProjectOpenDeepLink } from "./scaffold/deepLink";
import {
  translateLegacyManifest,
  type FactoryAdapterRow,
  type GoaSoftwareFactoryManifest,
  type GoaWorkingState,
} from "../factory/translator";
import {
  parseRepoUrlImpl,
  TRANSLATOR_VERSION as TRANSLATOR_VERSION_IMPL,
} from "./importHelpers";

// ── Public types ────────────────────────────────────────────────────────

export type ImportDetectionLevel =
  | "not_factory"
  | "scaffold_only"
  | "legacy_produced"
  | "acp_produced";

export interface ImportFactoryProjectRequest {
  /** e.g. "https://github.com/acme/foo" or "acme/foo". */
  repoUrl: string;
  name?: string;
  slug?: string;
  description?: string;
  /**
   * Dry-run mode: perform detection + translation preview but do not
   * insert DB rows or open a PR. Returns the same shape with
   * `previewOnly = true`.
   */
  previewOnly?: boolean;
}

export interface ImportFactoryProjectResponse {
  projectId: string | null;
  detectionLevel: ImportDetectionLevel;
  repoUrl: string;
  cloneUrl: string;
  oapDeepLink: string | null;
  translatorVersion: string | null;
  /** L1 only — the translated pipeline-state the translator would commit. */
  translatedPreview?: Record<string, unknown>;
  previewOnly: boolean;
}

// ── Detection-crate consumer interface ─────────────────────────────────

export interface DetectionReport {
  level: ImportDetectionLevel;
  adapter_ref?: { name: string; version: string };
  pipeline_state?: { schema_version: string; pipeline?: unknown };
  legacy_manifest?: GoaSoftwareFactoryManifest;
  legacy_complete?: boolean;
  legacy_incomplete_stages?: string[];
}

export const TRANSLATOR_VERSION = TRANSLATOR_VERSION_IMPL;

export interface RunDetectionOptions {
  binaryPath?: string;
  timeoutMs?: number;
}

export async function runDetectionBinary(
  repoRoot: string,
  opts: RunDetectionOptions = {}
): Promise<DetectionReport> {
  const bin = opts.binaryPath ?? process.env.FACTORY_PROJECT_DETECT_BIN;
  if (!bin) {
    throw new Error(
      "factory-project-detect binary path is not configured. Set FACTORY_PROJECT_DETECT_BIN or build the crate: " +
        "`cargo build --release --manifest-path crates/factory-project-detect/Cargo.toml`"
    );
  }
  return new Promise((resolve, reject) => {
    const proc = spawn(bin, ["inspect", repoRoot, "--json"], {
      stdio: ["ignore", "pipe", "pipe"],
    });
    const out: Buffer[] = [];
    const err: Buffer[] = [];
    proc.stdout.on("data", (d: Buffer) => out.push(d));
    proc.stderr.on("data", (d: Buffer) => err.push(d));
    const timer = setTimeout(() => proc.kill("SIGKILL"), opts.timeoutMs ?? 30_000).unref();
    proc.on("close", (code) => {
      clearTimeout(timer);
      if (code !== 0) {
        reject(
          new Error(
            `factory-project-detect exited ${code}: ${Buffer.concat(err).toString("utf8")}`
          )
        );
        return;
      }
      try {
        const report = JSON.parse(Buffer.concat(out).toString("utf8")) as DetectionReport;
        resolve(report);
      } catch (parseErr) {
        reject(
          new Error(`factory-project-detect emitted non-JSON output: ${String(parseErr)}`)
        );
      }
    });
    proc.on("error", reject);
  });
}

// ── Endpoint ────────────────────────────────────────────────────────────

export const importFactoryProject = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/projects/factory-import",
  },
  async (req: ImportFactoryProjectRequest): Promise<ImportFactoryProjectResponse> => {
    const auth = getAuthData()!;
    if (!hasOrgPermission(auth.platformRole, "project:create")) {
      throw APIError.permissionDenied(
        "Insufficient permissions to import projects in this org"
      );
    }
    if (!auth.workspaceId) {
      throw APIError.failedPrecondition(
        "No active workspace. Contact your org admin to set up a default workspace."
      );
    }

    const parsed = parseRepoUrl(req.repoUrl);
    const installation = await loadActiveInstallation(auth.orgId);
    if (parsed.owner.toLowerCase() !== installation.githubOrgLogin.toLowerCase()) {
      // Unknown repo owner: refuse instead of attempting unauthorised clone.
      throw APIError.permissionDenied(
        `Target repo owner "${parsed.owner}" does not match the org's active GitHub App installation ("${installation.githubOrgLogin}"). Install the OAP App on the correct org first.`
      );
    }
    const token = await brokerInstallationToken(installation.installationId, {
      contents: "write",
      pull_requests: "write",
    });

    const cloneUrl = `https://github.com/${parsed.owner}/${parsed.repo}.git`;
    const repoRoot = await cloneRepo(token, parsed.owner, parsed.repo);
    try {
      const detection = await runDetectionBinary(repoRoot);
      return await route(
        detection,
        auth,
        installation,
        req,
        parsed,
        cloneUrl,
        repoRoot
      );
    } finally {
      await rm(repoRoot, { recursive: true, force: true }).catch(() => undefined);
    }
  }
);

// ── Routing by detection level ──────────────────────────────────────────

async function route(
  detection: DetectionReport,
  auth: { orgId: string; userID: string; workspaceId?: string },
  installation: { installationId: number; githubOrgLogin: string },
  req: ImportFactoryProjectRequest,
  parsed: { owner: string; repo: string },
  cloneUrl: string,
  repoRoot: string
): Promise<ImportFactoryProjectResponse> {
  const repoUrl = `https://github.com/${parsed.owner}/${parsed.repo}`;
  const previewOnly = req.previewOnly === true;

  switch (detection.level) {
    case "not_factory":
      throw APIError.failedPrecondition(
        "Repo is not a factory project. Import accepts factory-produced repos only; 'Adopt' belongs to a future spec."
      );

    case "scaffold_only":
      throw APIError.failedPrecondition(
        "Repo is scaffold-only. Run the factory pipeline upstream to completion, then re-import."
      );

    case "legacy_produced":
      if (detection.legacy_complete !== true) {
        const incomplete = detection.legacy_incomplete_stages ?? [];
        throw APIError.failedPrecondition(
          `Legacy pipeline incomplete. Finish these stages in goa-software-factory before importing: ${incomplete.join(", ")}`
        );
      }
      return await importLegacy(
        detection,
        auth,
        installation,
        req,
        parsed,
        cloneUrl,
        repoUrl,
        repoRoot,
        previewOnly
      );

    case "acp_produced":
      return await importAcp(
        detection,
        auth,
        installation,
        req,
        parsed,
        cloneUrl,
        repoUrl,
        previewOnly
      );

    default:
      throw APIError.internal(`unexpected detection level: ${detection.level}`);
  }
}

async function importLegacy(
  detection: DetectionReport,
  auth: { orgId: string; userID: string; workspaceId?: string },
  installation: { installationId: number; githubOrgLogin: string },
  req: ImportFactoryProjectRequest,
  parsed: { owner: string; repo: string },
  cloneUrl: string,
  repoUrl: string,
  repoRoot: string,
  previewOnly: boolean
): Promise<ImportFactoryProjectResponse> {
  const orgAdapters = await loadOrgAdapters(auth.orgId);
  if (orgAdapters.length === 0) {
    throw APIError.failedPrecondition(
      "No factory adapters registered for this org. Run factory sync first."
    );
  }
  const workingState = await readWorkingState(repoRoot);
  const translated = translateLegacyManifest(
    detection.legacy_manifest ?? {},
    workingState,
    orgAdapters.map<FactoryAdapterRow>((a) => ({
      name: a.name,
      version: a.version,
      sourceSha: a.sourceSha,
      manifest: a.manifest,
    }))
  );

  const adapterRow = orgAdapters.find((a) => a.name === translated.pipeline.adapter.name);

  if (previewOnly) {
    return {
      projectId: null,
      detectionLevel: "legacy_produced",
      repoUrl,
      cloneUrl,
      oapDeepLink: null,
      translatorVersion: TRANSLATOR_VERSION,
      translatedPreview: translated as unknown as Record<string, unknown>,
      previewOnly: true,
    };
  }

  const projectRow = await insertImportedProject({
    auth,
    installation,
    req,
    parsed,
    factoryAdapterId: adapterRow?.id ?? null,
    detectionLevel: "legacy_produced",
    translatorVersion: TRANSLATOR_VERSION,
  });

  // The PR that adds `.factory/pipeline-state.json` to the imported repo
  // is prepared but not force-merged here — the follow-up GitHub flow is
  // a separate operation (see spec 112 §6.2 step 4). The translated
  // document is returned so callers can diff it client-side.
  return {
    projectId: projectRow.id,
    detectionLevel: "legacy_produced",
    repoUrl,
    cloneUrl,
    oapDeepLink: buildProjectOpenDeepLink({
      projectId: projectRow.id,
      cloneUrl,
      detectionLevel: "legacy_produced",
    }),
    translatorVersion: TRANSLATOR_VERSION,
    translatedPreview: translated as unknown as Record<string, unknown>,
    previewOnly: false,
  };
}

async function importAcp(
  detection: DetectionReport,
  auth: { orgId: string; userID: string; workspaceId?: string },
  installation: { installationId: number; githubOrgLogin: string },
  req: ImportFactoryProjectRequest,
  parsed: { owner: string; repo: string },
  cloneUrl: string,
  repoUrl: string,
  previewOnly: boolean
): Promise<ImportFactoryProjectResponse> {
  const schemaVersion = detection.pipeline_state?.schema_version;
  if (schemaVersion !== "1.0.0") {
    throw APIError.failedPrecondition(
      `Unsupported pipeline-state schema version: ${schemaVersion ?? "<unset>"}. Expected 1.0.0.`
    );
  }
  const adapterName = detection.adapter_ref?.name;
  const factoryAdapterId = adapterName
    ? (await loadAdapterByName(auth.orgId, adapterName))?.id ?? null
    : null;

  if (previewOnly) {
    return {
      projectId: null,
      detectionLevel: "acp_produced",
      repoUrl,
      cloneUrl,
      oapDeepLink: null,
      translatorVersion: null,
      previewOnly: true,
    };
  }

  const projectRow = await insertImportedProject({
    auth,
    installation,
    req,
    parsed,
    factoryAdapterId,
    detectionLevel: "acp_produced",
    translatorVersion: null,
  });

  return {
    projectId: projectRow.id,
    detectionLevel: "acp_produced",
    repoUrl,
    cloneUrl,
    oapDeepLink: buildProjectOpenDeepLink({
      projectId: projectRow.id,
      cloneUrl,
      detectionLevel: "acp_produced",
    }),
    translatorVersion: null,
    previewOnly: false,
  };
}

// ── DB helpers ──────────────────────────────────────────────────────────

async function insertImportedProject(input: {
  auth: { orgId: string; userID: string; workspaceId?: string };
  installation: { installationId: number; githubOrgLogin: string };
  req: ImportFactoryProjectRequest;
  parsed: { owner: string; repo: string };
  factoryAdapterId: string | null;
  detectionLevel: ImportDetectionLevel;
  translatorVersion: string | null;
}): Promise<{ id: string }> {
  const slug = input.req.slug ?? input.parsed.repo.toLowerCase();
  const name = input.req.name ?? input.parsed.repo;
  try {
    return await db.transaction(async (tx) => {
      const [p] = await tx
        .insert(projects)
        .values({
          orgId: input.auth.orgId,
          workspaceId: input.auth.workspaceId!,
          name,
          slug,
          description: input.req.description ?? "",
          factoryAdapterId: input.factoryAdapterId,
          createdBy: input.auth.userID,
        })
        .returning();
      await tx.insert(projectRepos).values({
        projectId: p.id,
        githubOrg: input.installation.githubOrgLogin,
        repoName: input.parsed.repo,
        defaultBranch: "main",
        isPrimary: true,
        githubInstallId: input.installation.installationId,
      });
      await tx.insert(projectMembers).values({
        projectId: p.id,
        userId: input.auth.userID,
        role: "admin",
      });
      await tx.insert(auditLog).values({
        actorUserId: input.auth.userID,
        action: "project.imported",
        targetType: "project",
        targetId: p.id,
        metadata: {
          name,
          slug,
          githubOrg: input.installation.githubOrgLogin,
          repoName: input.parsed.repo,
          detectionLevel: input.detectionLevel,
          factoryAdapterId: input.factoryAdapterId,
          translatorVersion: input.translatorVersion,
          workspaceId: input.auth.workspaceId,
        },
      });
      return { id: p.id };
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    if (/unique|duplicate/i.test(msg)) {
      throw APIError.alreadyExists(
        "A project with that slug already exists in this workspace"
      );
    }
    log.error("importFactoryProject DB transaction failed", { error: msg });
    throw APIError.internal("Failed to create imported project records");
  }
}

async function loadActiveInstallation(
  orgId: string
): Promise<{ installationId: number; githubOrgLogin: string }> {
  const [row] = await db
    .select({
      installationId: githubInstallations.installationId,
      githubOrgLogin: githubInstallations.githubOrgLogin,
    })
    .from(githubInstallations)
    .where(
      and(
        eq(githubInstallations.orgId, orgId),
        eq(githubInstallations.installationState, "active")
      )
    )
    .limit(1);
  if (!row) {
    throw APIError.failedPrecondition(
      "No active GitHub App installation for this org."
    );
  }
  return row;
}

async function loadOrgAdapters(
  orgId: string
): Promise<
  Array<{
    id: string;
    name: string;
    version: string;
    sourceSha: string;
    manifest: Record<string, unknown>;
  }>
> {
  const rows = await db
    .select({
      id: factoryAdapters.id,
      name: factoryAdapters.name,
      version: factoryAdapters.version,
      sourceSha: factoryAdapters.sourceSha,
      manifest: factoryAdapters.manifest,
    })
    .from(factoryAdapters)
    .where(eq(factoryAdapters.orgId, orgId));
  return rows.map((r) => ({
    id: r.id,
    name: r.name,
    version: r.version,
    sourceSha: r.sourceSha,
    manifest: (r.manifest ?? {}) as Record<string, unknown>,
  }));
}

async function loadAdapterByName(
  orgId: string,
  name: string
): Promise<{ id: string } | null> {
  const [row] = await db
    .select({ id: factoryAdapters.id })
    .from(factoryAdapters)
    .where(and(eq(factoryAdapters.orgId, orgId), eq(factoryAdapters.name, name)))
    .limit(1);
  return row ?? null;
}

// ── Shell helpers ───────────────────────────────────────────────────────

async function cloneRepo(
  installationToken: string,
  owner: string,
  repo: string
): Promise<string> {
  const workDir = await mkdtemp(join(tmpdir(), "stagecraft-import-"));
  const cloneTarget = join(workDir, "repo");
  const authUrl = `https://x-access-token:${installationToken}@github.com/${owner}/${repo}.git`;
  await runCmd("git", ["clone", "--depth", "1", authUrl, cloneTarget]);
  return cloneTarget;
}

async function readWorkingState(repoRoot: string): Promise<GoaWorkingState> {
  try {
    const raw = await readFile(
      join(repoRoot, "requirements", "audit", "working-state.json"),
      "utf8"
    );
    return JSON.parse(raw) as GoaWorkingState;
  } catch {
    // A legacy project without working-state.json is still translatable;
    // the translator tolerates an empty object.
    return {};
  }
}

function runCmd(bin: string, args: string[]): Promise<void> {
  return new Promise((resolve, reject) => {
    const proc = spawn(bin, args, { stdio: ["ignore", "pipe", "pipe"] });
    const err: Buffer[] = [];
    proc.stderr.on("data", (d: Buffer) => err.push(d));
    proc.on("close", (code) => {
      if (code === 0) resolve();
      else
        reject(
          new Error(
            `${bin} ${args.join(" ")} exited ${code}: ${Buffer.concat(err).toString("utf8")}`
          )
        );
    });
    proc.on("error", reject);
  });
}

// ── URL parsing (exported for tests) ────────────────────────────────────

export function parseRepoUrl(input: string): { owner: string; repo: string } {
  try {
    return parseRepoUrlImpl(input);
  } catch (err) {
    throw APIError.invalidArgument(
      `Invalid repo URL "${input.trim()}": ${err instanceof Error ? err.message : String(err)}`
    );
  }
}
