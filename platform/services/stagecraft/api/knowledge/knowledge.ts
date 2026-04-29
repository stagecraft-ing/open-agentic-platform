/**
 * Knowledge intake service (spec 087 Phase 2).
 *
 * Manages knowledge objects, source connectors, and document bindings.
 * Upload flow: request presigned URL → client uploads to S3 → confirm upload.
 */

import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import log from "encore.dev/log";
import { db } from "../db/drizzle";
import {
  knowledgeObjects,
  sourceConnectors,
  documentBindings,
  workspaces,
  projects,
  auditLog,
  syncRuns,
  knowledgeExtractionRuns,
} from "../db/schema";
import { and, eq, desc, inArray, sql } from "drizzle-orm";
import {
  getPresignedUploadUrl,
  getPresignedDownloadUrl,
  headObject,
  deleteObject,
} from "./storage";
import { randomUUID } from "crypto";
import { getConnectorImpl } from "./connectors";
import type { SyncContext, SyncedObject } from "./connectors";
import { broadcastToWorkspace } from "../sync/sync";
import { enqueueExtraction } from "./extractionCore";
import { KNOWLEDGE_EXTRACTION_RETRY_REQUESTED } from "./auditActions";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type KnowledgeObjectRow = {
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
  provenance: unknown;
  /** Spec 115 FR-025 — populated when the most recent extraction failed. */
  lastExtractionError: unknown;
  createdAt: Date;
  updatedAt: Date;
};

/**
 * Spec 115 FR-030 — denormalised "latest run" surface so the dashboard
 * does not need a second round-trip to render the status badge / extractor
 * footer / Retry banner.
 */
type LatestExtractionRun = {
  status: string;
  extractorKind: string | null;
  completedAt: string | null;
  durationMs: number | null;
};

type KnowledgeObjectListRow = KnowledgeObjectRow & {
  latestRun: LatestExtractionRun | null;
};

type SourceConnectorRow = {
  id: string;
  workspaceId: string;
  type: string;
  name: string;
  syncSchedule: string | null;
  status: string;
  lastSyncedAt: Date | null;
  createdAt: Date;
  updatedAt: Date;
};

type DocumentBindingRow = {
  id: string;
  projectId: string;
  knowledgeObjectId: string;
  boundBy: string;
  boundAt: Date;
};

type SyncRunRow = {
  id: string;
  connectorId: string;
  workspaceId: string;
  status: string;
  objectsCreated: number;
  objectsUpdated: number;
  objectsSkipped: number;
  error: string | null;
  deltaToken: string | null;
  startedAt: Date;
  completedAt: Date | null;
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function getWorkspaceBucket(workspaceId: string): Promise<string> {
  const [ws] = await db
    .select({ objectStoreBucket: workspaces.objectStoreBucket })
    .from(workspaces)
    .where(eq(workspaces.id, workspaceId))
    .limit(1);

  if (!ws) {
    throw APIError.notFound("workspace not found");
  }
  return ws.objectStoreBucket;
}

async function verifyProjectInScope(
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

// =========================================================================
// KNOWLEDGE OBJECTS
// =========================================================================

// ---------------------------------------------------------------------------
// List knowledge objects in workspace
// ---------------------------------------------------------------------------

export const listKnowledgeObjects = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/knowledge/objects",
  },
  async (req: {
    state?: string;
  }): Promise<{ objects: KnowledgeObjectListRow[] }> => {
    const auth = getAuthData()!;

    if (!auth.workspaceId) {
      throw APIError.invalidArgument("workspace context required");
    }

    const conditions = [eq(knowledgeObjects.workspaceId, auth.workspaceId)];
    if (req.state) {
      conditions.push(eq(knowledgeObjects.state, req.state as any));
    }

    const rows = await db
      .select()
      .from(knowledgeObjects)
      .where(and(...conditions))
      .orderBy(desc(knowledgeObjects.createdAt));

    if (rows.length === 0) {
      return { objects: [] };
    }

    // Spec 115 FR-030 — fetch the most-recent extraction run per object in
    // one query. DISTINCT ON keeps it indexed against the
    // (knowledgeObjectId, queuedAt DESC) covering index from migration 25.
    const objectIds = rows.map((r) => r.id);
    const latestRuns = await db.execute<{
      knowledge_object_id: string;
      status: string;
      extractor_kind: string | null;
      completed_at: Date | null;
      duration_ms: number | null;
    }>(sql`
      SELECT DISTINCT ON (knowledge_object_id)
        knowledge_object_id,
        status,
        extractor_kind,
        completed_at,
        duration_ms
      FROM knowledge_extraction_runs
      WHERE knowledge_object_id = ANY(${objectIds})
      ORDER BY knowledge_object_id, queued_at DESC
    `);

    const runsByObjectId = new Map<string, LatestExtractionRun>();
    for (const r of latestRuns.rows) {
      runsByObjectId.set(r.knowledge_object_id, {
        status: r.status,
        extractorKind: r.extractor_kind,
        completedAt: r.completed_at ? r.completed_at.toISOString() : null,
        durationMs: r.duration_ms,
      });
    }

    const objects: KnowledgeObjectListRow[] = rows.map((row) => ({
      ...row,
      latestRun: runsByObjectId.get(row.id) ?? null,
    }));

    return { objects };
  }
);

// ---------------------------------------------------------------------------
// Get a single knowledge object
// ---------------------------------------------------------------------------

