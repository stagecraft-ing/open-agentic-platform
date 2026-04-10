/**
 * GitHub OAuth login flow (spec 080 FR-002, hardened in spec 087 Phase 5).
 *
 * GET  /auth/github          — redirect to GitHub OAuth authorize
 * GET  /auth/github/callback  — exchange code, resolve membership, issue Rauthy JWT
 *
 * Phase 5: HMAC-signed session cookies replaced by Rauthy-issued JWTs.
 * Rauthy provisioning is now required (not best-effort).
 */

import { api } from "encore.dev/api";
import { secret } from "encore.dev/config";
import log from "encore.dev/log";
import crypto from "crypto";
import { db } from "../db/drizzle";
import { users, userIdentities, auditLog } from "../db/schema";
import { eq } from "drizzle-orm";
import { resolveOrgMemberships, type ResolvedOrg } from "./membership";
import { provisionRauthyUser, issueRauthySession } from "./rauthy";

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
// Pending org data store (replaces HMAC-signed __pending_org cookie)
// ---------------------------------------------------------------------------

interface PendingOrgData {
  userId: string;
  rauthyUserId: string;
  email: string;
  name: string;
  githubLogin: string;
  orgs: ResolvedOrg[];
  createdAt: number;
}

const pendingOrgSelections = new Map<string, PendingOrgData>();
const PENDING_ORG_TTL_MS = 5 * 60 * 1000; // 5 minutes

function cleanupStalePending() {
  const cutoff = Date.now() - PENDING_ORG_TTL_MS;
  for (const [key, val] of pendingOrgSelections) {
    if (val.createdAt < cutoff) pendingOrgSelections.delete(key);
  }
}

// ---------------------------------------------------------------------------
// GET /auth/pending-orgs — resolve pending org data for org-select page
// ---------------------------------------------------------------------------

