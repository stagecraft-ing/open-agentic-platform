import { api } from "encore.dev/api";
import { secret } from "encore.dev/config";
import log from "encore.dev/log";
import { createHmac, timingSafeEqual } from "crypto";
import { db } from "../db/drizzle";
import {
  projectRepos,
  organizations,
  projects,
  environments,
  githubInstallations,
  orgMemberships,
  users,
  workspaces,
  auditLog,
} from "../db/schema";
import { eq, and } from "drizzle-orm";
import {
  createPreviewDeployment,
  destroyPreviewDeployment,
  isDeploydConfigured,
} from "../deploy/deploydClient";
import { revokeOrgMembership } from "./teamSync";

const webhookSecret = secret("GITHUB_WEBHOOK_SECRET");

// POST /api/github/webhook — receive GitHub webhook events
export const handleWebhook = api.raw(
  { expose: true, method: "POST", path: "/api/github/webhook", auth: false },
  async (req, resp) => {
    // Read raw body (required for HMAC verification over exact bytes)
    const chunks: Buffer[] = [];
    for await (const chunk of req) {
      chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
    }
    const body = Buffer.concat(chunks).toString("utf-8");

    // Verify HMAC-SHA256 signature
    const signature = req.headers["x-hub-signature-256"] as string | undefined;
    if (!verifySignature(body, signature)) {
      resp.writeHead(401, { "Content-Type": "text/plain" });
      resp.end("Invalid signature");
      return;
    }

    const event = req.headers["x-github-event"] as string | undefined;
    const delivery = req.headers["x-github-delivery"] as string | undefined;

    if (!event) {
      resp.writeHead(400, { "Content-Type": "text/plain" });
      resp.end("Missing x-github-event header");
      return;
    }

    let payload: unknown;
    try {
      payload = JSON.parse(body);
    } catch {
      resp.writeHead(400, { "Content-Type": "text/plain" });
      resp.end("Invalid JSON body");
      return;
    }

    log.info("GitHub webhook received", {
      event,
      delivery: delivery ?? "unknown",
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      action: (payload as any)?.action ?? null,
    });

    try {
      await dispatchEvent(event, payload);
      resp.writeHead(200, { "Content-Type": "text/plain" });
      resp.end("ok");
    } catch (e) {
      log.error("Webhook handler failed", { event, error: String(e) });
      resp.writeHead(500, { "Content-Type": "text/plain" });
      resp.end("Internal error");
    }
  }
);

function verifySignature(body: string, signature: string | undefined): boolean {
  if (!signature) return false;
  const sec = webhookSecret();
  const expected =
    "sha256=" + createHmac("sha256", sec).update(body).digest("hex");

  // Constant-time comparison to prevent timing attacks
  try {
    return timingSafeEqual(Buffer.from(signature), Buffer.from(expected));
  } catch {
    // Buffers differ in length — signature is invalid
    return false;
  }
}

