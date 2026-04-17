import {
  pgTable,
  text,
  timestamp,
  uuid,
  boolean,
  integer,
  bigint,
  pgEnum,
  jsonb,
  unique,
  customType,
} from "drizzle-orm/pg-core";

const bytea = customType<{ data: Buffer; notNull: true; default: false }>({
  dataType() {
    return "bytea";
  },
});

export const roleEnum = pgEnum("role", ["user", "admin"]);
export const sessionKindEnum = pgEnum("session_kind", ["user", "admin"]);

// ---------------------------------------------------------------------------
// GitHub Identity Onboarding enums (spec 080)
// ---------------------------------------------------------------------------

export const installationStateEnum = pgEnum("installation_state", [
  "active",
  "suspended",
  "deleted",
]);

export const membershipSourceEnum = pgEnum("membership_source", [
  "github",
  "manual",
  "rauthy",
  "oidc",
]);

export const orgMembershipStatusEnum = pgEnum("org_membership_status", [
  "active",
  "suspended",
  "removed",
]);

export const platformRoleEnum = pgEnum("platform_role", [
  "owner",
  "admin",
  "member",
]);

export const users = pgTable("users", {
  id: uuid("id").defaultRandom().primaryKey(),
  email: text("email").notNull().unique(),
  name: text("name").notNull(),
  passwordHash: text("password_hash"),  // nullable: OAuth users have no password
  role: roleEnum("role").notNull().default("user"),
  disabled: boolean("disabled").notNull().default(false),
  githubUserId: bigint("github_user_id", { mode: "number" }).unique(),
  githubLogin: text("github_login"),
  avatarUrl: text("avatar_url"),
  rauthyUserId: text("rauthy_user_id").unique(),
  idpProvider: text("idp_provider"),    // 'github' | 'azure-ad' | 'okta' | etc.
  idpSubject: text("idp_subject"),      // provider-specific user ID
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  lastLoginAt: timestamp("last_login_at", { withTimezone: true }),
});

export const sessions = pgTable("sessions", {
  id: uuid("id").defaultRandom().primaryKey(),
  userId: uuid("user_id").notNull(),
  kind: sessionKindEnum("kind").notNull(),
  tokenHash: text("token_hash").notNull().unique(),
  expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  revokedAt: timestamp("revoked_at", { withTimezone: true }),
});

