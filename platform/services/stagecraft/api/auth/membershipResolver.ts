/**
 * Layered membership resolver (spec 106 FR-005).
 *
 * Replaces the user-OAuth-token `resolveOrgMemberships` used in spec 080, which
 * violated spec 080 Principle 4 ("server-side membership resolution uses the
 * App installation token"). Here we walk two strategies in order:
 *
 *   1. appInstallationStrategy — iterate every active `github_installations`
 *      row, fetch an installation access token via `signAppJwt`, and ask
 *      GitHub whether `githubLogin` is a member of each installed org.
 *      Requires the stagecraft GitHub App manifest to grant
 *      `Organization permissions: Members: Read`.
 *
 *   2. userPatStrategy — if (and only if) the installation strategy returned
 *      zero matches, look up the active row in `user_github_pats` and use
 *      the decrypted PAT to call `/user/orgs` and
 *      `/orgs/{org}/memberships/{login}` for any org that also has an active
 *      installation. This is the operator-documented escape hatch for orgs
 *      that refuse to install the stagecraft GitHub App.
 *
 * The resolver never falls through silently: if both strategies return empty,
 * it surfaces whichever signal is most actionable (`needs_pat`,
 * `pat_invalid`, `pat_saml_not_authorized`, etc.) so the callback can redirect
 * to `/auth/no-org` with a specific error code.
 *
 * It also keeps the spec 080 side-effects intact: on successful matches we
 * upsert `org_memberships` rows and mark stale ones removed, so the database
 * reflects the latest resolved state.
 */

import log from "encore.dev/log";
import { db } from "../db/drizzle";
import {
  githubInstallations,
  orgMemberships,
  organizations,
  userGithubPats,
} from "../db/schema";
import { and, eq, isNull, notInArray } from "drizzle-orm";
import { signAppJwt } from "../github/appJwt";
import { decryptPat } from "./patCrypto";
import type { ResolvedOrg } from "./membership";
import { errorForLog } from "./errorLog";

// ---------------------------------------------------------------------------
// Result shape
// ---------------------------------------------------------------------------

/**
 * Machine-readable reason codes. The callback maps these to URL error codes
 * (`/auth/no-org?error=pat_required`, etc.) and to user-facing messages in
 * the web + OPC shells.
 */
export type MembershipReason =
  | "ok"
  | "no_installed_orgs"
  | "pat_required"
  | "pat_invalid"
  | "pat_saml_not_authorized"
  | "pat_rate_limited"
  | "membership_api_failed";

export interface MembershipResult {
  orgs: ResolvedOrg[];
  reason: MembershipReason;
  // When reason is pat_saml_not_authorized, the org that needs SSO
  samlOrg?: string;
}

interface InstallationInfo {
  installationId: number;
  githubOrgId: number;
  githubOrgLogin: string;
  orgId: string;
  orgSlug: string;
}

