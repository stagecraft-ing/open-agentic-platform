/**
 * Org membership resolution (spec 080 FR-005).
 *
 * Resolves user's GitHub org memberships against active GitHub App installations,
 * upserts org_memberships, and marks stale memberships.
 */

import log from "encore.dev/log";
import { db } from "../db/drizzle";
import {
  githubInstallations,
  orgMemberships,
  organizations,
  workspaces,
} from "../db/schema";
import { eq, and, notInArray, inArray } from "drizzle-orm";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface GitHubOrg {
  id: number;
  login: string;
  role: string; // "admin" | "member"
}

export interface ResolvedOrg {
  orgId: string;
  orgSlug: string;
  workspaceId: string;
  githubOrgLogin: string;
  platformRole: "owner" | "admin" | "member";
}

// ---------------------------------------------------------------------------
// GitHub API helpers
// ---------------------------------------------------------------------------

/**
 * Fetch the user's GitHub org memberships using their OAuth access token.
 */
export async function fetchUserGitHubOrgs(
  accessToken: string
): Promise<GitHubOrg[]> {
  const orgs: GitHubOrg[] = [];
  let page = 1;

  while (true) {
    const resp = await fetch(
      `https://api.github.com/user/memberships/orgs?state=active&per_page=100&page=${page}`,
      {
        headers: {
          Authorization: `Bearer ${accessToken}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
        },
      }
    );

    if (!resp.ok) {
      // read:org scope might not be granted — log and return what we have
      if (resp.status === 403 || resp.status === 404) {
        log.warn("GitHub org membership API returned non-OK", {
          status: resp.status,
        });
        break;
      }
      throw new Error(`GitHub orgs API failed: ${resp.status}`);
    }

    const data = (await resp.json()) as Array<{
      organization: { id: number; login: string };
      role: string;
    }>;
    if (data.length === 0) break;

    for (const m of data) {
      orgs.push({
        id: m.organization.id,
        login: m.organization.login,
        role: m.role,
      });
    }

    if (data.length < 100) break;
    page++;
  }

  return orgs;
}

// ---------------------------------------------------------------------------
// Membership resolution
// ---------------------------------------------------------------------------

/**
 * Resolve org memberships for a user after GitHub login.
 *
 * 1. Get user's GitHub org memberships
 * 2. Match against active GitHub App installations
 * 3. Upsert org_memberships for matches
 * 4. Mark stale memberships (user left org)
 */
export async function resolveOrgMemberships(
  githubAccessToken: string,
  userId: string
): Promise<ResolvedOrg[]> {
  // 1. Fetch user's GitHub org memberships
  const ghOrgs = await fetchUserGitHubOrgs(githubAccessToken);
  log.info("Fetched GitHub org memberships", {
    userId,
    orgCount: ghOrgs.length,
  });

  // 2. Match against active installations in a single join query
  const ghOrgIds = ghOrgs.map((o) => o.id);
  const matchedRows =
    ghOrgIds.length > 0
      ? await db
          .select({
            installGithubOrgId: githubInstallations.githubOrgId,
            orgId: organizations.id,
            orgSlug: organizations.slug,
          })
          .from(githubInstallations)
          .innerJoin(
            organizations,
            eq(organizations.id, githubInstallations.orgId)
          )
          .where(
            and(
              eq(githubInstallations.installationState, "active"),
              inArray(githubInstallations.githubOrgId, ghOrgIds)
            )
          )
      : [];

  const matchedOrgs: ResolvedOrg[] = [];
  const matchedOrgIds: string[] = [];

  // 3. Upsert org_memberships for each match and resolve default workspace
  for (const row of matchedRows) {
    const ghOrg = ghOrgs.find((o) => o.id === row.installGithubOrgId);
    if (!ghOrg) continue;

    await db
      .insert(orgMemberships)
      .values({
        userId,
        orgId: row.orgId,
        source: "github",
        githubRole: ghOrg.role,
        platformRole: "member", // default; elevated by OAP policy
        status: "active",
        syncedAt: new Date(),
      })
      .onConflictDoUpdate({
        target: [orgMemberships.userId, orgMemberships.orgId],
        set: {
          githubRole: ghOrg.role,
          syncedAt: new Date(),
          status: "active",
          updatedAt: new Date(),
        },
      });

    // Resolve the default workspace for this org
    const [ws] = await db
      .select({ id: workspaces.id })
      .from(workspaces)
      .where(
        and(eq(workspaces.orgId, row.orgId), eq(workspaces.slug, "default"))
      )
      .limit(1);

    matchedOrgs.push({
      orgId: row.orgId,
      orgSlug: row.orgSlug,
      workspaceId: ws?.id ?? "",
      githubOrgLogin: ghOrg.login,
      platformRole: "member",
    });
    matchedOrgIds.push(row.orgId);
  }

  // 4. Mark stale memberships (user no longer in these orgs via GitHub)
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
    // No matches — mark all GitHub-sourced memberships as removed
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

  log.info("Org membership resolution complete", {
    userId,
    matched: matchedOrgs.length,
    ghOrgsTotal: ghOrgs.length,
  });

  return matchedOrgs;
}

// ---------------------------------------------------------------------------
// Org-level permissions (spec 080 FR-007)
// ---------------------------------------------------------------------------

export type OrgPermission =
  | "project:create"
  | "project:delete"
  | "org:manage_members"
  | "org:manage_policies"
  | "org:manage_billing"
  | "factory:init"
  | "factory:confirm"
  | "deploy:production";

const ORG_PERMISSION_MAP: Record<OrgPermission, Set<"owner" | "admin" | "member">> = {
  "project:create": new Set(["owner", "admin", "member"]),
  "project:delete": new Set(["owner", "admin"]),
  "org:manage_members": new Set(["owner", "admin"]),
  "org:manage_policies": new Set(["owner"]),
  "org:manage_billing": new Set(["owner"]),
  "factory:init": new Set(["owner", "admin", "member"]),
  "factory:confirm": new Set(["owner", "admin"]),
  "deploy:production": new Set(["owner"]),
};

/**
 * Check whether a platform role has a specific org-level permission.
 * Uses the default role-permission mapping from spec 080 FR-007.
 */
export function hasOrgPermission(
  platformRole: "owner" | "admin" | "member",
  permission: OrgPermission
): boolean {
  return ORG_PERMISSION_MAP[permission]?.has(platformRole) ?? false;
}

// ---------------------------------------------------------------------------
// Role lookups
// ---------------------------------------------------------------------------

/**
 * Get the platform role for a user in a specific org.
 * Reads from org_memberships (the resolved state).
 */
export async function getUserOrgRole(
  userId: string,
  orgId: string
): Promise<"owner" | "admin" | "member" | null> {
  const [row] = await db
    .select({ platformRole: orgMemberships.platformRole })
    .from(orgMemberships)
    .where(
      and(
        eq(orgMemberships.userId, userId),
        eq(orgMemberships.orgId, orgId),
        eq(orgMemberships.status, "active")
      )
    )
    .limit(1);

  return row?.platformRole ?? null;
}
