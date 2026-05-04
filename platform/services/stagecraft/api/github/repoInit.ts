/**
 * GitHub repo initialization helpers (spec 080 FR-008).
 *
 * Provides functions to create repos, seed adapter templates, configure
 * branch protection, and create OAP workflow files via the GitHub API
 * using a GitHub App installation token.
 */

import log from "encore.dev/log";
import { signAppJwt } from "./appJwt";

const GITHUB_API = "https://api.github.com";
const API_VERSION = "2022-11-28";

// ---------------------------------------------------------------------------
// Installation token broker
// ---------------------------------------------------------------------------

/**
 * Result of a successful installation-token exchange. Spec 112 §6.4
 * needs `expiresAt` to drive OPC-side refresh; existing callers that
 * only care about the token destructure `.token`.
 */
export interface BrokeredInstallationToken {
  token: string;
  expiresAt: Date;
}

/**
 * Broker a scoped installation token for a given GitHub App installation.
 */
export async function brokerInstallationToken(
  installationId: number,
  permissions: Record<string, string>
): Promise<BrokeredInstallationToken> {
  const jwt = await signAppJwt();

  const resp = await fetch(
    `${GITHUB_API}/app/installations/${installationId}/access_tokens`,
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${jwt}`,
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": API_VERSION,
      },
      body: JSON.stringify({ permissions }),
    }
  );

  if (!resp.ok) {
    const body = await resp.text();
    throw new Error(
      `Installation token exchange failed: ${resp.status} ${body}`
    );
  }

  const data = (await resp.json()) as { token: string; expires_at: string };
  log.info("Installation token issued", { installationId });
  return { token: data.token, expiresAt: new Date(data.expires_at) };
}

// ---------------------------------------------------------------------------
// GitHub API helpers
// ---------------------------------------------------------------------------

function githubHeaders(token: string): Record<string, string> {
  return {
    Authorization: `Bearer ${token}`,
    Accept: "application/vnd.github+json",
    "Content-Type": "application/json",
    "X-GitHub-Api-Version": API_VERSION,
  };
}

// ---------------------------------------------------------------------------
// FR-008: Repo creation
// ---------------------------------------------------------------------------

export interface CreateRepoResult {
  fullName: string;
  defaultBranch: string;
  cloneUrl: string;
  htmlUrl: string;
}

/**
 * Create a GitHub repository in the org using the installation token.
 *
 * `autoInit` defaults to `true` (the legacy behavior — repo ships with an
 * auto-generated README so `seedRepoFromAdapter` has a SHA to update). The
 * factory scaffold path passes `false` so we can push our scaffold tree as
 * commit #1 without force-overwriting the README.
 */
export async function createGitHubRepo(
  token: string,
  org: string,
  repoName: string,
  opts: { isPrivate: boolean; description: string; autoInit?: boolean }
): Promise<CreateRepoResult> {
  const resp = await fetch(`${GITHUB_API}/orgs/${org}/repos`, {
    method: "POST",
    headers: githubHeaders(token),
    body: JSON.stringify({
      name: repoName,
      description: opts.description,
      private: opts.isPrivate,
      auto_init: opts.autoInit ?? true,
      delete_branch_on_merge: true,
    }),
  });

  if (!resp.ok) {
    const body = await resp.text();
    if (resp.status === 422 && body.includes("already exists")) {
      throw new Error(`Repository ${org}/${repoName} already exists on GitHub`);
    }
    throw new Error(`GitHub create repo failed: ${resp.status} ${body}`);
  }

  const data = (await resp.json()) as {
    full_name: string;
    default_branch: string;
    clone_url: string;
    html_url: string;
  };

  log.info("GitHub repo created", { fullName: data.full_name });

  return {
    fullName: data.full_name,
    defaultBranch: data.default_branch,
    cloneUrl: data.clone_url,
    htmlUrl: data.html_url,
  };
}

// ---------------------------------------------------------------------------
// FR-008: Adapter template seeding
// ---------------------------------------------------------------------------

const VALID_ADAPTERS = new Set([
  "aim-vue-node",
  "encore-react",
  "next-prisma",
  "rust-axum",
]);

/**
 * Seed the repo with a minimal adapter README via the GitHub Contents API.
 */
export async function seedRepoFromAdapter(
  token: string,
  fullName: string,
  adapter: string
): Promise<void> {
  if (!VALID_ADAPTERS.has(adapter)) {
    throw new Error(`Unknown adapter: ${adapter}. Valid: ${[...VALID_ADAPTERS].join(", ")}`);
  }

  const readmeContent = Buffer.from(
    `# ${fullName.split("/")[1]}\n\n` +
      `Created by [Open Agentic Platform](https://github.com/open-agentic-platform/open-agentic-platform).\n\n` +
      `**Adapter:** \`${adapter}\`\n\n` +
      `## Getting Started\n\n` +
      `This project was scaffolded using the \`${adapter}\` adapter template.\n` +
      `See the adapter documentation for setup instructions.\n`
  ).toString("base64");

  // Update the auto-generated README with our adapter content
  // First get the existing README SHA (required for updates)
  const getResp = await fetch(
    `${GITHUB_API}/repos/${fullName}/contents/README.md`,
    { headers: githubHeaders(token) }
  );

  let sha: string | undefined;
  if (getResp.ok) {
    const existing = (await getResp.json()) as { sha: string };
    sha = existing.sha;
  }

  const putResp = await fetch(
    `${GITHUB_API}/repos/${fullName}/contents/README.md`,
    {
      method: "PUT",
      headers: githubHeaders(token),
      body: JSON.stringify({
        message: "chore: initialize project with OAP adapter template",
        content: readmeContent,
        ...(sha && { sha }),
      }),
    }
  );

  if (!putResp.ok) {
    const body = await putResp.text();
    log.warn("Failed to seed README", { fullName, status: putResp.status, body });
    // Non-fatal: repo still usable without custom README
    return;
  }

  log.info("Adapter README seeded", { fullName, adapter });
}

