/**
 * Encore auth handler — validates Rauthy JWTs on all authenticated API calls.
 * Spec 087 Phase 5: OIDC JWT enforcement — HMAC session fallback removed.
 *
 * All authenticated requests must present a valid Rauthy-issued JWT
 * in the Authorization header (Bearer) or __session cookie.
 */

import { Header, Gateway } from "encore.dev/api";
import { authHandler } from "encore.dev/auth";
import { validateJwt } from "./rauthy";

// ---------------------------------------------------------------------------
// Auth types
// ---------------------------------------------------------------------------

interface AuthParams {
  authorization?: Header<"Authorization">;
  cookie?: Header<"Cookie">;
}

export interface AuthData {
  userID: string;
  orgId: string;
  orgSlug: string;
  workspaceId: string;
  githubLogin: string;       // may be empty for enterprise IdP users
  idpProvider: string;        // github | azure-ad | okta | google-workspace | etc.
  idpLogin: string;           // provider-specific login/display name
  platformRole: "owner" | "admin" | "member";
}

// ---------------------------------------------------------------------------
// Auth handler
// ---------------------------------------------------------------------------

export const auth = authHandler<AuthParams, AuthData>(async (params) => {
  // Extract JWT from Authorization header or session cookie
  let token: string | undefined;

  if (params.authorization) {
    const parts = params.authorization.split(" ");
    if (parts[0]?.toLowerCase() === "bearer" && parts[1]) {
      token = parts[1];
    }
  }

  if (!token && params.cookie) {
    // Parse __session cookie
    const match = params.cookie.match(/(?:^|;\s*)__session=([^\s;]+)/);
    if (match) {
      token = match[1];
    }
  }

  if (!token) {
    throw new Error("No authentication token provided");
  }

  // Validate Rauthy JWT — the only accepted auth mechanism
  const claims = await validateJwt(token);
  if (!claims) {
    throw new Error("Invalid or expired JWT");
  }

  return {
    userID: claims.oap_user_id,
    orgId: claims.oap_org_id,
    orgSlug: claims.oap_org_slug,
    workspaceId: claims.oap_workspace_id ?? "",
    githubLogin: claims.github_login ?? "",
    idpProvider: claims.idp_provider ?? (claims.github_login ? "github" : ""),
    idpLogin: claims.idp_login ?? claims.github_login ?? "",
    platformRole: claims.platform_role as AuthData["platformRole"],
  };
});

// ---------------------------------------------------------------------------
// API Gateway — routes all external traffic through the auth handler
// ---------------------------------------------------------------------------

export const gateway = new Gateway({ authHandler: auth });
