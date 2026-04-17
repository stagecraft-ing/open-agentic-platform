/**
 * Rauthy OIDC integration client.
 *
 * Handles user provisioning, custom-attribute writes, token exchange/refresh,
 * session revocation, and JWT validation against the Rauthy identity provider.
 *
 * Spec 106 FR-003: the admin surface was corrected after verifying Rauthy 0.35
 * source. Admin endpoints live under `/auth/v1/users*` (not `/admin/users*`),
 * session revocation is `/auth/v1/sessions/{user_id}`, and the admin auth
 * scheme is `API-Key <name>$<secret>` rather than `Bearer`. Custom claims
 * are scope-driven (spec 106 FR-002): `validateJwt` reads them from
 * `payload.custom.*` first, falling back to legacy top-level keys for the
 * transition window.
 */

import { secret } from "encore.dev/config";
import log from "encore.dev/log";
import { createVerify } from "crypto";
import { errorForLog } from "./errorLog";

// Rauthy configuration secrets
export const rauthyUrl = secret("RAUTHY_URL"); // e.g. https://rauthy.localdev.online
const rauthyClientId = secret("RAUTHY_CLIENT_ID"); // Stagecraft OIDC client ID
const rauthyClientSecret = secret("RAUTHY_CLIENT_SECRET"); // Stagecraft OIDC client secret
const rauthyAdminToken = secret("RAUTHY_ADMIN_TOKEN"); // Rauthy admin API-Key secret (either `name$secret` or raw secret)
const rauthyAdminTokenName = secret("RAUTHY_ADMIN_TOKEN_NAME"); // Optional API-Key name if RAUTHY_ADMIN_TOKEN is the raw secret

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface RauthyUser {
  id: string;
  email: string;
  given_name?: string;
  family_name?: string;
  enabled: boolean;
}

/**
 * Stagecraft-visible claim shape. Rauthy emits custom scope attributes under
 * `payload.custom.*`; legacy top-level layouts are still accepted for the
 * spec 106 cutover window.
 */
export interface OapClaims {
  sub: string; // Rauthy user ID
  oap_user_id: string; // internal OAP user ID
  oap_org_id: string; // selected org ID
  oap_org_slug: string; // org slug
  oap_workspace_id?: string; // active workspace ID
  github_login?: string; // GitHub handle (absent for enterprise IdP users)
  idp_provider?: string; // identity provider type (github | azure-ad | okta | etc.)
  idp_login?: string; // provider-specific login/display name
  avatar_url?: string; // avatar URL
  platform_role: string; // owner | admin | member
  exp: number;
  iat: number;
}

export interface RauthyTokens {
  access_token: string;
  refresh_token: string;
  id_token: string;
  expires_in: number;
  token_type: string;
}

/**
 * The OAP custom-attribute set written to a Rauthy user. Keys map 1:1 onto
 * the attributes declared by the FR-002 seeder and onto the `oap` scope
 * `attr_include_access` / `attr_include_id` list.
 */
export interface OapUserAttributes {
  oap_user_id: string;
  oap_org_id: string;
  oap_org_slug: string;
  oap_workspace_id?: string;
  github_login?: string;
  idp_provider?: string;
  idp_login?: string;
  avatar_url?: string;
  platform_role: string;
}

// ---------------------------------------------------------------------------
// Admin auth header (spec 106 FR-003)
// ---------------------------------------------------------------------------

/**
 * Build the `API-Key <name>$<secret>` header Rauthy requires for admin calls.
 * Accepts either a fully-formed `name$secret` string in RAUTHY_ADMIN_TOKEN
 * (seeder-compatible) or a raw secret paired with RAUTHY_ADMIN_TOKEN_NAME.
 */
export function buildRauthyAdminAuth(): string {
  const token = rauthyAdminToken();
  if (token.includes("$")) {
    return `API-Key ${token}`;
  }
  const name = rauthyAdminTokenName();
  if (!name) {
    throw new Error("RAUTHY_ADMIN_TOKEN does not contain a `name$secret` pair and RAUTHY_ADMIN_TOKEN_NAME is empty");
  }
  return `API-Key ${name}$${token}`;
}

// ---------------------------------------------------------------------------
// JWKS cache for JWT validation
// ---------------------------------------------------------------------------

interface JwkKey {
  kty: string;
  kid: string;
  n?: string;
  e?: string;
  alg?: string;
  use?: string;
}

let jwksCache: { keys: JwkKey[]; fetchedAt: number } | null = null;
const JWKS_CACHE_TTL_MS = 3600_000; // 1 hour