export const getKnowledgeObject = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/knowledge/objects/:id",
  },
  async (
    req: { id: string }
  ): Promise<{
    object: KnowledgeObjectListRow;
    bindingsCount: number;
  }> => {
    const auth = getAuthData()!;

    const [row] = await db
      .select()
      .from(knowledgeObjects)
      .where(
        and(
          eq(knowledgeObjects.id, req.id),
          eq(knowledgeObjects.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!row) {
      throw APIError.notFound("knowledge object not found");
    }

    const bindings = await db
      .select({ id: documentBindings.id })
      .from(documentBindings)
      .where(eq(documentBindings.knowledgeObjectId, req.id));

    // Spec 115 FR-030 — denormalise the most recent extraction run.
    const [latest] = await db
      .select({
        status: knowledgeExtractionRuns.status,
        extractorKind: knowledgeExtractionRuns.extractorKind,
        completedAt: knowledgeExtractionRuns.completedAt,
        durationMs: knowledgeExtractionRuns.durationMs,
      })
      .from(knowledgeExtractionRuns)
      .where(eq(knowledgeExtractionRuns.knowledgeObjectId, req.id))
      .orderBy(desc(knowledgeExtractionRuns.queuedAt))
      .limit(1);

    const latestRun: LatestExtractionRun | null = latest
      ? {
          status: latest.status,
          extractorKind: latest.extractorKind,
          completedAt: latest.completedAt
            ? latest.completedAt.toISOString()
            : null,
          durationMs: latest.durationMs,
        }
      : null;

    return {
      object: { ...row, latestRun },
      bindingsCount: bindings.length,
    };
  }
);

// ---------------------------------------------------------------------------
// Request presigned upload URL
// ---------------------------------------------------------------------------

type RequestUploadRequest = {
  filename: string;
  mimeType: string;
  contentHash: string; // client-provided SHA-256 for dedup
  sizeBytes: number; // client-known file size; verified against S3 HEAD on confirm
  /**
   * Optional folder-relative path for batch/folder uploads. Stored in
   * provenance.sourceUri as `upload://<sourcePath>`; falls back to filename.
   */
  sourcePath?: string;
};

type RequestUploadResponse = {
  objectId: string;
  uploadUrl: string;
  storageKey: string;
};

export const requestUpload = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/knowledge/upload",
  },
  async (req: RequestUploadRequest): Promise<RequestUploadResponse> => {
    const auth = getAuthData()!;

    if (!auth.workspaceId) {
      throw APIError.invalidArgument("workspace context required");
    }
    if (!req.filename || !req.mimeType || !req.contentHash) {
      throw APIError.invalidArgument(
        "filename, mimeType, and contentHash are required"
      );
    }
    if (typeof req.sizeBytes !== "number" || req.sizeBytes < 0) {
      throw APIError.invalidArgument("sizeBytes must be a non-negative number");
    }

    const bucket = await getWorkspaceBucket(auth.workspaceId);
    const objectId = randomUUID();
    const storageKey = `knowledge/${objectId}/${req.filename}`;

    // Create the knowledge object record in "imported" state (pending upload).
    // sizeBytes is trusted from the client at request time and verified
    // against S3 metadata on confirmUpload.
    await db.insert(knowledgeObjects).values({
      id: objectId,
      workspaceId: auth.workspaceId,
      connectorId: null, // direct upload — no connector
      storageKey,
      filename: req.filename,
      mimeType: req.mimeType,
      sizeBytes: req.sizeBytes,
      contentHash: req.contentHash,
      state: "imported",
      provenance: {
        sourceType: "upload",
        sourceUri: `upload://${req.sourcePath ?? req.filename}`,
        importedAt: new Date().toISOString(),
      },
    });

    const uploadUrl = await getPresignedUploadUrl(
      bucket,
      storageKey,
      req.mimeType
    );

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.upload_requested",
      targetType: "knowledge_object",
      targetId: objectId,
      metadata: { filename: req.filename, mimeType: req.mimeType },
    });

    log.info("upload requested", { objectId, filename: req.filename });

    return { objectId, uploadUrl, storageKey };
  }
);

// ---------------------------------------------------------------------------
// Confirm upload (verifies object landed in S3, updates size)
// ---------------------------------------------------------------------------