// ---------------------------------------------------------------------------
// FR-008: Branch protection
// ---------------------------------------------------------------------------

/**
 * Configure branch protection on the default branch.
 * Requires the installation to have `administration: write` permission.
 * Gracefully handles 403 (missing permission) — logs warning and continues.
 */
export async function configureBranchProtection(
  token: string,
  fullName: string,
  branch: string
): Promise<void> {
  const resp = await fetch(
    `${GITHUB_API}/repos/${fullName}/branches/${branch}/protection`,
    {
      method: "PUT",
      headers: githubHeaders(token),
      body: JSON.stringify({
        required_status_checks: {
          strict: true,
          contexts: ["oap/verify"],
        },
        enforce_admins: false,
        required_pull_request_reviews: {
          required_approving_review_count: 1,
          dismiss_stale_reviews: true,
        },
        restrictions: null, // no push restrictions
        allow_force_pushes: false,
        allow_deletions: false,
      }),
    }
  );

  if (!resp.ok) {
    const body = await resp.text();
    if (resp.status === 403) {
      log.warn(
        "Branch protection skipped: GitHub App lacks administration:write permission",
        { fullName, branch }
      );
      return;
    }
    log.warn("Branch protection failed", {
      fullName,
      branch,
      status: resp.status,
      body,
    });
    // Non-fatal: project creation should succeed without branch protection
  } else {
    log.info("Branch protection configured", { fullName, branch });
  }
}

// ---------------------------------------------------------------------------
// FR-008: GitHub Actions workflow
// ---------------------------------------------------------------------------

const OAP_WORKFLOW_YAML = `name: OAP Verify
on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

permissions:
  contents: read
  checks: write
  pull-requests: read

jobs:
  verify:
    name: oap/verify
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: OAP Governance Check
        run: |
          echo "OAP governance verification"
          echo "Adapter compliance and policy checks will run here"
          echo "See: https://github.com/open-agentic-platform/open-agentic-platform"
`;

/**
 * Create the standard OAP GitHub Actions workflow file.
 */
export async function createOapWorkflow(
  token: string,
  fullName: string
): Promise<void> {
  const content = Buffer.from(OAP_WORKFLOW_YAML).toString("base64");

  const resp = await fetch(
    `${GITHUB_API}/repos/${fullName}/contents/.github/workflows/oap-verify.yml`,
    {
      method: "PUT",
      headers: githubHeaders(token),
      body: JSON.stringify({
        message: "ci: add OAP governance verification workflow",
        content,
      }),
    }
  );

  if (!resp.ok) {
    const body = await resp.text();
    log.warn("Failed to create OAP workflow", {
      fullName,
      status: resp.status,
      body,
    });
    // Non-fatal: project creation should succeed without the workflow
    return;
  }

  log.info("OAP workflow created", { fullName });
}