export async function getJwks(): Promise<JwkKey[]> {
  if (jwksCache && Date.now() - jwksCache.fetchedAt < JWKS_CACHE_TTL_MS) {
    return jwksCache.keys;
  }

  const resp = await fetch(`${rauthyUrl()}/auth/v1/.well-known/jwks.json`);
  if (!resp.ok) {
    throw new Error(`Failed to fetch JWKS: ${resp.status}`);
  }

  const data = (await resp.json()) as { keys: JwkKey[] };
  jwksCache = { keys: data.keys, fetchedAt: Date.now() };
  return data.keys;
}

// ---------------------------------------------------------------------------
// JWT Validation
// ---------------------------------------------------------------------------

/**
 * Pull OAP custom claims out of a raw JWT payload.
 *
 * Rauthy emits scope-mapped attributes under `payload.custom.<name>` (spec
 * 106 FR-002). Older admin-mint sessions put the same keys at the top level;
 * accept both to keep in-flight sessions valid across the cutover.
 */
export function extractOapClaims(payload: Record<string, unknown>): OapClaims | null {
  const custom = (payload.custom as Record<string, unknown> | undefined) ?? {};

  const read = (key: string): string | undefined => {
    const v = custom[key] ?? payload[key];
    return typeof v === "string" && v.length > 0 ? v : undefined;
  };

  const sub = typeof payload.sub === "string" ? payload.sub : "";
  const exp = typeof payload.exp === "number" ? payload.exp : 0;
  const iat = typeof payload.iat === "number" ? payload.iat : 0;

  const oapUserId = read("oap_user_id");
  const oapOrgId = read("oap_org_id");
  const oapOrgSlug = read("oap_org_slug");
  const platformRole = read("platform_role");

  if (!oapUserId || !oapOrgId || !oapOrgSlug || !platformRole) {
    return null;
  }

  return {
    sub,
    oap_user_id: oapUserId,
    oap_org_id: oapOrgId,
    oap_org_slug: oapOrgSlug,
    oap_workspace_id: read("oap_workspace_id"),
    github_login: read("github_login"),
    idp_provider: read("idp_provider"),
    idp_login: read("idp_login"),
    avatar_url: read("avatar_url"),
    platform_role: platformRole,
    exp,
    iat,
  };
}

/**
 * Validate a Rauthy JWT and extract OAP claims.
 * Returns null if the token is invalid or expired.
 */
export async function validateJwt(token: string): Promise<OapClaims | null> {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;

    const headerJson = Buffer.from(parts[0], "base64url").toString();
    const payloadJson = Buffer.from(parts[1], "base64url").toString();
    const header = JSON.parse(headerJson) as { alg: string; kid?: string };
    const payload = JSON.parse(payloadJson) as Record<string, unknown> & {
      iss?: string;
      aud?: string | string[];
      exp?: number;
    };

    // Require and check expiry
    if (!payload.exp || payload.exp < Math.floor(Date.now() / 1000)) {
      return null;
    }

    // Require and check issuer
    const expectedIssuer = `${rauthyUrl()}/auth/v1`;
    if (!payload.iss || payload.iss !== expectedIssuer) {
      return null;
    }

    // Validate audience
    const expectedAud = rauthyClientId();
    const aud = Array.isArray(payload.aud) ? payload.aud : payload.aud ? [payload.aud] : [];
    if (!aud.includes(expectedAud)) {
      return null;
    }

    // Verify signature using JWKS
    const keys = await getJwks();
    const key = header.kid ? keys.find((k) => k.kid === header.kid) : keys.find((k) => k.alg === header.alg || k.use === "sig");

    if (!key || key.kty !== "RSA" || !key.n || !key.e) {
      log.warn("No matching JWKS key found", { kid: header.kid });
      return null;
    }

    // Construct PEM from JWK components
    const pubKey = jwkToPem(key);
    const signatureInput = `${parts[0]}.${parts[1]}`;
    const signature = Buffer.from(parts[2], "base64url");

    const verifier = createVerify("RSA-SHA256");
    verifier.update(signatureInput);
    const valid = verifier.verify(pubKey, signature);

    if (!valid) {
      log.warn("JWT signature verification failed");
      return null;
    }

    return extractOapClaims(payload);
  } catch (err) {
    log.error("JWT validation error", { error: errorForLog(err) });
    return null;
  }
}

function jwkToPem(jwk: JwkKey): string {
  // Build DER-encoded RSA public key from n and e components
  const n = Buffer.from(jwk.n!, "base64url");
  const e = Buffer.from(jwk.e!, "base64url");

  const nEncoded = derEncodeUint(n);
  const eEncoded = derEncodeUint(e);

  const seq = Buffer.concat([Buffer.from([0x30]), derLength(nEncoded.length + eEncoded.length), nEncoded, eEncoded]);

  // Wrap in BIT STRING inside SEQUENCE with RSA OID
  const rsaOid = Buffer.from("300d06092a864886f70d0101010500", "hex");
  const bitString = Buffer.concat([Buffer.from([0x03]), derLength(seq.length + 1), Buffer.from([0x00]), seq]);
  const outer = Buffer.concat([Buffer.from([0x30]), derLength(rsaOid.length + bitString.length), rsaOid, bitString]);

  const b64 = outer.toString("base64");
  const lines = b64.match(/.{1,64}/g) || [];
  return `-----BEGIN PUBLIC KEY-----\n${lines.join("\n")}\n-----END PUBLIC KEY-----`;
}