export const getPendingOrgs = api.raw(
  { expose: true, method: "GET", path: "/auth/pending-orgs", auth: false },
  async (req, resp) => {
    const cookieHeader = req.headers.cookie || "";
    const match = cookieHeader.match(/(?:^|;\s*)__pending_org=([^\s;]+)/);

    if (!match) {
      resp.writeHead(404, { "Content-Type": "application/json" });
      resp.end(JSON.stringify({ error: "no pending org selection" }));
      return;
    }

    const data = pendingOrgSelections.get(match[1]);
    if (!data) {
      resp.writeHead(404, { "Content-Type": "application/json" });
      resp.end(JSON.stringify({ error: "pending org selection expired" }));
      return;
    }

    resp.writeHead(200, { "Content-Type": "application/json" });
    resp.end(
      JSON.stringify({
        githubLogin: data.githubLogin,
        orgs: data.orgs.map((o) => ({
          orgId: o.orgId,
          orgSlug: o.orgSlug,
          githubOrgLogin: o.githubOrgLogin,
          platformRole: o.platformRole,
        })),
      })
    );
  }
);

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

    // Stage 1: Exchange code for access token
    let tokenData: GitHubAccessTokenResponse;
    try {
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
        throw new Error(`Token exchange HTTP ${tokenResp.status}`);
      }

      tokenData = (await tokenResp.json()) as GitHubAccessTokenResponse;
      if (!tokenData.access_token) {
        throw new Error("No access_token in GitHub response");
      }
    } catch (err) {
      log.error("GitHub OAuth token exchange failed", { error: String(err) });
      resp.writeHead(302, { Location: "/signin?error=token_failed" });
      resp.end();
      return;
    }

    // Stage 2: Get GitHub user identity and email
    let ghUser: GitHubUser;
    let email: string;
    try {
      ghUser = await fetchGitHubUser(tokenData.access_token);

      let resolvedEmail = ghUser.email;
      if (!resolvedEmail) {
        resolvedEmail = await fetchGitHubPrimaryEmail(tokenData.access_token);
      }
      if (!resolvedEmail) {
        resp.writeHead(302, { Location: "/signin?error=no_email" });
        resp.end();
        return;
      }
      email = resolvedEmail;
    } catch (err) {
      log.error("GitHub API call failed", { error: String(err) });
      resp.writeHead(302, { Location: "/signin?error=github_api_failed" });
      resp.end();
      return;
    }

    // Stage 3: Find or create OAP user and upsert identity
    let user: { id: string; rauthyUserId: string | null };
    try {
      user = await findOrCreateUser({
        githubUserId: ghUser.id,
        githubLogin: ghUser.login,
        email,
        name: ghUser.name || ghUser.login,
        avatarUrl: ghUser.avatar_url,
      });

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
    } catch (err) {
      log.error("Account creation/linking failed", { error: String(err) });
      resp.writeHead(302, { Location: "/signin?error=account_error" });
      resp.end();
      return;
    }

    // Stage 4: Resolve org memberships
    let matchedOrgs: ResolvedOrg[];
    try {
      matchedOrgs = await resolveOrgMemberships(
        tokenData.access_token,
        user.id
      );
    } catch (err) {
      log.error("Org membership resolution failed", { error: String(err) });
      resp.writeHead(302, { Location: "/signin?error=membership_failed" });
      resp.end();
      return;
    }

    // Stage 5: Provision Rauthy user (required — Phase 5)
    let rauthyUserId = user.rauthyUserId;
    try {
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
    } catch (err) {
      log.error("Rauthy provisioning failed", { error: String(err) });
      resp.writeHead(302, { Location: "/signin?error=rauthy_unavailable" });
      resp.end();
      return;
    }

    // Stage 6: Finalize — update last login, audit, route by org count
    try {
      await db
        .update(users)
        .set({ lastLoginAt: new Date() })
        .where(eq(users.id, user.id));

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
    } catch (err) {
      // Audit/login-timestamp failures are non-fatal — log and continue
      log.warn("Post-login bookkeeping failed (non-fatal)", { error: String(err) });
    }

    // Route based on matched orgs
    try {
      if (matchedOrgs.length === 0) {
        resp.writeHead(302, {
          Location: "/auth/no-org?login=" + encodeURIComponent(ghUser.login),
        });
        resp.end();
        return;
      }

      if (matchedOrgs.length === 1) {
        const org = matchedOrgs[0];
        const sessionCookie = await buildRauthySessionCookie(
          {
            id: user.id,
            rauthyUserId: rauthyUserId!,
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

      // Multiple orgs — store pending data server-side and redirect to picker
      cleanupStalePending();
      const pendingId = crypto.randomBytes(32).toString("base64url");
      pendingOrgSelections.set(pendingId, {
        userId: user.id,
        rauthyUserId: rauthyUserId!,
        email,
        name: ghUser.name || ghUser.login,
        githubLogin: ghUser.login,
        orgs: matchedOrgs,
        createdAt: Date.now(),
      });

      const secure =
        process.env.NODE_ENV === "production" ? " Secure;" : "";
      resp.writeHead(302, {
        Location: "/auth/org-select",
        "Set-Cookie": `__pending_org=${pendingId}; Path=/auth; HttpOnly; SameSite=Lax; Max-Age=300;${secure}`,
      });
      resp.end();
    } catch (err) {
      log.error("Session creation failed", { error: String(err) });
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

    // Parse pending org ID from cookie
    const cookieHeader = req.headers.cookie || "";
    const match = cookieHeader.match(/(?:^|;\s*)__pending_org=([^\s;]+)/);
    if (!match) {
      resp.writeHead(302, { Location: "/signin?error=session_expired" });
      resp.end();
      return;
    }

    try {
      const pendingId = match[1];
      const pendingData = pendingOrgSelections.get(pendingId);

      if (!pendingData) {
        resp.writeHead(302, { Location: "/signin?error=session_expired" });
        resp.end();
        return;
      }

      // Clean up the pending data
      pendingOrgSelections.delete(pendingId);

      const selectedOrg = pendingData.orgs.find(
        (o) => o.orgId === selectedOrgId
      );
      if (!selectedOrg) {
        resp.writeHead(400, { "Content-Type": "text/plain" });
        resp.end("Invalid org selection");
        return;
      }

      const sessionCookie = await buildRauthySessionCookie(
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
 * Issue a Rauthy JWT and build the __session cookie (spec 087 Phase 5).
 *
 * Replaces the transitional HMAC-signed session cookie with a proper
 * Rauthy-issued JWT that can be validated by the auth handler.
 */
async function buildRauthySessionCookie(
  user: { id: string; rauthyUserId: string; email: string; name: string },
  org: ResolvedOrg
): Promise<string> {
  const { accessToken, expiresIn } = await issueRauthySession({
    rauthyUserId: user.rauthyUserId,
    oapUserId: user.id,
    orgId: org.orgId,
    orgSlug: org.orgSlug,
    workspaceId: org.workspaceId,
    githubLogin: org.githubOrgLogin,
    platformRole: org.platformRole,
  });

  const maxAge = Math.min(expiresIn, 14 * 24 * 60 * 60); // cap at 14 days
  const secure = process.env.NODE_ENV === "production" ? " Secure;" : "";
  return `__session=${accessToken}; Path=/; HttpOnly; SameSite=Lax; Max-Age=${maxAge};${secure}`;
}
