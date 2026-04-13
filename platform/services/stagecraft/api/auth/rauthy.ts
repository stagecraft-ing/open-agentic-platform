/**
 * Rauthy OIDC integration client (spec 080 FR-003).
 *
 * Handles user provisioning, session issuance, and JWT validation
 * against the Rauthy identity provider.
 */

import { secret } from "encore.dev/config";
import log from "encore.dev/log";
import { createVerify } from "crypto";

// Rauthy configuration secrets
export const rauthyUrl = secret("RAUTHY_URL");                     // e.g. https://rauthy.localdev.online
const rauthyClientId = secret("RAUTHY_CLIENT_ID");          // Stagecraft OIDC client ID
const rauthyClientSecret = secret("RAUTHY_CLIENT_SECRET");  // Stagecraft OIDC client secret
const rauthyAdminToken = secret("RAUTHY_ADMIN_TOKEN");      // Rauthy admin API bearer token

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

export interface OapClaims {
  sub: string;              // Rauthy user ID
  oap_user_id: string;      // internal OAP user ID
  oap_org_id: string;        // selected org ID
  oap_org_slug: string;      // org slug
  oap_workspace_id?: string; // active workspace ID
  github_login: string;      // GitHub handle
  avatar_url?: string;       // GitHub avatar URL
  platform_role: string;     // owner | admin | member
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

  const resp = await fetch(
    `${rauthyUrl()}/auth/v1/.well-known/jwks.json`
  );
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
    const payload = JSON.parse(payloadJson) as OapClaims & {
      iss?: string;
      aud?: string | string[];
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
    const key = header.kid
      ? keys.find((k) => k.kid === header.kid)
      : keys.find((k) => k.alg === header.alg || k.use === "sig");

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

    return payload;
  } catch (err) {
    log.error("JWT validation error", { error: String(err) });
    return null;
  }
}

function jwkToPem(jwk: JwkKey): string {
  // Build DER-encoded RSA public key from n and e components
  const n = Buffer.from(jwk.n!, "base64url");
  const e = Buffer.from(jwk.e!, "base64url");

  const nEncoded = derEncodeUint(n);
  const eEncoded = derEncodeUint(e);

  const seq = Buffer.concat([
    Buffer.from([0x30]),
    derLength(nEncoded.length + eEncoded.length),
    nEncoded,
    eEncoded,
  ]);

  // Wrap in BIT STRING inside SEQUENCE with RSA OID
  const rsaOid = Buffer.from(
    "300d06092a864886f70d0101010500",
    "hex"
  );
  const bitString = Buffer.concat([
    Buffer.from([0x03]),
    derLength(seq.length + 1),
    Buffer.from([0x00]),
    seq,
  ]);
  const outer = Buffer.concat([
    Buffer.from([0x30]),
    derLength(rsaOid.length + bitString.length),
    rsaOid,
    bitString,
  ]);

  const b64 = outer.toString("base64");
  const lines = b64.match(/.{1,64}/g) || [];
  return `-----BEGIN PUBLIC KEY-----\n${lines.join("\n")}\n-----END PUBLIC KEY-----`;
}

function derEncodeUint(buf: Buffer): Buffer {
  // Prepend 0x00 if high bit is set (to keep it positive)
  const padded = buf[0] & 0x80 ? Buffer.concat([Buffer.from([0x00]), buf]) : buf;
  return Buffer.concat([
    Buffer.from([0x02]),
    derLength(padded.length),
    padded,
  ]);
}

function derLength(len: number): Buffer {
  if (len < 0x80) return Buffer.from([len]);
  if (len < 0x100) return Buffer.from([0x81, len]);
  return Buffer.from([0x82, (len >> 8) & 0xff, len & 0xff]);
}

// ---------------------------------------------------------------------------
// User Provisioning (Rauthy Admin API)
// ---------------------------------------------------------------------------

/**
 * Find or create a user in Rauthy, linking to GitHub identity.
 * Returns the Rauthy user ID.
 */
