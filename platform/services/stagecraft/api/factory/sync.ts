import { api, APIError } from "encore.dev/api";
import log from "encore.dev/log";
import { getAuthData } from "~encore/auth";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  auditLog,
  factoryAdapters,
  factoryContracts,
  factoryProcesses,
  factoryUpstreams,
  githubInstallations,
} from "../db/schema";
import { hasOrgPermission } from "../auth/membership";
import { brokerInstallationToken } from "../github/repoInit";
import { withClonedRepo } from "./clone";
import {
  translateUpstreams,
  type TranslationResult,
} from "./translator";

// ---------------------------------------------------------------------------
// Spec 108 Phase 3 — inline sync worker.
//
// POST /api/factory/upstreams/sync clones both upstream repos, runs the
// deterministic translator, and upserts the three derived tables inside a
// single transaction. Status is recorded on the factory_upstreams row so the
// existing GET /api/factory/upstreams endpoint serves double-duty as the
// polling target. If the sync open question in §10 resolves toward pubsub
// later, this entrypoint can flip to enqueueing work without changing the
// translator or upsert logic.
// ---------------------------------------------------------------------------

type SyncResponse = {
  status: "ok" | "failed";
  syncedAt: string;
  counts: { adapters: number; contracts: number; processes: number };
  factorySha: string | null;
  templateSha: string | null;
  error: string | null;
};

export const syncUpstreams = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/factory/upstreams/sync",
  },
  async (): Promise<SyncResponse> => {
    const auth = getAuthData()!;

    if (!hasOrgPermission(auth.platformRole, "factory:configure")) {
      throw APIError.permissionDenied(
        "Only org admins can trigger factory sync"
      );
    }

    const [upstreamRow] = await db
      .select()
      .from(factoryUpstreams)
      .where(eq(factoryUpstreams.orgId, auth.orgId))
      .limit(1);

    if (!upstreamRow) {
      throw APIError.failedPrecondition(
        "No factory upstream configured for this org. Configure sources first."
      );
    }

    // Mark the run as running so UI polls can see the transition. Any later
    // failure overwrites this with 'failed' + error; success overwrites with
    // 'ok' + the new shas/timestamp.
    await db
      .update(factoryUpstreams)
      .set({
        lastSyncStatus: "running",
        lastSyncError: null,
        updatedAt: new Date(),
      })
      .where(eq(factoryUpstreams.orgId, auth.orgId));

    try {
      const token = await resolveInstallationToken(auth.orgId);
      const translation = await cloneAndTranslate({
        factorySource: upstreamRow.factorySource,
        factoryRef: upstreamRow.factoryRef,
        templateSource: upstreamRow.templateSource,
        templateRef: upstreamRow.templateRef,
        token,
      });

      const syncedAt = new Date();
      await applyTranslation({
        orgId: auth.orgId,
        translation: translation.result,
        factorySha: translation.factorySha,
        templateSha: translation.templateSha,
        syncedAt,
      });

      await db.insert(auditLog).values({
        actorUserId: auth.userID,
        action: "factory.upstreams.sync_ok",
        targetType: "factory_upstreams",
        targetId: auth.orgId,
        metadata: {
          factorySha: translation.factorySha,
          templateSha: translation.templateSha,
          counts: {
            adapters: translation.result.adapters.length,
            contracts: translation.result.contracts.length,
            processes: translation.result.processes.length,
          },
        },
      });

      return {
        status: "ok",
        syncedAt: syncedAt.toISOString(),
        counts: {
          adapters: translation.result.adapters.length,
          contracts: translation.result.contracts.length,
          processes: translation.result.processes.length,
        },
        factorySha: translation.factorySha,
        templateSha: translation.templateSha,
        error: null,
      };
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      log.error("factory sync failed", { orgId: auth.orgId, err: message });

      await db
        .update(factoryUpstreams)
        .set({
          lastSyncStatus: "failed",
          lastSyncError: message,
          updatedAt: new Date(),
        })
        .where(eq(factoryUpstreams.orgId, auth.orgId));

      await db.insert(auditLog).values({
        actorUserId: auth.userID,
        action: "factory.upstreams.sync_failed",
        targetType: "factory_upstreams",
        targetId: auth.orgId,
        metadata: { error: message },
      });

      return {
        status: "failed",
        syncedAt: new Date().toISOString(),
        counts: { adapters: 0, contracts: 0, processes: 0 },
        factorySha: null,
        templateSha: null,
        error: message,
      };
    }
  }
);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function resolveInstallationToken(
  orgId: string
): Promise<string | undefined> {
  const [installation] = await db
    .select({
      installationId: githubInstallations.installationId,
      githubOrgLogin: githubInstallations.githubOrgLogin,
    })
    .from(githubInstallations)
    .where(
      and(
        eq(githubInstallations.orgId, orgId),
        eq(githubInstallations.installationState, "active")
      )
    )
    .limit(1);

  if (!installation) {
    // Public upstream sources work without a token; if the clone fails with a
    // 404/401 the caller will surface the error and the org admin can install
    // the GitHub App (spec 108 §10 open question).
    return undefined;
  }

  try {
    return await brokerInstallationToken(installation.installationId, {
      contents: "read",
      metadata: "read",
    });
  } catch (err) {
    log.warn("factory sync: installation token broker failed, trying anonymous", {
      orgId,
      err: String(err),
    });
    return undefined;
  }
}

