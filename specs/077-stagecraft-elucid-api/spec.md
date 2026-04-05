---
id: "077-stagecraft-elucid-api"
title: "Stagecraft Elucid Lifecycle API — Project Init, Stage Confirmation, Audit Trail"
feature_branch: "feat/077-stagecraft-elucid-api"
status: active
kind: platform
created: "2026-04-04"
authors: ["open-agentic-platform"]
language: en
summary: >
  Extends the Stagecraft platform service with Elucid-specific API endpoints for
  project initialization, adapter selection, stage confirmation/rejection, policy
  bundle compilation, audit trail capture, and deployment handoff.
code_aliases: ["STAGECRAFT_ELUCID", "ELUCID_API"]
---

# Feature Specification: Stagecraft Elucid Lifecycle API

## Purpose

Stagecraft is OAP's organizational control plane — it manages identity, projects, policies, and audit trails. Elucid pipelines are executed locally by OPC desktop, but the organizational lifecycle (who created the project, which adapter, who approved which stage, total token spend, deployment readiness) must be tracked centrally.

This spec adds Elucid-specific endpoints to Stagecraft for:
1. Project initialization with adapter selection and business document references
2. Policy bundle compilation with Elucid-specific shards
3. Stage confirmation/rejection with approver identity
4. Audit trail for full pipeline lifecycle
5. Deployment readiness signaling to deployd-api-rs

## Scope

### In scope

- Stagecraft REST API endpoints for Elucid project lifecycle
- Database schema extensions (Drizzle ORM migrations)
- Policy bundle compilation with Elucid adapter shards
- Audit trail integration (every gate decision, token spend, error)
- Webhook notifications (pipeline started, stage confirmed, pipeline complete)
- Deployment handoff to deployd-api-rs

### Out of scope

- OPC desktop UI (spec 076)
- Orchestrator logic (spec 075)
- Kubernetes deployment orchestration (deployd-api-rs scope)

## Requirements

### Functional Requirements

**FR-001: Elucid Project Initialization**

```
POST /api/projects/:id/elucid/init
Authorization: Bearer <token>
Content-Type: application/json

{
  "adapter": "next-prisma",
  "business_docs": [
    { "name": "requirements.pdf", "storage_ref": "s3://bucket/docs/requirements.pdf" },
    { "name": "data-dictionary.xlsx", "storage_ref": "s3://bucket/docs/data-dict.xlsx" }
  ],
  "policy_overrides": {            // optional org-level overrides
    "max_retry_per_feature": 5,
    "token_budget_total": 3000000
  }
}

→ 201 Created
{
  "pipeline_id": "uuid",
  "adapter": "next-prisma",
  "policy_bundle_id": "uuid",
  "status": "initialized",
  "created_at": "2026-04-04T12:00:00Z"
}
```

This endpoint SHALL:
- Validate adapter name against known adapters (from Elucid adapter registry)
- Store business document references in `elucid_pipelines` table
- Compile and store policy bundle (FR-003)
- Return pipeline ID for subsequent API calls

**FR-002: Pipeline Status**

```
GET /api/projects/:id/elucid/status

→ 200 OK
{
  "pipeline_id": "uuid",
  "status": "running",
  "adapter": "next-prisma",
  "current_stage": "s2-service-requirements",
  "stages": {
    "s0-preflight": { "status": "completed", "completed_at": "..." },
    "s1-business-requirements": { "status": "completed", "confirmed_by": "user@org.com", "confirmed_at": "..." },
    "s2-service-requirements": { "status": "in_progress", "started_at": "..." }
  },
  "scaffolding": null,
  "token_spend": {
    "total": 45200,
    "budget": 2000000,
    "by_stage": { "s0": 1200, "s1": 32000, "s2": 12000 }
  },
  "started_at": "2026-04-04T12:00:00Z"
}
```

**FR-003: Policy Bundle Compilation**
Stagecraft SHALL compile an Elucid-specific policy bundle combining:

1. **Organization defaults** — from org's policy settings in Stagecraft DB
2. **Adapter-specific shard** — generated from adapter manifest (allowed paths, commands, invariants)
3. **Project overrides** — from `policy_overrides` in init request

```typescript
interface ElucidPolicyBundle {
  id: string;
  project_id: string;
  adapter: string;
  compiled_at: string;
  
  rules: {
    allowed_adapters: string[];
    max_retry_per_feature: number;
    require_stage_approval: number[];    // which stages need human sign-off
    auto_approve_stages: number[];       // stages that can auto-proceed
    token_budget: {
      per_stage_agent: number;
      per_feature_agent: number;
      total_pipeline: number;
    };
    file_write_scope: string[];          // from adapter directory_conventions
    allowed_commands: string[];          // from adapter commands
    blocked_patterns: string[];          // dangerous command patterns
  };
}
```

The compiled bundle travels to OPC via the status endpoint and is applied by axiomregent during pipeline execution.

**FR-004: Stage Confirmation**

