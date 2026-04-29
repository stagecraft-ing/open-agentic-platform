import { api, APIError, Header } from "encore.dev/api";
import { validateM2mRequest } from "../auth/m2mAuth.js";
import { db } from "../db/drizzle";
import { projectGrants } from "../db/schema";
import { and, eq } from "drizzle-orm";

type GrantsRequest = {
  authorization: Header<"Authorization">;
  userId: string;
  projectId: string;
};

type GrantsResponse = {
  enable_file_read: boolean;
  enable_file_write: boolean;
  enable_network: boolean;
  max_tier: number;
};

/**
 * Spec 119 §6.4 — Serve project-scoped permission grants to OPC desktop app.
 * GET /api/grants/:userId/:projectId — M2M bearer token auth (OIDC JWT or static fallback).
 *
 * Returns the grant row if found, otherwise restrictive defaults (read-only, tier 1).
 */
export const getGrants = api(
  { expose: true, method: "GET", path: "/api/grants/:userId/:projectId" },
  async (req: GrantsRequest): Promise<GrantsResponse> => {
    await validateM2mRequest(req.authorization, "platform:grants:read");

    const rows = await db
      .select()
      .from(projectGrants)
      .where(
        and(
          eq(projectGrants.userId, req.userId),
          eq(projectGrants.projectId, req.projectId)
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
