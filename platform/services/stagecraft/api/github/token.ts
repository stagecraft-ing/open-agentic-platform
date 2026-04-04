import { api, APIError } from "encore.dev/api";
import { secret } from "encore.dev/config";
import log from "encore.dev/log";
import { db } from "../db/drizzle";
import { projectRepos } from "../db/schema";
import { eq, and } from "drizzle-orm";

// GitHub App credentials (CSI-mounted secrets)
const githubAppId = secret("GITHUB_APP_ID");
const githubPrivateKey = secret("GITHUB_APP_PRIVATE_KEY");

// Scope mapping per tool category
const SCOPE_MAP: Record<string, Record<string, string>> = {
  "contents:read": { contents: "read" },
  "issues:read": { issues: "read" },
  "pull_requests:read": { pull_requests: "read" },
  "metadata:read": { metadata: "read" },
  "checks:write": { checks: "write" },
  "actions:write": { actions: "write" },
};

interface TokenRequest {
  repo: string;  // "owner/name"
  scope: string; // e.g. "contents:read"
}

interface TokenResponse {
  token: string;
  expires_at: string;
  permissions: Record<string, string>;
}

// POST /api/github/token — broker a scoped installation token
export const getToken = api(
  { expose: true, method: "POST", path: "/api/github/token", auth: true },
  async (req: TokenRequest): Promise<TokenResponse> => {
    const [owner, name] = req.repo.split("/");
    if (!owner || !name) {
      throw APIError.invalidArgument("repo must be owner/name format");
    }

    // Look up the installation ID from projectRepos
    const rows = await db
      .select({ githubInstallId: projectRepos.githubInstallId })
      .from(projectRepos)
      .where(
        and(
          eq(projectRepos.githubOrg, owner),
          eq(projectRepos.repoName, name)
        )
      )
      .limit(1);

    let installationId: number | null = null;
    if (rows.length > 0 && rows[0].githubInstallId != null) {
      installationId = rows[0].githubInstallId;
    }

    // Fall back to environment variable for local dev / bootstrap
    if (installationId == null) {
      const envId = process.env.GITHUB_INSTALLATION_ID;
      if (!envId) {
        throw APIError.notFound(
          `No GitHub App installation found for repo ${req.repo}`
        );
      }
      installationId = parseInt(envId, 10);
    }

    // Sign JWT as the GitHub App
    const jwt = await signAppJwt();

    // Exchange for a scoped installation token
    const permissions = SCOPE_MAP[req.scope] ?? { metadata: "read" };
    const resp = await fetch(
      `https://api.github.com/app/installations/${installationId}/access_tokens`,
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${jwt}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
        },
        body: JSON.stringify({ permissions }),
      }
    );

    if (!resp.ok) {
      const body = await resp.text();
      throw new Error(`GitHub token exchange failed: ${resp.status} ${body}`);
    }

    const data = (await resp.json()) as {
      token: string;
      expires_at: string;
      permissions: Record<string, string>;
    };

    log.info("GitHub installation token issued", {
      repo: req.repo,
      scope: req.scope,
    });

    return {
      token: data.token,
      expires_at: data.expires_at,
      permissions: data.permissions,
    };
  }
);

// Sign a JWT as the GitHub App (RS256, 10-minute TTL)
async function signAppJwt(): Promise<string> {
  const appId = githubAppId();
  const privateKey = githubPrivateKey();

  const now = Math.floor(Date.now() / 1000);
  const header = Buffer.from(
    JSON.stringify({ alg: "RS256", typ: "JWT" })
  ).toString("base64url");
  const payload = Buffer.from(
    JSON.stringify({
      iat: now - 60,   // 60 seconds in the past for clock skew
      exp: now + 600,  // 10-minute TTL
      iss: appId,
    })
  ).toString("base64url");

  const { createSign } = await import("crypto");
  const sign = createSign("RSA-SHA256");
  sign.update(`${header}.${payload}`);
  const signature = sign.sign(privateKey, "base64url");

  return `${header}.${payload}.${signature}`;
}