export const confirmUpload = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/knowledge/objects/:id/confirm",
  },
  async (req: { id: string }): Promise<{ object: KnowledgeObjectRow }> => {
    const auth = getAuthData()!;

    const [obj] = await db
      .select()
      .from(knowledgeObjects)
      .where(
        and(
          eq(knowledgeObjects.id, req.id),
          eq(knowledgeObjects.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!obj) {
      throw APIError.notFound("knowledge object not found");
    }

    if (obj.state !== "imported") {
      throw APIError.invalidArgument(
        `cannot confirm upload: object is in state "${obj.state}"`
      );
    }

    // Verify the object actually exists in S3
    const bucket = await getWorkspaceBucket(auth.workspaceId);
    const meta = await headObject(bucket, obj.storageKey);

    if (!meta) {
      throw APIError.invalidArgument(
        "upload not found in object store — upload the file before confirming"
      );
    }

    // Trust the client-reported sizeBytes from requestUpload, but overwrite
    // with the S3 HEAD value when it disagrees and is non-zero. Some
    // S3-compatible stores omit Content-Length on HEAD; in that case we keep
    // the request-time size rather than clobbering it with 0.
    const updateSet: Record<string, unknown> = { updatedAt: new Date() };
    if (meta.contentLength > 0 && meta.contentLength !== obj.sizeBytes) {
      updateSet.sizeBytes = meta.contentLength;
    }

    const [updated] = await db
      .update(knowledgeObjects)
      .set(updateSet)
      .where(eq(knowledgeObjects.id, req.id))
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.upload_confirmed",
      targetType: "knowledge_object",
      targetId: req.id,
      metadata: {
        sizeBytes: updated.sizeBytes,
        contentType: meta.contentType,
      },
    });

    // Spec 115 FR-008 — automatic extraction. Enqueue happens after the
    // audit insert; failure to enqueue MUST NOT roll back the upload (the
    // bytes are already in S3). The Retry endpoint re-enqueues if needed.
    try {
      await enqueueExtraction({
        knowledgeObjectId: req.id,
        workspaceId: auth.workspaceId,
        reason: "upload_confirmed",
      });
    } catch (err) {
      log.error("confirmUpload: enqueueExtraction failed; upload kept", {
        objectId: req.id,
        err: err instanceof Error ? err.message : String(err),
      });
    }

    log.info("upload confirmed", {
      objectId: req.id,
      sizeBytes: updated.sizeBytes,
    });

    return { object: updated };
  }
);

// ---------------------------------------------------------------------------
// Get presigned download URL for a knowledge object
// ---------------------------------------------------------------------------

export const getDownloadUrl = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/knowledge/objects/:id/download",
  },
  async (req: { id: string }): Promise<{ downloadUrl: string }> => {
    const auth = getAuthData()!;

    const [obj] = await db
      .select({
        storageKey: knowledgeObjects.storageKey,
        workspaceId: knowledgeObjects.workspaceId,
      })
      .from(knowledgeObjects)
      .where(
        and(
          eq(knowledgeObjects.id, req.id),
          eq(knowledgeObjects.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!obj) {
      throw APIError.notFound("knowledge object not found");
    }

    const bucket = await getWorkspaceBucket(auth.workspaceId);
    try {
      const downloadUrl = await getPresignedDownloadUrl(bucket, obj.storageKey);
      return { downloadUrl };
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      log.error("getDownloadUrl: failed to generate presigned url", {
        objectId: req.id,
        bucket,
        storageKey: obj.storageKey,
        error: msg,
      });
      throw APIError.internal(`failed to generate download url: ${msg}`);
    }
  }
);

// ---------------------------------------------------------------------------
// Transition knowledge object state
// ---------------------------------------------------------------------------

const VALID_TRANSITIONS: Record<string, string[]> = {
  imported: ["extracting"],
  extracting: ["extracted"],
  extracted: ["classified"],
  classified: ["available"],
};

type TransitionStateRequest = {
  id: string;
  targetState: string;
  extractionOutput?: Record<string, unknown>;
  classification?: string[];
};

export const transitionState = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/knowledge/objects/:id/transition",
  },
  async (
    req: TransitionStateRequest
  ): Promise<{ object: KnowledgeObjectRow }> => {
    const auth = getAuthData()!;

    // Spec 115 FR-027 — the legacy click-walk endpoint is gated off in
    // default builds. Operators flip the env to "true" during incident
    // response only; every successful legacy call is tagged with
    // `legacy_path = true` in the audit row so usage shows up in reports.
    if (process.env.STAGECRAFT_EXTRACT_LEGACY_TRANSITION !== "true") {
      throw APIError.failedPrecondition(
        "legacy_transition_disabled: use POST /api/knowledge/objects/:id/retry-extraction or rely on the automatic pipeline",
      );
    }

    const [obj] = await db
      .select()
      .from(knowledgeObjects)
      .where(
        and(
          eq(knowledgeObjects.id, req.id),
          eq(knowledgeObjects.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!obj) {
      throw APIError.notFound("knowledge object not found");
    }

    const allowed = VALID_TRANSITIONS[obj.state];
    if (!allowed || !allowed.includes(req.targetState)) {
      throw APIError.invalidArgument(
        `invalid transition: ${obj.state} → ${req.targetState}`
      );
    }

    const updates: Record<string, unknown> = {
      state: req.targetState,
      updatedAt: new Date(),
    };

    if (req.extractionOutput !== undefined) {
      updates.extractionOutput = req.extractionOutput;
    }
    if (req.classification !== undefined) {
      updates.classification = req.classification;
    }

    const [updated] = await db
      .update(knowledgeObjects)
      .set(updates)
      .where(eq(knowledgeObjects.id, req.id))
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.state_transition",
      targetType: "knowledge_object",
      targetId: req.id,
      metadata: {
        from: obj.state,
        to: req.targetState,
        legacy_path: true,
      },
    });

    return { object: updated };
  }
);

// ---------------------------------------------------------------------------
// Spec 115 FR-010 — Retry extraction
// ---------------------------------------------------------------------------
//
// Operator-initiated re-enqueue of the extraction pipeline for an object
// whose previous attempt failed. Refuses with `not_failed` when
// `lastExtractionError` is null. The dispatcher re-resolves at run time so
// a Retry against a deterministic failure routes to the agent path when
// policy allows — Retry never re-runs the failing extractor verbatim.