export const workspaceGrants = pgTable("workspace_grants", {
  id: uuid("id").defaultRandom().primaryKey(),
  userId: uuid("user_id").notNull(),
  workspaceId: text("workspace_id").notNull(),
  enableFileRead: boolean("enable_file_read").notNull().default(true),
  enableFileWrite: boolean("enable_file_write").notNull().default(true),
  enableNetwork: boolean("enable_network").notNull().default(true),
  maxTier: integer("max_tier").notNull().default(2),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

export const agentPolicies = pgTable("agent_policies", {
  id: uuid("id").defaultRandom().primaryKey(),
  orgId: text("org_id").notNull().default("default"),
  slug: text("slug").notNull(),
  blocked: boolean("blocked").notNull().default(false),
  reason: text("reason").notNull().default(""),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

// ---------------------------------------------------------------------------
// Project model enums
// ---------------------------------------------------------------------------

export const projectMemberRoleEnum = pgEnum("project_member_role", [
  "viewer",
  "developer",
  "deployer",
  "admin",
]);

export const environmentKindEnum = pgEnum("environment_kind", [
  "preview",
  "development",
  "staging",
  "production",
]);

// ---------------------------------------------------------------------------
// Organizations
// ---------------------------------------------------------------------------

export const organizations = pgTable("organizations", {
  id: uuid("id").defaultRandom().primaryKey(),
  name: text("name").notNull(),
  slug: text("slug").notNull().unique(),
  createdBy: uuid("created_by"),  // nullable: orgs auto-created from GitHub App install
  githubOrgId: bigint("github_org_id", { mode: "number" }).unique(),
  githubOrgLogin: text("github_org_login"),
  githubInstallationId: bigint("github_installation_id", { mode: "number" }),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

// ---------------------------------------------------------------------------
// Workspaces (spec 087)
// ---------------------------------------------------------------------------

export const workspaces = pgTable(
  "workspaces",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    orgId: uuid("org_id").notNull(),
    name: text("name").notNull(),
    slug: text("slug").notNull(),
    objectStoreBucket: text("object_store_bucket").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
  },
  (t) => [unique().on(t.orgId, t.slug)]
);

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

export const projects = pgTable(
  "projects",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    orgId: uuid("org_id").notNull(),
    workspaceId: uuid("workspace_id").notNull(),
    name: text("name").notNull(),
    slug: text("slug").notNull(),
    description: text("description").notNull().default(""),
    createdBy: uuid("created_by").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
  },
  (t) => [unique().on(t.workspaceId, t.slug)]
);

// ---------------------------------------------------------------------------
// Project Repos
// ---------------------------------------------------------------------------

export const projectRepos = pgTable("project_repos", {
  id: uuid("id").defaultRandom().primaryKey(),
  projectId: uuid("project_id").notNull(),
  githubOrg: text("github_org").notNull(),
  repoName: text("repo_name").notNull(),
  defaultBranch: text("default_branch").notNull().default("main"),
  isPrimary: boolean("is_primary").notNull().default(false),
  githubInstallId: bigint("github_install_id", { mode: "number" }),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

// ---------------------------------------------------------------------------
// Environments
// ---------------------------------------------------------------------------

export const environments = pgTable("environments", {
  id: uuid("id").defaultRandom().primaryKey(),
  projectId: uuid("project_id").notNull(),
  name: text("name").notNull(),
  kind: environmentKindEnum("kind").notNull().default("development"),
  k8sNamespace: text("k8s_namespace"),
  autoDeployBranch: text("auto_deploy_branch"),
  requiresApproval: boolean("requires_approval").notNull().default(false),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

// ---------------------------------------------------------------------------
// Project Members
// ---------------------------------------------------------------------------

export const projectMembers = pgTable(
  "project_members",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    projectId: uuid("project_id").notNull(),
    userId: uuid("user_id").notNull(),
    role: projectMemberRoleEnum("role").notNull().default("viewer"),
    createdAt: timestamp("created_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
  },
  (t) => [unique().on(t.projectId, t.userId)]
);

// ---------------------------------------------------------------------------
// Audit Log
// ---------------------------------------------------------------------------

export const auditLog = pgTable("audit_log", {
  id: uuid("id").defaultRandom().primaryKey(),
  actorUserId: uuid("actor_user_id").notNull(),
  action: text("action").notNull(),
  targetType: text("target_type").notNull(),
  targetId: text("target_id").notNull(),
  metadata: jsonb("metadata").notNull().default({}),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

// ---------------------------------------------------------------------------
// GitHub App Installations (spec 080)
// ---------------------------------------------------------------------------

export const githubInstallations = pgTable("github_installations", {
  id: uuid("id").defaultRandom().primaryKey(),
  githubOrgId: bigint("github_org_id", { mode: "number" }).notNull().unique(),
  githubOrgLogin: text("github_org_login").notNull(),
  installationId: bigint("installation_id", { mode: "number" })
    .notNull()
    .unique(),
  installationState: installationStateEnum("installation_state")
    .notNull()
    .default("active"),
  allowedRepos: text("allowed_repos"),
  orgId: uuid("org_id"),
  installedBy: text("installed_by"),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

// ---------------------------------------------------------------------------
// User Identity Linkage (spec 080)
// ---------------------------------------------------------------------------

export const userIdentities = pgTable(
  "user_identities",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    userId: uuid("user_id").notNull(),
    provider: text("provider").notNull().default("github"),
    providerUserId: text("provider_user_id").notNull(),
    providerLogin: text("provider_login").notNull(),
    providerEmail: text("provider_email"),
    avatarUrl: text("avatar_url"),
    accessTokenEnc: text("access_token_enc"),
    refreshTokenEnc: text("refresh_token_enc"),
    tokenExpiresAt: timestamp("token_expires_at", { withTimezone: true }),
    createdAt: timestamp("created_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
  },
  (t) => [unique().on(t.provider, t.providerUserId)]
);

// ---------------------------------------------------------------------------
// Org Membership Linkage (spec 080)
// ---------------------------------------------------------------------------

export const orgMemberships = pgTable(
  "org_memberships",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    userId: uuid("user_id").notNull(),
    orgId: uuid("org_id").notNull(),
    source: membershipSourceEnum("source").notNull().default("github"),
    githubRole: text("github_role"),
    platformRole: platformRoleEnum("platform_role").notNull().default("member"),
    status: orgMembershipStatusEnum("status").notNull().default("active"),
    syncedAt: timestamp("synced_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
    createdAt: timestamp("created_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
  },
  (t) => [unique().on(t.userId, t.orgId)]
);

// ---------------------------------------------------------------------------
// Factory Pipeline Lifecycle (spec 077)
// ---------------------------------------------------------------------------

export const factoryPipelineStatusEnum = pgEnum("factory_pipeline_status", [
  "initialized",
  "running",
  "paused",
  "completed",
  "failed",
  "cancelled",
]);

export const factoryStageStatusEnum = pgEnum("factory_stage_status", [
  "pending",
  "in_progress",
  "completed",
  "confirmed",
  "rejected",
]);

export const factoryScaffoldStatusEnum = pgEnum("factory_scaffold_status", [
  "pending",
  "in_progress",
  "completed",
  "failed",
]);

export const factoryScaffoldCategoryEnum = pgEnum("factory_scaffold_category", [
  "data",
  "api",
  "ui",
  "configure",
  "trim",
  "validate",
]);

export const factoryPipelines = pgTable("factory_pipelines", {
  id: uuid("id").defaultRandom().primaryKey(),
  projectId: uuid("project_id").notNull(),
  adapterName: text("adapter_name").notNull(),
  status: factoryPipelineStatusEnum("status").notNull().default("initialized"),
  policyBundleId: uuid("policy_bundle_id"),
  buildSpecHash: text("build_spec_hash"),
  previousPipelineId: uuid("previous_pipeline_id"),
  startedAt: timestamp("started_at", { withTimezone: true }),
  completedAt: timestamp("completed_at", { withTimezone: true }),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

export const factoryBusinessDocs = pgTable("factory_business_docs", {
  id: uuid("id").defaultRandom().primaryKey(),
  pipelineId: uuid("pipeline_id").notNull(),
  name: text("name").notNull(),
  storageRef: text("storage_ref").notNull(),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

export const factoryStages = pgTable(
  "factory_stages",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    pipelineId: uuid("pipeline_id").notNull(),
    stageId: text("stage_id").notNull(),
    status: factoryStageStatusEnum("status").notNull().default("pending"),
    startedAt: timestamp("started_at", { withTimezone: true }),
    completedAt: timestamp("completed_at", { withTimezone: true }),
    confirmedBy: text("confirmed_by"),
    confirmedAt: timestamp("confirmed_at", { withTimezone: true }),
    rejectedBy: text("rejected_by"),
    rejectedAt: timestamp("rejected_at", { withTimezone: true }),
    rejectionFeedback: text("rejection_feedback"),
    promptTokens: integer("prompt_tokens").default(0),
    completionTokens: integer("completion_tokens").default(0),
    model: text("model"),
  },
  (t) => [unique().on(t.pipelineId, t.stageId)]
);

export const factoryScaffoldFeatures = pgTable(
  "factory_scaffold_features",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    pipelineId: uuid("pipeline_id").notNull(),
    featureId: text("feature_id").notNull(),
    category: factoryScaffoldCategoryEnum("category").notNull(),
    status: factoryScaffoldStatusEnum("status").notNull().default("pending"),
    retryCount: integer("retry_count").default(0),
    lastError: text("last_error"),
    filesCreated: text("files_created").array(),
    promptTokens: integer("prompt_tokens").default(0),
    completionTokens: integer("completion_tokens").default(0),
    startedAt: timestamp("started_at", { withTimezone: true }),
    completedAt: timestamp("completed_at", { withTimezone: true }),
  },
  (t) => [unique().on(t.pipelineId, t.featureId)]
);

export const factoryAuditLog = pgTable("factory_audit_log", {
  id: uuid("id").defaultRandom().primaryKey(),
  pipelineId: uuid("pipeline_id").notNull(),
  timestamp: timestamp("timestamp", { withTimezone: true })
    .notNull()
    .defaultNow(),
  event: text("event").notNull(),
  actor: text("actor"),
  stageId: text("stage_id"),
  featureId: text("feature_id"),
  details: jsonb("details").notNull().default({}),
});

export const factoryPolicyBundles = pgTable("factory_policy_bundles", {
  id: uuid("id").defaultRandom().primaryKey(),
  projectId: uuid("project_id").notNull(),
  adapterName: text("adapter_name").notNull(),
  rules: jsonb("rules").notNull(),
  compiledAt: timestamp("compiled_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

// ---------------------------------------------------------------------------
// Knowledge Intake Domain (spec 087 Phase 2)
// ---------------------------------------------------------------------------

export const connectorTypeEnum = pgEnum("connector_type", [
  "upload",
  "sharepoint",
  "s3",
  "azure-blob",
  "gcs",
]);

export const connectorStatusEnum = pgEnum("connector_status", [
  "active",
  "paused",
  "error",
  "disabled",
]);

export const knowledgeObjectStateEnum = pgEnum("knowledge_object_state", [
  "imported",
  "extracting",
  "extracted",
  "classified",
  "available",
]);

export const sourceConnectors = pgTable("source_connectors", {
  id: uuid("id").defaultRandom().primaryKey(),
  workspaceId: uuid("workspace_id").notNull(),
  type: connectorTypeEnum("type").notNull(),
  name: text("name").notNull(),
  configEncrypted: jsonb("config_encrypted"),
  syncSchedule: text("sync_schedule"),
  status: connectorStatusEnum("status").notNull().default("active"),
  lastSyncedAt: timestamp("last_synced_at", { withTimezone: true }),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

export const knowledgeObjects = pgTable("knowledge_objects", {
  id: uuid("id").defaultRandom().primaryKey(),
  workspaceId: uuid("workspace_id").notNull(),
  connectorId: uuid("connector_id"),
  storageKey: text("storage_key").notNull(),
  filename: text("filename").notNull(),
  mimeType: text("mime_type").notNull(),
  sizeBytes: bigint("size_bytes", { mode: "number" }).notNull(),
  contentHash: text("content_hash").notNull(),
  state: knowledgeObjectStateEnum("state").notNull().default("imported"),
  extractionOutput: jsonb("extraction_output"),
  classification: jsonb("classification"),
  provenance: jsonb("provenance").notNull(),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  updatedAt: timestamp("updated_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

export const documentBindings = pgTable(
  "document_bindings",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    projectId: uuid("project_id").notNull(),
    knowledgeObjectId: uuid("knowledge_object_id").notNull(),
    boundBy: uuid("bound_by").notNull(),
    boundAt: timestamp("bound_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
  },
  (t) => [unique().on(t.projectId, t.knowledgeObjectId)]
);

// ---------------------------------------------------------------------------
// Sync Runs (spec 087 Phase 4)
// ---------------------------------------------------------------------------

export const syncRunStatusEnum = pgEnum("sync_run_status", [
  "running",
  "completed",
  "failed",
]);

export const syncRuns = pgTable("sync_runs", {
  id: uuid("id").defaultRandom().primaryKey(),
  connectorId: uuid("connector_id").notNull(),
  workspaceId: uuid("workspace_id").notNull(),
  status: syncRunStatusEnum("status").notNull().default("running"),
  objectsCreated: integer("objects_created").notNull().default(0),
  objectsUpdated: integer("objects_updated").notNull().default(0),
  objectsSkipped: integer("objects_skipped").notNull().default(0),
  error: text("error"),
  deltaToken: text("delta_token"),
  startedAt: timestamp("started_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  completedAt: timestamp("completed_at", { withTimezone: true }),
});

// ---------------------------------------------------------------------------
// Factory Artifact Registry (spec 082 Phase 3)
// ---------------------------------------------------------------------------

export const factoryArtifacts = pgTable("factory_artifacts", {
  id: uuid("id").defaultRandom().primaryKey(),
  pipelineId: uuid("pipeline_id").notNull(),
  stageId: text("stage_id").notNull(),
  artifactType: text("artifact_type").notNull(),
  contentHash: text("content_hash").notNull(),
  storagePath: text("storage_path").notNull(),
  sizeBytes: integer("size_bytes").notNull().default(0),
  // Workspace scoping (spec 094 Slice 5).
  workspaceId: uuid("workspace_id"),
  // Provenance: which agent produced this artifact (spec 094 Slice 5).
  producerAgent: text("producer_agent"),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

// ---------------------------------------------------------------------------
// Promotions (spec 097 — Promotion-Grade Platform Mirror)
// ---------------------------------------------------------------------------

export const promotionStatusEnum = pgEnum("promotion_status", [
  "promoted",
  "revoked",
]);

export const promotions = pgTable("promotions", {
  id: uuid("id").defaultRandom().primaryKey(),
  workspaceId: uuid("workspace_id").notNull(),
  pipelineId: uuid("pipeline_id").notNull(),
  workflowId: text("workflow_id").notNull(),
  status: promotionStatusEnum("status").notNull().default("promoted"),
  promotedBy: text("promoted_by"),
  evidence: jsonb("evidence").notNull().default({}),
  promotedAt: timestamp("promoted_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

// ---------------------------------------------------------------------------
// GitHub Team Role Mappings (spec 080 Phase 3 — FR-009)
// ---------------------------------------------------------------------------

export const targetScopeEnum = pgEnum("target_scope", ["org", "project"]);

export const githubTeamRoleMappings = pgTable(
  "github_team_role_mappings",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    orgId: uuid("org_id").notNull(),
    githubTeamSlug: text("github_team_slug").notNull(),
    githubTeamId: bigint("github_team_id", { mode: "number" }).notNull(),
    targetScope: targetScopeEnum("target_scope").notNull(),
    targetId: uuid("target_id"), // NULL for org-level, project_id for project-level
    role: text("role").notNull(), // platform_role or project_member_role
    createdAt: timestamp("created_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
  },
  (t) => [unique().on(t.orgId, t.githubTeamSlug, t.targetScope, t.targetId)]
);

// ---------------------------------------------------------------------------
// Desktop Refresh Tokens (spec 080 Phase 1 — OPC PKCE auth)
// ---------------------------------------------------------------------------

export const desktopRefreshTokens = pgTable("desktop_refresh_tokens", {
  id: uuid("id").defaultRandom().primaryKey(),
  tokenHash: text("token_hash").notNull().unique(),
  userId: uuid("user_id").notNull(),
  orgId: uuid("org_id").notNull(),
  workspaceId: text("workspace_id").notNull().default(""),
  orgSlug: text("org_slug").notNull().default(""),
  githubLogin: text("github_login").default(""),
  idpProvider: text("idp_provider").notNull().default(""),
  idpLogin: text("idp_login").notNull().default(""),
  platformRole: text("platform_role").notNull().default("member"),
  rauthyUserId: text("rauthy_user_id").notNull(),
  expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
});

// ---------------------------------------------------------------------------
// User GitHub Personal Access Tokens (spec 106 FR-005/FR-006)
// ---------------------------------------------------------------------------

export const userGithubPats = pgTable("user_github_pats", {
  id: uuid("id").defaultRandom().primaryKey(),
  userId: uuid("user_id").notNull(),
  tokenEnc: bytea("token_enc").notNull(),
  tokenNonce: bytea("token_nonce").notNull(),
  tokenPrefix: text("token_prefix").notNull(),
  scopes: text("scopes").array().notNull().default([]),
  isFineGrained: boolean("is_fine_grained").notNull().default(false),
  lastUsedAt: timestamp("last_used_at", { withTimezone: true }),
  lastCheckedAt: timestamp("last_checked_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  createdAt: timestamp("created_at", { withTimezone: true })
    .notNull()
    .defaultNow(),
  revokedAt: timestamp("revoked_at", { withTimezone: true }),
});

// ---------------------------------------------------------------------------
// OIDC Providers (spec 080 Phase 4 — Enterprise OIDC Federation)
// ---------------------------------------------------------------------------

export const oidcProviders = pgTable(
  "oidc_providers",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    orgId: uuid("org_id").notNull(),
    name: text("name").notNull(),
    providerType: text("provider_type").notNull().default("oidc"), // oidc | azure-ad | okta | google-workspace | saml-bridge
    issuer: text("issuer").notNull(),
    clientId: text("client_id").notNull(),
    clientSecretEnc: text("client_secret_enc").notNull(),
    scopes: text("scopes").notNull().default("openid profile email"),
    claimsMapping: jsonb("claims_mapping").notNull().default({}),
    emailDomain: text("email_domain"),
    autoProvision: boolean("auto_provision").notNull().default(true),
    status: text("status").notNull().default("active"),
    createdAt: timestamp("created_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
    updatedAt: timestamp("updated_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
  },
  (t) => [unique().on(t.orgId, t.issuer)]
);

// ---------------------------------------------------------------------------
// OIDC Group-to-Role Mappings (spec 080 Phase 4)
// ---------------------------------------------------------------------------

export const oidcGroupRoleMappings = pgTable(
  "oidc_group_role_mappings",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    orgId: uuid("org_id").notNull(),
    providerId: uuid("provider_id").notNull(),
    idpGroupId: text("idp_group_id").notNull(),
    idpGroupName: text("idp_group_name"),
    targetScope: targetScopeEnum("target_scope").notNull(),
    targetId: uuid("target_id"), // NULL for org-level, project_id for project-level
    role: text("role").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true })
      .notNull()
      .defaultNow(),
  },
  (t) => [unique().on(t.orgId, t.providerId, t.idpGroupId, t.targetScope, t.targetId)]
);
