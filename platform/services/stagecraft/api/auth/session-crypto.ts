/**
 * HMAC-signed session cookie utilities (spec 080).
 *
 * Signs and verifies session payloads using HMAC-SHA256 to prevent
 * cookie forgery. This is the transitional mechanism before full
 * Rauthy JWT issuance is wired.
 */

import { createHmac, timingSafeEqual } from "crypto";
import { secret } from "encore.dev/config";

const sessionSecret = secret("SESSION_SECRET"); // 32+ char random secret

/**
 * Sign a JSON payload and return `payload.signature` format.
 */
export function signPayload(data: object): string {
  const payload = Buffer.from(JSON.stringify(data)).toString("base64url");
  const sig = createHmac("sha256", sessionSecret())
    .update(payload)
    .digest("base64url");
  return `${payload}.${sig}`;
}

/**
 * Verify a signed cookie value and return the parsed payload.
 * Returns null if the signature is invalid or the payload can't be parsed.
 */
export function verifyPayload<T = unknown>(signed: string): T | null {
  const dotIdx = signed.lastIndexOf(".");
  if (dotIdx === -1) return null;

  const payload = signed.substring(0, dotIdx);
  const sig = signed.substring(dotIdx + 1);

  const expected = createHmac("sha256", sessionSecret())
    .update(payload)
    .digest("base64url");

  try {
    if (!timingSafeEqual(Buffer.from(sig), Buffer.from(expected))) {
      return null;
    }
  } catch {
    return null; // length mismatch
  }

  try {
    return JSON.parse(Buffer.from(payload, "base64url").toString()) as T;
  } catch {
    return null;
  }
}
