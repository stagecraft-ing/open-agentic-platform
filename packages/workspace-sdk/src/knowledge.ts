/**
 * Knowledge intake domain types (spec 087 section 4).
 *
 * Knowledge objects are workspace-scoped, not project-scoped.
 * They exist independently of projects and may be consumed by
 * multiple factories across multiple projects.
 */

// ---------------------------------------------------------------------------
// Knowledge object lifecycle
// ---------------------------------------------------------------------------

export type KnowledgeObjectState =
  | "imported"
  | "extracting"
  | "extracted"
  | "classified"
  | "available";

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

export interface KnowledgeObject {
  id: string;
  workspaceId: string;
  connectorId: string | null;
  storageKey: string;
  filename: string;
  mimeType: string;
  sizeBytes: number;
  contentHash: string;
  state: KnowledgeObjectState;
  extractionOutput: Record<string, unknown> | null;
  classification: string[] | null;
  provenance: KnowledgeProvenance;
  createdAt: string;
  updatedAt: string;
}

export interface KnowledgeProvenance {
  sourceType: string;
  sourceUri: string;
  importedAt: string;
  lastSyncedAt?: string;
  versionId?: string;
}

// ---------------------------------------------------------------------------
// Source connectors
// ---------------------------------------------------------------------------

export type ConnectorType =
  | "upload"
  | "sharepoint"
  | "s3"
  | "azure-blob"
  | "gcs";

export type ConnectorStatus = "active" | "paused" | "error" | "disabled";

export interface SourceConnector {
  id: string;
  workspaceId: string;
  type: ConnectorType;
  name: string;
  syncSchedule: string | null;
  status: ConnectorStatus;
  lastSyncedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

// ---------------------------------------------------------------------------
// Document bindings (link knowledge objects to projects)
// ---------------------------------------------------------------------------

export interface DocumentBinding {
  id: string;
  projectId: string;
  knowledgeObjectId: string;
  boundBy: string;
  boundAt: string;
}
