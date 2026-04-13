/**
 * Desktop (OPC) OAuth endpoints (spec 080 Phase 1).
 *
 * Implements Authorization Code + PKCE for the Tauri desktop app.
 * OPC opens a browser to /auth/desktop/authorize, which redirects to GitHub.
 * After GitHub callback (handled in github.ts), OPC receives a deep-link
 * with a one-time auth code, which it exchanges here for tokens.
 *
 * Endpoints:
 *   GET  /auth/desktop/authorize   — initiate desktop PKCE flow
 *   POST /auth/desktop/token       — exchange auth code for tokens
 *   POST /auth/desktop/org-select  — finalize org selection (multi-org)
 *   POST /auth/desktop/refresh     — refresh access token
 */

import { api, APIError } from "encore.dev/api";
import log from "encore.dev/log";
import crypto from "crypto";
import { db } from "../db/drizzle";
import { desktopRefreshTokens, oidcProviders } from "../db/schema";
import { eq, and, gt } from "drizzle-orm";
import { issueRauthySession } from "./rauthy";
import {
  pendingDesktopFlows,
  pendingDesktopSessions,
  consumeDesktopSession,
  cleanupDesktopState,
  type PendingDesktopSession,
} from "./desktop-state";
import {
  githubOAuthClientId,
  appBaseUrl,
  pendingStates,
} from "./github";
import { buildAuthorizationUrl } from "./rauthy";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface DesktopUser {
  id: string;
  email: string;
  name: string;
  githubLogin: string;
  idpProvider: string;
  idpLogin: string;
  avatarUrl: string;
}

interface DesktopOrg {
  orgId: string;
  orgSlug: string;
  githubOrgLogin: string;
  orgDisplayName: string;
  platformRole: string;
}

interface DesktopTokenResponse {
  type: "authenticated";
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
  user: DesktopUser;
  org: DesktopOrg;
}

interface DesktopOrgSelectionResponse {
  type: "org_selection";
  pendingId: string;
  orgs: DesktopOrg[];
  user: DesktopUser;
}

type DesktopTokenResult = DesktopTokenResponse | DesktopOrgSelectionResponse;

interface DesktopRefreshResponse {
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
}

// Refresh token TTL: 14 days
const REFRESH_TOKEN_TTL_MS = 14 * 24 * 60 * 60 * 1000;

// ---------------------------------------------------------------------------
// GET /auth/desktop/authorize — initiate desktop PKCE flow
// ---------------------------------------------------------------------------

export const desktopAuthorize = api.raw(
  { expose: true, method: "GET", path: "/auth/desktop/authorize", auth: false },
  async (req, resp) => {
    const url = new URL(req.url!, `http://${req.headers.host}`);
    const codeChallenge = url.searchParams.get("code_challenge");
    const codeChallengeMethod = url.searchParams.get("code_challenge_method");
    const desktopState = url.searchParams.get("state");
    const redirectUri = url.searchParams.get("redirect_uri");
    const idpHint = url.searchParams.get("idp_hint");   // optional: OIDC provider ID or email domain

    // Validate required params
    if (!codeChallenge || !codeChallengeMethod || !desktopState || !redirectUri) {
      resp.writeHead(400, { "Content-Type": "application/json" });
      resp.end(JSON.stringify({ error: "Missing required parameters: code_challenge, code_challenge_method, state, redirect_uri" }));
      return;
    }

    if (codeChallengeMethod !== "S256") {
      resp.writeHead(400, { "Content-Type": "application/json" });
      resp.end(JSON.stringify({ error: "Only S256 code_challenge_method is supported" }));
      return;
    }

    if (!redirectUri.startsWith("opc://")) {
      resp.writeHead(400, { "Content-Type": "application/json" });
      resp.end(JSON.stringify({ error: "redirect_uri must use the opc:// scheme" }));
      return;
    }

    cleanupDesktopState();

    // If an IdP hint is provided, try to resolve an enterprise OIDC provider
    // and route through Rauthy instead of GitHub
    if (idpHint) {
      let providerRow;
      // Check if it's a UUID (provider ID) or email domain
      if (idpHint.includes("@")) {
        const domain = idpHint.split("@")[1].toLowerCase();
        [providerRow] = await db
          .select()
          .from(oidcProviders)
          .where(and(eq(oidcProviders.emailDomain, domain), eq(oidcProviders.status, "active")))
          .limit(1);
      } else {
        [providerRow] = await db
          .select()
          .from(oidcProviders)
          .where(and(eq(oidcProviders.id, idpHint), eq(oidcProviders.status, "active")))
          .limit(1);
      }

      if (providerRow) {
        // Route through Rauthy's OIDC authorization endpoint
        const rauthyState = crypto.randomBytes(32).toString("base64url");
        pendingDesktopFlows.set(rauthyState, {
          codeChallenge,
          codeChallengeMethod,
          redirectUri,
          desktopState,
          createdAt: Date.now(),
        });

        const authUrl = buildAuthorizationUrl({
          redirectUri: `${appBaseUrl()}/auth/oidc/callback`,
          state: rauthyState,
          scopes: providerRow.scopes.split(" ").filter(Boolean),
        });

        const authUrlObj = new URL(authUrl);
        if (idpHint.includes("@")) authUrlObj.searchParams.set("login_hint", idpHint);
        authUrlObj.searchParams.set("upstream_auth_provider_id", providerRow.name);

        resp.writeHead(302, { Location: authUrlObj.toString() });
        resp.end();
        return;
      }
    }

    // Default: GitHub OAuth flow
    const githubState = crypto.randomBytes(32).toString("base64url");

    // Store in BOTH maps:
    // - pendingStates so github.ts callback accepts it
    // - pendingDesktopFlows so github.ts detects it's a desktop flow
    pendingStates.set(githubState, { createdAt: Date.now() });
    pendingDesktopFlows.set(githubState, {
      codeChallenge,
      codeChallengeMethod,
      redirectUri,
      desktopState,
      createdAt: Date.now(),
    });

    // Redirect to GitHub OAuth authorize
    const params = new URLSearchParams({
      client_id: githubOAuthClientId(),
      redirect_uri: `${appBaseUrl()}/auth/github/callback`,
      scope: "read:user read:org user:email",
      state: githubState,
    });

    resp.writeHead(302, { Location: `https://github.com/login/oauth/authorize?${params}` });
    resp.end();
  }
);