export const retryExtraction = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/knowledge/objects/:id/retry-extraction",
  },
  async (
    req: { id: string },
  ): Promise<{ runId: string; outcome: "enqueued" | "deduped" }> => {
    const auth = getAuthData()!;
    if (!auth.workspaceId) {
      throw APIError.invalidArgument("workspace context required");
    }

    const [obj] = await db
      .select({
        id: knowledgeObjects.id,
        workspaceId: knowledgeObjects.workspaceId,
        lastExtractionError: knowledgeObjects.lastExtractionError,
      })
      .from(knowledgeObjects)
      .where(
        and(
          eq(knowledgeObjects.id, req.id),
          eq(knowledgeObjects.workspaceId, auth.workspaceId),
        ),
      )
      .limit(1);
    if (!obj) {
      throw APIError.notFound("knowledge object not found");
    }
    if (obj.lastExtractionError == null) {
      throw APIError.failedPrecondition(
        "not_failed: object has no lastExtractionError; nothing to retry",
      );
    }

    const result = await enqueueExtraction({
      knowledgeObjectId: obj.id,
      workspaceId: obj.workspaceId,
      reason: "retry",
    });

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: KNOWLEDGE_EXTRACTION_RETRY_REQUESTED,
      targetType: "knowledge_object",
      targetId: req.id,
      metadata: {
        runId: result.runId,
        outcome: result.outcome,
        previousError: obj.lastExtractionError,
      },
    });

    return result;
  },
);

// ---------------------------------------------------------------------------
// Delete knowledge object
// ---------------------------------------------------------------------------

export const deleteKnowledgeObject = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/api/knowledge/objects/:id",
  },
  async (
    req: { id: string }
  ): Promise<{ deleted: boolean; bindingsRemoved: number }> => {
    const auth = getAuthData()!;

    const [obj] = await db
      .select()
      .from(knowledgeObjects)
      .where(
        and(
          eq(knowledgeObjects.id, req.id),
          eq(knowledgeObjects.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!obj) {
      throw APIError.notFound("knowledge object not found");
    }

    // Delete from object store. Best-effort: if the blob is already missing
    // (e.g. a partially-uploaded object), continue with DB cleanup so the
    // user is not stuck with a ghost row they cannot remove.
    const bucket = await getWorkspaceBucket(auth.workspaceId);
    try {
      await deleteObject(bucket, obj.storageKey);
    } catch (err) {
      log.warn("deleteKnowledgeObject: object store delete failed, continuing", {
        objectId: req.id,
        storageKey: obj.storageKey,
        error: err instanceof Error ? err.message : String(err),
      });
    }

    // Cascade: unbind from any projects that reference this object. Deleting
    // an object in "available" state is allowed — downstream factory runs
    // that try to re-resolve it will fail loudly with `invalidArgument`,
    // which is preferable to silent staleness.
    const removed = await db
      .delete(documentBindings)
      .where(eq(documentBindings.knowledgeObjectId, req.id))
      .returning({ id: documentBindings.id });

    await db
      .delete(knowledgeObjects)
      .where(eq(knowledgeObjects.id, req.id));

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.object_deleted",
      targetType: "knowledge_object",
      targetId: req.id,
      metadata: {
        filename: obj.filename,
        state: obj.state,
        bindingsRemoved: removed.length,
      },
    });

    return { deleted: true, bindingsRemoved: removed.length };
  }
);

// =========================================================================
// SOURCE CONNECTORS
// =========================================================================

// ---------------------------------------------------------------------------
// List connectors for workspace
// ---------------------------------------------------------------------------

export const listConnectors = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/knowledge/connectors",
  },
  async (): Promise<{ connectors: SourceConnectorRow[] }> => {
    const auth = getAuthData()!;

    const rows = await db
      .select({
        id: sourceConnectors.id,
        workspaceId: sourceConnectors.workspaceId,
        type: sourceConnectors.type,
        name: sourceConnectors.name,
        syncSchedule: sourceConnectors.syncSchedule,
        status: sourceConnectors.status,
        lastSyncedAt: sourceConnectors.lastSyncedAt,
        createdAt: sourceConnectors.createdAt,
        updatedAt: sourceConnectors.updatedAt,
      })
      .from(sourceConnectors)
      .where(eq(sourceConnectors.workspaceId, auth.workspaceId))
      .orderBy(desc(sourceConnectors.createdAt));

    return { connectors: rows };
  }
);

// ---------------------------------------------------------------------------
// Create connector
// ---------------------------------------------------------------------------

type CreateConnectorRequest = {
  type: string;
  name: string;
  config?: Record<string, unknown>;
  syncSchedule?: string;
};

export const createConnector = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/knowledge/connectors",
  },
  async (
    req: CreateConnectorRequest
  ): Promise<{ connector: SourceConnectorRow }> => {
    const auth = getAuthData()!;

    if (!auth.workspaceId) {
      throw APIError.invalidArgument("workspace context required");
    }
    if (!req.type || !req.name) {
      throw APIError.invalidArgument("type and name are required");
    }

    const validTypes = ["upload", "sharepoint", "s3", "azure-blob", "gcs"];
    if (!validTypes.includes(req.type)) {
      throw APIError.invalidArgument(`invalid connector type: ${req.type}`);
    }

    // Validate config via connector implementation
    if (req.config) {
      const impl = getConnectorImpl(req.type);
      if (impl) {
        const validation = impl.validateConfig(req.config);
        if (!validation.valid) {
          throw APIError.invalidArgument(
            `invalid config: ${validation.errors.join(", ")}`
          );
        }
      }
    }

    const [connector] = await db
      .insert(sourceConnectors)
      .values({
        workspaceId: auth.workspaceId,
        type: req.type as any,
        name: req.name,
        configEncrypted: req.config ?? null,
        syncSchedule: req.syncSchedule ?? null,
      })
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.connector_created",
      targetType: "source_connector",
      targetId: connector.id,
      metadata: { type: req.type, name: req.name },
    });

    return {
      connector: {
        id: connector.id,
        workspaceId: connector.workspaceId,
        type: connector.type,
        name: connector.name,
        syncSchedule: connector.syncSchedule,
        status: connector.status,
        lastSyncedAt: connector.lastSyncedAt,
        createdAt: connector.createdAt,
        updatedAt: connector.updatedAt,
      },
    };
  }
);

