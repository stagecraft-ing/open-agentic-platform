import { api, APIError } from "encore.dev/api";
import { db } from "../db/drizzle";
import {
  projects,
  factoryPipelines,
  factoryBusinessDocs,
  factoryStages,
  factoryScaffoldFeatures,
  factoryAuditLog,
  factoryPolicyBundles,
} from "../db/schema";
import { and, eq, desc, gte, asc, sql } from "drizzle-orm";

// ---------------------------------------------------------------------------
// Known adapters (from factory/adapters/)
// ---------------------------------------------------------------------------

const KNOWN_ADAPTERS = [
  "aim-vue-node",
  "encore-react",
  "next-prisma",
  "rust-axum",
] as const;

// ---------------------------------------------------------------------------
// Org scope — matches existing DEFAULT_ORG_ID pattern in projects.ts
// ---------------------------------------------------------------------------

const DEFAULT_ORG_ID = "00000000-0000-0000-0000-000000000001";

// ---------------------------------------------------------------------------
// Factory pipeline stages (7-stage pipeline)
// ---------------------------------------------------------------------------

const PIPELINE_STAGES = [
  "s0-preflight",
  "s1-business-requirements",
  "s2-service-requirements",
  "s3-data-model",
  "s4-api-spec",
  "s5-ui-spec",
  "s6-scaffolding",
] as const;

// ---------------------------------------------------------------------------
// Stage status transitions allowed for confirm/reject
// ---------------------------------------------------------------------------

const CONFIRMABLE_STATUSES = ["completed", "in_progress"] as const;

// ---------------------------------------------------------------------------
// Default policy rules (merged with org overrides)
// ---------------------------------------------------------------------------

