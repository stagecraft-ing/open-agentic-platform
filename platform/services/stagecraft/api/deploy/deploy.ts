import { api } from "encore.dev/api";
// zod 4: see extractionOutput.ts for why namespace-import is required.
import * as z from "zod";
import { readSecretFromDir } from "./secrets";
import { getCachedDeploydAuthHeader } from "./oidcM2m";

const DEPLOYD_URL =
  process.env.DEPLOYD_URL ?? "http://deployd-api.deployd-system.svc.cluster.local";
const OIDC_ENDPOINT = process.env.OIDC_ENDPOINT ?? process.env.LOGTO_ENDPOINT ?? "";
const DEPLOYD_AUDIENCE = process.env.DEPLOYD_AUDIENCE ?? "";
const DEPLOYD_SCOPE = process.env.DEPLOYD_SCOPE ?? "";

const CreateDeployment = z.object({
  tenant_id: z.string().min(1),
  app_id: z.string().min(1),
  app_slug: z.string().min(1),
  env_id: z.string().min(1),
  env_slug: z.string().min(1),

  release_sha: z.string().min(7),
  artifact_ref: z.string().min(1),
  lane: z.enum(["LANE_A", "LANE_B"]),

  desired_routes: z
    .array(
      z.object({
        host: z.string().optional(),
        path: z.string().default("/"),
      })
    )
    .optional()
    .default([]),

  config_refs: z.record(z.string(), z.string()).optional().default({}),
});

function safeJson(s: string): unknown {
  try {
    return JSON.parse(s);
  } catch {
    return { raw: s };
  }
}

async function getDeploydAuthHeader(): Promise<string> {
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

export const createDeployment = api.raw(
  { expose: true, path: "/v1/deployments", method: "POST", auth: false },
  async (req, res) => {
    let body: unknown;
    try {
      const chunks: Buffer[] = [];
      for await (const chunk of req) {
        chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
      }
      body = JSON.parse(Buffer.concat(chunks).toString("utf8"));
    } catch {
      res.statusCode = 400;
      res.setHeader("Content-Type", "application/json");
      res.end(
        JSON.stringify({
          error: "invalid_request",
          message: "Invalid JSON body",
        })
      );
      return;
    }

    const parsed = CreateDeployment.safeParse(body);
    if (!parsed.success) {
      res.statusCode = 400;
      res.setHeader("Content-Type", "application/json");
      res.end(
        JSON.stringify({
          error: "invalid_request",
          details: parsed.error.flatten(),
        })
      );
      return;
    }

    const data = parsed.data;

    // POC: when deployed (SECRETS_DIR set), validate CSI wiring by checking STAGECRAFT_DB_URL
    const secretsDir = process.env.SECRETS_DIR;
    if (secretsDir) {
      const stagecraftDbUrl =
        (await readSecretFromDir("STAGECRAFT_DB_URL")) ?? process.env.STAGECRAFT_DB_URL ?? null;

      if (!stagecraftDbUrl) {
        res.statusCode = 500;
        res.setHeader("Content-Type", "application/json");
        res.end(
          JSON.stringify({
            error: "missing_secret",
            message:
              "STAGECRAFT_DB_URL not found in secrets mount or env; set Key Vault secret or env var",
          })
        );
        return;
      }
    }

    try {
      const authHeader = await getDeploydAuthHeader();
      const resp = await fetch(`${DEPLOYD_URL}/v1/deployments`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          authorization: authHeader,
        },
        body: JSON.stringify({
          tenant_id: data.tenant_id,
          app_id: data.app_id,
          app_slug: data.app_slug,
          env_id: data.env_id,
          env_slug: data.env_slug,
          release_sha: data.release_sha,
          artifact_ref: data.artifact_ref,
          lane: data.lane,
          config_refs: data.config_refs,
          desired_routes: data.desired_routes,
        }),
      });

      const text = await resp.text();

      res.statusCode = resp.status;
      res.setHeader("Content-Type", "application/json");

      if (!resp.ok) {
        res.end(
          JSON.stringify({
            error: "deployd_failed",
            status: resp.status,
            body: safeJson(text),
          })
        );
        return;
      }

      res.end(
        JSON.stringify({
          env_id: data.env_id,
          deployd: safeJson(text),
        })
      );
    } catch (err) {
      res.statusCode = 500;
      res.setHeader("Content-Type", "application/json");
      res.end(
        JSON.stringify({
          error: "internal",
          message: err instanceof Error ? err.message : "Unknown error",
        })
      );
    }
  }
);

export const healthz = api.raw(
  { expose: true, path: "/healthz", method: "GET", auth: false },
  async (_req, res) => {
    res.statusCode = 200;
    res.end("ok");
  }
);
