/**
 * S3-compatible object store client for knowledge intake (spec 087 Phase 2).
 *
 * Uses MinIO for local dev, S3/Azure Blob for production.
 * Provides presigned URLs so clients upload directly to the store
 * without routing blobs through the Encore API.
 */

import {
  S3Client,
  HeadObjectCommand,
  DeleteObjectCommand,
} from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { PutObjectCommand, GetObjectCommand } from "@aws-sdk/client-s3";
import { secret } from "encore.dev/config";
import log from "encore.dev/log";

// ---------------------------------------------------------------------------
// Configuration (Encore secrets)
// ---------------------------------------------------------------------------

const s3Endpoint = secret("S3_ENDPOINT"); // e.g. http://localhost:9000 (MinIO) or https://s3.amazonaws.com
const s3Region = secret("S3_REGION"); // e.g. us-east-1
const s3AccessKey = secret("S3_ACCESS_KEY");
const s3SecretKey = secret("S3_SECRET_KEY");

// ---------------------------------------------------------------------------
// Client singleton (lazy-init)
// ---------------------------------------------------------------------------

let _client: S3Client | null = null;

function getClient(): S3Client {
  if (!_client) {
    _client = new S3Client({
      endpoint: s3Endpoint(),
      region: s3Region(),
      credentials: {
        accessKeyId: s3AccessKey(),
        secretAccessKey: s3SecretKey(),
      },
      forcePathStyle: true, // required for MinIO
    });
  }
  return _client;
}

// ---------------------------------------------------------------------------
// Presigned upload URL
// ---------------------------------------------------------------------------

export async function getPresignedUploadUrl(
  bucket: string,
  key: string,
  contentType: string,
  expiresInSeconds = 3600
): Promise<string> {
  const cmd = new PutObjectCommand({
    Bucket: bucket,
    Key: key,
    ContentType: contentType,
  });
  const url = await getSignedUrl(getClient(), cmd, {
    expiresIn: expiresInSeconds,
  });
  log.info("presigned upload URL generated", { bucket, key });
  return url;
}

// ---------------------------------------------------------------------------
// Presigned download URL
// ---------------------------------------------------------------------------

export async function getPresignedDownloadUrl(
  bucket: string,
  key: string,
  expiresInSeconds = 3600
): Promise<string> {
  const cmd = new GetObjectCommand({
    Bucket: bucket,
    Key: key,
  });
  return getSignedUrl(getClient(), cmd, { expiresIn: expiresInSeconds });
}

// ---------------------------------------------------------------------------
// Head object (verify upload + get metadata)
// ---------------------------------------------------------------------------

export type ObjectMeta = {
  contentLength: number;
  contentType: string;
  etag: string;
};

export async function headObject(
  bucket: string,
  key: string
): Promise<ObjectMeta | null> {
  try {
    const res = await getClient().send(
      new HeadObjectCommand({ Bucket: bucket, Key: key })
    );
    return {
      contentLength: res.ContentLength ?? 0,
      contentType: res.ContentType ?? "application/octet-stream",
      etag: (res.ETag ?? "").replace(/"/g, ""),
    };
  } catch (err: unknown) {
    if (
      err instanceof Error &&
      "name" in err &&
      err.name === "NotFound"
    ) {
      return null;
    }
    throw err;
  }
}

// ---------------------------------------------------------------------------
// Delete object
// ---------------------------------------------------------------------------

export async function deleteObject(
  bucket: string,
  key: string
): Promise<void> {
  await getClient().send(
    new DeleteObjectCommand({ Bucket: bucket, Key: key })
  );
  log.info("object deleted", { bucket, key });
}
