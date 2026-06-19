import log from "encore.dev/log";
import { getCachedDeploydAuthHeader } from "./oidcM2m";
import { readSecretFromDir } from "./secrets";

// Spec 215 FR-007: this module is the single deployd-api client. M2M secret
// resolution and the token cache (via oidcM2m.ts) live here once; `deploy.ts`
// (the raw proxy) and the new trigger path (deployments.ts) import
// `getDeploydAuthHeader` / `DEPLOYD_URL` rather than re-deriving credentials.
export const DEPLOYD_URL =
  process.env.DEPLOYD_URL ?? "http://deployd-api.deployd-system.svc.cluster.local";
const OIDC_ENDPOINT = process.env.OIDC_ENDPOINT ?? process.env.LOGTO_ENDPOINT ?? "";
const DEPLOYD_AUDIENCE = process.env.DEPLOYD_AUDIENCE ?? "";
const DEPLOYD_SCOPE = process.env.DEPLOYD_SCOPE ?? "";

/**
 * Resolve a cached M2M bearer header for calling deployd-api. Single source of
 * credential resolution (spec 215 FR-007): reads OIDC_M2M_CLIENT_ID/SECRET from
 * the CSI secrets mount or env (LOGTO_* fallback), then delegates to the shared
 * token cache in oidcM2m.ts. Throws with a specific diagnostic when config or
 * credentials are absent.
 */
export async function getDeploydAuthHeader(): Promise<string> {
  if (!OIDC_ENDPOINT || !DEPLOYD_AUDIENCE) {
    throw new Error("Missing OIDC_ENDPOINT or DEPLOYD_AUDIENCE");
  }

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

  if (!clientId || !clientSecret) {
    throw new Error(
      "Missing OIDC_M2M_CLIENT_ID or OIDC_M2M_CLIENT_SECRET in secrets mount or env"
    );
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
  const authHeader = await getDeploydAuthHeader();
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
 * Destroy a preview deployment via deployd-api DELETE /v1/deployments/:id.
 * Tears down K8s resources and marks the deployment as DESTROYED.
 */
export async function destroyPreviewDeployment(releaseId: string): Promise<void> {
  if (!isDeploydConfigured()) {
    log.info("Preview destroy requested (no deployd configured)", { releaseId });
    return;
  }

  const authHeader = await getDeploydAuthHeader();
  const resp = await fetch(`${DEPLOYD_URL}/v1/deployments/${releaseId}`, {
    method: "DELETE",
    headers: { authorization: authHeader },
  });

  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`deployd-api delete failed: ${resp.status} ${text}`);
  }

  log.info("Preview deployment destroyed", { releaseId });
}
