/**
 * Shared M2M authentication middleware for platform seams (spec 082 Phase 2).
 *
 * Validates machine-to-machine requests using OIDC JWT first, with a static
 * token fallback for backward compatibility.
 *
 * M2M JWTs use client_credentials grant and carry a `scope` claim, which
 * differs from user JWTs that carry `OapClaims` (oap_user_id, github_login,
 * etc.). This middleware validates M2M tokens separately from user tokens.
 */

import { APIError } from "encore.dev/api";
import { getJwks, rauthyUrl } from "./rauthy.js";
import { createVerify } from "node:crypto";
import log from "encore.dev/log";

const M2M_TOKEN = process.env.PLATFORM_M2M_TOKEN;

/** Minimal claims expected in an M2M client_credentials JWT. */
interface M2mClaims {
  sub: string;
  scope?: string;
  exp: number;
  iss?: string;
  aud?: string | string[];
}

/**
 * Validate an M2M request using OIDC JWT with static token fallback.
 * Throws APIError.unauthenticated() on failure.
 *
 * @param authorization - Raw value of the Authorization header (e.g. "Bearer <token>").
 * @param requiredScope - The OAuth2 scope the token must include (e.g. "platform:audit:write").
 */
export async function validateM2mRequest(
  authorization: string | undefined,
  requiredScope: string
): Promise<void> {
  if (!authorization) {
    throw APIError.unauthenticated("missing authorization header");
  }

  const token = authorization.startsWith("Bearer ")
    ? authorization.slice(7)
    : authorization;

  if (!token) {
    throw APIError.unauthenticated("empty bearer token");
  }

  // Try M2M JWT validation first.
  const claims = await validateM2mJwt(token);
  if (claims) {
    const scopes = claims.scope?.split(" ") ?? [];
    if (scopes.includes(requiredScope)) {
      return; // Valid JWT with required scope.
    }
    throw APIError.permissionDenied(`missing required scope: ${requiredScope}`);
  }

  // Fallback: static M2M token comparison.
  if (M2M_TOKEN && token === M2M_TOKEN) {
    return; // Static token match.
  }

  throw APIError.unauthenticated("invalid or missing bearer token");
}

/**
 * Validate an M2M JWT (client_credentials grant).
 *
 * Unlike validateJwt() in rauthy.ts, this:
 * - Does NOT check audience (M2M tokens may target any resource)
 * - Returns M2mClaims (with scope) instead of OapClaims (with user fields)
 * - Still verifies issuer, expiry, and RS256 signature via JWKS
 */
async function validateM2mJwt(token: string): Promise<M2mClaims | null> {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;

    const headerJson = Buffer.from(parts[0], "base64url").toString();
    const payloadJson = Buffer.from(parts[1], "base64url").toString();
    const header = JSON.parse(headerJson) as { alg: string; kid?: string };
    const payload = JSON.parse(payloadJson) as M2mClaims;

    // Check expiry
    if (!payload.exp || payload.exp < Math.floor(Date.now() / 1000)) {
      return null;
    }

    // Check issuer (same Rauthy instance)
    const expectedIssuer = `${rauthyUrl()}/auth/v1`;
    if (payload.iss && payload.iss !== expectedIssuer) {
      return null;
    }

    // Verify RS256 signature using JWKS
    const keys = await getJwks();
    const key = header.kid
      ? keys.find((k) => k.kid === header.kid)
      : keys.find((k) => k.alg === header.alg || k.use === "sig");

    if (!key || key.kty !== "RSA" || !key.n || !key.e) {
      log.warn("No matching JWKS key for M2M token", { kid: header.kid });
      return null;
    }

    // Construct PEM from JWK components (n and e guaranteed by guard above)
    const pubKey = jwkToPem({ n: key.n!, e: key.e! });
    const signatureInput = `${parts[0]}.${parts[1]}`;
    const signature = Buffer.from(parts[2], "base64url");

    const verifier = createVerify("RSA-SHA256");
    verifier.update(signatureInput);
    const valid = verifier.verify(pubKey, signature);

    if (!valid) {
      log.warn("M2M JWT signature verification failed");
      return null;
    }

    return payload;
  } catch (e) {
    log.warn("M2M JWT validation error", { error: String(e) });
    return null;
  }
}

/** Convert a JWK RSA public key to PEM format. */
function jwkToPem(key: { n: string; e: string }): string {
  const n = Buffer.from(key.n, "base64url");
  const e = Buffer.from(key.e, "base64url");

  // DER encode the RSA public key
  const nBytes = encodeUnsignedInteger(n);
  const eBytes = encodeUnsignedInteger(e);
  const seq = encodeSequence(Buffer.concat([nBytes, eBytes]));
  const bitString = encodeBitString(seq);
  const algorithmIdentifier = Buffer.from(
    "300d06092a864886f70d0101010500",
    "hex"
  ); // RSA OID
  const outer = encodeSequence(
    Buffer.concat([algorithmIdentifier, bitString])
  );

  const b64 = outer.toString("base64");
  const lines = b64.match(/.{1,64}/g) || [];
  return `-----BEGIN PUBLIC KEY-----\n${lines.join("\n")}\n-----END PUBLIC KEY-----`;
}

function encodeLength(len: number): Buffer {
  if (len < 0x80) return Buffer.from([len]);
  if (len < 0x100) return Buffer.from([0x81, len]);
  return Buffer.from([0x82, (len >> 8) & 0xff, len & 0xff]);
}

function encodeUnsignedInteger(buf: Buffer): Buffer {
  // Ensure positive integer (prepend 0x00 if high bit set)
  const padded = buf[0] & 0x80 ? Buffer.concat([Buffer.from([0]), buf]) : buf;
  return Buffer.concat([Buffer.from([0x02]), encodeLength(padded.length), padded]);
}

function encodeSequence(buf: Buffer): Buffer {
  return Buffer.concat([Buffer.from([0x30]), encodeLength(buf.length), buf]);
}

function encodeBitString(buf: Buffer): Buffer {
  const content = Buffer.concat([Buffer.from([0x00]), buf]); // 0 unused bits
  return Buffer.concat([Buffer.from([0x03]), encodeLength(content.length), content]);
}
