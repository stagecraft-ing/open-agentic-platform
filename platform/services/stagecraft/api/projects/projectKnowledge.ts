// Spec 087 Phase 2 + spec 112 §6 — project-scoped knowledge views and the
// per-artifact "Advance to extracted" transition.
//
// These endpoints sit in the projects service (not `knowledge/`) because
// they are keyed on a project id and enforce project-in-workspace scoping
// before touching knowledge rows.

import { spawn } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import log from "encore.dev/log";
import { and, desc, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  auditLog,
  documentBindings,
  knowledgeObjects,
  projects,
  workspaces,
} from "../db/schema";
import { getObject, putObject } from "../knowledge/storage";

// ── Public row shape ────────────────────────────────────────────────────

export interface ProjectKnowledgeObject {
  id: string;
  filename: string;
  mimeType: string;
  sizeBytes: number;
  contentHash: string;
  state: string;
  storageKey: string;
  extractedStorageKey: string | null;
  provenance: Record<string, unknown>;
  boundAt: string;
  updatedAt: string;
}

// ── List ────────────────────────────────────────────────────────────────

export const listProjectKnowledge = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/projects/:projectId/knowledge",
  },
  async (req: {
    projectId: string;
  }): Promise<{ objects: ProjectKnowledgeObject[] }> => {
    const auth = getAuthData()!;
    await assertProjectInWorkspace(req.projectId, auth.workspaceId);

    const rows = await db
      .select({
        id: knowledgeObjects.id,
        filename: knowledgeObjects.filename,
        mimeType: knowledgeObjects.mimeType,
        sizeBytes: knowledgeObjects.sizeBytes,
        contentHash: knowledgeObjects.contentHash,
        state: knowledgeObjects.state,
        storageKey: knowledgeObjects.storageKey,
        extractionOutput: knowledgeObjects.extractionOutput,
        provenance: knowledgeObjects.provenance,
        updatedAt: knowledgeObjects.updatedAt,
        boundAt: documentBindings.boundAt,
      })
      .from(documentBindings)
      .innerJoin(
        knowledgeObjects,
        eq(knowledgeObjects.id, documentBindings.knowledgeObjectId)
      )
      .where(eq(documentBindings.projectId, req.projectId))
      .orderBy(desc(documentBindings.boundAt));

    return {
      objects: rows.map((r) => ({
        id: r.id,
        filename: r.filename,
        mimeType: r.mimeType,
        sizeBytes: r.sizeBytes,
        contentHash: r.contentHash,
        state: r.state,
        storageKey: r.storageKey,
        extractedStorageKey: readExtractedKey(r.extractionOutput),
        provenance: (r.provenance ?? {}) as Record<string, unknown>,
        boundAt: r.boundAt.toISOString(),
        updatedAt: r.updatedAt.toISOString(),
      })),
    };
  }
);

function readExtractedKey(extractionOutput: unknown): string | null {
  if (!extractionOutput || typeof extractionOutput !== "object") return null;
  const v = (extractionOutput as Record<string, unknown>).extractedStorageKey;
  return typeof v === "string" ? v : null;
}

// ── Advance to extracted ────────────────────────────────────────────────

export interface AdvanceKnowledgeToExtractedResponse {
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
}