// ---------------------------------------------------------------------------
// Delete connector
// ---------------------------------------------------------------------------

export const deleteConnector = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/api/knowledge/connectors/:id",
  },
  async (req: { id: string }): Promise<{ deleted: boolean }> => {
    const auth = getAuthData()!;

    const [conn] = await db
      .select()
      .from(sourceConnectors)
      .where(
        and(
          eq(sourceConnectors.id, req.id),
          eq(sourceConnectors.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!conn) {
      throw APIError.notFound("connector not found");
    }

    // Check if any knowledge objects reference this connector
    const [ref] = await db
      .select({ id: knowledgeObjects.id })
      .from(knowledgeObjects)
      .where(eq(knowledgeObjects.connectorId, req.id))
      .limit(1);

    if (ref) {
      throw APIError.invalidArgument(
        "cannot delete connector: knowledge objects still reference it"
      );
    }

    await db
      .delete(sourceConnectors)
      .where(eq(sourceConnectors.id, req.id));

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.connector_deleted",
      targetType: "source_connector",
      targetId: req.id,
      metadata: { name: conn.name },
    });

    return { deleted: true };
  }
);

// ---------------------------------------------------------------------------
// Get a single connector
// ---------------------------------------------------------------------------

export const getConnector = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/knowledge/connectors/:id",
  },
  async (req: { id: string }): Promise<{ connector: SourceConnectorRow }> => {
    const auth = getAuthData()!;

    const [row] = await db
      .select({
        id: sourceConnectors.id,
        workspaceId: sourceConnectors.workspaceId,
        type: sourceConnectors.type,
        name: sourceConnectors.name,
        syncSchedule: sourceConnectors.syncSchedule,
        status: sourceConnectors.status,
        lastSyncedAt: sourceConnectors.lastSyncedAt,
        createdAt: sourceConnectors.createdAt,
        updatedAt: sourceConnectors.updatedAt,
      })
      .from(sourceConnectors)
      .where(
        and(
          eq(sourceConnectors.id, req.id),
          eq(sourceConnectors.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!row) {
      throw APIError.notFound("connector not found");
    }

    return { connector: row };
  }
);

// ---------------------------------------------------------------------------
// Update connector
// ---------------------------------------------------------------------------

type UpdateConnectorRequest = {
  id: string;
  name?: string;
  config?: Record<string, unknown>;
  syncSchedule?: string | null;
  status?: string;
};

export const updateConnector = api(
  {
    expose: true,
    auth: true,
    method: "PATCH",
    path: "/api/knowledge/connectors/:id",
  },
  async (
    req: UpdateConnectorRequest
  ): Promise<{ connector: SourceConnectorRow }> => {
    const auth = getAuthData()!;

    const [existing] = await db
      .select()
      .from(sourceConnectors)
      .where(
        and(
          eq(sourceConnectors.id, req.id),
          eq(sourceConnectors.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!existing) {
      throw APIError.notFound("connector not found");
    }

    // Validate config if provided
    if (req.config) {
      const impl = getConnectorImpl(existing.type);
      if (impl) {
        const validation = impl.validateConfig(req.config);
        if (!validation.valid) {
          throw APIError.invalidArgument(
            `invalid config: ${validation.errors.join(", ")}`
          );
        }
      }
    }

    if (req.status) {
      const validStatuses = ["active", "paused", "error", "disabled"];
      if (!validStatuses.includes(req.status)) {
        throw APIError.invalidArgument(`invalid status: ${req.status}`);
      }
    }

    const updates: Record<string, unknown> = { updatedAt: new Date() };
    if (req.name !== undefined) updates.name = req.name;
    if (req.config !== undefined) updates.configEncrypted = req.config;
    if (req.syncSchedule !== undefined) updates.syncSchedule = req.syncSchedule;
    if (req.status !== undefined) updates.status = req.status;

    const [updated] = await db
      .update(sourceConnectors)
      .set(updates)
      .where(eq(sourceConnectors.id, req.id))
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.connector_updated",
      targetType: "source_connector",
      targetId: req.id,
      metadata: { fields: Object.keys(updates).filter((k) => k !== "updatedAt") },
    });

    return {
      connector: {
        id: updated.id,
        workspaceId: updated.workspaceId,
        type: updated.type,
        name: updated.name,
        syncSchedule: updated.syncSchedule,
        status: updated.status,
        lastSyncedAt: updated.lastSyncedAt,
        createdAt: updated.createdAt,
        updatedAt: updated.updatedAt,
      },
    };
  }
);

// ---------------------------------------------------------------------------
// Test connector connection
// ---------------------------------------------------------------------------

export const testConnectorConnection = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/knowledge/connectors/:id/test",
  },
  async (req: { id: string }): Promise<{ success: boolean; error?: string }> => {
    const auth = getAuthData()!;

    const [conn] = await db
      .select()
      .from(sourceConnectors)
      .where(
        and(
          eq(sourceConnectors.id, req.id),
          eq(sourceConnectors.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!conn) {
      throw APIError.notFound("connector not found");
    }

    const impl = getConnectorImpl(conn.type);
    if (!impl) {
      throw APIError.invalidArgument(`no implementation for connector type: ${conn.type}`);
    }

    try {
      await impl.testConnection(
        (conn.configEncrypted as Record<string, unknown>) ?? {}
      );
      return { success: true };
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      return { success: false, error: message };
    }
  }
);

// ---------------------------------------------------------------------------
// Trigger sync for a connector
// ---------------------------------------------------------------------------

export const triggerSync = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/knowledge/connectors/:id/sync",
  },
  async (req: { id: string }): Promise<{ syncRunId: string }> => {
    const auth = getAuthData()!;

    const [conn] = await db
      .select()
      .from(sourceConnectors)
      .where(
        and(
          eq(sourceConnectors.id, req.id),
          eq(sourceConnectors.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!conn) {
      throw APIError.notFound("connector not found");
    }

    if (conn.status !== "active") {
      throw APIError.invalidArgument(
        `connector is ${conn.status} — only active connectors can be synced`
      );
    }

    if (conn.type === "upload") {
      throw APIError.invalidArgument(
        "upload connectors do not support sync — use the upload endpoint"
      );
    }

    const bucket = await getWorkspaceBucket(auth.workspaceId);

    // Get the last successful delta token
    const [lastRun] = await db
      .select({ deltaToken: syncRuns.deltaToken })
      .from(syncRuns)
      .where(
        and(
          eq(syncRuns.connectorId, conn.id),
          eq(syncRuns.status, "completed")
        )
      )
      .orderBy(desc(syncRuns.completedAt))
      .limit(1);

    const syncRunId = await executeSyncRun(
      conn.id,
      auth.workspaceId,
      bucket,
      conn.type,
      (conn.configEncrypted as Record<string, unknown>) ?? {},
      lastRun?.deltaToken ?? null,
      auth.userID
    );

    return { syncRunId };
  }
);

// ---------------------------------------------------------------------------
// List sync runs for a connector
// ---------------------------------------------------------------------------

export const listSyncRuns = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/knowledge/connectors/:id/sync-runs",
  },
  async (req: { id: string }): Promise<{ runs: SyncRunRow[] }> => {
    const auth = getAuthData()!;

    // Verify the connector belongs to this workspace
    const [conn] = await db
      .select({ id: sourceConnectors.id })
      .from(sourceConnectors)
      .where(
        and(
          eq(sourceConnectors.id, req.id),
          eq(sourceConnectors.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);

    if (!conn) {
      throw APIError.notFound("connector not found");
    }

    const rows = await db
      .select()
      .from(syncRuns)
      .where(eq(syncRuns.connectorId, req.id))
      .orderBy(desc(syncRuns.startedAt))
      .limit(50);

    return { runs: rows };
  }
);

// ---------------------------------------------------------------------------
// Sync execution orchestrator (used by both on-demand and scheduled sync)
// ---------------------------------------------------------------------------

export async function executeSyncRun(
  connectorId: string,
  workspaceId: string,
  bucket: string,
  connectorType: string,
  config: Record<string, unknown>,
  previousDeltaToken: string | null,
  actorUserId: string
): Promise<string> {
  const impl = getConnectorImpl(connectorType);
  if (!impl) {
    throw new Error(`no implementation for connector type: ${connectorType}`);
  }

  // Create sync run record
  const [run] = await db
    .insert(syncRuns)
    .values({
      connectorId,
      workspaceId,
      status: "running",
    })
    .returning();

  const ctx: SyncContext = {
    connectorId,
    workspaceId,
    bucket,
    config,
    previousDeltaToken,
  };

  // Execute the sync asynchronously (don't block the request)
  setImmediate(async () => {
    try {
      const result = await impl.sync(ctx);

      let created = 0;
      let updated = 0;
      let skipped = 0;

      // Create/update knowledge objects for each synced file
      for (const obj of result.objects) {
        if (obj.action === "skipped") {
          skipped++;
          continue;
        }

        // Check if a knowledge object with this content hash already exists
        const [existing] = await db
          .select({ id: knowledgeObjects.id, contentHash: knowledgeObjects.contentHash })
          .from(knowledgeObjects)
          .where(
            and(
              eq(knowledgeObjects.workspaceId, workspaceId),
              eq(knowledgeObjects.connectorId, connectorId),
              eq(knowledgeObjects.storageKey, obj.storageKey)
            )
          )
          .limit(1);

        // Spec 115 FR-009 — track which rows are newly inserted or had a
        // content-hash change so we can enqueue extraction for them after
        // the sync transaction.
        let enqueueObjectId: string | null = null;

        if (existing) {
          if (existing.contentHash === obj.contentHash) {
            skipped++;
            continue;
          }
          // Update existing object (content changed)
          await db
            .update(knowledgeObjects)
            .set({
              contentHash: obj.contentHash,
              sizeBytes: obj.sizeBytes,
              mimeType: obj.mimeType,
              provenance: obj.provenance,
              state: "imported",
              extractionOutput: null,
              lastExtractionError: null,
              updatedAt: new Date(),
            })
            .where(eq(knowledgeObjects.id, existing.id));
          enqueueObjectId = existing.id;
          updated++;
        } else {
          const [created_] = await db
            .insert(knowledgeObjects)
            .values({
              workspaceId,
              connectorId,
              storageKey: obj.storageKey,
              filename: obj.filename,
              mimeType: obj.mimeType,
              sizeBytes: obj.sizeBytes,
              contentHash: obj.contentHash,
              state: "imported",
              provenance: obj.provenance,
            })
            .returning({ id: knowledgeObjects.id });
          enqueueObjectId = created_.id;
          created++;
        }

        if (enqueueObjectId) {
          try {
            await enqueueExtraction({
              knowledgeObjectId: enqueueObjectId,
              workspaceId,
              reason: "connector_sync",
            });
          } catch (err) {
            log.warn("connector sync: enqueueExtraction failed; row kept", {
              connectorId,
              objectId: enqueueObjectId,
              err: err instanceof Error ? err.message : String(err),
            });
          }
        }
      }

      // Mark sync run as completed
      await db
        .update(syncRuns)
        .set({
          status: "completed",
          objectsCreated: created,
          objectsUpdated: updated,
          objectsSkipped: skipped,
          deltaToken: result.deltaToken,
          completedAt: new Date(),
        })
        .where(eq(syncRuns.id, run.id));

      // Update connector's last_synced_at
      await db
        .update(sourceConnectors)
        .set({ lastSyncedAt: new Date(), updatedAt: new Date() })
        .where(eq(sourceConnectors.id, connectorId));

      // Audit
      await db.insert(auditLog).values({
        actorUserId,
        action: "knowledge.sync_completed",
        targetType: "source_connector",
        targetId: connectorId,
        metadata: { syncRunId: run.id, created, updated, skipped },
      });

      // Broadcast sync completion to connected clients
      broadcastToWorkspace(workspaceId, {
        type: "connector_sync_complete",
        workspaceId,
        timestamp: new Date().toISOString(),
        payload: {
          connectorId,
          syncRunId: run.id,
          objectsCreated: created,
          objectsUpdated: updated,
          objectsSkipped: skipped,
        },
      });

      log.info("sync run completed", {
        syncRunId: run.id,
        connectorId,
        created,
        updated,
        skipped,
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);

      await db
        .update(syncRuns)
        .set({
          status: "failed",
          error: message,
          completedAt: new Date(),
        })
        .where(eq(syncRuns.id, run.id));

      // Set connector to error status
      await db
        .update(sourceConnectors)
        .set({ status: "error" as any, updatedAt: new Date() })
        .where(eq(sourceConnectors.id, connectorId));

      log.error("sync run failed", {
        syncRunId: run.id,
        connectorId,
        error: message,
      });
    }
  });

  return run.id;
}

// =========================================================================
// DOCUMENT BINDINGS
// =========================================================================

// ---------------------------------------------------------------------------
// List bindings for a project
// ---------------------------------------------------------------------------

export const listBindings = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/knowledge/bindings/:projectId",
  },
  async (req: {
    projectId: string;
  }): Promise<{ bindings: DocumentBindingRow[] }> => {
    const auth = getAuthData()!;

    await verifyProjectInScope(req.projectId, auth.workspaceId);

    const rows = await db
      .select()
      .from(documentBindings)
      .where(eq(documentBindings.projectId, req.projectId));

    return { bindings: rows };
  }
);

