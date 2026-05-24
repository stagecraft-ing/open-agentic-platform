import { api, APIError, Header } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { and, desc, eq, or, sql } from "drizzle-orm";
import { db } from "../db/drizzle";
import { auditLog, projects } from "../db/schema";
import { validateM2mRequest } from "../auth/m2mAuth.js";

const SYSTEM_USER_ID = "00000000-0000-0000-0000-000000000000";

type IngestAuditRequest = {
  authorization: Header<"Authorization">;
  action: string;
  targetType: string;
  targetId: string;
  metadata?: Record<string, unknown>;
};

type IngestAuditResponse = { ok: true };

/**
 * Seam B: Ingest audit records from OPC axiomregent.
 * POST /api/audit-records — M2M bearer token auth (OIDC JWT or static fallback).
 */
export const ingestAuditRecord = api(
  { expose: true, method: "POST", path: "/api/audit-records" },
  async (req: IngestAuditRequest): Promise<IngestAuditResponse> => {
    await validateM2mRequest(req.authorization, "platform:audit:write");

    await db.insert(auditLog).values({
      actorUserId: SYSTEM_USER_ID,
      action: req.action,
      targetType: req.targetType,
      targetId: req.targetId,
      metadata: req.metadata ?? {},
    });

    return { ok: true };
  }
);

// ---------------------------------------------------------------------------
// Spec 175 — project-scoped audit summary read.
//
// The dashboard endpoint composes this read into the audit summary panel.
// Filter shape mirrors the existing `metadata->>'projectId'` query used by
// `factory/runDuplexHandlers.test.ts` and `knowledge/orphanSweeper`: a
// row matches the project when either `target_type='project' AND
// target_id=:projectId` or the metadata jsonb carries the project id
// under the `projectId` key.
// ---------------------------------------------------------------------------

export type ProjectAuditRow = {
  id: string;
  actorUserId: string;
  action: string;
  targetType: string;
  targetId: string;
  createdAt: string;
  metadata: Record<string, unknown>;
};

export type ListProjectAuditRecordsResponse = {
  rows: ProjectAuditRow[];
};

async function assertProjectInOrg(projectId: string): Promise<void> {
  const auth = getAuthData();
  if (!auth) {
    throw APIError.unauthenticated("authentication required");
  }
  const rows = await db
    .select({ id: projects.id })
    .from(projects)
    .where(and(eq(projects.id, projectId), eq(projects.orgId, auth.orgId)))
    .limit(1);
  if (rows.length === 0) {
    throw APIError.notFound("project not found");
  }
}

/**
 * Spec 175 FR-007 — list recent audit_log rows scoped to a project.
 *
 * The default `limit` is 5 to match the dashboard panel; callers asking
 * for more raise to a hard cap of 50.
 */
export const listProjectAuditRecords = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/projects/:projectId/audit",
  },
  async (req: {
    projectId: string;
    limit?: number;
  }): Promise<ListProjectAuditRecordsResponse> => {
    await assertProjectInOrg(req.projectId);
    const limit = Math.min(Math.max(req.limit ?? 5, 1), 50);

    const rows = await db
      .select({
        id: auditLog.id,
        actorUserId: auditLog.actorUserId,
        action: auditLog.action,
        targetType: auditLog.targetType,
        targetId: auditLog.targetId,
        metadata: auditLog.metadata,
        createdAt: auditLog.createdAt,
      })
      .from(auditLog)
      .where(
        or(
          and(
            eq(auditLog.targetType, "project"),
            eq(auditLog.targetId, req.projectId)
          ),
          sql`${auditLog.metadata}->>'projectId' = ${req.projectId}`
        )
      )
      .orderBy(desc(auditLog.createdAt))
      .limit(limit);

    return {
      rows: rows.map((r) => ({
        id: r.id,
        actorUserId: r.actorUserId,
        action: r.action,
        targetType: r.targetType,
        targetId: r.targetId,
        metadata: (r.metadata ?? {}) as Record<string, unknown>,
        createdAt: r.createdAt.toISOString(),
      })),
    };
  }
);
