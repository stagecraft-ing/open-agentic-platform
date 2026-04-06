/**
 * GitHub OAuth login flow (spec 080 FR-002).
 *
 * GET  /auth/github          — redirect to GitHub OAuth authorize
 * GET  /auth/github/callback  — exchange code, resolve membership, issue session
 */

import { api } from "encore.dev/api";
import { secret } from "encore.dev/config";
import log from "encore.dev/log";
import crypto from "crypto";
import { db } from "../db/drizzle";
import { users, userIdentities, auditLog } from "../db/schema";
import { eq } from "drizzle-orm";
import { resolveOrgMemberships, type ResolvedOrg } from "./membership";
import { provisionRauthyUser } from "./rauthy";
import { signPayload, verifyPayload } from "./session-crypto";

// GitHub OAuth App credentials (separate from the GitHub App)
const githubOAuthClientId = secret("GITHUB_OAUTH_CLIENT_ID");
const githubOAuthClientSecret = secret("GITHUB_OAUTH_CLIENT_SECRET");

// Base URL for constructing callback URLs
const appBaseUrl = secret("APP_BASE_URL"); // e.g. https://stagecraft.localdev.online

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface GitHubUser {
  id: number;
  login: string;
  name: string | null;
  email: string | null;
  avatar_url: string;
}

interface GitHubAccessTokenResponse {
  access_token: string;
  token_type: string;
  scope: string;
  refresh_token?: string;
  expires_in?: number;
  refresh_token_expires_in?: number;
}

// Ephemeral state store for OAuth CSRF protection (in-memory, short-lived).
// In production, use Redis or a DB-backed store.
const pendingStates = new Map<string, { createdAt: number }>();
const STATE_TTL_MS = 10 * 60 * 1000; // 10 minutes

function cleanupStaleStates() {
  const cutoff = Date.now() - STATE_TTL_MS;
  for (const [key, val] of pendingStates) {
    if (val.createdAt < cutoff) pendingStates.delete(key);
  }
}

// ---------------------------------------------------------------------------
// GET /auth/github — initiate GitHub OAuth flow
// ---------------------------------------------------------------------------

export const githubLogin = api.raw(
  { expose: true, method: "GET", path: "/auth/github", auth: false },
  async (_req, resp) => {
    cleanupStaleStates();

    const state = crypto.randomBytes(32).toString("base64url");
    pendingStates.set(state, { createdAt: Date.now() });

    const params = new URLSearchParams({
      client_id: githubOAuthClientId(),
      redirect_uri: `${appBaseUrl()}/auth/github/callback`,
      scope: "read:user read:org user:email",
      state,
    });

    const url = `https://github.com/login/oauth/authorize?${params.toString()}`;
    resp.writeHead(302, { Location: url });
    resp.end();
  }
);

// ---------------------------------------------------------------------------
// GET /auth/github/callback — handle OAuth callback
// ---------------------------------------------------------------------------