// ---------------------------------------------------------------------------
// Bind knowledge objects to a project
// ---------------------------------------------------------------------------

type BindRequest = {
  projectId: string;
  knowledgeObjectIds: string[];
};

export const bindToProject = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/knowledge/bindings/:projectId",
  },
  async (req: BindRequest): Promise<{ bindings: DocumentBindingRow[] }> => {
    const auth = getAuthData()!;

    await verifyProjectInScope(req.projectId, auth.workspaceId);

    if (!req.knowledgeObjectIds || req.knowledgeObjectIds.length === 0) {
      throw APIError.invalidArgument("knowledgeObjectIds required");
    }

    // Verify all knowledge objects belong to this workspace
    const objs = await db
      .select({ id: knowledgeObjects.id })
      .from(knowledgeObjects)
      .where(
        and(
          inArray(knowledgeObjects.id, req.knowledgeObjectIds),
          eq(knowledgeObjects.workspaceId, auth.workspaceId)
        )
      );

    if (objs.length !== req.knowledgeObjectIds.length) {
      throw APIError.invalidArgument(
        "some knowledge objects not found in workspace"
      );
    }

    const created: DocumentBindingRow[] = [];
    for (const koId of req.knowledgeObjectIds) {
      const [binding] = await db
        .insert(documentBindings)
        .values({
          projectId: req.projectId,
          knowledgeObjectId: koId,
          boundBy: auth.userID,
        })
        .onConflictDoNothing()
        .returning();

      if (binding) {
        created.push(binding);
      }
    }

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.objects_bound",
      targetType: "project",
      targetId: req.projectId,
      metadata: { knowledgeObjectIds: req.knowledgeObjectIds },
    });

    return { bindings: created };
  }
);