function derEncodeUint(buf: Buffer): Buffer {
  // Prepend 0x00 if high bit is set (to keep it positive)
  const padded = buf[0] & 0x80 ? Buffer.concat([Buffer.from([0x00]), buf]) : buf;
  return Buffer.concat([Buffer.from([0x02]), derLength(padded.length), padded]);
}

function derLength(len: number): Buffer {
  if (len < 0x80) return Buffer.from([len]);
  if (len < 0x100) return Buffer.from([0x81, len]);
  return Buffer.from([0x82, (len >> 8) & 0xff, len & 0xff]);
}

// ---------------------------------------------------------------------------
// User provisioning — Rauthy admin API (spec 106 FR-003)
// ---------------------------------------------------------------------------

/**
 * Find or create a user in Rauthy. Returns the Rauthy user ID.
 *
 * Endpoints used (Rauthy 0.35):
 *   GET  /auth/v1/users/email/{email}   — 200 returns the user, 404 = unknown
 *   POST /auth/v1/users                 — create (email + minimal profile)
 */
export async function provisionRauthyUser(opts: {
  email: string;
  name: string;
  loginHint?: string; // GitHub login, enterprise username, etc. (fallback for given_name)
  githubLogin?: string; // kept for backward compat — alias for loginHint
}): Promise<string> {
  const baseUrl = rauthyUrl();
  const adminAuth = buildRauthyAdminAuth();
  const hint = opts.loginHint ?? opts.githubLogin ?? "";

  // Look up by email
  const lookupResp = await fetch(`${baseUrl}/auth/v1/users/email/${encodeURIComponent(opts.email)}`, {
    headers: { Authorization: adminAuth, Accept: "application/json" },
  });

  if (lookupResp.ok) {
    const existing = (await lookupResp.json()) as RauthyUser;
    if (existing?.id) {
      log.info("Found existing Rauthy user", { rauthyUserId: existing.id });
      return existing.id;
    }
  } else if (lookupResp.status !== 404) {
    const body = await lookupResp.text();
    throw new Error(`Rauthy user lookup failed: ${lookupResp.status} ${body.slice(0, 300)}`);
  }

  // Create new user
  const [givenName, ...familyParts] = opts.name.split(" ");
  const createResp = await fetch(`${baseUrl}/auth/v1/users`, {
    method: "POST",
    headers: {
      Authorization: adminAuth,
      "Content-Type": "application/json",
      Accept: "application/json",
    },
    body: JSON.stringify({
      email: opts.email,
      given_name: givenName || hint,
      family_name: familyParts.join(" ") || "",
      enabled: true,
    }),
  });

  if (!createResp.ok) {
    const body = await createResp.text();
    throw new Error(`Rauthy user creation failed: ${createResp.status} ${body.slice(0, 300)}`);
  }

  const created = (await createResp.json()) as RauthyUser;
  log.info("Created Rauthy user", { rauthyUserId: created.id });
  return created.id;
}

/**
 * Write the full OAP custom-attribute set for a Rauthy user.
 *
 * Uses `PUT /auth/v1/users/{id}/attr` which accepts
 * `{ values: { <attr_name>: <string_value> } }` (Rauthy 0.35). Called on
 * first login and whenever the user's org, workspace, or role context
 * changes. The attribute names must match the ones seeded in FR-002 and
 * the ones mapped by the `oap` scope.
 */
export async function setRauthyUserAttributes(rauthyUserId: string, attrs: OapUserAttributes): Promise<void> {
  const baseUrl = rauthyUrl();
  const adminAuth = buildRauthyAdminAuth();

  // Rauthy stores attribute values as strings; empty strings are allowed.
  const values: Record<string, string> = {
    oap_user_id: attrs.oap_user_id,
    oap_org_id: attrs.oap_org_id,
    oap_org_slug: attrs.oap_org_slug,
    oap_workspace_id: attrs.oap_workspace_id ?? "",
    github_login: attrs.github_login ?? "",
    idp_provider: attrs.idp_provider ?? "",
    idp_login: attrs.idp_login ?? "",
    avatar_url: attrs.avatar_url ?? "",
    platform_role: attrs.platform_role,
  };

  const resp = await fetch(`${baseUrl}/auth/v1/users/${rauthyUserId}/attr`, {
    method: "PUT",
    headers: {
      Authorization: adminAuth,
      "Content-Type": "application/json",
      Accept: "application/json",
    },
    body: JSON.stringify({ values }),
  });

  if (!resp.ok) {
    const body = await resp.text();
    throw new Error(`Rauthy attribute write failed: ${resp.status} ${body.slice(0, 300)}`);
  }
}