export const githubCallback = api.raw(
  { expose: true, method: "GET", path: "/auth/github/callback", auth: false },
  async (req, resp) => {
    const url = new URL(req.url!, `http://${req.headers.host}`);
    const code = url.searchParams.get("code");
    const state = url.searchParams.get("state");
    const error = url.searchParams.get("error");

    // Handle OAuth errors
    if (error) {
      log.warn("GitHub OAuth error", { error });
      resp.writeHead(302, { Location: "/signin?error=github_denied" });
      resp.end();
      return;
    }

    if (!code || !state) {
      resp.writeHead(400, { "Content-Type": "text/plain" });
      resp.end("Missing code or state parameter");
      return;
    }

    // Validate CSRF state
    if (!pendingStates.has(state)) {
      resp.writeHead(400, { "Content-Type": "text/plain" });
      resp.end("Invalid or expired state parameter");
      return;
    }
    pendingStates.delete(state);

    try {
      // Step 1: Exchange code for access token
      const tokenResp = await fetch(
        "https://github.com/login/oauth/access_token",
        {
          method: "POST",
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            client_id: githubOAuthClientId(),
            client_secret: githubOAuthClientSecret(),
            code,
          }),
        }
      );

      if (!tokenResp.ok) {
        throw new Error(`Token exchange failed: ${tokenResp.status}`);
      }

      const tokenData =
        (await tokenResp.json()) as GitHubAccessTokenResponse;
      if (!tokenData.access_token) {
        throw new Error("No access_token in GitHub response");
      }

      // Step 2: Get GitHub user identity
      const ghUser = await fetchGitHubUser(tokenData.access_token);

      // Step 3: Get user's email if not in profile
      let email = ghUser.email;
      if (!email) {
        email = await fetchGitHubPrimaryEmail(tokenData.access_token);
      }
      if (!email) {
        resp.writeHead(302, {
          Location: "/signin?error=no_email",
        });
        resp.end();
        return;
      }

      // Step 4: Find or create OAP user
      const user = await findOrCreateUser({
        githubUserId: ghUser.id,
        githubLogin: ghUser.login,
        email,
        name: ghUser.name || ghUser.login,
        avatarUrl: ghUser.avatar_url,
      });

      // Step 5: Upsert user_identities row
      await db
        .insert(userIdentities)
        .values({
          userId: user.id,
          provider: "github",
          providerUserId: String(ghUser.id),
          providerLogin: ghUser.login,
          providerEmail: email,
          avatarUrl: ghUser.avatar_url,
          accessTokenEnc: tokenData.access_token, // TODO: encrypt at rest (NFR-001)
          refreshTokenEnc: tokenData.refresh_token ?? null,
          tokenExpiresAt: tokenData.expires_in
            ? new Date(Date.now() + tokenData.expires_in * 1000)
            : null,
        })
        .onConflictDoUpdate({
          target: [userIdentities.provider, userIdentities.providerUserId],
          set: {
            providerLogin: ghUser.login,
            providerEmail: email,
            avatarUrl: ghUser.avatar_url,
            accessTokenEnc: tokenData.access_token,
            refreshTokenEnc: tokenData.refresh_token ?? null,
            tokenExpiresAt: tokenData.expires_in
              ? new Date(Date.now() + tokenData.expires_in * 1000)
              : null,
            updatedAt: new Date(),
          },
        });

      // Step 6: Resolve org memberships
      const matchedOrgs = await resolveOrgMemberships(
        tokenData.access_token,
        user.id
      );

      // Step 7: Provision Rauthy user
      let rauthyUserId = user.rauthyUserId;
      if (!rauthyUserId) {
        rauthyUserId = await provisionRauthyUser({
          email,
          githubLogin: ghUser.login,
          name: ghUser.name || ghUser.login,
        });
        await db
          .update(users)
          .set({ rauthyUserId })
          .where(eq(users.id, user.id));
      }

      // Step 8: Update last login
      await db
        .update(users)
        .set({ lastLoginAt: new Date() })
        .where(eq(users.id, user.id));

      // Audit
      await db.insert(auditLog).values({
        actorUserId: user.id,
        action: "user.github_login",
        targetType: "user",
        targetId: user.id,
        metadata: {
          github_login: ghUser.login,
          orgs_matched: matchedOrgs.length,
        },
      });

      // Step 9: Route based on matched orgs
      if (matchedOrgs.length === 0) {
        resp.writeHead(302, {
          Location: "/auth/no-org?login=" + encodeURIComponent(ghUser.login),
        });
        resp.end();
        return;
      }

      if (matchedOrgs.length === 1) {
        // Auto-select the single org — issue session cookie and redirect
        const org = matchedOrgs[0];
        const sessionCookie = buildSessionCookie(
          {
            id: user.id,
            rauthyUserId: user.rauthyUserId ?? rauthyUserId,
            email,
            name: ghUser.name || ghUser.login,
          },
          org
        );
        resp.writeHead(302, {
          Location: "/app",
          "Set-Cookie": sessionCookie,
        });
        resp.end();
        return;
      }

      // Multiple orgs — redirect to org picker with HMAC-signed cookie
      const pendingSigned = signPayload({
        userId: user.id,
        rauthyUserId,
        email,
        name: ghUser.name || ghUser.login,
        githubLogin: ghUser.login,
        orgs: matchedOrgs,
        iat: Math.floor(Date.now() / 1000),
      });

      const secure =
        process.env.NODE_ENV === "production" ? " Secure;" : "";
      resp.writeHead(302, {
        Location: "/auth/org-select",
        "Set-Cookie": `__pending_org=${pendingSigned}; Path=/auth; HttpOnly; SameSite=Lax; Max-Age=300;${secure}`,
      });
      resp.end();
    } catch (err) {
      log.error("GitHub OAuth callback failed", { error: String(err) });
      resp.writeHead(302, { Location: "/signin?error=oauth_failed" });
      resp.end();
    }
  }
);

// ---------------------------------------------------------------------------
// GET /auth/org-select/complete — finalize org selection
// ---------------------------------------------------------------------------