// ---------------------------------------------------------------------------
// Unbind a knowledge object from a project
// ---------------------------------------------------------------------------

type UnbindRequest = {
  projectId: string;
  knowledgeObjectId: string;
};

export const unbindFromProject = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/api/knowledge/bindings/:projectId/:knowledgeObjectId",
  },
  async (req: UnbindRequest): Promise<{ deleted: boolean }> => {
    const auth = getAuthData()!;

    await verifyProjectInScope(req.projectId, auth.workspaceId);

    const [binding] = await db
      .select()
      .from(documentBindings)
      .where(
        and(
          eq(documentBindings.projectId, req.projectId),
          eq(documentBindings.knowledgeObjectId, req.knowledgeObjectId)
        )
      )
      .limit(1);

    if (!binding) {
      throw APIError.notFound("binding not found");
    }

    await db
      .delete(documentBindings)
      .where(eq(documentBindings.id, binding.id));

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.object_unbound",
      targetType: "project",
      targetId: req.projectId,
      metadata: { knowledgeObjectId: req.knowledgeObjectId },
    });

    return { deleted: true };
  }
);

// =========================================================================
// FACTORY INTEGRATION
// =========================================================================

// ---------------------------------------------------------------------------
// Resolve bound knowledge objects into factory business doc references.
// Called by factory initPipeline when knowledge_object_ids are provided.
// ---------------------------------------------------------------------------