async function dispatchEvent(event: string, payload: unknown): Promise<void> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const p = payload as any;

  switch (event) {
    case "installation":
      await handleInstallationEvent(p);
      break;

    case "repository":
      if (p.action === "created") {
        log.info("New repository created via webhook", {
          repo: p.repository.full_name,
        });
        // Auto-register: find a project whose org matches this GitHub org
        const [project] = await db
          .select({ id: projects.id })
          .from(projects)
          .innerJoin(organizations, eq(organizations.id, projects.orgId))
          .where(eq(organizations.slug, p.repository.owner.login))
          .limit(1);

        if (project) {
          const [owner, repoName] = p.repository.full_name.split("/");
          await db
            .insert(projectRepos)
            .values({
              projectId: project.id,
              githubOrg: owner,
              repoName,
              defaultBranch: p.repository.default_branch ?? "main",
              isPrimary: false,
              githubInstallId: p.installation?.id ?? null,
            })
            .onConflictDoNothing();
          log.info("Auto-registered repo", { repo: p.repository.full_name, projectId: project.id });
        }
      }
      break;

    case "pull_request":
      if (p.action === "opened" || p.action === "synchronize") {
        log.info("PR preview deploy trigger", {
          repo: p.repository.full_name,
          pr: p.number,
          sha: p.pull_request.head.sha,
        });
        if (isDeploydConfigured()) {
          const repoRow = await findRepoRow(p.repository.full_name);
          if (repoRow) {
            const previewEnv = await findOrCreatePreviewEnv(repoRow.projectId, p.number);
            try {
              const result = await createPreviewDeployment({
                tenant_id: "default",
                app_id: repoRow.projectId,
                env_id: previewEnv.id,
                release_sha: p.pull_request.head.sha,
                artifact_ref: `ghcr.io/${p.repository.full_name}:pr-${p.number}`,
                lane: "LANE_A",
              });
              log.info("Preview deploy triggered", { releaseId: result.release_id, pr: p.number });
            } catch (err) {
              log.error("Preview deploy failed", { error: String(err), pr: p.number });
            }
          }
        }
      } else if (p.action === "closed") {
        log.info("PR preview destroy trigger", {
          repo: p.repository.full_name,
          pr: p.number,
        });
        if (isDeploydConfigured()) {
          const repoRow = await findRepoRow(p.repository.full_name);
          if (repoRow) {
            try {
              await destroyPreviewDeployment(`preview-${repoRow.projectId}-pr-${p.number}`);
              log.info("Preview destroy triggered", { pr: p.number });
            } catch (err) {
              log.error("Preview destroy failed", { error: String(err), pr: p.number });
            }
          }
        }
      }
      break;

    case "push":
      if (
        p.ref === `refs/heads/${p.repository.default_branch}` &&
        p.after !== "0000000000000000000000000000000000000000"
      ) {
        log.info("Push to default branch", {
          repo: p.repository.full_name,
          sha: p.after,
        });
        const pushRepo = await findRepoRow(p.repository.full_name);
        if (pushRepo) {
          log.info("Governance check triggered", {
            repo: p.repository.full_name,
            sha: p.after,
            projectId: pushRepo.projectId,
          });
          // Governance checks run adapter invariants against the pushed code.
          // The actual check runner is crates/orchestrator/src/verify.rs invoked
          // via the factory pipeline. This emits the audit event for the trigger.
        }
      }
      break;

    case "organization":
      await handleOrganizationEvent(p);
      break;

    default:
      log.debug("Unhandled webhook event", { event });
  }
}

// ---------------------------------------------------------------------------
// FR-001: GitHub App Installation Handling (spec 080)
// ---------------------------------------------------------------------------