export const orgSelectComplete = api.raw(
  {
    expose: true,
    method: "GET",
    path: "/auth/org-select/complete",
    auth: false,
  },
  async (req, resp) => {
    const url = new URL(req.url!, `http://${req.headers.host}`);
    const selectedOrgId = url.searchParams.get("org");

    if (!selectedOrgId) {
      resp.writeHead(400, { "Content-Type": "text/plain" });
      resp.end("Missing org parameter");
      return;
    }

    // Parse pending org data from cookie
    const cookieHeader = req.headers.cookie || "";
    const match = cookieHeader.match(/(?:^|;\s*)__pending_org=([^\s;]+)/);
    if (!match) {
      resp.writeHead(302, { Location: "/signin?error=session_expired" });
      resp.end();
      return;
    }

    try {
      // Verify HMAC signature on the pending org cookie
      const pendingData = verifyPayload<{
        userId: string;
        rauthyUserId: string;
        email: string;
        name: string;
        githubLogin: string;
        orgs: ResolvedOrg[];
      }>(match[1]);

      if (!pendingData) {
        resp.writeHead(302, { Location: "/signin?error=session_expired" });
        resp.end();
        return;
      }

      const selectedOrg = pendingData.orgs.find(
        (o) => o.orgId === selectedOrgId
      );
      if (!selectedOrg) {
        resp.writeHead(400, { "Content-Type": "text/plain" });
        resp.end("Invalid org selection");
        return;
      }

      const sessionCookie = buildSessionCookie(
        {
          id: pendingData.userId,
          rauthyUserId: pendingData.rauthyUserId,
          email: pendingData.email,
          name: pendingData.name,
        },
        selectedOrg
      );

      const secure =
        process.env.NODE_ENV === "production" ? " Secure;" : "";

      resp.writeHead(302, {
        Location: "/app",
        "Set-Cookie": [
          sessionCookie,
          // Clear the pending org cookie
          `__pending_org=; Path=/auth; HttpOnly; SameSite=Lax; Max-Age=0;${secure}`,
        ],
      });
      resp.end();
    } catch (err) {
      log.error("Org select complete failed", { error: String(err) });
      resp.writeHead(302, { Location: "/signin?error=session_expired" });
      resp.end();
    }
  }
);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function fetchGitHubUser(accessToken: string): Promise<GitHubUser> {
  const resp = await fetch("https://api.github.com/user", {
    headers: {
      Authorization: `Bearer ${accessToken}`,
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
    },
  });

  if (!resp.ok) {
    throw new Error(`GitHub user API failed: ${resp.status}`);
  }

  return (await resp.json()) as GitHubUser;
}

async function fetchGitHubPrimaryEmail(
  accessToken: string
): Promise<string | null> {
  const resp = await fetch("https://api.github.com/user/emails", {
    headers: {
      Authorization: `Bearer ${accessToken}`,
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
    },
  });

  if (!resp.ok) return null;

  const emails = (await resp.json()) as Array<{
    email: string;
    primary: boolean;
    verified: boolean;
  }>;

  const primary = emails.find((e) => e.primary && e.verified);
  return primary?.email ?? emails.find((e) => e.verified)?.email ?? null;
}

async function findOrCreateUser(opts: {
  githubUserId: number;
  githubLogin: string;
  email: string;
  name: string;
  avatarUrl: string;
}): Promise<{ id: string; rauthyUserId: string | null }> {
  // Try to find by GitHub user ID first
  const [byGithubId] = await db
    .select({ id: users.id, rauthyUserId: users.rauthyUserId })
    .from(users)
    .where(eq(users.githubUserId, opts.githubUserId))
    .limit(1);

  if (byGithubId) {
    // Update profile fields
    await db
      .update(users)
      .set({
        githubLogin: opts.githubLogin,
        avatarUrl: opts.avatarUrl,
        name: opts.name,
      })
      .where(eq(users.id, byGithubId.id));
    return byGithubId;
  }

  // Try to find by email (link existing local account to GitHub)
  const [byEmail] = await db
    .select({ id: users.id, rauthyUserId: users.rauthyUserId })
    .from(users)
    .where(eq(users.email, opts.email.toLowerCase()))
    .limit(1);

  if (byEmail) {
    // Link GitHub identity to existing account
    await db
      .update(users)
      .set({
        githubUserId: opts.githubUserId,
        githubLogin: opts.githubLogin,
        avatarUrl: opts.avatarUrl,
      })
      .where(eq(users.id, byEmail.id));
    return byEmail;
  }

  // Create new user (no password — OAuth only)
  const [created] = await db
    .insert(users)
    .values({
      email: opts.email.toLowerCase(),
      name: opts.name,
      githubUserId: opts.githubUserId,
      githubLogin: opts.githubLogin,
      avatarUrl: opts.avatarUrl,
    })
    .returning({ id: users.id, rauthyUserId: users.rauthyUserId });

  return created;
}

/**
 * Build the __session cookie value as HMAC-signed payload.
 * Transitional mechanism — once Rauthy is fully wired, this will be replaced
 * by Rauthy-issued JWTs. The HMAC signature prevents cookie forgery.
 */
function buildSessionCookie(
  user: { id: string; rauthyUserId: string | null; email: string; name: string },
  org: ResolvedOrg
): string {
  const signed = signPayload({
    userId: user.id,
    rauthyUserId: user.rauthyUserId,
    orgId: org.orgId,
    orgSlug: org.orgSlug,
    githubLogin: org.githubOrgLogin,
    platformRole: org.platformRole,
    email: user.email,
    name: user.name,
    iat: Math.floor(Date.now() / 1000),
  });

  const maxAge = 14 * 24 * 60 * 60; // 14 days
  const secure = process.env.NODE_ENV === "production" ? " Secure;" : "";
  return `__session=${signed}; Path=/; HttpOnly; SameSite=Lax; Max-Age=${maxAge};${secure}`;
}