// ---------------------------------------------------------------------------
// OIDC token endpoints (standard flow — spec 106 FR-003)
// ---------------------------------------------------------------------------

/**
 * Exchange an authorization code for Rauthy tokens.
 * Used in the OAuth callback after Rauthy-driven login.
 */
export async function exchangeCodeForTokens(code: string, redirectUri: string): Promise<RauthyTokens> {
  const baseUrl = rauthyUrl();

  const body = new URLSearchParams();
  body.set("grant_type", "authorization_code");
  body.set("code", code);
  body.set("redirect_uri", redirectUri);
  body.set("client_id", rauthyClientId());
  body.set("client_secret", rauthyClientSecret());

  const resp = await fetch(`${baseUrl}/auth/v1/oidc/token`, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body,
  });

  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`Rauthy token exchange failed: ${resp.status} ${text}`);
  }

  return (await resp.json()) as RauthyTokens;
}

/**
 * Build the Rauthy authorization URL for initiating login.
 */
export function buildAuthorizationUrl(opts: {
  redirectUri: string;
  state: string;
  scopes?: string[];
  idpHint?: string;
  codeChallenge?: string;
  codeChallengeMethod?: "S256";
}): string {
  const baseUrl = rauthyUrl();
  const scopes = opts.scopes ?? ["openid", "profile", "email"];

  const params = new URLSearchParams({
    response_type: "code",
    client_id: rauthyClientId(),
    redirect_uri: opts.redirectUri,
    scope: scopes.join(" "),
    state: opts.state,
  });

  if (opts.idpHint) params.set("upstream_auth_provider_id", opts.idpHint);
  if (opts.codeChallenge) {
    params.set("code_challenge", opts.codeChallenge);
    params.set("code_challenge_method", opts.codeChallengeMethod ?? "S256");
  }

  return `${baseUrl}/auth/v1/oidc/authorize?${params.toString()}`;
}

/**
 * Refresh an expired access token using a refresh token.
 */
export async function refreshTokens(refreshToken: string): Promise<RauthyTokens> {
  const baseUrl = rauthyUrl();

  const body = new URLSearchParams();
  body.set("grant_type", "refresh_token");
  body.set("refresh_token", refreshToken);
  body.set("client_id", rauthyClientId());
  body.set("client_secret", rauthyClientSecret());

  const resp = await fetch(`${baseUrl}/auth/v1/oidc/token`, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body,
  });

  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`Rauthy token refresh failed: ${resp.status} ${text}`);
  }

  return (await resp.json()) as RauthyTokens;
}

/**
 * @deprecated Spec 106 FR-003 removed the admin-mint endpoint — Rauthy 0.35
 * has no `POST /auth/v1/admin/users/:id/sessions` surface. This function is
 * kept as a hard-failing stub so the module surface stays stable while
 * FR-004 rewires call sites (`api/auth/github.ts`, `api/auth/oidc.ts`,
 * `api/auth/desktop.ts`) onto `setRauthyUserAttributes` + `refreshTokens`.
 * Any runtime call is a programmer error and must be fixed at the caller.
 */
export async function issueRauthySession(_opts: {
  rauthyUserId: string;
  oapUserId: string;
  orgId: string;
  orgSlug: string;
  workspaceId: string;
  githubLogin?: string;
  idpProvider?: string;
  idpLogin?: string;
  avatarUrl?: string;
  platformRole: string;
}): Promise<{ accessToken: string; expiresIn: number }> {
  throw new Error(
    "issueRauthySession was removed in spec 106 FR-003. Rauthy 0.35 has no admin-mint endpoint. Use setRauthyUserAttributes + refreshTokens via /auth/rauthy/callback (FR-004)."
  );
}

/**
 * Revoke all active Rauthy sessions for a user.
 * Uses `DELETE /auth/v1/sessions/{user_id}` with admin API-Key auth.
 */
export async function revokeSession(rauthyUserId: string): Promise<void> {
  const baseUrl = rauthyUrl();
  const adminAuth = buildRauthyAdminAuth();

  const resp = await fetch(`${baseUrl}/auth/v1/sessions/${rauthyUserId}`, {
    method: "DELETE",
    headers: { Authorization: adminAuth },
  });

  if (!resp.ok && resp.status !== 404) {
    log.warn("Failed to revoke Rauthy sessions", {
      rauthyUserId,
      status: resp.status,
    });
  }
}