function compileDefaultRules(adapter: string, overrides?: PolicyOverrides) {
  // TODO (Phase 3): Load adapter-specific file_write_scope and allowed_commands
  // from the adapter manifest in factory/adapters/<adapter>/adapter-manifest.md
  // instead of using hard-coded defaults.
  return {
    allowed_adapters: [adapter],
    max_retry_per_feature: overrides?.max_retry_per_feature ?? 3,
    require_stage_approval: [1, 2, 3],  // s1, s2, s3 need human sign-off
    auto_approve_stages: [0, 4, 5, 6],  // s0, s4, s5, s6 auto-proceed
    token_budget: {
      per_stage_agent: overrides?.token_budget_total
        ? Math.floor(overrides.token_budget_total / 7)
        : 300000,
      per_feature_agent: 50000,
      total_pipeline: overrides?.token_budget_total ?? 2000000,
    },
    file_write_scope: ["src/", "prisma/", "public/", "app/"],
    allowed_commands: ["npm", "npx", "node", "tsc"],
    blocked_patterns: ["rm -rf", "sudo", "curl | sh"],
  };
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type PolicyOverrides = {
  max_retry_per_feature?: number;
  token_budget_total?: number;
};

type BusinessDocRef = {
  name: string;
  storage_ref: string;
};

type PipelineRow = {
  id: string;
  projectId: string;
  adapterName: string;
  status: string;
  policyBundleId: string | null;
  buildSpecHash: string | null;
  startedAt: Date | null;
  completedAt: Date | null;
  createdAt: Date;
  updatedAt: Date;
};

type AuditEntry = {
  id: string;
  pipelineId: string;
  timestamp: Date;
  event: string;
  actor: string | null;
  stageId: string | null;
  featureId: string | null;
  details: unknown;
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function verifyProjectInOrg(projectId: string): Promise<void> {
  const rows = await db
    .select({ id: projects.id })
    .from(projects)
    .where(
      and(eq(projects.id, projectId), eq(projects.orgId, DEFAULT_ORG_ID))
    )
    .limit(1);

  if (rows.length === 0) {
    throw APIError.notFound("project not found");
  }
}

async function getActivePipeline(projectId: string): Promise<PipelineRow> {
  // Org-scoped: join through projects to verify org membership
  await verifyProjectInOrg(projectId);

  const rows = await db
    .select()
    .from(factoryPipelines)
    .where(eq(factoryPipelines.projectId, projectId))
    .orderBy(desc(factoryPipelines.createdAt))
    .limit(1);

  if (rows.length === 0) {
    throw APIError.notFound("no Factory pipeline found for this project");
  }
  return rows[0];
}

async function appendAudit(
  entry: {
    pipelineId: string;
    event: string;
    actor?: string;
    stageId?: string;
    featureId?: string;
    details?: Record<string, unknown>;
  },
  txDb?: typeof db,
): Promise<string> {
  const d = txDb ?? db;
  const [row] = await d
    .insert(factoryAuditLog)
    .values({
      pipelineId: entry.pipelineId,
      event: entry.event,
      actor: entry.actor ?? null,
      stageId: entry.stageId ?? null,
      featureId: entry.featureId ?? null,
      details: entry.details ?? {},
    })
    .returning({ id: factoryAuditLog.id });
  return row.id;
}

// ---------------------------------------------------------------------------
// FR-001: Factory Project Initialization
// ---------------------------------------------------------------------------

type InitRequest = {
  id: string; // project ID (path param)
  adapter: string;
  business_docs?: BusinessDocRef[];
  policy_overrides?: PolicyOverrides;
  actorUserId: string;
};

type InitResponse = {
  pipeline_id: string;
  adapter: string;
  policy_bundle_id: string;
  status: string;
  created_at: string;
};

export const initPipeline = api(
  { expose: true, method: "POST", path: "/api/projects/:id/factory/init" },
  async (req: InitRequest): Promise<InitResponse> => {
    await verifyProjectInOrg(req.id);

    // Validate adapter
    if (!KNOWN_ADAPTERS.includes(req.adapter as typeof KNOWN_ADAPTERS[number])) {
      throw APIError.invalidArgument(
        `unknown adapter "${req.adapter}". Known adapters: ${KNOWN_ADAPTERS.join(", ")}`
      );
    }

    // All inserts in a single transaction to prevent orphaned rows
    const result = await db.transaction(async (tx) => {
      // Compile policy bundle
      const rules = compileDefaultRules(req.adapter, req.policy_overrides);
      const [bundle] = await tx
        .insert(factoryPolicyBundles)
        .values({
          projectId: req.id,
          adapterName: req.adapter,
          rules,
        })
        .returning();

      // Create pipeline
      const [pipeline] = await tx
        .insert(factoryPipelines)
        .values({
          projectId: req.id,
          adapterName: req.adapter,
          policyBundleId: bundle.id,
        })
        .returning();

      // Store business document references
      if (req.business_docs && req.business_docs.length > 0) {
        await tx.insert(factoryBusinessDocs).values(
          req.business_docs.map((doc) => ({
            pipelineId: pipeline.id,
            name: doc.name,
            storageRef: doc.storage_ref,
          }))
        );
      }

      // Seed stage rows
      await tx.insert(factoryStages).values(
        PIPELINE_STAGES.map((stageId) => ({
          pipelineId: pipeline.id,
          stageId,
        }))
      );

      // Audit entry
      await appendAudit(
        {
          pipelineId: pipeline.id,
          event: "pipeline_initialized",
          actor: req.actorUserId,
          details: {
            adapter: req.adapter,
            doc_count: req.business_docs?.length ?? 0,
            policy_bundle_id: bundle.id,
          },
        },
        tx,
      );

      return { pipeline, bundle };
    });

    return {
      pipeline_id: result.pipeline.id,
      adapter: result.pipeline.adapterName,
      policy_bundle_id: result.bundle.id,
      status: result.pipeline.status,
      created_at: result.pipeline.createdAt.toISOString(),
    };
  }
);

// ---------------------------------------------------------------------------
// FR-002: Pipeline Status
// ---------------------------------------------------------------------------

type StatusResponse = {
  pipeline_id: string;
  status: string;
  adapter: string;
  current_stage: string | null;
  stages: Record<
    string,
    {
      status: string;
      started_at?: string;
      completed_at?: string;
      confirmed_by?: string;
      confirmed_at?: string;
    }
  >;
  policy_bundle: unknown | null;
  token_spend: {
    total: number;
    budget: number;
    by_stage: Record<string, number>;
  };
  started_at: string | null;
};

export const getStatus = api(
  { expose: true, method: "GET", path: "/api/projects/:id/factory/status" },
  async (req: { id: string }): Promise<StatusResponse> => {
    const pipeline = await getActivePipeline(req.id);

    // Stages
    const stageRows = await db
      .select()
      .from(factoryStages)
      .where(eq(factoryStages.pipelineId, pipeline.id))
      .orderBy(asc(factoryStages.stageId));

    const stages: StatusResponse["stages"] = {};
    let currentStage: string | null = null;

    for (const s of stageRows) {
      stages[s.stageId] = {
        status: s.status,
        ...(s.startedAt && { started_at: s.startedAt.toISOString() }),
        ...(s.completedAt && { completed_at: s.completedAt.toISOString() }),
        ...(s.confirmedBy && { confirmed_by: s.confirmedBy }),
        ...(s.confirmedAt && { confirmed_at: s.confirmedAt.toISOString() }),
      };
      if (s.status === "in_progress") {
        currentStage = s.stageId;
      }
    }

    // Token spend
    const stageTokens = stageRows.reduce(
      (acc, s) => {
        const total = (s.promptTokens ?? 0) + (s.completionTokens ?? 0);
        if (total > 0) acc[s.stageId] = total;
        return acc;
      },
      {} as Record<string, number>
    );

    const scaffoldRows = await db
      .select({
        promptTokens: factoryScaffoldFeatures.promptTokens,
        completionTokens: factoryScaffoldFeatures.completionTokens,
      })
      .from(factoryScaffoldFeatures)
      .where(eq(factoryScaffoldFeatures.pipelineId, pipeline.id));

    const scaffoldTotal = scaffoldRows.reduce(
      (acc, r) => acc + (r.promptTokens ?? 0) + (r.completionTokens ?? 0),
      0
    );

    const stageTotal = Object.values(stageTokens).reduce((a, b) => a + b, 0);
    const totalSpend = stageTotal + scaffoldTotal;

    // Policy bundle
    let policyBundle: unknown | null = null;
    if (pipeline.policyBundleId) {
      const bundles = await db
        .select()
        .from(factoryPolicyBundles)
        .where(eq(factoryPolicyBundles.id, pipeline.policyBundleId))
        .limit(1);
      if (bundles.length > 0) {
        policyBundle = bundles[0].rules;
      }
    }

    return {
      pipeline_id: pipeline.id,
      status: pipeline.status,
      adapter: pipeline.adapterName,
      current_stage: currentStage,
      stages,
      policy_bundle: policyBundle,
      token_spend: {
        total: totalSpend,
        budget: (policyBundle as { token_budget?: { total_pipeline?: number } })
          ?.token_budget?.total_pipeline ?? 2000000,
        by_stage: stageTokens,
      },
      started_at: pipeline.startedAt?.toISOString() ?? null,
    };
  }
);

// ---------------------------------------------------------------------------
// FR-004: Stage Confirmation
// ---------------------------------------------------------------------------

type ConfirmRequest = {
  id: string; // project ID
  stageId: string;
  notes?: string;
  actorUserId: string;
};

type ConfirmResponse = {
  stage: string;
  confirmed_by: string;
  confirmed_at: string;
  audit_entry_id: string;
};

export const confirmStage = api(
  {
    expose: true,
    method: "POST",
    path: "/api/projects/:id/factory/stage/:stageId/confirm",
  },
  async (req: ConfirmRequest): Promise<ConfirmResponse> => {
    const pipeline = await getActivePipeline(req.id);
    const now = new Date();

    // Find stage
    const stageRows = await db
      .select()
      .from(factoryStages)
      .where(
        and(
          eq(factoryStages.pipelineId, pipeline.id),
          eq(factoryStages.stageId, req.stageId)
        )
      )
      .limit(1);

    if (stageRows.length === 0) {
      throw APIError.notFound(`stage "${req.stageId}" not found`);
    }

    // Guard: only confirmable statuses
    const stage = stageRows[0];
    if (stage.status === "confirmed") {
      throw APIError.failedPrecondition(
        `stage "${req.stageId}" is already confirmed`
      );
    }
    if (!CONFIRMABLE_STATUSES.includes(stage.status as typeof CONFIRMABLE_STATUSES[number])) {
      throw APIError.failedPrecondition(
        `stage "${req.stageId}" has status "${stage.status}" and cannot be confirmed`
      );
    }

    // Update stage
    await db
      .update(factoryStages)
      .set({
        status: "confirmed",
        confirmedBy: req.actorUserId,
        confirmedAt: now,
      })
      .where(eq(factoryStages.id, stage.id));

    // Audit
    const auditId = await appendAudit({
      pipelineId: pipeline.id,
      event: "stage_confirmed",
      actor: req.actorUserId,
      stageId: req.stageId,
      details: { notes: req.notes },
    });

    return {
      stage: req.stageId,
      confirmed_by: req.actorUserId,
      confirmed_at: now.toISOString(),
      audit_entry_id: auditId,
    };
  }
);

// ---------------------------------------------------------------------------
// FR-005: Stage Rejection
// ---------------------------------------------------------------------------

type RejectRequest = {
  id: string;
  stageId: string;
  feedback: string;
  actorUserId: string;
};

type RejectResponse = {
  stage: string;
  rejected_by: string;
  rejected_at: string;
  feedback: string;
  audit_entry_id: string;
};

export const rejectStage = api(
  {
    expose: true,
    method: "POST",
    path: "/api/projects/:id/factory/stage/:stageId/reject",
  },
  async (req: RejectRequest): Promise<RejectResponse> => {
    const pipeline = await getActivePipeline(req.id);
    const now = new Date();

    if (!req.feedback) {
      throw APIError.invalidArgument("feedback is required for rejection");
    }

    // Find stage
    const stageRows = await db
      .select()
      .from(factoryStages)
      .where(
        and(
          eq(factoryStages.pipelineId, pipeline.id),
          eq(factoryStages.stageId, req.stageId)
        )
      )
      .limit(1);

    if (stageRows.length === 0) {
      throw APIError.notFound(`stage "${req.stageId}" not found`);
    }

    // Guard: cannot reject already-rejected or confirmed stages
    const stage = stageRows[0];
    if (stage.status === "rejected") {
      throw APIError.failedPrecondition(
        `stage "${req.stageId}" is already rejected`
      );
    }
    if (stage.status === "confirmed") {
      throw APIError.failedPrecondition(
        `stage "${req.stageId}" is already confirmed and cannot be rejected`
      );
    }

    // Update stage
    await db
      .update(factoryStages)
      .set({
        status: "rejected",
        rejectedBy: req.actorUserId,
        rejectedAt: now,
        rejectionFeedback: req.feedback,
      })
      .where(eq(factoryStages.id, stage.id));

    // Audit
    const auditId = await appendAudit({
      pipelineId: pipeline.id,
      event: "stage_rejected",
      actor: req.actorUserId,
      stageId: req.stageId,
      details: { feedback: req.feedback },
    });

    return {
      stage: req.stageId,
      rejected_by: req.actorUserId,
      rejected_at: now.toISOString(),
      feedback: req.feedback,
      audit_entry_id: auditId,
    };
  }
);

// ---------------------------------------------------------------------------
// FR-006: Audit Trail
// ---------------------------------------------------------------------------

type AuditRequest = {
  id: string;
  from?: string; // ISO date string
  limit?: number;
};

type AuditResponse = {
  entries: AuditEntry[];
  total: number;
};

export const getAudit = api(
  { expose: true, method: "GET", path: "/api/projects/:id/factory/audit" },
  async (req: AuditRequest): Promise<AuditResponse> => {
    const pipeline = await getActivePipeline(req.id);
    const limit = Math.min(req.limit ?? 100, 500);

    const conditions = [eq(factoryAuditLog.pipelineId, pipeline.id)];
    if (req.from) {
      const parsed = new Date(req.from);
      if (isNaN(parsed.getTime())) {
        throw APIError.invalidArgument(
          "'from' must be a valid ISO date string"
        );
      }
      conditions.push(gte(factoryAuditLog.timestamp, parsed));
    }

    const entries = await db
      .select()
      .from(factoryAuditLog)
      .where(and(...conditions))
      .orderBy(asc(factoryAuditLog.timestamp))
      .limit(limit);

    // Total count
    const [countRow] = await db
      .select({ count: sql<number>`count(*)::int` })
      .from(factoryAuditLog)
      .where(and(...conditions));

    return {
      entries,
      total: countRow.count,
    };
  }
);

// ---------------------------------------------------------------------------
// FR-007: Deployment Handoff
// ---------------------------------------------------------------------------

type DeployRequest = {
  id: string;
  environment: string;
  git_ref: string;
  registry_image?: string;
  actorUserId: string;
};

type DeployResponse = {
  deployment_id: string;
  target: string;
  status: string;
};

export const triggerDeploy = api(
  { expose: true, method: "POST", path: "/api/projects/:id/factory/deploy" },
  async (req: DeployRequest): Promise<DeployResponse> => {
    const pipeline = await getActivePipeline(req.id);

    if (pipeline.status !== "completed") {
      throw APIError.failedPrecondition(
        `pipeline status is "${pipeline.status}", must be "completed" to deploy`
      );
    }

    if (!req.environment || !req.git_ref) {
      throw APIError.invalidArgument("environment and git_ref are required");
    }

    // Generate a deployment ID (in production this would come from deployd-api-rs)
    const deploymentId = crypto.randomUUID();

    // Audit
    await appendAudit({
      pipelineId: pipeline.id,
      event: "deployment_triggered",
      actor: req.actorUserId,
      details: {
        deployment_id: deploymentId,
        environment: req.environment,
        git_ref: req.git_ref,
        registry_image: req.registry_image,
      },
    });

    // TODO: Forward to deployd-api-rs when HTTP client is wired
    // const deploydResponse = await fetch("http://deployd-api:8080/api/deployments", { ... });

    return {
      deployment_id: deploymentId,
      target: req.environment,
      status: "queued",
    };
  }
);

// ---------------------------------------------------------------------------
// FR-008: Pipeline Token Spend Reporting
// ---------------------------------------------------------------------------

type TokenSpendRequest = {
  id: string;
  run_id: string;
  stage_id: string;
  prompt_tokens: number;
  completion_tokens: number;
  model: string;
};

export const reportTokenSpend = api(
  {
    expose: true,
    method: "POST",
    path: "/api/projects/:id/factory/token-spend",
  },
  async (req: TokenSpendRequest): Promise<void> => {
    const pipeline = await getActivePipeline(req.id);

    // Idempotency check: skip if this run_id + stage_id combo was already recorded
    const [existing] = await db
      .select({ id: factoryAuditLog.id })
      .from(factoryAuditLog)
      .where(
        and(
          eq(factoryAuditLog.pipelineId, pipeline.id),
          eq(factoryAuditLog.event, "token_spend_reported"),
          eq(factoryAuditLog.stageId, req.stage_id),
          sql`${factoryAuditLog.details}->>'run_id' = ${req.run_id}`
        )
      )
      .limit(1);

    if (existing) {
      // Already recorded — idempotent no-op
      return;
    }

    // Find stage row
    const stageRows = await db
      .select()
      .from(factoryStages)
      .where(
        and(
          eq(factoryStages.pipelineId, pipeline.id),
          eq(factoryStages.stageId, req.stage_id)
        )
      )
      .limit(1);

    if (stageRows.length === 0) {
      throw APIError.notFound(
        `stage "${req.stage_id}" not found for this pipeline`
      );
    }

    // Accumulate token spend
    await db
      .update(factoryStages)
      .set({
        promptTokens: sql`${factoryStages.promptTokens} + ${req.prompt_tokens}`,
        completionTokens: sql`${factoryStages.completionTokens} + ${req.completion_tokens}`,
        model: req.model,
      })
      .where(eq(factoryStages.id, stageRows[0].id));

    // Audit entry (idempotency key: run_id in details)
    await appendAudit({
      pipelineId: pipeline.id,
      event: "token_spend_reported",
      stageId: req.stage_id,
      details: {
        run_id: req.run_id,
        prompt_tokens: req.prompt_tokens,
        completion_tokens: req.completion_tokens,
        model: req.model,
      },
    });
  }
);
