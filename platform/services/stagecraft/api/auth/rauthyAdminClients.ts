// Spec 137 Phase 3 — Rauthy admin client provisioning for tenant gates.
//
// Wraps the four-verb Rauthy admin API (GET / POST / PUT / DELETE on
// `/auth/v1/clients`) with stagecraft's idempotent provision +
// deprovision flows for per-environment OIDC clients.
//
// Empirical contract (spec 137 T003, 2026-05-15):
// - Auth: `Authorization: API-Key <name>$<secret>` (not Bearer).
// - Update verb: full-object PUT (no PATCH endpoint exists).
// - Delete: 200 OK (not 204).
// - 14-field client schema; no `password_login_enabled` / `auth_provider_id`.
// - Admin endpoints are cluster-internal-only (PROXY_MODE rejects
//   external origins). In tests we point fetchOverride at a stub
//   server; in production stagecraft-api runs in the trusted CIDR.

import log from "encore.dev/log";
import { rauthyUrl, buildRauthyAdminAuth } from "./rauthy";
import {
  assertNoPasswordFlow,
  buildTenantGateClientPayload,
  type RauthyClientPayload,
  type TenantGateClientSpec,
} from "./rauthyAdminClientsHelpers";

export {
  tenantGateClientId,
  tenantGateRedirectUri,
  buildTenantGateClientPayload,
  assertNoPasswordFlow,
  type TenantGateClientSpec,
  type RauthyClientPayload,
} from "./rauthyAdminClientsHelpers";

// ---------------------------------------------------------------------------
// Low-level admin verbs (fetch-injectable for tests)
// ---------------------------------------------------------------------------

type FetchLike = typeof globalThis.fetch;

interface AdminCallOptions {
  fetchImpl?: FetchLike;
  baseUrl?: string;
  authHeader?: string;
}

function resolveAdminContext(opts?: AdminCallOptions): {
  baseUrl: string;
  auth: string;
  fetchImpl: FetchLike;
} {
  return {
    baseUrl: (opts?.baseUrl ?? rauthyUrl()).replace(/\/+$/, ""),
    auth: opts?.authHeader ?? buildRauthyAdminAuth(),
    fetchImpl: opts?.fetchImpl ?? globalThis.fetch,
  };
}

export async function getRauthyClient(
  clientId: string,
  opts?: AdminCallOptions,
): Promise<RauthyClientPayload | null> {
  const { baseUrl, auth, fetchImpl } = resolveAdminContext(opts);
  const resp = await fetchImpl(
    `${baseUrl}/auth/v1/clients/${encodeURIComponent(clientId)}`,
    { headers: { Authorization: auth, Accept: "application/json" } },
  );
  if (resp.status === 404) return null;
  if (!resp.ok) {
    const body = await resp.text();
    throw new Error(`getRauthyClient ${clientId} failed: ${resp.status} ${body.slice(0, 300)}`);
  }
  return (await resp.json()) as RauthyClientPayload;
}

/**
 * POST /auth/v1/clients. Returns the freshly-minted client's secret —
 * Rauthy issues the confidential-client secret exactly once at create
 * time and never exposes it on GET (T003 readback, 14-field schema).
 * Stagecraft must capture it here or lose it.
 *
 * Defensive shape parsing: Rauthy 0.35's POST response body is parsed
 * once and the secret extracted from `secret` or `client_secret`
 * (whichever is present). If neither field is present, this throws —
 * the deploy descriptor cannot be assembled without it and silent
 * fallback would surface as a useless oauth2-proxy that 401s every
 * request. Fail loud at the boundary.
 */
export async function createRauthyClient(
  payload: RauthyClientPayload,
  opts?: AdminCallOptions,
): Promise<{ clientSecret: string }> {
  assertNoPasswordFlow(payload);
  const { baseUrl, auth, fetchImpl } = resolveAdminContext(opts);
  const resp = await fetchImpl(`${baseUrl}/auth/v1/clients`, {
    method: "POST",
    headers: {
      Authorization: auth,
      "Content-Type": "application/json",
      Accept: "application/json",
    },
    body: JSON.stringify(payload),
  });
  if (!resp.ok) {
    const body = await resp.text();
    throw new Error(`createRauthyClient ${payload.id} failed: ${resp.status} ${body.slice(0, 400)}`);
  }
  // Rauthy 0.35 returns the created client in the response body. The
  // secret may live under either `secret` or `client_secret` depending
  // on the version's serialiser conventions — accept either, demand
  // one, never silently default.
  let body: unknown;
  try {
    body = await resp.json();
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    throw new Error(
      `createRauthyClient ${payload.id} returned non-JSON body: ${msg}`,
    );
  }
  const secret = extractClientSecret(body);
  if (!secret) {
    throw new Error(
      `createRauthyClient ${payload.id} succeeded but response contained no client secret ` +
        `(expected 'secret' or 'client_secret' field). Rauthy admin contract drift — ` +
        `re-run T003 smoke against this Rauthy version to capture the new shape.`,
    );
  }
  return { clientSecret: secret };
}

