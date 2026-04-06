/**
 * Encore auth handler — validates Rauthy JWTs on all authenticated API calls.
 * Spec 080 FR-003: Rauthy session integration.
 *
 * This wires Encore's built-in auth system to validate Rauthy-issued JWTs.
 * Once this handler exists, any API endpoint with `auth: true` will require
 * a valid Rauthy JWT in the Authorization header or __session cookie.
 */

import { Header, Gateway } from "encore.dev/api";
import { authHandler } from "encore.dev/auth";
import { validateJwt } from "./rauthy";
import { verifyPayload } from "./session-crypto";

// ---------------------------------------------------------------------------
// Auth types
// ---------------------------------------------------------------------------

interface AuthParams {
  authorization: Header<"Authorization">;
  cookie: Header<"Cookie">;
}

export interface AuthData {
  userId: string;
  orgId: string;
  orgSlug: string;
  githubLogin: string;
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

  // Try Rauthy JWT first (production path)
  if (token.split(".").length === 3) {
    const claims = await validateJwt(token);
    if (claims) {
      return {
        userId: claims.oap_user_id,
        orgId: claims.oap_org_id,
        orgSlug: claims.oap_org_slug,
        githubLogin: claims.github_login,
        platformRole: claims.platform_role as AuthData["platformRole"],
      };
    }
  }

  // Try HMAC-signed session cookie (transitional path)
  const session = verifyPayload<{
    userId: string;
    orgId: string;
    orgSlug: string;
    githubLogin: string;
    platformRole: string;
    iat?: number;
  }>(token);

  if (session) {
    // Check cookie age (14 days max)
    if (session.iat && session.iat < Math.floor(Date.now() / 1000) - 14 * 86400) {
      throw new Error("Session expired");
    }
    return {
      userId: session.userId,
      orgId: session.orgId,
      orgSlug: session.orgSlug,
      githubLogin: session.githubLogin,
      platformRole: session.platformRole as AuthData["platformRole"],
    };
  }

  throw new Error("Invalid or expired token");
});

// ---------------------------------------------------------------------------
// API Gateway — routes all external traffic through the auth handler
// ---------------------------------------------------------------------------

export const gateway = new Gateway({ authHandler: auth });