type CloneInputs = {
  factorySource: string;
  factoryRef: string;
  templateSource: string;
  templateRef: string;
  token: string | undefined;
};

type CloneAndTranslateResult = {
  result: TranslationResult;
  factorySha: string;
  templateSha: string;
};

async function cloneAndTranslate(
  inputs: CloneInputs
): Promise<CloneAndTranslateResult> {
  return withClonedRepo(
    {
      repo: inputs.factorySource,
      ref: inputs.factoryRef,
      token: inputs.token,
    },
    async (factoryRepo) => {
      return withClonedRepo(
        {
          repo: inputs.templateSource,
          ref: inputs.templateRef,
          token: inputs.token,
        },
        async (templateRepo) => {
          const result = await translateUpstreams({
            factorySourcePath: factoryRepo.path,
            factorySourceSha: factoryRepo.sha,
            templatePath: templateRepo.path,
            templateSha: templateRepo.sha,
          });
          return {
            result,
            factorySha: factoryRepo.sha,
            templateSha: templateRepo.sha,
          };
        }
      );
    }
  );
}

type ApplyArgs = {
  orgId: string;
  translation: TranslationResult;
  factorySha: string;
  templateSha: string;
  syncedAt: Date;
};

async function applyTranslation(args: ApplyArgs): Promise<void> {
  await db.transaction(async (tx) => {
    // Prune + upsert adapters
    const adapterNames = args.translation.adapters.map((a) => a.name);
    if (adapterNames.length > 0) {
      const existing = await tx
        .select({ name: factoryAdapters.name })
        .from(factoryAdapters)
        .where(eq(factoryAdapters.orgId, args.orgId));
      const toDelete = existing
        .map((r) => r.name)
        .filter((n) => !adapterNames.includes(n));
      for (const name of toDelete) {
        await tx
          .delete(factoryAdapters)
          .where(
            and(
              eq(factoryAdapters.orgId, args.orgId),
              eq(factoryAdapters.name, name)
            )
          );
      }
    } else {
      await tx
        .delete(factoryAdapters)
        .where(eq(factoryAdapters.orgId, args.orgId));
    }

    for (const a of args.translation.adapters) {
      await tx
        .insert(factoryAdapters)
        .values({
          orgId: args.orgId,
          name: a.name,
          version: a.version,
          manifest: a.manifest,
          sourceSha: a.sourceSha,
          syncedAt: args.syncedAt,
        })
        .onConflictDoUpdate({
          target: [factoryAdapters.orgId, factoryAdapters.name],
          set: {
            version: a.version,
            manifest: a.manifest,
            sourceSha: a.sourceSha,
            syncedAt: args.syncedAt,
          },
        });
    }

    // Prune + upsert contracts (unique on org+name+version, but we keep only
    // the latest version per name for now — the spec defers versioning to a
    // later phase, so a new sync replaces prior rows by name).
    await tx
      .delete(factoryContracts)
      .where(eq(factoryContracts.orgId, args.orgId));
    for (const c of args.translation.contracts) {
      await tx.insert(factoryContracts).values({
        orgId: args.orgId,
        name: c.name,
        version: c.version,
        schema: c.schema,
        sourceSha: c.sourceSha,
        syncedAt: args.syncedAt,
      });
    }

    // Same strategy for processes.
    await tx
      .delete(factoryProcesses)
      .where(eq(factoryProcesses.orgId, args.orgId));
    for (const p of args.translation.processes) {
      await tx.insert(factoryProcesses).values({
        orgId: args.orgId,
        name: p.name,
        version: p.version,
        definition: p.definition,
        sourceSha: p.sourceSha,
        syncedAt: args.syncedAt,
      });
    }

    await tx
      .update(factoryUpstreams)
      .set({
        lastSyncedAt: args.syncedAt,
        lastSyncSha: {
          factory: args.factorySha,
          template: args.templateSha,
        },
        lastSyncStatus: "ok",
        lastSyncError: null,
        updatedAt: args.syncedAt,
      })
      .where(eq(factoryUpstreams.orgId, args.orgId));
  });
}