// ---------------------------------------------------------------------------
// POST /auth/desktop/token — exchange one-time auth code for tokens
// ---------------------------------------------------------------------------

interface DesktopTokenRequest {
  code: string;
  codeVerifier: string;
  redirectUri: string;
}

export const desktopToken = api<DesktopTokenRequest, DesktopTokenResult>(
  { expose: true, method: "POST", path: "/auth/desktop/token", auth: false },
  async (req) => {
    const { code, codeVerifier, redirectUri } = req;

    if (!code || !codeVerifier || !redirectUri) {
      throw APIError.invalidArgument("Missing required fields: code, codeVerifier, redirectUri");
    }

    // Consume the pending session
    const session = consumeDesktopSession(code);
    if (!session) {
      throw APIError.invalidArgument("Invalid or expired auth code");
    }

    // PKCE verification: SHA256(code_verifier) must match the stored code_challenge
    const expectedChallenge = crypto
      .createHash("sha256")
      .update(codeVerifier)
      .digest("base64url");

    if (expectedChallenge !== session.codeChallenge) {
      throw APIError.invalidArgument("PKCE verification failed: code_verifier does not match code_challenge");
    }

    const user: DesktopUser = {
      id: session.userId,
      email: session.email,
      name: session.name,
      githubLogin: session.githubLogin,
      idpProvider: session.idpProvider,
      idpLogin: session.idpLogin,
      avatarUrl: session.avatarUrl,
    };

    // Multi-org: return org list for picker
    if (session.matchedOrgs.length > 1) {
      const pendingId = crypto.randomBytes(32).toString("base64url");
      pendingDesktopSessions.set(pendingId, session);

      return {
        type: "org_selection" as const,
        pendingId,
        orgs: session.matchedOrgs.map((o) => ({
          orgId: o.orgId,
          orgSlug: o.orgSlug,
          githubOrgLogin: o.githubOrgLogin,
          orgDisplayName: o.orgDisplayName,
          platformRole: o.platformRole,
        })),
        user,
      };
    }

    // Single org: issue tokens
    const org = session.matchedOrgs[0];
    const tokens = await issueDesktopTokens(session, org);

    return {
      type: "authenticated" as const,
      ...tokens,
      user,
      org: {
        orgId: org.orgId,
        orgSlug: org.orgSlug,
        githubOrgLogin: org.githubOrgLogin,
        orgDisplayName: org.orgDisplayName,
        platformRole: org.platformRole,
      },
    };
  }
);

// ---------------------------------------------------------------------------
// POST /auth/desktop/org-select — finalize org selection for multi-org users
// ---------------------------------------------------------------------------

interface DesktopOrgSelectRequest {
  pendingId: string;
  orgId: string;
}