```
POST /api/projects/:id/elucid/stage/:stageId/confirm
Authorization: Bearer <token>

{
  "notes": "Entities look correct. Added a note about fiscal_year field."
}

→ 200 OK
{
  "stage": "s1-business-requirements",
  "confirmed_by": "user@org.com",
  "confirmed_at": "2026-04-04T14:30:00Z",
  "audit_entry_id": "uuid"
}
```

This endpoint SHALL:
- Verify the user has permission to confirm stages for this project
- Record the confirmation in `elucid_audit_log` with approver identity
- Notify OPC desktop to release the gate (via WebSocket or polling)

**FR-005: Stage Rejection**

```
POST /api/projects/:id/elucid/stage/:stageId/reject
Authorization: Bearer <token>

{
  "feedback": "Missing 'fiscal_year' field on FundingRequest entity. Also need state machine for approval workflow."
}

→ 200 OK
{
  "stage": "s1-business-requirements",
  "rejected_by": "user@org.com",
  "rejected_at": "2026-04-04T14:30:00Z",
  "feedback": "...",
  "audit_entry_id": "uuid"
}
```

Rejection SHALL:
- Record in audit log with full feedback text
- Signal OPC to re-run the stage with feedback prepended to agent instruction

**FR-006: Audit Trail**

```
GET /api/projects/:id/elucid/audit?from=2026-04-04&limit=100

→ 200 OK
{
  "entries": [
    {
      "id": "uuid",
      "timestamp": "2026-04-04T12:00:00Z",
      "event": "pipeline_initialized",
      "actor": "user@org.com",
      "details": { "adapter": "next-prisma", "doc_count": 3 }
    },
    {
      "id": "uuid",
      "timestamp": "2026-04-04T14:30:00Z",
      "event": "stage_confirmed",
      "actor": "user@org.com",
      "stage": "s1-business-requirements",
      "details": { "entity_count": 19, "uc_count": 24, "br_count": 15 }
    },
    {
      "id": "uuid",
      "timestamp": "2026-04-04T15:00:00Z",
      "event": "scaffold_feature_failed",
      "stage": "s6c-api-list-assessments",
      "details": { "retry": 3, "error": "Type 'Assessment' not found", "tokens_spent": 4200 }
    }
  ],
  "total": 47
}
```

Audit events:
- `pipeline_initialized` — adapter, doc count, policy bundle
- `stage_started` — stage ID, agent model
- `stage_completed` — stage ID, artifact count, tokens spent
- `stage_confirmed` — stage ID, confirmer, summary stats
- `stage_rejected` — stage ID, rejector, feedback
- `build_spec_frozen` — hash, entity/operation/page counts
- `scaffold_feature_completed` — feature ID, files created, tokens
- `scaffold_feature_failed` — feature ID, error, retry count
- `pipeline_completed` — total tokens, wall time, success rate
- `pipeline_failed` — failure reason, partial progress
- `deployment_triggered` — target environment

**FR-007: Deployment Handoff**

```
POST /api/projects/:id/elucid/deploy
Authorization: Bearer <token>

{
  "environment": "staging",
  "git_ref": "feat/074-elucid-ingestion",
  "registry_image": "acr.azurecr.io/project:v1"
}

→ 202 Accepted
{
  "deployment_id": "uuid",
  "target": "staging",
  "status": "queued"
}
```

This endpoint SHALL:
- Verify pipeline status is `completed`
- Forward deployment request to deployd-api-rs
- Record deployment in audit trail

**FR-008: Pipeline Token Spend Reporting**

```
POST /api/projects/:id/elucid/token-spend
Authorization: Bearer <internal-service-token>

{
  "run_id": "uuid",
  "stage_id": "s1-business-requirements",
  "prompt_tokens": 2400,
  "completion_tokens": 29600,
  "model": "claude-opus-4-6"
}

→ 204 No Content
```

OPC desktop reports token spend per-step to Stagecraft for centralized tracking and billing.

### Non-Functional Requirements

**NF-001: Audit Immutability**
Audit log entries SHALL be append-only. No endpoint permits modification or deletion of audit records.

**NF-002: Rate Limiting**
Elucid API endpoints SHALL be rate-limited per project (not per user) to prevent runaway pipelines:
- Stage confirm/reject: 10/min per project
- Token spend reporting: 100/min per project (one per scaffold step)
- Status polling: 60/min per project

**NF-003: Multi-Tenancy**
All queries SHALL be scoped to the authenticated user's organization. Cross-org pipeline access SHALL be denied.

## Architecture

### Database Schema Extensions

