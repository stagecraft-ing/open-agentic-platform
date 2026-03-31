import express from "express";
import { z } from "zod";
import crypto from "node:crypto";
import { createFileSecretsReader } from "./secrets.js";
import { getByKey, getByReleaseId, put, type Lane, type DeploymentRecord } from "./store.js";
import { verifyLogtoJwt } from "./auth/logtoJwt.js";
import path from "node:path";
import { ensureNamespaceWithBaseline } from "./k8s/namespaceBaseline.js";
import { helmUpsertTenantApp } from "./laneA/helmDeploy.js";

const PORT = Number(process.env.PORT ?? "8080");
const SECRETS_DIR = process.env.SECRETS_DIR ?? "/mnt/secrets-store";
const LOGTO_ENDPOINT = process.env.LOGTO_ENDPOINT ?? "";
const DEPLOYD_AUDIENCE = process.env.DEPLOYD_AUDIENCE ?? "";
const DEPLOYD_REQUIRED_SCOPE = process.env.DEPLOYD_REQUIRED_SCOPE ?? "";
const secrets = createFileSecretsReader(SECRETS_DIR);

const app = express();
app.use(express.json({ limit: "1mb" }));

app.get("/healthz", (_req, res) => res.status(200).send("ok"));

function assertAuthConfig() {
    if (!LOGTO_ENDPOINT) throw new Error("LOGTO_ENDPOINT is required");
    if (!DEPLOYD_AUDIENCE) throw new Error("DEPLOYD_AUDIENCE is required");
}

function hasScope(payload: any, required: string): boolean {
    if (!required) return true;
    const scope = typeof payload?.scope === "string" ? payload.scope : "";
    const scopes = scope.split(" ").map((s: string) => s.trim()).filter(Boolean);
    return scopes.includes(required);
}

async function requireAuth(req: express.Request, res: express.Response): Promise<boolean> {
    try {
        assertAuthConfig();
        const payload = await verifyLogtoJwt({
            authorizationHeader: req.header("authorization") ?? undefined,
            logtoEndpoint: LOGTO_ENDPOINT,
            audience: DEPLOYD_AUDIENCE,
        });

        if (!hasScope(payload, DEPLOYD_REQUIRED_SCOPE)) {
            res.status(403).json({ error: "forbidden", message: `missing scope ${DEPLOYD_REQUIRED_SCOPE}` });
            return false;
        }

        (req as any).auth = payload;
        return true;
    } catch (e: any) {
        res.status(401).json({ error: "unauthorized", message: e?.message ?? "invalid token" });
        return false;
    }
}

function splitImage(image: string): [string, string] {
    // Supports repo:tag and repo@sha256:digest. If no tag, default to "latest".
    if (image.includes("@")) {
        const idx = image.lastIndexOf("@");
        return [image.slice(0, idx), image.slice(idx)];
    }
    const idx = image.lastIndexOf(":");
    if (idx === -1) return [image, "latest"];
    const repo = image.slice(0, idx);
    const tag = image.slice(idx + 1);
    return [repo, tag || "latest"];
}

const DeploymentRequest = z.object({
    tenant_id: z.string().min(1),

    app_id: z.string().min(1),
    env_id: z.string().min(1),
    release_sha: z.string().min(7),

    // Use artifact_ref as the image ref for Lane A MVP.
    artifact_ref: z.string().min(1),

    lane: z.enum(["LANE_A", "LANE_B"]),

    // MVP Lane A fields for k8s naming
    app_slug: z.string().min(1),   // k8s-safe
    env_slug: z.string().min(1),   // k8s-safe

    desired_routes: z.array(
        z.object({
            host: z.string().optional(),
            path: z.string().default("/"),
        })
    ).optional().default([]),
});

