import { api } from "encore.dev/api";
import { readSecretFromDir } from "./secrets";
import { getCachedDeploydAuthHeader } from "./oidcM2m";

// Hand-rolled validator for the `/v1/deployments` raw body. zod is
// avoided here because Encore.ts's TS parser walks zod 4's `.d.cts`
// during codegen and crashes on `TsFnOrConstructorType` — see
// `api/knowledge/extractionOutput.ts` for the longer explanation.

const DEPLOYD_URL =
  process.env.DEPLOYD_URL ?? "http://deployd-api.deployd-system.svc.cluster.local";
const OIDC_ENDPOINT = process.env.OIDC_ENDPOINT ?? process.env.LOGTO_ENDPOINT ?? "";
const DEPLOYD_AUDIENCE = process.env.DEPLOYD_AUDIENCE ?? "";
const DEPLOYD_SCOPE = process.env.DEPLOYD_SCOPE ?? "";

interface DesiredRoute {
  host?: string;
  path: string;
}

interface CreateDeploymentBody {
  tenant_id: string;
  app_id: string;
  app_slug: string;
  env_id: string;
  env_slug: string;
  release_sha: string;
  artifact_ref: string;
  lane: "LANE_A" | "LANE_B";
  desired_routes: DesiredRoute[];
  config_refs: Record<string, string>;
}

interface FieldIssue {
  field: string;
  message: string;
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function parseCreateDeployment(
  raw: unknown,
):
  | { ok: true; data: CreateDeploymentBody }
  | { ok: false; issues: FieldIssue[] } {
  const issues: FieldIssue[] = [];
  if (!isObject(raw)) {
    return { ok: false, issues: [{ field: "<root>", message: "expected object" }] };
  }

  const reqStr = (key: keyof CreateDeploymentBody, min = 1): string => {
    const v = raw[key as string];
    if (typeof v !== "string" || v.length < min) {
      issues.push({
        field: key as string,
        message: `expected string with length >= ${min}`,
      });
      return "";
    }
    return v;
  };

  const tenant_id = reqStr("tenant_id");
  const app_id = reqStr("app_id");
  const app_slug = reqStr("app_slug");
  const env_id = reqStr("env_id");
  const env_slug = reqStr("env_slug");
  const release_sha = reqStr("release_sha", 7);
  const artifact_ref = reqStr("artifact_ref");

  let lane: "LANE_A" | "LANE_B" = "LANE_A";
  if (raw.lane === "LANE_A" || raw.lane === "LANE_B") {
    lane = raw.lane;
  } else {
    issues.push({ field: "lane", message: "expected 'LANE_A' or 'LANE_B'" });
  }

  const desired_routes: DesiredRoute[] = [];
  if (raw.desired_routes !== undefined) {
    if (!Array.isArray(raw.desired_routes)) {
      issues.push({ field: "desired_routes", message: "expected array" });
    } else {
      raw.desired_routes.forEach((entry, i) => {
        if (!isObject(entry)) {
          issues.push({
            field: `desired_routes[${i}]`,
            message: "expected object",
          });
          return;
        }
        const host = entry.host;
        if (host !== undefined && typeof host !== "string") {
          issues.push({
            field: `desired_routes[${i}].host`,
            message: "expected string",
          });
          return;
        }
        const path = typeof entry.path === "string" ? entry.path : "/";
        desired_routes.push({ host, path });
      });
    }
  }

  const config_refs: Record<string, string> = {};
  if (raw.config_refs !== undefined) {
    if (!isObject(raw.config_refs)) {
      issues.push({ field: "config_refs", message: "expected object" });
    } else {
      for (const [k, v] of Object.entries(raw.config_refs)) {
        if (typeof v !== "string") {
          issues.push({
            field: `config_refs.${k}`,
            message: "expected string value",
          });
          continue;
        }
        config_refs[k] = v;
      }
    }
  }

  if (issues.length > 0) {
    return { ok: false, issues };
  }
  return {
    ok: true,
    data: {
      tenant_id,
      app_id,
      app_slug,
      env_id,
      env_slug,
      release_sha,
      artifact_ref,
      lane,
      desired_routes,
      config_refs,
    },
  };
}

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

    const parsed = parseCreateDeployment(body);
    if (!parsed.ok) {
      res.statusCode = 400;
      res.setHeader("Content-Type", "application/json");
      res.end(
        JSON.stringify({
          error: "invalid_request",
          details: parsed.issues,
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
