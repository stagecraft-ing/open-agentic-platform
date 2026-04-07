import { api, APIError, Header } from "encore.dev/api";
import { validateM2mRequest } from "../auth/m2mAuth.js";
import * as fs from "node:fs";
import * as path from "node:path";

/**
 * Directory where policy bundle JSON files are stored (one per workspace).
 * Mount a config volume here in production, or set POLICY_BUNDLE_DIR env var.
 */
const BUNDLE_DIR = process.env.POLICY_BUNDLE_DIR ?? "/var/lib/stagecraft/policy-bundles";

type PolicyBundleRequest = {
  authorization: Header<"Authorization">;
  workspaceId: string;
};

type PolicyBundleResponse = {
  constitution: unknown[];
  shards: Record<string, unknown[]>;
};

/**
 * Seam A: Serve compiled policy bundles to OPC axiomregent.
 * GET /api/policy-bundle/:workspaceId — M2M bearer token auth (OIDC JWT or static fallback).
 */
export const getPolicyBundle = api(
  { expose: true, method: "GET", path: "/api/policy-bundle/:workspaceId" },
  async (req: PolicyBundleRequest): Promise<PolicyBundleResponse> => {
    await validateM2mRequest(req.authorization, "platform:policy:read");

    // Validate workspaceId to prevent path traversal (082 Phase 2).
    if (!/^[a-zA-Z0-9_-]+$/.test(req.workspaceId)) {
      throw APIError.invalidArgument("invalid workspace ID");
    }

    const filePath = path.join(BUNDLE_DIR, `${req.workspaceId}.json`);

    if (!fs.existsSync(filePath)) {
      throw APIError.notFound(`no policy bundle for workspace ${req.workspaceId}`);
    }

    const raw = fs.readFileSync(filePath, "utf-8");
    const bundle = JSON.parse(raw) as PolicyBundleResponse;

    return bundle;
  }
);
