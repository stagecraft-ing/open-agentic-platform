import log from "encore.dev/log";
import { getCachedDeploydAuthHeader } from "./oidcM2m";
import { readSecretFromDir } from "./secrets";

const DEPLOYD_URL =
  process.env.DEPLOYD_URL ?? "http://deployd-api.deployd-system.svc.cluster.local";
const OIDC_ENDPOINT = process.env.OIDC_ENDPOINT ?? process.env.LOGTO_ENDPOINT ?? "";
const DEPLOYD_AUDIENCE = process.env.DEPLOYD_AUDIENCE ?? "";
const DEPLOYD_SCOPE = process.env.DEPLOYD_SCOPE ?? "";

async function getAuthHeader(): Promise<string> {
  const clientId =
    (await readSecretFromDir("OIDC_M2M_CLIENT_ID")) ??
    process.env.OIDC_M2M_CLIENT_ID ??
    (await readSecretFromDir("LOGTO_M2M_CLIENT_ID")) ??
    process.env.LOGTO_M2M_CLIENT_ID ??
    "";
  const clientSecret =
    (await readSecretFromDir("OIDC_M2M_CLIENT_SECRET")) ??
    process.env.OIDC_M2M_CLIENT_SECRET ??
    (await readSecretFromDir("LOGTO_M2M_CLIENT_SECRET")) ??
    process.env.LOGTO_M2M_CLIENT_SECRET ??
    "";

  if (!OIDC_ENDPOINT || !DEPLOYD_AUDIENCE || !clientId || !clientSecret) {
    throw new Error("Missing OIDC/deployd credentials for M2M auth");
  }

  return getCachedDeploydAuthHeader({
    oidcEndpoint: OIDC_ENDPOINT,
    resource: DEPLOYD_AUDIENCE,
    scope: DEPLOYD_SCOPE || undefined,
    clientId,
    clientSecret,
    skewSeconds: 30,
  });
}

/** Whether deployd-api credentials are configured (false in local dev). */
export function isDeploydConfigured(): boolean {
  return !!(OIDC_ENDPOINT && DEPLOYD_AUDIENCE);
}

export type DeploydDeploymentResult = {
  release_id: string;
  status: string;
};

/**
 * Create a deployment via deployd-api-rs POST /v1/deployments.
 * Throws on HTTP or auth errors.
 */
export async function createPreviewDeployment(opts: {
  tenant_id: string;
  app_id: string;
  env_id: string;
  release_sha: string;
  artifact_ref: string;
  lane: string;
}): Promise<DeploydDeploymentResult> {
  const authHeader = await getAuthHeader();
  const resp = await fetch(`${DEPLOYD_URL}/v1/deployments`, {
    method: "POST",
    headers: { "content-type": "application/json", authorization: authHeader },
    body: JSON.stringify(opts),
  });

  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`deployd-api create failed: ${resp.status} ${text}`);
  }

  return (await resp.json()) as DeploydDeploymentResult;
}

/**
 * Request destruction of a preview deployment.
 * deployd-api does not yet expose a DELETE endpoint — log intent for now.
 */
export async function destroyPreviewDeployment(releaseId: string): Promise<void> {
  log.info("Preview destroy requested", { releaseId });
  // Wire to deployd-api DELETE /v1/deployments/:id when the endpoint ships.
}
