import { api, APIError } from "encore.dev/api";
import log from "encore.dev/log";
import { eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import { environments, projects, organizations } from "../db/schema";
import { validateM2mRequest } from "../auth/m2mAuth.js";
import { readSecretFromDir } from "./secrets";
import { getDeploydAuthHeader, DEPLOYD_URL } from "./deploydClient";
import { loadDeployDescriptorForEnv } from "../environments/accessGatesDeploy";
import {
  resolveChartSelection,
  isReservedConfigRefKey,
  RESERVED_CONFIG_REF_PREFIXES,
} from "./deployResolve";
import { deriveHost } from "./hostname";
import type { HostVariant } from "./hostname";

// Hand-rolled validator for the `/v1/deployments` raw body. zod is
// avoided here because Encore.ts's TS parser walks zod 4's `.d.cts`
// during codegen and crashes on `TsFnOrConstructorType` — see
// `api/knowledge/extractionOutput.ts` for the longer explanation.

// DEPLOYD_URL and getDeploydAuthHeader are imported from ./deploydClient: the
// single deployd client module (spec 215 FR-007). The M2M token endpoint
// (OIDC_ENDPOINT), audience, and scope are resolved there, not here.
// Spec 137 — Rauthy issuer URL passed through to deployd-api so the
// rendered oauth2-proxy points at our identity provider. Distinct from
// OIDC_ENDPOINT (which is the M2M token endpoint used to call deployd-api).
const RAUTHY_ISSUER_URL = process.env.RAUTHY_ISSUER_URL ?? "";
// Spec 214 FR-007 / Clarification 3: the apex the tenant wildcard cert covers
// (`*.tenants.${TENANTS_BASE_DOMAIN}`). Operational config surfaced to the
// deploy service; the host is derived as a single label below `tenants.`.
// When unset, route derivation is skipped and the caller's desired_routes (if
// any) flow through unchanged.
const TENANTS_BASE_DOMAIN = process.env.TENANTS_BASE_DOMAIN ?? "";

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
  // Spec 214 optional dispatch fields. All default-derivable: chart from the
  // project's adapter (FR-002), namespace from the environment (FR-008), host
  // from slugs (FR-007). A caller may still supply them explicitly and they
  // take precedence over the derived values.
  chart?: string;
  chart_version?: string;
  image_pull_secret_name?: string;
  namespace?: string;
  variant?: HostVariant;
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
        // Spec 214 FR-004: ENCORE_/KUBERNETES_ are runtime/platform-owned.
        if (isReservedConfigRefKey(k)) {
          issues.push({
            field: `config_refs.${k}`,
            message: `reserved prefix: keys starting with ${RESERVED_CONFIG_REF_PREFIXES.join(" or ")} are platform-managed and cannot be set via config_refs`,
          });
          continue;
        }
        config_refs[k] = v;
      }
    }
  }

  // Spec 214 optional dispatch fields. Absent fields stay undefined and are
  // derived (or defaulted) downstream; present non-strings are a field error.
  const optStr = (key: string): string | undefined => {
    const v = raw[key];
    if (v === undefined || v === null) return undefined;
    if (typeof v !== "string") {
      issues.push({ field: key, message: "expected string" });
      return undefined;
    }
    return v;
  };
  const chart = optStr("chart");
  const chart_version = optStr("chart_version");
  const image_pull_secret_name = optStr("image_pull_secret_name");
  const namespace = optStr("namespace");

  let variant: HostVariant | undefined;
  if (raw.variant !== undefined && raw.variant !== null) {
    if (raw.variant === "public" || raw.variant === "internal") {
      variant = raw.variant;
    } else {
      issues.push({ field: "variant", message: "expected 'public' or 'internal'" });
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
      chart,
      chart_version,
      image_pull_secret_name,
      namespace,
      variant,
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

export const createDeployment = api.raw(
  // Spec 215 FR-006: `auth: false` keeps the Encore gateway's user-session
  // handler off this seam, but the handler now requires a valid Rauthy M2M
  // JWT carrying `platform:deploy:write`. Browser-reachable anonymity ends;
  // machine callers present a client_credentials token (validateM2mRequest).
  { expose: true, path: "/v1/deployments", method: "POST", auth: false },
  async (req, res) => {
    // FR-006: validate the M2M bearer before doing any work. In a raw handler
    // a thrown APIError is not auto-rendered, so map its code to a status and
    // write the response ourselves.
    try {
      const authHeader =
        typeof req.headers["authorization"] === "string"
          ? req.headers["authorization"]
          : undefined;
      await validateM2mRequest(authHeader, "platform:deploy:write");
    } catch (err) {
      const code = err instanceof APIError ? err.code : "internal";
      const status =
        code === "unauthenticated" ? 401 : code === "permission_denied" ? 403 : 500;
      res.statusCode = status;
      res.setHeader("Content-Type", "application/json");
      res.end(
        JSON.stringify({
          error: String(code),
          message: err instanceof Error ? err.message : "authentication failed",
        })
      );
      return;
    }

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

    // region: access-gate-wire
    // Spec 137 Phase 4↔5 integration — load the per-env access-gate
    // descriptor and forward as `access_gate` to deployd-api. The
    // descriptor lives in stagecraft Postgres; deployd-api only sees the
    // rendered wire shape. Absent descriptor (no gate ever configured)
    // produces `null`, which deployd-api treats identically to
    // `enabled: false` (no auth-url annotation, no gate release).
    //
    // RAUTHY_ISSUER_URL is required when any tenant in this cluster
    // wants an enabled gate; gate-disabled deploys can flow through even
    // when it's missing (we surface the env var only when we actually
    // emit a non-null access_gate).
    let access_gate: Awaited<ReturnType<typeof loadDeployDescriptorForEnv>> = null;
    try {
      access_gate = await loadDeployDescriptorForEnv(
        data.env_id,
        RAUTHY_ISSUER_URL,
      );
      if (access_gate?.enabled && !RAUTHY_ISSUER_URL) {
        res.statusCode = 500;
        res.setHeader("Content-Type", "application/json");
        res.end(
          JSON.stringify({
            error: "missing_config",
            message:
              "RAUTHY_ISSUER_URL is required when a tenant access gate is enabled",
          }),
        );
        return;
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      log.error("deploy.access_gate.load_failed", {
        envId: data.env_id,
        error: msg,
      });
      res.statusCode = 500;
      res.setHeader("Content-Type", "application/json");
      res.end(
        JSON.stringify({
          error: "access_gate_load_failed",
          message: msg,
        }),
      );
      return;
    }

    // region: shape-and-routing
    // Spec 214: enrich the dispatch from stagecraft's own records. Resolve the
    // chart shape from the project's factory adapter (FR-002), forward the
    // stored namespace (FR-008), and derive the tenant host from
    // org/project/env slugs when the caller supplied no routes (FR-007).
    // Best-effort: any lookup miss or DB error falls back to caller-supplied
    // values so existing /v1/deployments callers keep working unchanged.
    let chartSelection = resolveChartSelection(undefined, data.chart);
    let forwardedNamespace: string | undefined = data.namespace || undefined;
    let desiredRoutes = data.desired_routes;
    try {
      const [env] = await db
        .select({
          projectId: environments.projectId,
          k8sNamespace: environments.k8sNamespace,
        })
        .from(environments)
        .where(eq(environments.id, data.env_id))
        .limit(1);
      if (env) {
        if (!forwardedNamespace && env.k8sNamespace) {
          forwardedNamespace = env.k8sNamespace;
        }
        const [project] = await db
          .select({
            slug: projects.slug,
            orgId: projects.orgId,
            factoryAdapterId: projects.factoryAdapterId,
          })
          .from(projects)
          .where(eq(projects.id, env.projectId))
          .limit(1);
        if (project) {
          chartSelection = resolveChartSelection(project.factoryAdapterId, data.chart);
          if (desiredRoutes.length === 0 && TENANTS_BASE_DOMAIN) {
            const [org] = await db
              .select({ slug: organizations.slug })
              .from(organizations)
              .where(eq(organizations.id, project.orgId))
              .limit(1);
            if (org) {
              const host = deriveHost({
                orgSlug: org.slug,
                projectSlug: project.slug,
                envSlug: data.env_slug,
                variant: data.variant,
                baseDomain: TENANTS_BASE_DOMAIN,
              });
              desiredRoutes = [{ host, path: "/" }];
            }
          }
        }
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      log.warn("deploy.enrichment.lookup_failed", { envId: data.env_id, error: msg });
      // Fall through with caller-supplied chart/namespace/routes.
    }
    // FR-005: default the pull secret to the reflected `ghcr-pull` dockerconfig.
    const imagePullSecretName = data.image_pull_secret_name || "ghcr-pull";
    // endregion shape-and-routing

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
          desired_routes: desiredRoutes,
          access_gate,
          // Spec 214: chart (FR-002), namespace (FR-008), pull secret (FR-005).
          // Undefined fields are omitted by JSON.stringify, so deployd-api's
          // Option<_> defaults apply (chart falls back to its own default).
          chart: chartSelection?.chart,
          chart_version: data.chart_version || chartSelection?.version,
          image_pull_secret_name: imagePullSecretName,
          namespace: forwardedNamespace,
        }),
      });
    // endregion access-gate-wire

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
