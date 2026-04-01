import { api, APIError } from "encore.dev/api";
import { db } from "../db/drizzle";
import { workspaceGrants } from "../db/schema";
import { and, eq } from "drizzle-orm";

type GrantsResponse = {
  enable_file_read: boolean;
  enable_file_write: boolean;
  enable_network: boolean;
  max_tier: number;
};

/**
 * Seam C: Serve workspace-scoped permission grants to OPC desktop app.
 * GET /api/grants/:userId/:workspaceId
 *
 * Returns the grant row if found, otherwise returns sensible defaults (read-only, tier 1).
 */
export const getGrants = api(
  { expose: true, method: "GET", path: "/api/grants/:userId/:workspaceId" },
  async (req: {
    userId: string;
    workspaceId: string;
  }): Promise<GrantsResponse> => {
    const rows = await db
      .select()
      .from(workspaceGrants)
      .where(
        and(
          eq(workspaceGrants.userId, req.userId),
          eq(workspaceGrants.workspaceId, req.workspaceId)
        )
      )
      .limit(1);

    if (rows.length === 0) {
      // No explicit grant: return restrictive defaults.
      return {
        enable_file_read: true,
        enable_file_write: false,
        enable_network: false,
        max_tier: 1,
      };
    }

    const g = rows[0];
    return {
      enable_file_read: g.enableFileRead,
      enable_file_write: g.enableFileWrite,
      enable_network: g.enableNetwork,
      max_tier: g.maxTier,
    };
  }
);