export const advanceKnowledgeToExtracted = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/projects/:projectId/knowledge/:objectId/advance-extracted",
  },
  async (req: {
    projectId: string;
    objectId: string;
  }): Promise<AdvanceKnowledgeToExtractedResponse> => {
    const auth = getAuthData()!;
    await assertProjectInWorkspace(req.projectId, auth.workspaceId);

    const [obj] = await db
      .select({
        id: knowledgeObjects.id,
        workspaceId: knowledgeObjects.workspaceId,
        storageKey: knowledgeObjects.storageKey,
        filename: knowledgeObjects.filename,
        mimeType: knowledgeObjects.mimeType,
        state: knowledgeObjects.state,
      })
      .from(knowledgeObjects)
      .innerJoin(
        documentBindings,
        eq(documentBindings.knowledgeObjectId, knowledgeObjects.id)
      )
      .where(
        and(
          eq(knowledgeObjects.id, req.objectId),
          eq(documentBindings.projectId, req.projectId),
          eq(knowledgeObjects.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!obj) {
      throw APIError.notFound(
        "knowledge object not bound to this project"
      );
    }
    if (obj.state !== "imported") {
      throw APIError.failedPrecondition(
        `cannot advance: object is in state "${obj.state}" (expected imported)`
      );
    }

    const bucket = await loadWorkspaceBucket(obj.workspaceId);

    // Move imported → extracting before shelling out so concurrent callers
    // see the in-flight transition.
    const moved = await db
      .update(knowledgeObjects)
      .set({ state: "extracting", updatedAt: new Date() })
      .where(
        and(
          eq(knowledgeObjects.id, obj.id),
          eq(knowledgeObjects.state, "imported")
        )
      )
      .returning({ id: knowledgeObjects.id });
    if (moved.length === 0) {
      throw APIError.failedPrecondition(
        "knowledge object is no longer in imported state"
      );
    }

    let workDir: string | null = null;
    try {
      workDir = await mkdtemp(join(tmpdir(), "stagecraft-extract-"));
      const rawDir = join(workDir, "raw");
      const outDir = join(workDir, "extracted");
      await mkdir(rawDir, { recursive: true });
      await mkdir(outDir, { recursive: true });
      const body = await getObject(bucket, obj.storageKey);
      const stagedPath = join(rawDir, obj.filename);
      await writeFile(stagedPath, body);

      const { summary, message } = await runArtifactExtract(
        workDir,
        obj.filename
      );

      const extractedPath = join(outDir, `${obj.filename}.txt`);
      const extractedBody = await readFile(extractedPath);
      const extractedKey =
        `knowledge/${obj.id}/extracted/${obj.filename}.txt`;
      await putObject(
        bucket,
        extractedKey,
        extractedBody,
        "text/plain"
      );

      const extractionOutput = {
        extractedStorageKey: extractedKey,
        summary,
        extractedBytes: extractedBody.length,
        extractedAt: new Date().toISOString(),
        extractor: {
          binary: artifactExtractBinary(),
          message,
        },
      };
      await db
        .update(knowledgeObjects)
        .set({
          state: "extracted",
          extractionOutput,
          updatedAt: new Date(),
        })
        .where(eq(knowledgeObjects.id, obj.id));

      await db.insert(auditLog).values({
        actorUserId: auth.userID,
        action: "knowledge.advanced_to_extracted",
        targetType: "knowledge_object",
        targetId: obj.id,
        metadata: {
          projectId: req.projectId,
          extractedStorageKey: extractedKey,
          summary,
        },
      });

      return {
        objectId: obj.id,
        state: "extracted",
        extractedStorageKey: extractedKey,
        summary,
        extractorMessage: message,
      };
    } catch (err) {
      // Roll the state back so the UI's "Advance" button reappears; the
      // failure is surfaced via the API error, so we don't leave the row
      // stuck in `extracting`.
      await db
        .update(knowledgeObjects)
        .set({ state: "imported", updatedAt: new Date() })
        .where(eq(knowledgeObjects.id, obj.id));
      const msg = err instanceof Error ? err.message : String(err);
      log.error("advanceKnowledgeToExtracted failed", {
        objectId: obj.id,
        projectId: req.projectId,
        error: msg,
      });
      if (err instanceof APIError) throw err;
      throw APIError.internal(`extraction failed: ${msg}`);
    } finally {
      if (workDir) {
        await rm(workDir, { recursive: true, force: true }).catch(
          () => undefined
        );
      }
    }
  }
);

// ── Helpers ─────────────────────────────────────────────────────────────

async function assertProjectInWorkspace(
  projectId: string,
  workspaceId: string
): Promise<void> {
  const [p] = await db
    .select({ id: projects.id })
    .from(projects)
    .where(
      and(eq(projects.id, projectId), eq(projects.workspaceId, workspaceId))
    )
    .limit(1);
  if (!p) {
    throw APIError.notFound("project not found in workspace");
  }
}

async function loadWorkspaceBucket(workspaceId: string): Promise<string> {
  const [ws] = await db
    .select({ bucket: workspaces.objectStoreBucket })
    .from(workspaces)
    .where(eq(workspaces.id, workspaceId))
    .limit(1);
  if (!ws) {
    throw APIError.notFound("workspace not found");
  }
  return ws.bucket;
}

function artifactExtractBinary(): string {
  const bin = process.env.ARTIFACT_EXTRACT_BIN;
  if (!bin) {
    throw APIError.failedPrecondition(
      "artifact-extract binary path is not configured. Set ARTIFACT_EXTRACT_BIN or build the crate: " +
        "`cargo build --release --manifest-path crates/artifact-extract/Cargo.toml`"
    );
  }
  return bin;
}

interface ExtractRunResult {
  summary: {
    ok: number;
    cached: number;
    error: number;
    skip_unsupported: number;
  };
  message: string;
}

async function runArtifactExtract(
  workRoot: string,
  filename: string
): Promise<ExtractRunResult> {
  const bin = artifactExtractBinary();
  return new Promise<ExtractRunResult>((resolve, reject) => {
    const proc = spawn(
      bin,
      ["--root", workRoot, "--json", "--force"],
      { stdio: ["ignore", "pipe", "pipe"] }
    );
    const out: Buffer[] = [];
    const err: Buffer[] = [];
    proc.stdout.on("data", (d: Buffer) => out.push(d));
    proc.stderr.on("data", (d: Buffer) => err.push(d));
    const timer = setTimeout(
      () => proc.kill("SIGKILL"),
      5 * 60 * 1000
    ).unref();
    proc.on("close", (code) => {
      clearTimeout(timer);
      const stdout = Buffer.concat(out).toString("utf8");
      const stderr = Buffer.concat(err).toString("utf8");
      if (code !== 0) {
        reject(
          new Error(
            `artifact-extract exited ${code}: ${stderr || stdout}`
          )
        );
        return;
      }
      try {
        const lines = stdout
          .split("\n")
          .map((l) => l.trim())
          .filter((l) => l.length > 0);
        let summary: ExtractRunResult["summary"] | null = null;
        let fileMessage = "";
        let fileStatus = "";
        for (const line of lines) {
          const parsed = JSON.parse(line) as Record<string, unknown>;
          if (parsed.kind === "summary") {
            summary = {
              ok: numberOr(parsed.ok, 0),
              cached: numberOr(parsed.cached, 0),
              error: numberOr(parsed.error, 0),
              skip_unsupported: numberOr(parsed.skip_unsupported, 0),
            };
          } else if (parsed.kind === "file" && parsed.path === filename) {
            fileStatus = String(parsed.status ?? "");
            fileMessage = String(parsed.message ?? "");
          }
        }
        if (!summary) {
          reject(
            new Error(
              `artifact-extract produced no summary line; stdout=${stdout.slice(0, 500)}`
            )
          );
          return;
        }
        if (summary.error > 0 || fileStatus === "error") {
          reject(
            new Error(
              `artifact-extract reported error for ${filename}: ${fileMessage || stderr}`
            )
          );
          return;
        }
        if (fileStatus === "skip-unsupported") {
          reject(
            new Error(
              `artifact-extract does not support ${filename}: ${fileMessage}`
            )
          );
          return;
        }
        resolve({ summary, message: fileMessage || "ok" });
      } catch (parseErr) {
        reject(
          new Error(
            `artifact-extract emitted non-JSON output: ${String(parseErr)}; stdout=${stdout.slice(0, 500)}`
          )
        );
      }
    });
    proc.on("error", reject);
  });
}

function numberOr(v: unknown, fallback: number): number {
  return typeof v === "number" && Number.isFinite(v) ? v : fallback;
}