interface MembershipMatch {
  orgId: string;
  orgSlug: string;
  githubOrgLogin: string;
  role: "admin" | "member";
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Resolve the orgs the given GitHub user belongs to, per spec 106 FR-005.
 *
 * Also upserts `org_memberships` rows (active for matches, removed for stale
 * GitHub-sourced rows), so subsequent logins / JWT refreshes see the same
 * resolved state.
 */
export async function resolveMembership(
  githubLogin: string,
  userId: string
): Promise<MembershipResult> {
  if (!githubLogin) {
    return { orgs: [], reason: "no_installed_orgs" };
  }

  const installations = await loadActiveInstallations();
  if (installations.length === 0) {
    log.info("No active GitHub App installations", { userId });
    return afterResolution(userId, [], { orgs: [], reason: "no_installed_orgs" });
  }

  // Strategy 1: App installation token
  const installResult = await appInstallationStrategy(githubLogin, installations);

  if (installResult.matches.length > 0) {
    const orgs = await buildResolvedOrgs(installResult.matches);
    return afterResolution(userId, installResult.matches.map((m) => m.orgId), {
      orgs,
      reason: "ok",
    });
  }

  // Strategy 2: User PAT
  const patResult = await userPatStrategy(githubLogin, userId, installations);

  if (patResult.matches.length > 0) {
    const orgs = await buildResolvedOrgs(patResult.matches);
    return afterResolution(userId, patResult.matches.map((m) => m.orgId), {
      orgs,
      reason: "ok",
    });
  }

  // Both strategies returned empty — surface the most actionable reason.
  const reason: MembershipReason =
    patResult.reason !== "ok" ? patResult.reason : installResult.reason;

  return afterResolution(userId, [], {
    orgs: [],
    reason,
    samlOrg: patResult.samlOrg,
  });
}

// ---------------------------------------------------------------------------
// Strategy 1: GitHub App installation token
// ---------------------------------------------------------------------------

interface StrategyResult {
  matches: MembershipMatch[];
  reason: MembershipReason;
  samlOrg?: string;
}

async function appInstallationStrategy(
  githubLogin: string,
  installations: InstallationInfo[]
): Promise<StrategyResult> {
  const matches: MembershipMatch[] = [];
  let anyApiFailure = false;

  for (const inst of installations) {
    try {
      const token = await fetchInstallationToken(inst.installationId);
      const match = await fetchOrgMembership(token, inst.githubOrgLogin, githubLogin);
      if (match) {
        matches.push({
          orgId: inst.orgId,
          orgSlug: inst.orgSlug,
          githubOrgLogin: inst.githubOrgLogin,
          role: match.role,
        });
      }
    } catch (err) {
      anyApiFailure = true;
      log.warn("App-installation membership probe failed", {
        orgLogin: inst.githubOrgLogin,
        error: errorForLog(err),
      });
    }
  }

  if (matches.length > 0) {
    return { matches, reason: "ok" };
  }
  return {
    matches: [],
    reason: anyApiFailure ? "membership_api_failed" : "no_installed_orgs",
  };
}

/** Fetch a scoped installation access token for the given installation ID. */
async function fetchInstallationToken(installationId: number): Promise<string> {
  const jwt = await signAppJwt();
  const resp = await fetch(
    `https://api.github.com/app/installations/${installationId}/access_tokens`,
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${jwt}`,
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
      },
      // Default permissions from the app manifest are sufficient —
      // spec 106 FR-005 Q1 confirmed members:read is granted.
      body: JSON.stringify({}),
    }
  );

  if (!resp.ok) {
    const body = await resp.text();
    throw new Error(
      `GitHub installation token exchange failed: ${resp.status} ${body.slice(0, 300)}`
    );
  }

  const data = (await resp.json()) as { token: string };
  return data.token;
}

/**
 * Query `GET /orgs/{org}/memberships/{username}` using an installation token.
 * Returns the role on 200, null on 404, and throws on other failures.
 */
async function fetchOrgMembership(
  token: string,
  orgLogin: string,
  username: string
): Promise<{ role: "admin" | "member" } | null> {
  const resp = await fetch(
    `https://api.github.com/orgs/${encodeURIComponent(orgLogin)}/memberships/${encodeURIComponent(username)}`,
    {
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
      },
    }
  );

  if (resp.status === 200) {
    const data = (await resp.json()) as { role?: string; state?: string };
    if (data.state !== "active") return null;
    const role = data.role === "admin" ? "admin" : "member";
    return { role };
  }

  if (resp.status === 404) return null;

  // 403 (forbidden, e.g. missing permission) / other codes — treat as
  // non-match but surface as a probe failure upstream.
  throw new Error(`GitHub membership probe returned ${resp.status}`);
}

// ---------------------------------------------------------------------------
// Strategy 2: User PAT
// ---------------------------------------------------------------------------