app.post("/v1/deployments", async (req, res) => {
    if (!(await requireAuth(req, res))) return;

    const parsed = DeploymentRequest.safeParse(req.body);
    if (!parsed.success) {
        return res.status(400).json({ error: "invalid_request", details: parsed.error.flatten() });
    }

    const body = parsed.data;
    const deployment_key = `${body.app_id}|${body.env_id}|${body.release_sha}`;

    const existing = getByKey(deployment_key);
    if (existing) {
        return res.status(200).json({
            release_id: existing.release_id,
            status: existing.status,
            endpoints: existing.endpoints,
            logs_pointer: existing.logs_pointer,
            idempotent_replay: true,
        });
    }

    const deploydDbUrl = (await secrets.read("DEPLOYD_DB_URL")) ?? process.env.DEPLOYD_DB_URL ?? null;
    if (!deploydDbUrl) {
        return res.status(500).json({
            error: "missing_secret",
            message: "DEPLOYD_DB_URL not found in secrets mount or env; set Key Vault secret or env var",
        });
    }

    const release_id = `rel_${crypto.randomBytes(8).toString("hex")}`;
    const now = new Date().toISOString();

    // POC: fake endpoints. In real: compute from desired routes + ingress controller.
    const endpoints = body.desired_routes.length
        ? body.desired_routes.map((r) => {
            const host = r.host ?? "unknown-host";
            const path = r.path ?? "/";
            return `https://${host}${path}`;
        })
        : [];

    const rec: DeploymentRecord = {
        deployment_key,
        release_id,
        app_id: body.app_id,
        env_id: body.env_id,
        release_sha: body.release_sha,
        lane: body.lane as Lane,
        artifact_ref: body.artifact_ref,
        created_at: now,
        status: "PENDING",
        events: [{ at: now, type: "requested" }],
        endpoints,
        logs_pointer: `/v1/deployments/${release_id}/logs`,
    };

    if (body.lane === "LANE_A") {
        const ns = `tenant--${body.tenant_id}--app--${body.app_slug}--env--${body.env_slug}`;

        const labels = {
            "stagecraft.ing/tenant-id": body.tenant_id,
            "stagecraft.ing/app-id": body.app_id,
            "stagecraft.ing/env-id": body.env_id,
            "stagecraft.ing/release-sha": body.release_sha,
        };

        rec.events.push({ at: new Date().toISOString(), type: "applying", message: `namespace ${ns}` });
        rec.status = "APPLYING";
        put(rec);

        await ensureNamespaceWithBaseline({ namespace: ns, labels });

        // Parse artifact_ref like "ghcr.io/org/img:tag" or "acr.azurecr.io/img:tag"
        // Supports repo:tag and repo@sha256:digest. If no tag, default to "latest".
        const [repo, tag] = splitImage(body.artifact_ref);

        const chartPath = path.resolve("./charts/tenant-app");
        const releaseName = `${body.app_slug}-${body.env_slug}`;

        // Group routes by host
        const hostsMap = new Map<string, { path: string; pathType: string }[]>();
        for (const route of body.desired_routes) {
            if (!route.host) continue;
            const existing = hostsMap.get(route.host) || [];
            existing.push({ path: route.path, pathType: "Prefix" });
            hostsMap.set(route.host, existing);
        }

        const ingressHosts = Array.from(hostsMap.entries()).map(([host, paths]) => ({ host, paths }));
        const ingressTls = ingressHosts.length > 0
            ? [{ secretName: `${releaseName}-tls`, hosts: ingressHosts.map(h => h.host) }]
            : [];

        await helmUpsertTenantApp({
            releaseName,
            namespace: ns,
            chartPath,
            values: {
                appName: body.app_slug,
                labels,
                imageRepository: repo,
                imageTag: tag,
                ingress: ingressHosts.length > 0
                    ? {
                        enabled: true,
                        hosts: ingressHosts,
                        tls: ingressTls,
                    }
                    : { enabled: false },
            },
        });

        rec.events.push({ at: new Date().toISOString(), type: "rolled_out", message: "helm upgrade --install complete" });
        rec.status = "ROLLED_OUT";
        put(rec);

        return res.status(200).json({
            release_id: rec.release_id,
            status: rec.status,
            endpoints: endpoints,
            logs_pointer: rec.logs_pointer,
        });
    }

    // Fallback for LANE_B conceptually if it existed
    rec.status = "ROLLED_OUT";
    rec.events.push({ at: new Date().toISOString(), type: "rolled_out", message: "POC: no-op rollout for Lane B" });
    put(rec);

    return res.status(200).json({
        release_id: rec.release_id,
        status: rec.status,
        endpoints: rec.endpoints,
        logs_pointer: rec.logs_pointer,
    });
});

app.get("/v1/deployments/:releaseId/status", async (req, res) => {
    if (!(await requireAuth(req, res))) return;
    const rec = getByReleaseId(req.params.releaseId);
    if (!rec) return res.status(404).json({ error: "not_found" });
    return res.status(200).json({ release_id: rec.release_id, status: rec.status, events: rec.events });
});

app.get("/v1/deployments/:releaseId/logs", async (req, res) => {
    if (!(await requireAuth(req, res))) return;
    const rec = getByReleaseId(req.params.releaseId);
    if (!rec) return res.status(404).json({ error: "not_found" });
    // POC: return events as "logs"
    return res.status(200).json({ release_id: rec.release_id, logs: rec.events });
});

app.listen(PORT, () => {
    // eslint-disable-next-line no-console
    console.log(`[deployd-api] listening on :${PORT}; secrets dir: ${SECRETS_DIR}`);
});
