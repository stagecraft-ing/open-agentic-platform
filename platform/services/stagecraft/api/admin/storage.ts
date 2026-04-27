/**
 * Org-admin storage purge.
 *
 * Walks the workspace's object-store bucket, diffs every key against the
 * union of DB-referenced storage keys, and deletes the unreferenced
 * remainder. Recovers bytes left behind when a project delete partially
 * failed (S3 sweep error, pre-cascade migration, manual SQL deletes,
 * legacy direct uploads).
 *
 * Owner/admin-only. Defaults to dry-run so the caller can preview what
 * would be deleted before committing. Every invocation lands in
 * audit_log with the manifest counts so this is also a recoverable
 * trail.
 */

import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import log from "encore.dev/log";
import { db } from "../db/drizzle";
import {
  auditLog,
  factoryArtifacts,
  factoryPipelines,
  knowledgeObjects,
  projects,
  workspaces,
} from "../db/schema";
import { and, eq } from "drizzle-orm";
import { deleteObject, listAllObjects } from "../knowledge/storage";

interface PurgeOrphansRequest {
  /**
   * Optional workspace target. Defaults to the caller's active workspace.
   * If supplied, it MUST belong to the caller's org or the request is
   * rejected.
   */
  workspaceId?: string;
  /**
   * Default true. When true, no objects are deleted; the response carries
   * the orphan list so an operator can review before re-running with
   * dryRun=false.
   */
  dryRun?: boolean;
  /**
   * Cap on how many orphan keys are reported in the response sample.
   * Hard-clamped to [0, 500].
   */
  sampleLimit?: number;
}

interface PurgeOrphansResponse {
  workspaceId: string;
  bucket: string;
  totalKeys: number;
  referencedKeys: number;
  orphanKeys: number;
  purged: number;
  failed: number;
  dryRun: boolean;
  /** Up to `sampleLimit` orphan keys (or every orphan when fewer). */
  orphanSample: string[];
}

export const purgeOrphanStorage = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/admin/storage/purge-orphans",
  },
  async (req: PurgeOrphansRequest): Promise<PurgeOrphansResponse> => {
    const auth = getAuthData()!;
    if (auth.platformRole !== "owner" && auth.platformRole !== "admin") {
      throw APIError.permissionDenied("Owner or admin role required");
    }

    const targetWorkspaceId = req.workspaceId ?? auth.workspaceId;
    if (!targetWorkspaceId) {
      throw APIError.invalidArgument("workspaceId is required");
    }

    const [ws] = await db
      .select({
        id: workspaces.id,
        orgId: workspaces.orgId,
        objectStoreBucket: workspaces.objectStoreBucket,
      })
      .from(workspaces)
      .where(eq(workspaces.id, targetWorkspaceId))
      .limit(1);

    if (!ws) {
      throw APIError.notFound("workspace not found");
    }
    if (ws.orgId !== auth.orgId) {
      throw APIError.permissionDenied(
        "Cannot purge storage for another organization's workspace"
      );
    }
    if (!ws.objectStoreBucket) {
      throw APIError.failedPrecondition(
        "Workspace has no object store bucket configured"
      );
    }

    const dryRun = req.dryRun ?? true;
    const sampleLimit = Math.max(0, Math.min(500, req.sampleLimit ?? 100));

    // ── Gather referenced keys ────────────────────────────────────────
    const referenced = new Set<string>();

    // knowledge_objects (workspace-scoped) — primary + extracted derivative.
    const koRows = await db
      .select({
        storageKey: knowledgeObjects.storageKey,
        extractionOutput: knowledgeObjects.extractionOutput,
      })
      .from(knowledgeObjects)
      .where(eq(knowledgeObjects.workspaceId, targetWorkspaceId));
    for (const k of koRows) {
      referenced.add(k.storageKey);
      const e = readExtractedKey(k.extractionOutput);
      if (e) referenced.add(e);
    }

    // factory_artifacts produced by pipelines whose project lives in this
    // workspace. Workspace_id on factory_artifacts is nullable on legacy
    // rows, so route through the project to be safe.
    const faRows = await db
      .select({ storagePath: factoryArtifacts.storagePath })
      .from(factoryArtifacts)
      .innerJoin(
        factoryPipelines,
        eq(factoryPipelines.id, factoryArtifacts.pipelineId)
      )
      .innerJoin(projects, eq(projects.id, factoryPipelines.projectId))
      .where(eq(projects.workspaceId, targetWorkspaceId));
    for (const a of faRows) {
      referenced.add(a.storagePath);
    }

    // ── Walk the bucket ───────────────────────────────────────────────
    let allKeys: string[];
    try {
      allKeys = await listAllObjects(ws.objectStoreBucket);
    } catch (err) {
      log.error("purgeOrphanStorage: list bucket failed", {
        bucket: ws.objectStoreBucket,
        err: err instanceof Error ? err.message : String(err),
      });
      throw APIError.internal("Failed to list workspace bucket");
    }

    const orphans = allKeys.filter((k) => !referenced.has(k));

    // ── Delete (unless dry-run) ───────────────────────────────────────
    let purged = 0;
    let failed = 0;
    if (!dryRun) {
      for (const key of orphans) {
        try {
          await deleteObject(ws.objectStoreBucket, key);
          purged++;
        } catch (err) {
          failed++;
          log.warn("purgeOrphanStorage: delete failed", {
            bucket: ws.objectStoreBucket,
            key,
            err: err instanceof Error ? err.message : String(err),
          });
        }
      }
    }

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "admin.storage.purge_orphans",
      targetType: "workspace",
      targetId: ws.id,
      metadata: {
        bucket: ws.objectStoreBucket,
        totalKeys: allKeys.length,
        referencedKeys: referenced.size,
        orphanKeys: orphans.length,
        purged,
        failed,
        dryRun,
      },
    });

    log.info("purgeOrphanStorage done", {
      workspaceId: ws.id,
      bucket: ws.objectStoreBucket,
      totalKeys: allKeys.length,
      referenced: referenced.size,
      orphans: orphans.length,
      purged,
      failed,
      dryRun,
    });

    return {
      workspaceId: ws.id,
      bucket: ws.objectStoreBucket,
      totalKeys: allKeys.length,
      referencedKeys: referenced.size,
      orphanKeys: orphans.length,
      purged,
      failed,
      dryRun,
      orphanSample: orphans.slice(0, sampleLimit),
    };
  }
);

function readExtractedKey(extractionOutput: unknown): string | null {
  if (!extractionOutput || typeof extractionOutput !== "object") return null;
  const v = (extractionOutput as Record<string, unknown>).extractedStorageKey;
  return typeof v === "string" ? v : null;
}
