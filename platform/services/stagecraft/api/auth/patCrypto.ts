/**
 * AES-256-GCM helpers for PAT storage at rest (spec 106 FR-006).
 *
 * The full CRUD surface (POST/DELETE/GET/validate `/auth/pat`) is implemented
 * in FR-006. This file is introduced in FR-005 because the membership
 * resolver must be able to decrypt a stored PAT during login. Only the
 * symmetric primitives live here — storage and validation logic belong
 * to the endpoint module.
 *
 * Key: 32 random bytes, base64-encoded, supplied through the
 * `PAT_ENCRYPTION_KEY` Encore secret. Each row uses a fresh per-row nonce
 * (`token_nonce`); ciphertext (`token_enc`) includes the GCM auth tag.
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

/**
 * Encrypt a plaintext PAT. Returns the per-row nonce and the ciphertext+tag
 * concatenation (what spec 106 FR-006 calls `token_enc`).
 */
export function encryptPat(plaintext: string): { tokenEnc: Buffer; tokenNonce: Buffer } {
  const key = loadKey();
  const nonce = randomBytes(NONCE_BYTES);
  const cipher = createCipheriv("aes-256-gcm", key, nonce);
  const ciphertext = Buffer.concat([cipher.update(plaintext, "utf-8"), cipher.final()]);
  const tag = cipher.getAuthTag();
  return { tokenEnc: Buffer.concat([ciphertext, tag]), tokenNonce: nonce };
}

/** Decrypt a stored PAT back to plaintext. Throws on tag-mismatch / key-drift. */
export function decryptPat(tokenEnc: Buffer, tokenNonce: Buffer): string {
  const key = loadKey();
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
