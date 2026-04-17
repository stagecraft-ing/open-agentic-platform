/**
 * AES-256-GCM helpers for PAT storage at rest (spec 106 FR-006).
 *
 * The full CRUD surface (POST/DELETE/GET/validate `/auth/pat`) is implemented
 * in FR-006. The membership resolver depends on `decryptPat` during login.
 *
 * Key: 32 random bytes, base64-encoded, supplied through the
 * `PAT_ENCRYPTION_KEY` Encore secret. Each row uses a fresh per-row nonce
 * (`token_nonce`); ciphertext (`token_enc`) includes the GCM auth tag.
 *
 * The core `encryptWithKey`/`decryptWithKey` helpers are exported so the
 * unit test can exercise the crypto without needing the Encore secret
 * runtime binding. Production code uses `encryptPat` / `decryptPat`, which
 * load the key from the secret on every call.
 */

import { createCipheriv, createDecipheriv, randomBytes } from "crypto";
import { secret } from "encore.dev/config";

const patEncryptionKey = secret("PAT_ENCRYPTION_KEY");

const KEY_BYTES = 32; // AES-256
const NONCE_BYTES = 12; // GCM recommended
const TAG_BYTES = 16;

function loadKey(): Buffer {
  const raw = patEncryptionKey();
  if (!raw) {
    throw new Error("PAT_ENCRYPTION_KEY secret is not set");
  }
  const key = Buffer.from(raw, "base64");
  if (key.length !== KEY_BYTES) {
    throw new Error(
      `PAT_ENCRYPTION_KEY must decode to ${KEY_BYTES} bytes (got ${key.length}). ` +
        "Generate with: openssl rand -base64 32"
    );
  }
  return key;
}

export function encryptWithKey(
  key: Buffer,
  plaintext: string
): { tokenEnc: Buffer; tokenNonce: Buffer } {
  if (key.length !== KEY_BYTES) {
    throw new Error(`encryption key must be ${KEY_BYTES} bytes (got ${key.length})`);
  }
  const nonce = randomBytes(NONCE_BYTES);
  const cipher = createCipheriv("aes-256-gcm", key, nonce);
  const ciphertext = Buffer.concat([cipher.update(plaintext, "utf-8"), cipher.final()]);
  const tag = cipher.getAuthTag();
  return { tokenEnc: Buffer.concat([ciphertext, tag]), tokenNonce: nonce };
}

export function decryptWithKey(
  key: Buffer,
  tokenEnc: Buffer,
  tokenNonce: Buffer
): string {
  if (key.length !== KEY_BYTES) {
    throw new Error(`encryption key must be ${KEY_BYTES} bytes (got ${key.length})`);
  }
  if (tokenEnc.length <= TAG_BYTES) {
    throw new Error("Stored PAT ciphertext is shorter than the GCM tag size");
  }
  if (tokenNonce.length !== NONCE_BYTES) {
    throw new Error(`Stored PAT nonce is ${tokenNonce.length} bytes (expected ${NONCE_BYTES})`);
  }
  const ciphertext = tokenEnc.subarray(0, tokenEnc.length - TAG_BYTES);
  const tag = tokenEnc.subarray(tokenEnc.length - TAG_BYTES);
  const decipher = createDecipheriv("aes-256-gcm", key, tokenNonce);
  decipher.setAuthTag(tag);
  return Buffer.concat([decipher.update(ciphertext), decipher.final()]).toString("utf-8");
}

/** Encrypt a PAT using the PAT_ENCRYPTION_KEY secret. */
export function encryptPat(plaintext: string): { tokenEnc: Buffer; tokenNonce: Buffer } {
  return encryptWithKey(loadKey(), plaintext);
}

/** Decrypt a stored PAT. Throws on tag-mismatch / key-drift. */
export function decryptPat(tokenEnc: Buffer, tokenNonce: Buffer): string {
  return decryptWithKey(loadKey(), tokenEnc, tokenNonce);
}