```sql
-- Elucid pipeline tracking
CREATE TABLE elucid_pipelines (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id),
  adapter_name VARCHAR(100) NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'initialized',  -- initialized, running, paused, completed, failed
  policy_bundle_id UUID,
  build_spec_hash VARCHAR(64),  -- SHA-256, set after freeze
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Business document references
CREATE TABLE elucid_business_docs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pipeline_id UUID NOT NULL REFERENCES elucid_pipelines(id),
  name VARCHAR(255) NOT NULL,
  storage_ref TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Stage progress tracking (synced from OPC)
CREATE TABLE elucid_stages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pipeline_id UUID NOT NULL REFERENCES elucid_pipelines(id),
  stage_id VARCHAR(50) NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'pending',
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  confirmed_by VARCHAR(255),
  confirmed_at TIMESTAMPTZ,
  rejected_by VARCHAR(255),
  rejected_at TIMESTAMPTZ,
  rejection_feedback TEXT,
  prompt_tokens INTEGER DEFAULT 0,
  completion_tokens INTEGER DEFAULT 0,
  model VARCHAR(50),
  UNIQUE(pipeline_id, stage_id)
);

-- Scaffolding feature tracking
CREATE TABLE elucid_scaffold_features (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pipeline_id UUID NOT NULL REFERENCES elucid_pipelines(id),
  feature_id VARCHAR(100) NOT NULL,
  category VARCHAR(20) NOT NULL,  -- data, api, ui, configure, trim, validate
  status VARCHAR(20) NOT NULL DEFAULT 'pending',
  retry_count INTEGER DEFAULT 0,
  last_error TEXT,
  files_created TEXT[],  -- array of file paths
  prompt_tokens INTEGER DEFAULT 0,
  completion_tokens INTEGER DEFAULT 0,
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  UNIQUE(pipeline_id, feature_id)
);

-- Immutable audit log
CREATE TABLE elucid_audit_log (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pipeline_id UUID NOT NULL REFERENCES elucid_pipelines(id),
  timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  event VARCHAR(50) NOT NULL,
  actor VARCHAR(255),
  stage_id VARCHAR(50),
  feature_id VARCHAR(100),
  details JSONB NOT NULL DEFAULT '{}'
);
CREATE INDEX idx_elucid_audit_pipeline ON elucid_audit_log(pipeline_id, timestamp);

-- Policy bundles
CREATE TABLE elucid_policy_bundles (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id),
  adapter_name VARCHAR(100) NOT NULL,
  rules JSONB NOT NULL,
  compiled_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Encore.ts Service Structure

```
platform/services/stagecraft/
  elucid/
    elucid.service.ts       — Encore service definition
    elucid.controller.ts    — API endpoint handlers
    elucid.repository.ts    — Database queries (Drizzle)
    elucid.policy.ts        — Policy bundle compilation logic
    elucid.types.ts         — TypeScript interfaces
    elucid.test.ts          — Integration tests
  schema/
    elucid-tables.ts        — Drizzle schema definitions
```

### Integration Points

```
OPC Desktop ──POST /token-spend──→ Stagecraft (audit tracking)
OPC Desktop ──GET /status──→ Stagecraft (policy bundle, confirmations)
OPC Desktop ──WebSocket──→ Stagecraft (gate release notifications)
Stagecraft ──POST /deploy──→ deployd-api-rs (deployment handoff)
Stagecraft ──webhook──→ Slack/GitHub (pipeline notifications)
```

## Implementation Approach

### Phase 1: Schema & Core Endpoints (3 days)

1. Create Drizzle schema for Elucid tables
2. Run migration
3. Implement `POST /elucid/init` — project initialization
4. Implement `GET /elucid/status` — pipeline status
5. Implement `GET /elucid/audit` — audit trail query

### Phase 2: Stage Lifecycle (2 days)

1. Implement `POST /elucid/stage/:id/confirm` — with audit logging
2. Implement `POST /elucid/stage/:id/reject` — with feedback storage
3. Implement `POST /elucid/token-spend` — token reporting from OPC

### Phase 3: Policy Bundle (2 days)

1. Implement policy compilation logic (org defaults + adapter shard + overrides)
2. Store compiled bundles in DB
3. Serve bundle via status endpoint for OPC consumption

### Phase 4: Deployment Handoff (1 day)

1. Implement `POST /elucid/deploy` — forward to deployd-api-rs
2. Record deployment in audit trail
3. Webhook notification on deployment trigger

## Success Criteria

- **SC-001**: `POST /elucid/init` creates pipeline, stores docs, compiles policy bundle
- **SC-002**: Stage confirm/reject records in audit log with approver identity
- **SC-003**: Policy bundle includes adapter-specific allowed paths and commands
- **SC-004**: Audit trail returns chronological entries with correct event types
- **SC-005**: Token spend accumulates correctly across stages and scaffolding
- **SC-006**: Deployment handoff reaches deployd-api-rs with correct payload

## Dependencies

| Spec | Relationship |
|------|-------------|
| 074-elucid-ingestion | Adapter names and validation |
| 075-elucid-workflow-engine | Pipeline state synced from OPC |
| 076-elucid-desktop-panel | UI calls these endpoints |
| deployd-api-rs | Receives deployment requests |

## Risks

| Risk | Mitigation |
|------|-----------|
| OPC ↔ Stagecraft sync lag | OPC is source of truth for execution; Stagecraft is source of truth for governance. Async sync with eventual consistency. |
| Token spend double-counting | Idempotent token-spend endpoint keyed on (run_id, stage_id, step_id) |
| Audit log growth | Partition by pipeline_id; retention policy per org |
