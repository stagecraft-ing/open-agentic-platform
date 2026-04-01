import { api, APIError } from "encore.dev/api";
import { db } from "../db/drizzle";
import { auditLog } from "../db/schema";

const SYSTEM_USER_ID = "00000000-0000-0000-0000-000000000000";

/** Expected bearer token for M2M auth (set via PLATFORM_M2M_TOKEN env var). */
const M2M_TOKEN = process.env.PLATFORM_M2M_TOKEN;

type IngestAuditRequest = {
  action: string;
  targetType: string;
  targetId: string;
  metadata?: Record<string, unknown>;
};

type IngestAuditResponse = { ok: true };

/**
 * Seam B: Ingest audit records from OPC axiomregent.
 * POST /api/audit-records — M2M bearer token auth.
 */
export const ingestAuditRecord = api(
  { expose: true, method: "POST", path: "/api/audit-records" },
  async (req: IngestAuditRequest): Promise<IngestAuditResponse> => {
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
