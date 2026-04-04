import { api } from "encore.dev/api";
import { secret } from "encore.dev/config";
import log from "encore.dev/log";
import { createHmac, timingSafeEqual } from "crypto";
import { db } from "../db/drizzle";
import { projectRepos } from "../db/schema";
import { eq, and } from "drizzle-orm";

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
      if (p.action === "created") {
        const installId: number = p.installation.id;
        const account: string = p.installation.account.login;
        log.info("GitHub App installed", {
          installation_id: installId,
          account,
        });

        // Persist installation_id to all repos in projectRepos that belong
        // to this account and have no installation ID yet
        if (Array.isArray(p.repositories)) {
          for (const r of p.repositories as Array<{ name: string }>) {
            await db
              .update(projectRepos)
              .set({ githubInstallId: installId })
              .where(
                and(
                  eq(projectRepos.githubOrg, account),
                  eq(projectRepos.repoName, r.name)
                )
              );
          }
        }
      } else if (p.action === "deleted") {
        log.info("GitHub App uninstalled", {
          installation_id: p.installation.id,
          account: p.installation.account.login,
        });
        // Clear installation IDs for all repos owned by this account
        await db
          .update(projectRepos)
          .set({ githubInstallId: null })
          .where(eq(projectRepos.githubOrg, p.installation.account.login));
      }
      break;

    case "repository":
      if (p.action === "created") {
        log.info("New repository created via webhook", {
          repo: p.repository.full_name,
        });
        // TODO: Auto-register in projectRepos when project discovery is wired
      }
      break;

    case "pull_request":
      if (p.action === "opened" || p.action === "synchronize") {
        log.info("PR preview deploy trigger", {
          repo: p.repository.full_name,
          pr: p.number,
          sha: p.pull_request.head.sha,
        });
        // TODO: Trigger preview deploy via deployd-api
      } else if (p.action === "closed") {
        log.info("PR preview destroy trigger", {
          repo: p.repository.full_name,
          pr: p.number,
        });
        // TODO: Trigger preview destroy via deployd-api
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
        // TODO: Trigger governance checks
      }
      break;

    default:
      log.debug("Unhandled webhook event", { event });
  }
}