export async function provisionRauthyUser(opts: {
  email: string;
  githubLogin: string;
  name: string;
}): Promise<string> {
  const baseUrl = rauthyUrl();
  const adminAuth = `Bearer ${rauthyAdminToken()}`;

  // Try to find existing user by email
  const searchResp = await fetch(
    `${baseUrl}/auth/v1/admin/users?email=${encodeURIComponent(opts.email)}`,
    { headers: { Authorization: adminAuth } }
  );

  if (searchResp.ok) {
    const users = (await searchResp.json()) as RauthyUser[];
    const existing = users.find(
      (u) => u.email.toLowerCase() === opts.email.toLowerCase()
    );
    if (existing) {
      log.info("Found existing Rauthy user", { rauthyUserId: existing.id });
      return existing.id;
    }
  }

  // Create new user
  const [givenName, ...familyParts] = opts.name.split(" ");
  const createResp = await fetch(`${baseUrl}/auth/v1/admin/users`, {
    method: "POST",
    headers: {
      Authorization: adminAuth,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      email: opts.email,
      given_name: givenName || opts.githubLogin,
      family_name: familyParts.join(" ") || "",
      enabled: true,
    }),
  });

  if (!createResp.ok) {
    const body = await createResp.text();
    throw new Error(`Rauthy user creation failed: ${createResp.status} ${body}`);
  }

  const created = (await createResp.json()) as RauthyUser;
  log.info("Created Rauthy user", { rauthyUserId: created.id });
  return created.id;
}

// ---------------------------------------------------------------------------
// Session Token Issuance
// ---------------------------------------------------------------------------

/**
 * Exchange an authorization code for Rauthy tokens.
 * Used in the OAuth callback after GitHub login.
 */
export async function exchangeCodeForTokens(
  code: string,
  redirectUri: string
): Promise<RauthyTokens> {
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

  return `${baseUrl}/auth/v1/authorize?${params.toString()}`;
}

/**
 * Refresh an expired access token using a refresh token.
 */
export async function refreshTokens(
  refreshToken: string
): Promise<RauthyTokens> {
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
 * Issue a Rauthy session for a provisioned user (spec 087 Phase 5).
 *
 * After GitHub OAuth completes and the user is provisioned in Rauthy, we call
 * Rauthy's admin API to create a session and obtain an access token (JWT).
 * This replaces the transitional HMAC-signed session cookie.
 *
 * The Rauthy admin API's /auth/v1/admin/users/:id/sessions endpoint creates
 * a session and returns tokens. The access_token is a signed JWT that our
 * auth handler can validate via JWKS.
 */
export async function issueRauthySession(opts: {
  rauthyUserId: string;
  oapUserId: string;
  orgId: string;
  orgSlug: string;
  workspaceId: string;
  githubLogin: string;
  avatarUrl?: string;
  platformRole: string;
}): Promise<{ accessToken: string; expiresIn: number }> {
  const baseUrl = rauthyUrl();
  const adminAuth = `Bearer ${rauthyAdminToken()}`;

  const resp = await fetch(
    `${baseUrl}/auth/v1/admin/users/${opts.rauthyUserId}/sessions`,
    {
      method: "POST",
      headers: {
        Authorization: adminAuth,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        client_id: rauthyClientId(),
        scope: "openid profile email",
        custom_claims: {
          oap_user_id: opts.oapUserId,
          oap_org_id: opts.orgId,
          oap_org_slug: opts.orgSlug,
          oap_workspace_id: opts.workspaceId,
          github_login: opts.githubLogin,
          avatar_url: opts.avatarUrl ?? "",
          platform_role: opts.platformRole,
        },
      }),
    }
  );

  if (!resp.ok) {
    const body = await resp.text();
    throw new Error(`Rauthy session issuance failed: ${resp.status} ${body}`);
  }

  const data = (await resp.json()) as {
    access_token: string;
    expires_in: number;
    token_type: string;
  };

  return { accessToken: data.access_token, expiresIn: data.expires_in };
}

/**
 * Revoke a Rauthy session (used when org membership is removed).
 */
export async function revokeSession(rauthyUserId: string): Promise<void> {
  const baseUrl = rauthyUrl();
  const adminAuth = `Bearer ${rauthyAdminToken()}`;

  const resp = await fetch(
    `${baseUrl}/auth/v1/admin/users/${rauthyUserId}/sessions`,
    {
      method: "DELETE",
      headers: { Authorization: adminAuth },
    }
  );

  if (!resp.ok && resp.status !== 404) {
    log.warn("Failed to revoke Rauthy sessions", {
      rauthyUserId,
      status: resp.status,
    });
  }
}