function extractClientSecret(body: unknown): string | null {
  if (!body || typeof body !== "object") return null;
  const obj = body as Record<string, unknown>;
  const candidates = [obj.secret, obj.client_secret, obj.clientSecret];
  for (const c of candidates) {
    if (typeof c === "string" && c.length > 0) return c;
  }
  return null;
}

export async function putRauthyClient(
  payload: RauthyClientPayload,
  opts?: AdminCallOptions,
): Promise<void> {
  assertNoPasswordFlow(payload);
  const { baseUrl, auth, fetchImpl } = resolveAdminContext(opts);
  const resp = await fetchImpl(
    `${baseUrl}/auth/v1/clients/${encodeURIComponent(payload.id)}`,
    {
      method: "PUT",
      headers: {
        Authorization: auth,
        "Content-Type": "application/json",
        Accept: "application/json",
      },
      body: JSON.stringify(payload),
    },
  );
  if (!resp.ok) {
    const body = await resp.text();
    throw new Error(`putRauthyClient ${payload.id} failed: ${resp.status} ${body.slice(0, 400)}`);
  }
}

export async function deleteRauthyClient(
  clientId: string,
  opts?: AdminCallOptions,
): Promise<{ existed: boolean }> {
  const { baseUrl, auth, fetchImpl } = resolveAdminContext(opts);
  const resp = await fetchImpl(
    `${baseUrl}/auth/v1/clients/${encodeURIComponent(clientId)}`,
    { method: "DELETE", headers: { Authorization: auth } },
  );
  if (resp.status === 404) return { existed: false };
  if (!resp.ok) {
    const body = await resp.text();
    throw new Error(`deleteRauthyClient ${clientId} failed: ${resp.status} ${body.slice(0, 400)}`);
  }
  return { existed: true };
}

// ---------------------------------------------------------------------------
// Tenant gate domain operations (idempotent)
// ---------------------------------------------------------------------------

export interface ProvisionResult {
  clientId: string;
  /** `created` on first provision, `updated` on subsequent runs against an existing client. */
  action: "created" | "updated";
  /**
   * Non-null on `action === "created"` only. Rauthy issues the
   * confidential-client secret once at POST time and never on GET; the
   * caller MUST persist it (descriptor row) to be able to assemble the
   * deploy descriptor on subsequent calls. On `"updated"`, the existing
   * secret remains valid — the caller keeps its previously-persisted
   * value.
   */
  clientSecret: string | null;
}

/**
 * Idempotent create-or-update of a tenant gate Rauthy client. Returns
 * the client_id stagecraft writes into
 * `environment_access_gates.rauthy_client_ref`.
 *
 * Two branches:
 *   - Client absent → POST /auth/v1/clients (action="created")
 *   - Client present → PUT  /auth/v1/clients/:id with the full
 *                            stagecraft-authoritative payload
 *                            (action="updated"). Per T003, Rauthy 0.35
 *                            has no PATCH endpoint; PUT is the only
 *                            non-destructive update verb.
 *
 * FR-004 guard: assertNoPasswordFlow runs inside both create and put,
 * so even a future code path that builds a payload by hand without
 * `buildTenantGateClientPayload` cannot accidentally enable password
 * grant.
 */
export async function provisionTenantGateClient(
  spec: TenantGateClientSpec,
  opts?: AdminCallOptions,
): Promise<ProvisionResult> {
  const payload = buildTenantGateClientPayload(spec);
  const existing = await getRauthyClient(spec.clientId, opts);
  if (existing === null) {
    const { clientSecret } = await createRauthyClient(payload, opts);
    log.info("rauthy.tenant_gate.client.created", { clientId: spec.clientId });
    return { clientId: spec.clientId, action: "created", clientSecret };
  }
  await putRauthyClient(payload, opts);
  log.info("rauthy.tenant_gate.client.updated", { clientId: spec.clientId });
  return { clientId: spec.clientId, action: "updated", clientSecret: null };
}

/**
 * Idempotent delete. Returns `{ existed }` so the caller can audit
 * "we tried, here's whether there was anything to remove."
 */
export async function deprovisionTenantGateClient(
  clientId: string,
  opts?: AdminCallOptions,
): Promise<{ existed: boolean }> {
  const result = await deleteRauthyClient(clientId, opts);
  log.info("rauthy.tenant_gate.client.deprovisioned", {
    clientId,
    existed: result.existed,
  });
  return result;
}