export const desktopOrgSelect = api<DesktopOrgSelectRequest, DesktopTokenResponse>(
  { expose: true, method: "POST", path: "/auth/desktop/org-select", auth: false },
  async (req) => {
    const { pendingId, orgId } = req;

    if (!pendingId || !orgId) {
      throw APIError.invalidArgument("Missing required fields: pendingId, orgId");
    }

    const session = pendingDesktopSessions.get(pendingId);
    if (!session) {
      throw APIError.invalidArgument("Invalid or expired pending ID");
    }
    pendingDesktopSessions.delete(pendingId);

    const org = session.matchedOrgs.find((o) => o.orgId === orgId);
    if (!org) {
      throw APIError.invalidArgument("Selected org not in matched org list");
    }

    const tokens = await issueDesktopTokens(session, org);

    return {
      type: "authenticated" as const,
      ...tokens,
      user: {
        id: session.userId,
        email: session.email,
        name: session.name,
        githubLogin: session.githubLogin,
        idpProvider: session.idpProvider,
        idpLogin: session.idpLogin,
        avatarUrl: session.avatarUrl,
      },
      org: {
        orgId: org.orgId,
        orgSlug: org.orgSlug,
        githubOrgLogin: org.githubOrgLogin,
        orgDisplayName: org.orgDisplayName,
        platformRole: org.platformRole,
      },
    };
  }
);

// ---------------------------------------------------------------------------
// POST /auth/desktop/refresh — refresh access token using refresh token
// ---------------------------------------------------------------------------

interface DesktopRefreshRequest {
  refreshToken: string;
}

export const desktopRefresh = api<DesktopRefreshRequest, DesktopRefreshResponse>(
  { expose: true, method: "POST", path: "/auth/desktop/refresh", auth: false },
  async (req) => {
    const { refreshToken } = req;

    if (!refreshToken) {
      throw APIError.invalidArgument("Missing refreshToken");
    }

    // Hash the token and look up in DB
    const tokenHash = crypto.createHash("sha256").update(refreshToken).digest("hex");

    const [row] = await db
      .select()
      .from(desktopRefreshTokens)
      .where(
        and(
          eq(desktopRefreshTokens.tokenHash, tokenHash),
          gt(desktopRefreshTokens.expiresAt, new Date())
        )
      )
      .limit(1);

    if (!row) {
      throw APIError.unauthenticated("Invalid or expired refresh token");
    }

    // Issue new Rauthy access token
    const { accessToken, expiresIn } = await issueRauthySession({
      rauthyUserId: row.rauthyUserId,
      oapUserId: row.userId,
      orgId: row.orgId,
      orgSlug: row.orgSlug,
      workspaceId: row.workspaceId,
      githubLogin: row.githubLogin || undefined,
      idpProvider: row.idpProvider || undefined,
      idpLogin: row.idpLogin || undefined,
      platformRole: row.platformRole,
    });

    // Rotate refresh token: delete old, create new
    await db.delete(desktopRefreshTokens).where(eq(desktopRefreshTokens.id, row.id));

    const newRefreshToken = crypto.randomBytes(48).toString("base64url");
    const newTokenHash = crypto.createHash("sha256").update(newRefreshToken).digest("hex");

    await db.insert(desktopRefreshTokens).values({
      tokenHash: newTokenHash,
      userId: row.userId,
      orgId: row.orgId,
      workspaceId: row.workspaceId,
      orgSlug: row.orgSlug,
      githubLogin: row.githubLogin || "",
      idpProvider: row.idpProvider || "",
      idpLogin: row.idpLogin || "",
      platformRole: row.platformRole,
      rauthyUserId: row.rauthyUserId,
      expiresAt: new Date(Date.now() + REFRESH_TOKEN_TTL_MS),
    });

    return {
      accessToken,
      refreshToken: newRefreshToken,
      expiresIn,
    };
  }
);

// ---------------------------------------------------------------------------
// Internal helper: issue Rauthy access token + create refresh token
// ---------------------------------------------------------------------------

async function issueDesktopTokens(
  session: PendingDesktopSession,
  org: PendingDesktopSession["matchedOrgs"][0]
): Promise<{ accessToken: string; refreshToken: string; expiresIn: number }> {
  // Issue Rauthy access token
  const { accessToken, expiresIn } = await issueRauthySession({
    rauthyUserId: session.rauthyUserId,
    oapUserId: session.userId,
    orgId: org.orgId,
    orgSlug: org.orgSlug,
    workspaceId: org.workspaceId,
    githubLogin: session.githubLogin || undefined,
    idpProvider: session.idpProvider,
    idpLogin: session.idpLogin,
    platformRole: org.platformRole,
  });

  // Generate and store refresh token
  const refreshToken = crypto.randomBytes(48).toString("base64url");
  const tokenHash = crypto.createHash("sha256").update(refreshToken).digest("hex");

  await db.insert(desktopRefreshTokens).values({
    tokenHash,
    userId: session.userId,
    orgId: org.orgId,
    workspaceId: org.workspaceId,
    orgSlug: org.orgSlug,
    githubLogin: session.githubLogin || "",
    idpProvider: session.idpProvider || "",
    idpLogin: session.idpLogin || "",
    platformRole: org.platformRole,
    rauthyUserId: session.rauthyUserId,
    expiresAt: new Date(Date.now() + REFRESH_TOKEN_TTL_MS),
  });

  return { accessToken, refreshToken, expiresIn };
}
