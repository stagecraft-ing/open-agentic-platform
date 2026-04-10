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
} from "../db/schema";
import { and, eq, desc, inArray } from "drizzle-orm";
import {
  getPresignedUploadUrl,
  getPresignedDownloadUrl,
  headObject,
  deleteObject,
} from "./storage";
import { randomUUID } from "crypto";

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
  createdAt: Date;
  updatedAt: Date;
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

async function verifyProjectInWorkspace(
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
  }): Promise<{ objects: KnowledgeObjectRow[] }> => {
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

    return { objects: rows };
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
  async (req: { id: string }): Promise<{ object: KnowledgeObjectRow }> => {
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

    return { object: row };
  }
);

// ---------------------------------------------------------------------------
// Request presigned upload URL
// ---------------------------------------------------------------------------

type RequestUploadRequest = {
  filename: string;
  mimeType: string;
  contentHash: string; // client-provided SHA-256 for dedup
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

    const bucket = await getWorkspaceBucket(auth.workspaceId);
    const objectId = randomUUID();
    const storageKey = `knowledge/${objectId}/${req.filename}`;

    // Create the knowledge object record in "imported" state (pending upload)
    await db.insert(knowledgeObjects).values({
      id: objectId,
      workspaceId: auth.workspaceId,
      connectorId: null, // direct upload — no connector
      storageKey,
      filename: req.filename,
      mimeType: req.mimeType,
      sizeBytes: 0, // updated on confirm
      contentHash: req.contentHash,
      state: "imported",
      provenance: {
        sourceType: "upload",
        sourceUri: `upload://${req.filename}`,
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

    const [updated] = await db
      .update(knowledgeObjects)
      .set({
        sizeBytes: meta.contentLength,
        updatedAt: new Date(),
      })
      .where(eq(knowledgeObjects.id, req.id))
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.upload_confirmed",
      targetType: "knowledge_object",
      targetId: req.id,
      metadata: {
        sizeBytes: meta.contentLength,
        contentType: meta.contentType,
      },
    });

    log.info("upload confirmed", {
      objectId: req.id,
      sizeBytes: meta.contentLength,
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
    const downloadUrl = await getPresignedDownloadUrl(bucket, obj.storageKey);

    return { downloadUrl };
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
      },
    });

    return { object: updated };
  }
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
  async (req: { id: string }): Promise<{ deleted: boolean }> => {
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

    // NF-001: objects in "available" state are immutable
    if (obj.state === "available") {
      throw APIError.invalidArgument(
        "cannot delete knowledge objects in 'available' state — they are immutable"
      );
    }

    // Delete from object store
    const bucket = await getWorkspaceBucket(auth.workspaceId);
    await deleteObject(bucket, obj.storageKey);

    // Remove any document bindings
    await db
      .delete(documentBindings)
      .where(eq(documentBindings.knowledgeObjectId, req.id));

    // Delete the record
    await db
      .delete(knowledgeObjects)
      .where(eq(knowledgeObjects.id, req.id));

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "knowledge.object_deleted",
      targetType: "knowledge_object",
      targetId: req.id,
      metadata: { filename: obj.filename },
    });

    return { deleted: true };
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

    await verifyProjectInWorkspace(req.projectId, auth.workspaceId);

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

    await verifyProjectInWorkspace(req.projectId, auth.workspaceId);

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

    await verifyProjectInWorkspace(req.projectId, auth.workspaceId);

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
