import { api, APIError, Header } from "encore.dev/api";
import { db } from "../db/drizzle";
import { auditLog } from "../db/schema";
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
