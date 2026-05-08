// Spec 143 FR-011 — server-side upload size cap regression test.
//
// Live DB (gated to `encore test` via vite.config.ts). Validates that
// requestUpload rejects sizeBytes > KNOWLEDGE_UPLOAD_MAX_BYTES with
// APIError.invalidArgument, mirroring the browser-side pre-check.
// The two layers MUST agree on the same number — see
// uploadLimits.test.ts for the constant pinning.

import { describe, expect, it, beforeAll, afterAll } from "vitest";
import { sql } from "drizzle-orm";
import { db } from "../db/drizzle";
import { requestUpload } from "./knowledge";
import { KNOWLEDGE_UPLOAD_MAX_BYTES } from "./uploadLimits";

const ORG_ID = "55000000-0000-0000-0000-0000000000e1";
const USER_ID = "55000000-0000-0000-0000-0000000000e2";
const PROJECT_ID = "55000000-0000-0000-0000-0000000000e3";

async function deleteFixtures() {
  await db.execute(sql`
    DELETE FROM knowledge_objects WHERE project_id = ${PROJECT_ID}
  `);
  await db.execute(sql`
    DELETE FROM projects WHERE id = ${PROJECT_ID}
  `);
  await db.execute(sql`DELETE FROM organizations WHERE id = ${ORG_ID}`);
  await db.execute(sql`DELETE FROM users WHERE id = ${USER_ID}`);
}

describe("requestUpload size cap (spec 143 FR-011)", () => {
  beforeAll(async () => {
    await deleteFixtures();
    await db.execute(sql`
      INSERT INTO organizations (id, name, slug)
        VALUES (${ORG_ID}, 'request-upload-test', 'request-upload-test')
    `);
    await db.execute(sql`
      INSERT INTO users (id, email, password_hash, name, role)
        VALUES (${USER_ID}, 'requp@test', 'x', 'ReqUp Tester', 'user')
    `);
    await db.execute(sql`
      INSERT INTO projects (id, org_id, name, slug, description, object_store_bucket, created_by)
        VALUES (${PROJECT_ID}, ${ORG_ID}, 'requp-p', 'requp-p', '',
                'requp-bucket', ${USER_ID})
    `);
  });

  afterAll(async () => {
    await deleteFixtures();
  });

  it("rejects sizeBytes above KNOWLEDGE_UPLOAD_MAX_BYTES with invalid_argument", async () => {
    const oversize = KNOWLEDGE_UPLOAD_MAX_BYTES + 1;
    await expect(
      requestUpload({
        projectId: PROJECT_ID,
        filename: "huge.bin",
        mimeType: "application/octet-stream",
        contentHash: "sha256-oversize",
        sizeBytes: oversize,
      }),
    ).rejects.toMatchObject({
      code: "invalid_argument",
    });
  });

  it("accepts sizeBytes at the cap exactly", async () => {
    const result = await requestUpload({
      projectId: PROJECT_ID,
      filename: "exactly-cap.bin",
      mimeType: "application/octet-stream",
      contentHash: "sha256-at-cap",
      sizeBytes: KNOWLEDGE_UPLOAD_MAX_BYTES,
    });
    expect(result.objectId).toBeTruthy();
    expect(result.uploadUrl).toMatch(/^https?:\/\//);

    // Cleanup the row so afterAll doesn't fight a different content_hash.
    await db.execute(sql`
      DELETE FROM knowledge_objects WHERE id = ${result.objectId}
    `);
  });
});
