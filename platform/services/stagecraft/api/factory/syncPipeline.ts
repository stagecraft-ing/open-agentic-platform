/**
 * Core Factory sync pipeline (spec 109 §5).
 *
 * The actual clone + translate + upsert logic, lifted out of the old
 * inline endpoint so it can be shared between the PubSub worker and any
 * future admin CLI. Pure helpers — no HTTP surface — so they can be
 * unit-tested against a real Postgres via `encore test`.
 */

import log from "encore.dev/log";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  factoryAdapters,
  factoryContracts,
  factoryProcesses,
  factoryUpstreams,
} from "../db/schema";
import { withClonedRepo } from "./clone";
import {
  translateUpstreams,
  type TranslationResult,
} from "./translator";

export type SyncPipelineInputs = {
  orgId: string;
  factorySource: string;
  factoryRef: string;
  templateSource: string;
  templateRef: string;
  token: string | undefined;
};

export type SyncPipelineResult = {
  factorySha: string;
  templateSha: string;
  counts: { adapters: number; contracts: number; processes: number };
};

export async function runSyncPipeline(
  inputs: SyncPipelineInputs
): Promise<SyncPipelineResult> {
  const translation = await cloneAndTranslate(inputs);
  const syncedAt = new Date();

  await applyTranslation({
    orgId: inputs.orgId,
    translation: translation.result,
    factorySha: translation.factorySha,
    templateSha: translation.templateSha,
    syncedAt,
  });

  log.info("factory sync pipeline completed", {
    orgId: inputs.orgId,
    factorySha: translation.factorySha,
    templateSha: translation.templateSha,
    adapters: translation.result.adapters.length,
    contracts: translation.result.contracts.length,
    processes: translation.result.processes.length,
  });

  return {
    factorySha: translation.factorySha,
    templateSha: translation.templateSha,
    counts: {
      adapters: translation.result.adapters.length,
      contracts: translation.result.contracts.length,
      processes: translation.result.processes.length,
    },
  };
}

type CloneAndTranslateResult = {
  result: TranslationResult;
  factorySha: string;
  templateSha: string;
};

async function cloneAndTranslate(
  inputs: SyncPipelineInputs
): Promise<CloneAndTranslateResult> {
  return withClonedRepo(
    {
      repo: inputs.factorySource,
      ref: inputs.factoryRef,
      token: inputs.token,
    },
    async (factoryRepo) =>
      withClonedRepo(
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
      )
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
    // Prune + upsert adapters.
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

    // Replace contracts (keep-latest semantics, matching Phase 3 behaviour).
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

    // Same for processes.
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

    // Denormalised "current state" mirror on factory_upstreams.
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