export type FactoryDocRef = {
  name: string;
  storage_ref: string;
};

export async function resolveKnowledgeForFactory(
  workspaceId: string,
  knowledgeObjectIds: string[]
): Promise<FactoryDocRef[]> {
  if (knowledgeObjectIds.length === 0) return [];

  const objs = await db
    .select({
      id: knowledgeObjects.id,
      filename: knowledgeObjects.filename,
      storageKey: knowledgeObjects.storageKey,
      state: knowledgeObjects.state,
      workspaceId: knowledgeObjects.workspaceId,
    })
    .from(knowledgeObjects)
    .where(
      and(
        inArray(knowledgeObjects.id, knowledgeObjectIds),
        eq(knowledgeObjects.workspaceId, workspaceId)
      )
    );

  // Verify all requested objects exist
  if (objs.length !== knowledgeObjectIds.length) {
    const found = new Set(objs.map((o) => o.id));
    const missing = knowledgeObjectIds.filter((id) => !found.has(id));
    throw APIError.invalidArgument(
      `knowledge objects not found: ${missing.join(", ")}`
    );
  }

  // Verify all are in "available" state
  const notReady = objs.filter((o) => o.state !== "available");
  if (notReady.length > 0) {
    throw APIError.invalidArgument(
      `knowledge objects not in 'available' state: ${notReady.map((o) => o.id).join(", ")}`
    );
  }

  return objs.map((o) => ({
    name: o.filename,
    storage_ref: o.storageKey,
  }));
}

// ---------------------------------------------------------------------------
// Resolve bound knowledge objects into full wire-level KnowledgeBundle
// entries for the spec 110 factory.run.request envelope. Unlike
// resolveKnowledgeForFactory this generates a presigned download URL so the
// OPC consumer can materialise the blob locally before handing it to the
// engine.
// ---------------------------------------------------------------------------

export type KnowledgeBundleEntry = {
  objectId: string;
  filename: string;
  contentHash: string;
  downloadUrl: string;
};

/**
 * Presigned URL TTL for knowledge bundles dispatched on factory.run.request
 * (spec 110 §2.3, open question 1). Short enough that a leaked URL expires
 * quickly; long enough that a busy OPC can download the blob on a cold
 * cache. Resync regenerates it if the run rides an outbox retry.
 */
const KNOWLEDGE_BUNDLE_URL_TTL_SECONDS = 15 * 60;

export async function resolveKnowledgeBundlesForFactory(
  workspaceId: string,
  knowledgeObjectIds: string[]
): Promise<KnowledgeBundleEntry[]> {
  if (knowledgeObjectIds.length === 0) return [];

  const objs = await db
    .select({
      id: knowledgeObjects.id,
      filename: knowledgeObjects.filename,
      storageKey: knowledgeObjects.storageKey,
      contentHash: knowledgeObjects.contentHash,
      state: knowledgeObjects.state,
    })
    .from(knowledgeObjects)
    .where(
      and(
        inArray(knowledgeObjects.id, knowledgeObjectIds),
        eq(knowledgeObjects.workspaceId, workspaceId)
      )
    );

  if (objs.length !== knowledgeObjectIds.length) {
    const found = new Set(objs.map((o) => o.id));
    const missing = knowledgeObjectIds.filter((id) => !found.has(id));
    throw APIError.invalidArgument(
      `knowledge objects not found: ${missing.join(", ")}`
    );
  }

  const notReady = objs.filter((o) => o.state !== "available");
  if (notReady.length > 0) {
    throw APIError.invalidArgument(
      `knowledge objects not in 'available' state: ${notReady.map((o) => o.id).join(", ")}`
    );
  }

  const bucket = await getWorkspaceBucket(workspaceId);
  return Promise.all(
    objs.map(async (o) => ({
      objectId: o.id,
      filename: o.filename,
      contentHash: o.contentHash,
      downloadUrl: await getPresignedDownloadUrl(
        bucket,
        o.storageKey,
        KNOWLEDGE_BUNDLE_URL_TTL_SECONDS
      ),
    }))
  );
}
