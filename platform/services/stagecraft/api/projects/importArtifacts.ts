// Spec 087 Phase 2 + spec 112 §6 — wire raw artifacts discovered during
// Import into the knowledge_objects domain.
//
// When an imported repo contains `.artifacts/raw/`, each file found below
// that directory is:
//   1. hashed (SHA-256 of the raw bytes)
//   2. uploaded into the workspace bucket under `knowledge/<uuid>/<name>`
//   3. recorded as a `knowledge_objects` row (state=imported)
//   4. bound to the new project via `document_bindings`
//
// The `requirements/` folder in the same repo is deliberately NOT treated
// as knowledge — it is factory pipeline output. See factory/README.md
// §"requirements/ vs knowledge objects" for the decision record.

import { createHash } from "node:crypto";
import { readdir, readFile, stat } from "node:fs/promises";
import { join, relative, sep } from "node:path";
import { randomUUID } from "node:crypto";
import log from "encore.dev/log";
import { db } from "../db/drizzle";
import {
  documentBindings,
  knowledgeObjects,
  workspaces,
} from "../db/schema";
import { eq } from "drizzle-orm";
import { putObject } from "../knowledge/storage";
import { guessMimeType } from "./importHelpers";

export interface RegisterRawArtifactsInput {
  projectId: string;
  workspaceId: string;
  boundBy: string;
  repoRoot: string;
  /** `<owner>/<repo>` — recorded in provenance so the audit trail survives
      across re-imports. */
  sourceRepo: string;
}

export interface RegisteredArtifact {
  objectId: string;
  filename: string;
  relativePath: string;
  contentHash: string;
  sizeBytes: number;
  mimeType: string;
}

export interface RegisterRawArtifactsResult {
  registered: RegisteredArtifact[];
  skipped: Array<{ path: string; reason: string }>;
}

/**
 * Walk `<repoRoot>/.artifacts/raw/` and persist every regular file as a
 * knowledge object bound to `projectId`. Missing `.artifacts/raw/` is not
 * an error — the function returns an empty result.
 *
 * Per-file failures are logged and surfaced via `skipped`; they do not
 * abort the caller. Repeated import runs re-upload the file (new uuid +
 * new row) — dedup is not attempted here because knowledge_objects has
 * no uniqueness constraint on (workspace_id, content_hash) today.
 */
export async function registerRawArtifactsFromRepo(
  input: RegisterRawArtifactsInput
): Promise<RegisterRawArtifactsResult> {
  const rawDir = join(input.repoRoot, ".artifacts", "raw");
  let topEntries: string[];
  try {
    const s = await stat(rawDir);
    if (!s.isDirectory()) {
      return { registered: [], skipped: [] };
    }
    topEntries = await readdir(rawDir);
  } catch (err) {
    if ((err as NodeJS.ErrnoException).code === "ENOENT") {
      return { registered: [], skipped: [] };
    }
    throw err;
  }

  const bucket = await loadWorkspaceBucket(input.workspaceId);
  const files = await walkFiles(rawDir);
  if (files.length === 0 && topEntries.length > 0) {
    log.info("import: .artifacts/raw/ contains no regular files", {
      projectId: input.projectId,
      rawDir,
    });
  }

  const registered: RegisteredArtifact[] = [];
  const skipped: Array<{ path: string; reason: string }> = [];

  for (const absPath of files) {
    const relPath = toPosix(relative(rawDir, absPath));
    try {
      const body = await readFile(absPath);
      const contentHash = sha256Hex(body);
      const objectId = randomUUID();
      const filename = relPath.split("/").pop() ?? relPath;
      const mimeType = guessMimeType(filename);
      const storageKey = `knowledge/${objectId}/${filename}`;

      await putObject(bucket, storageKey, body, mimeType);

      await db.transaction(async (tx) => {
        await tx.insert(knowledgeObjects).values({
          id: objectId,
          workspaceId: input.workspaceId,
          connectorId: null,
          storageKey,
          filename,
          mimeType,
          sizeBytes: body.length,
          contentHash,
          state: "imported",
          provenance: {
            sourceType: "import-artifacts",
            sourceRepo: input.sourceRepo,
            sourcePath: `.artifacts/raw/${relPath}`,
            importedAt: new Date().toISOString(),
          },
        });
        await tx.insert(documentBindings).values({
          projectId: input.projectId,
          knowledgeObjectId: objectId,
          boundBy: input.boundBy,
        });
      });

      registered.push({
        objectId,
        filename,
        relativePath: relPath,
        contentHash,
        sizeBytes: body.length,
        mimeType,
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      log.error("import: failed to register raw artifact", {
        projectId: input.projectId,
        path: relPath,
        error: msg,
      });
      skipped.push({ path: relPath, reason: msg });
    }
  }

  log.info("import: raw artifacts registered", {
    projectId: input.projectId,
    registered: registered.length,
    skipped: skipped.length,
  });

  return { registered, skipped };
}

async function walkFiles(root: string): Promise<string[]> {
  const out: string[] = [];
  async function visit(dir: string): Promise<void> {
    const entries = await readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const abs = join(dir, entry.name);
      if (entry.isDirectory()) {
        await visit(abs);
      } else if (entry.isFile()) {
        out.push(abs);
      }
    }
  }
  await visit(root);
  out.sort();
  return out;
}

function toPosix(p: string): string {
  return sep === "/" ? p : p.split(sep).join("/");
}

function sha256Hex(body: Buffer): string {
  return createHash("sha256").update(body).digest("hex");
}

async function loadWorkspaceBucket(workspaceId: string): Promise<string> {
  const [ws] = await db
    .select({ bucket: workspaces.objectStoreBucket })
    .from(workspaces)
    .where(eq(workspaces.id, workspaceId))
    .limit(1);
  if (!ws) {
    throw new Error(`workspace ${workspaceId} not found`);
  }
  return ws.bucket;
}