async function userPatStrategy(
  githubLogin: string,
  userId: string,
  installations: InstallationInfo[]
): Promise<StrategyResult> {
  const [patRow] = await db
    .select()
    .from(userGithubPats)
    .where(and(eq(userGithubPats.userId, userId), isNull(userGithubPats.revokedAt)))
    .limit(1);

  if (!patRow) {
    return { matches: [], reason: "pat_required" };
  }

  let pat: string;
  try {
    pat = decryptPat(patRow.tokenEnc, patRow.tokenNonce);
  } catch (err) {
    log.error(
      "PAT decryption failed — check PAT_ENCRYPTION_KEY secret and stored ciphertext",
      { userId, error: errorForLog(err) }
    );
    return { matches: [], reason: "pat_invalid" };
  }

  // Touch last_used_at so settings UI shows the most recent use.
  await db
    .update(userGithubPats)
    .set({ lastUsedAt: new Date() })
    .where(eq(userGithubPats.id, patRow.id));

  const matches: MembershipMatch[] = [];
  let samlOrg: string | undefined;

  for (const inst of installations) {
    const resp = await fetch(
      `https://api.github.com/orgs/${encodeURIComponent(inst.githubOrgLogin)}/memberships/${encodeURIComponent(githubLogin)}`,
      {
        headers: {
          Authorization: `Bearer ${pat}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
        },
      }
    );

    if (resp.status === 200) {
      const data = (await resp.json()) as { role?: string; state?: string };
      if (data.state === "active") {
        matches.push({
          orgId: inst.orgId,
          orgSlug: inst.orgSlug,
          githubOrgLogin: inst.githubOrgLogin,
          role: data.role === "admin" ? "admin" : "member",
        });
      }
      continue;
    }

    if (resp.status === 401) {
      // PAT revoked or invalid — clear the row and abort.
      await db
        .update(userGithubPats)
        .set({ revokedAt: new Date() })
        .where(eq(userGithubPats.id, patRow.id));
      return { matches: [], reason: "pat_invalid" };
    }

    if (resp.status === 403) {
      const body = await resp.text();
      if (/saml/i.test(body)) {
        samlOrg = inst.githubOrgLogin;
      }
      continue;
    }

    if (resp.status === 404) continue;

    if (resp.status === 429) {
      return { matches: [], reason: "pat_rate_limited" };
    }

    log.warn("PAT membership probe returned unexpected status", {
      userId,
      orgLogin: inst.githubOrgLogin,
      status: resp.status,
    });
  }

  if (matches.length > 0) return { matches, reason: "ok" };
  if (samlOrg) return { matches: [], reason: "pat_saml_not_authorized", samlOrg };
  return { matches: [], reason: "no_installed_orgs" };
}

// ---------------------------------------------------------------------------
// Helpers: installations, DB upserts, ResolvedOrg build
// ---------------------------------------------------------------------------

async function loadActiveInstallations(): Promise<InstallationInfo[]> {
  const rows = await db
    .select({
      installationId: githubInstallations.installationId,
      githubOrgId: githubInstallations.githubOrgId,
      githubOrgLogin: githubInstallations.githubOrgLogin,
      orgId: organizations.id,
      orgSlug: organizations.slug,
    })
    .from(githubInstallations)
    .innerJoin(organizations, eq(organizations.id, githubInstallations.orgId))
    .where(eq(githubInstallations.installationState, "active"));

  return rows.map((r) => ({
    installationId: r.installationId,
    githubOrgId: r.githubOrgId,
    githubOrgLogin: r.githubOrgLogin,
    orgId: r.orgId,
    orgSlug: r.orgSlug,
  }));
}

async function buildResolvedOrgs(matches: MembershipMatch[]): Promise<ResolvedOrg[]> {
  return matches.map((m) => ({
    orgId: m.orgId,
    orgSlug: m.orgSlug,
    githubOrgLogin: m.githubOrgLogin,
    orgDisplayName: m.githubOrgLogin || m.orgSlug,
    platformRole: "member",
  }));
}

/**
 * Persist `org_memberships` side-effects after a resolver run, mirroring the
 * behavior the spec 080 resolver had.
 */
async function afterResolution(
  userId: string,
  matchedOrgIds: string[],
  result: MembershipResult
): Promise<MembershipResult> {
  // Upsert active rows for every match.
  for (const orgId of matchedOrgIds) {
    await db
      .insert(orgMemberships)
      .values({
        userId,
        orgId,
        source: "github",
        platformRole: "member",
        status: "active",
        syncedAt: new Date(),
      })
      .onConflictDoUpdate({
        target: [orgMemberships.userId, orgMemberships.orgId],
        set: {
          status: "active",
          syncedAt: new Date(),
          updatedAt: new Date(),
        },
      });
  }

  // Mark stale GitHub-sourced rows removed.
  if (matchedOrgIds.length > 0) {
    await db
      .update(orgMemberships)
      .set({ status: "removed", updatedAt: new Date() })
      .where(
        and(
          eq(orgMemberships.userId, userId),
          eq(orgMemberships.source, "github"),
          eq(orgMemberships.status, "active"),
          notInArray(orgMemberships.orgId, matchedOrgIds)
        )
      );
  } else {
    await db
      .update(orgMemberships)
      .set({ status: "removed", updatedAt: new Date() })
      .where(
        and(
          eq(orgMemberships.userId, userId),
          eq(orgMemberships.source, "github"),
          eq(orgMemberships.status, "active")
        )
      );
  }

  return result;
}
