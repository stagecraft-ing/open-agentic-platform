import { api, APIError } from "encore.dev/api";

type AgentAuthorizedResponse = { authorized: true };

/**
 * Seam D: Validate agent execution against org-level policies.
 * GET /api/agents/:slug/authorized
 *
 * Returns 200 if the agent is authorized, 403 if blocked, 404 if unknown.
 * Current implementation: allow all agents (policy enforcement can be added later).
 */
export const isAgentAuthorized = api(
  { expose: true, method: "GET", path: "/api/agents/:slug/authorized" },
  async (req: { slug: string }): Promise<AgentAuthorizedResponse> => {
    // Placeholder: all agents authorized by default.
    // Future: look up agent slug in an org-level allowlist/blocklist table.
    return { authorized: true };
  }
);