const SYSTEM_USER_ID = "00000000-0000-0000-0000-000000000000";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function handleInstallationEvent(p: any): Promise<void> {
  const installId: number = p.installation.id;
  const githubOrgId: number = p.installation.account.id;
  const githubOrgLogin: string = p.installation.account.login;
  const installedBy: string | undefined = p.sender?.login;

  if (p.action === "created") {
    log.info("GitHub App installed", {
      installation_id: installId,
      account: githubOrgLogin,
    });

    // Determine allowed repos
    const allowedRepos = Array.isArray(p.repositories)
      ? (p.repositories as Array<{ name: string }>).map((r) => r.name).join(",")
      : "all";

    // Create or link OAP org
    let orgId: string;
    const [existingOrg] = await db
      .select({ id: organizations.id })
      .from(organizations)
      .where(eq(organizations.githubOrgId, githubOrgId))
      .limit(1);

    if (existingOrg) {
      orgId = existingOrg.id;
      await db
        .update(organizations)
        .set({
          githubOrgLogin,
          githubInstallationId: installId,
          updatedAt: new Date(),
        })
        .where(eq(organizations.id, orgId));
    } else {
      const [created] = await db
        .insert(organizations)
        .values({
          name: githubOrgLogin,
          slug: githubOrgLogin.toLowerCase(),
          githubOrgId,
          githubOrgLogin,
          githubInstallationId: installId,
        })
        .onConflictDoNothing()
        .returning({ id: organizations.id });

      if (created) {
        orgId = created.id;
      } else {
        // Slug conflict — find by slug
        const [bySlug] = await db
          .select({ id: organizations.id })
          .from(organizations)
          .where(eq(organizations.slug, githubOrgLogin.toLowerCase()))
          .limit(1);
        orgId = bySlug!.id;
        await db
          .update(organizations)
          .set({
            githubOrgId,
            githubOrgLogin,
            githubInstallationId: installId,
            updatedAt: new Date(),
          })
          .where(eq(organizations.id, orgId));
      }
    }

    // Ensure a default workspace exists for the org (spec 080 — resolveOrgMemberships needs it)
    await db
      .insert(workspaces)
      .values({
        orgId,
        name: "Default",
        slug: "default",
        objectStoreBucket: `oap-${githubOrgLogin.toLowerCase()}-default`,
      })
      .onConflictDoNothing(); // unique(org_id, slug)

    // Upsert github_installations row
    await db
      .insert(githubInstallations)
      .values({
        githubOrgId,
        githubOrgLogin,
        installationId: installId,
        installationState: "active",
        allowedRepos,
        orgId,
        installedBy,
      })
      .onConflictDoUpdate({
        target: githubInstallations.installationId,
        set: {
          installationState: "active",
          allowedRepos,
          orgId,
          installedBy,
          updatedAt: new Date(),
        },
      });

    // Also persist installation_id to matching repos (preserve existing behavior)
    if (Array.isArray(p.repositories)) {
      for (const r of p.repositories as Array<{ name: string }>) {
        await db
          .update(projectRepos)
          .set({ githubInstallId: installId })
          .where(
            and(
              eq(projectRepos.githubOrg, githubOrgLogin),
              eq(projectRepos.repoName, r.name)
            )
          );
      }
    }

    // Audit log
    await db.insert(auditLog).values({
      actorUserId: SYSTEM_USER_ID,
      action: "org.github_app_installed",
      targetType: "organization",
      targetId: orgId,
      metadata: {
        installation_id: installId,
        github_org_login: githubOrgLogin,
        installed_by: installedBy,
      },
    });
  } else if (p.action === "deleted") {
    log.info("GitHub App uninstalled", {
      installation_id: installId,
      account: githubOrgLogin,
    });

    // Read org linkage BEFORE updating state
    const [inst] = await db
      .select({ orgId: githubInstallations.orgId })
      .from(githubInstallations)
      .where(eq(githubInstallations.installationId, installId))
      .limit(1);

    // Soft-transition: mark as deleted, don't remove data
    await db
      .update(githubInstallations)
      .set({ installationState: "deleted", updatedAt: new Date() })
      .where(eq(githubInstallations.installationId, installId));

    // Clear installation IDs for all repos owned by this account
    await db
      .update(projectRepos)
      .set({ githubInstallId: null })
      .where(eq(projectRepos.githubOrg, githubOrgLogin));

    // Audit log (only if we found the installation row)
    if (inst?.orgId) {
      await db.insert(auditLog).values({
        actorUserId: SYSTEM_USER_ID,
        action: "org.github_app_uninstalled",
        targetType: "organization",
        targetId: inst.orgId,
        metadata: {
          installation_id: installId,
          github_org_login: githubOrgLogin,
        },
      });
    } else {
      log.warn("Uninstall webhook for unknown installation", {
        installation_id: installId,
      });
    }
  } else if (p.action === "suspend") {
    log.info("GitHub App suspended", { installation_id: installId });
    await db
      .update(githubInstallations)
      .set({ installationState: "suspended", updatedAt: new Date() })
      .where(eq(githubInstallations.installationId, installId));
  } else if (p.action === "unsuspend") {
    log.info("GitHub App unsuspended", { installation_id: installId });
    await db
      .update(githubInstallations)
      .set({ installationState: "active", updatedAt: new Date() })
      .where(eq(githubInstallations.installationId, installId));
  }
}

// ---------------------------------------------------------------------------
// Organization membership events (spec 080 FR-011)
// ---------------------------------------------------------------------------

// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function handleOrganizationEvent(p: any): Promise<void> {
  if (p.action === "member_removed") {
    const ghUserId: number = p.membership?.user?.id;
    const installId: number | undefined = p.installation?.id;

    if (!ghUserId) {
      log.warn("organization.member_removed: no user ID in payload");
      return;
    }

    log.info("Organization member removed", {
      github_user_id: ghUserId,
      org: p.organization?.login,
    });

    // Look up the OAP user by github_user_id
    const [oapUser] = await db
      .select({ id: users.id })
      .from(users)
      .where(eq(users.githubUserId, ghUserId))
      .limit(1);

    if (!oapUser) {
      log.info("Removed user not found in OAP, skipping", {
        github_user_id: ghUserId,
      });
      return;
    }

    // Resolve the org via installation or github_org_id
    let orgId: string | null = null;
    if (installId) {
      const [inst] = await db
        .select({ orgId: githubInstallations.orgId })
        .from(githubInstallations)
        .where(eq(githubInstallations.installationId, installId))
        .limit(1);
      orgId = inst?.orgId ?? null;
    }

    if (!orgId && p.organization?.id) {
      const [org] = await db
        .select({ id: organizations.id })
        .from(organizations)
        .where(eq(organizations.githubOrgId, p.organization.id))
        .limit(1);
      orgId = org?.id ?? null;
    }

    if (!orgId) {
      log.warn("Could not resolve OAP org for member removal", {
        github_user_id: ghUserId,
        org: p.organization?.login,
      });
      return;
    }

    await revokeOrgMembership(oapUser.id, orgId);
    log.info("Revoked membership via webhook", {
      userId: oapUser.id,
      orgId,
      github_user_id: ghUserId,
    });
  } else if (p.action === "member_added") {
    // Spec 106 FR-005 addendum: proactively upsert membership so the
    // resolver DB reflects the add immediately, not on next login. Kept
    // conservative — only writes when the user already exists in OAP;
    // first-time users are still provisioned by /auth/rauthy/callback.
    const ghUserId: number | undefined = p.membership?.user?.id;
    const installId: number | undefined = p.installation?.id;
    const githubOrgId: number | undefined = p.organization?.id;

    if (!ghUserId) {
      log.warn("organization.member_added: no user ID in payload");
      return;
    }

    const [oapUser] = await db
      .select({ id: users.id })
      .from(users)
      .where(eq(users.githubUserId, ghUserId))
      .limit(1);

    if (!oapUser) {
      log.info("Added user not known to OAP yet (will resolve on login)", {
        github_user_id: ghUserId,
        org: p.organization?.login,
      });
      return;
    }

    let orgId: string | null = null;
    if (installId) {
      const [inst] = await db
        .select({ orgId: githubInstallations.orgId })
        .from(githubInstallations)
        .where(eq(githubInstallations.installationId, installId))
        .limit(1);
      orgId = inst?.orgId ?? null;
    }

    if (!orgId && githubOrgId) {
      const [org] = await db
        .select({ id: organizations.id })
        .from(organizations)
        .where(eq(organizations.githubOrgId, githubOrgId))
        .limit(1);
      orgId = org?.id ?? null;
    }

    if (!orgId) {
      log.warn("Could not resolve OAP org for member addition", {
        github_user_id: ghUserId,
        org: p.organization?.login,
      });
      return;
    }

    await db
      .insert(orgMemberships)
      .values({
        userId: oapUser.id,
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

    await db.insert(auditLog).values({
      actorUserId: SYSTEM_USER_ID,
      action: "membership.added",
      targetType: "user",
      targetId: oapUser.id,
      metadata: {
        org_id: orgId,
        reason: "github_membership_added",
        github_user_id: ghUserId,
      },
    });

    log.info("Upserted membership via webhook", {
      userId: oapUser.id,
      orgId,
      github_user_id: ghUserId,
    });
  }
}

async function findRepoRow(fullName: string) {
  const [owner, name] = fullName.split("/");
  if (!owner || !name) return null;
  const rows = await db
    .select()
    .from(projectRepos)
    .where(and(eq(projectRepos.githubOrg, owner), eq(projectRepos.repoName, name)))
    .limit(1);
  return rows[0] ?? null;
}

async function findOrCreatePreviewEnv(projectId: string, prNumber: number) {
  const envName = `preview-pr-${prNumber}`;
  const existing = await db
    .select()
    .from(environments)
    .where(and(eq(environments.projectId, projectId), eq(environments.name, envName)))
    .limit(1);
  if (existing.length > 0) return existing[0];

  const [created] = await db
    .insert(environments)
    .values({
      projectId,
      name: envName,
      kind: "preview",
      autoDeployBranch: null,
      requiresApproval: false,
    })
    .returning();
  return created;
}
