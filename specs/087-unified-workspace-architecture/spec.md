---
id: "087"
slug: unified-workspace-architecture
title: Unified Workspace Architecture
status: active
owner: bart
created: 2026-04-09
depends_on:
  - "074"  # factory-ingestion
  - "075"  # factory-workflow-engine
  - "076"  # factory-desktop-panel
  - "077"  # stagecraft-factory-api
  - "078"  # platform-completion-plan
  - "080"  # github-identity-onboarding
  - "082"  # artifact-integrity-platform-hardening
---

# 087 — Unified Workspace Architecture

## 1. Problem Statement

Open Agentic Platform has two user-facing planes (web and desktop), a delivery engine (factory), and an organisational control plane (stagecraft). All are individually functional but architecturally siloed: separate auth paths, separate state models, and no shared workspace abstraction. Users experience three tools rather than one system.

Additionally, the business documents that feed factory pipelines are treated as loose attachments rather than a first-class knowledge substrate. Documents may arrive from direct upload, SharePoint, or other enterprise sources, but there is no normalisation layer, no provenance model, and no workspace-scoped knowledge domain.

This spec defines the unified architecture that makes the two planes parts of one system, elevates knowledge intake to a first-class workspace domain, and positions the factory as an execution artifact that transforms workspace knowledge into project output.

## 2. Core Model

### 2.1 Entity Hierarchy

```
GitHub Organization (trust anchor)
  └── Workspace (operational container)
        ├── Identity & Membership
        │     ├── members[] (via GitHub org membership + RBAC roles)
        │     └── service accounts[] (M2M tokens)
        │
        ├── Projects[]
        │     ├── repos[] (GitHub repositories)
        │     ├── environments[] (preview / dev / staging / prod)
        │     └── document_bindings[] → Knowledge Objects
        │
        ├── Knowledge Intake
        │     ├── source_connectors[] (upload, SharePoint, S3, Azure Blob, GCS)
        │     ├── knowledge_objects[] (canonical normalised documents)
        │     ├── extraction_state[] (OCR, classification, structured output)
        │     └── provenance[] (source origin, sync state, version history)
        │
        ├── Factories[]
        │     ├── scoped to: workspace + project
        │     ├── consumes: selected knowledge_objects
        │     └── produces: project artifacts + repository changes
        │
        ├── Policy Bundle (compiled governance rules)
        │
        ├── Grants (per-user permission matrix)
        │
        └── Audit Trail (append-only event log)
```

### 2.2 Key Relationships

- A **Workspace** is scoped to exactly one GitHub Organization (the trust anchor).
- A **Project** belongs to one Workspace and is associated with one or more GitHub repositories.
- **Knowledge Objects** belong to the Workspace, not to any specific project. They exist independently of projects and may be consumed by multiple factories across multiple projects.
- A **Factory** is scoped to a workspace + project pair. It consumes selected Knowledge Objects from the workspace and produces project artifacts.
- **Document Bindings** link Knowledge Objects to Projects. A project may bind to many Knowledge Objects; a Knowledge Object may be bound to many projects.
- The **Policy Bundle** is workspace-scoped and includes factory policy shards.

### 2.3 Workspace Definition

> The workspace is the unit of identity, governance, collaboration, and knowledge intake. Business documents may enter through direct upload or enterprise connectors such as SharePoint, but are normalised into workspace object storage with provenance preserved. Projects then bind code repositories to selected workspace knowledge, while factories act as the execution artifact that transforms that knowledge into software outcomes across the desktop and web planes.

A Workspace is the unit of:
- **Billing** — token spend, compute, storage
- **Governance** — policy bundle scope
- **Collaboration** — team membership boundary
- **Knowledge** — canonical document substrate
- **Factory execution** — one active factory per project within a workspace

## 3. Two Planes, One System

### 3.1 Plane Responsibilities

#### Web Plane — stagecraft.ing

The web plane is the governance, management, and collaboration surface. It is read-heavy: observe, govern, approve, configure.

**Owns:**
1. Identity and onboarding — GitHub OAuth, Rauthy sessions, org discovery, workspace creation
2. Workspace administration — team invites, role assignment, billing, usage dashboards
3. Project lifecycle — create project, link repos, configure environments, set auto-deploy rules
4. Knowledge intake — connector configuration, document import, extraction monitoring, object browsing
5. Pipeline overview — read-only dashboard of active factory pipelines across the workspace
6. Gate approval (web) — approve/reject factory stages from the browser (for stakeholders without OPC)
7. Deploy promotion — promote builds between environments, trigger rollbacks
8. Audit and compliance — full audit trail viewer, export, retention policies

**Does NOT:**
- Execute Claude Code sessions
- Run factory pipeline stages
- Generate code or scaffolding
- Directly interact with the filesystem or git

#### Desktop Plane — OPC

The desktop plane is the execution surface. It is write-heavy: execute, generate, verify, commit.

**Owns:**
1. Local execution — Claude Code sessions, agent runs, worktree-based parallel agents
2. Factory stage execution — runs the 7-stage pipeline locally via `crates/factory-engine`
3. Code generation — scaffold fan-out, adapter-specific code generation
4. Git operations — commit, branch, checkpoint, restore, diff
5. Governance enforcement — axiomregent sidecar, permission gating, safety tiers
6. Artifact inspection — view generated files, diffs, build outputs
7. Gate approval (local) — approve/reject stages from the desktop (for active developers)
8. Offline capability — all of the above works without platform connectivity

**Does NOT:**
- Manage team membership or billing
- Store the authoritative audit trail
- Own identity (receives identity from the platform)
- Make deployment decisions (requests deployments via the platform)

### 3.2 Factory Position

The Factory is the execution artifact of the workspace:
- Scoped by workspace and project
- Fed by selected workspace knowledge objects
- Executed locally on the desktop plane
- Observed and governed through the web plane

Factory initiation can happen from either plane, but execution always happens on Desktop because code generation requires filesystem access, agent execution requires local compute, git operations require a local working tree, and governance enforcement requires the axiomregent sidecar.

## 4. Knowledge Intake Domain

### 4.1 Architecture

```
External Sources                    Workspace
┌──────────────┐                   ┌──────────────────────────────┐
│  Direct      │──── upload ──────→│                              │
│  Upload      │                   │   Canonical Object Store     │
├──────────────┤                   │   (S3-compatible)            │
│  SharePoint  │──── connector ──→│                              │
│  Online      │     sync/import  │   knowledge_objects[]        │
├──────────────┤                   │     ├── content (blob)       │
│  Azure Blob  │──── connector ──→│     ├── content_hash         │
│  / S3 / GCS  │     import       │     ├── mime_type            │
├──────────────┤                   │     ├── extraction_state     │
│  Future      │──── connector ──→│     ├── classification[]     │
│  Connectors  │                   │     ├── provenance           │
└──────────────┘                   │     │    ├── source_type     │
                                   │     │    ├── source_uri      │
                                   │     │    ├── imported_at     │
                                   │     │    ├── last_synced_at  │
                                   │     │    └── version_id      │
                                   │     └── structured_extract   │
                                   │                              │
                                   │   source_connectors[]        │
                                   │     ├── type (sharepoint,    │
                                   │     │   s3, upload, ...)     │
                                   │     ├── config (encrypted)   │
                                   │     ├── sync_schedule        │
                                   │     └── status               │
                                   └──────────────────────────────┘
```

### 4.2 Design Principles

1. **Canonical storage, not pass-through.** All documents are materialised into the workspace's S3-compatible object store regardless of source. S3 is the normalisation layer, not just "where uploads go."

2. **Connectors are origins, not authorities.** SharePoint, Azure Blob, GCS, etc. are external knowledge sources. After intake, the platform treats documents as workspace knowledge objects in canonical storage. This prevents split-brain document models.

3. **Provenance is preserved.** Every knowledge object carries provenance metadata linking back to its source connector, original URI, sync timestamp, and version. This enables re-sync, audit, and traceability without keeping the external source as a live dependency.

4. **Workspace-scoped, not project-scoped.** Knowledge objects belong to the workspace and may be consumed by multiple projects and factories. Documents may be imported before a project exists, reused across factories, or bound to new projects later. Projects bind to knowledge objects via explicit document bindings.

5. **Extraction is a pipeline stage.** After intake, documents pass through extraction (OCR, text extraction), classification (by platform concern: client, API, data, infra), and structured output generation. This state is tracked per knowledge object.

6. **Factory consumes normalised knowledge.** A factory's document input is a selection of knowledge objects from the workspace store. The factory does not know or care whether a document came from upload or SharePoint — it operates over a consistent document model.

### 4.3 Knowledge Object Lifecycle

```
source → intake/import → canonical storage → extraction → classification → selectable
                                                                              │
                                                              ┌───────────────┘
                                                              ▼
                                                    Factory pipeline
                                                    (selected documents
                                                     become stage 0-1 input)
```

States: `imported` → `extracting` → `extracted` → `classified` → `available`

A knowledge object in state `available` can be bound to a project and selected as input to a factory pipeline.

### 4.4 Connector Model

| Connector Type | Auth | Sync | Notes |
|---------------|------|------|-------|
| `upload` | Session (user-initiated) | One-shot | Direct browser/API upload |
| `sharepoint` | OAuth2 (Microsoft Graph) | Scheduled or on-demand | Folder-level sync, delta query |
| `azure-blob` | SAS token or managed identity | Scheduled | Container/prefix scoped |
| `s3` | IAM role or access key | Scheduled | Bucket/prefix scoped |
| `gcs` | Service account | Scheduled | Bucket/prefix scoped |

Connectors are workspace-scoped configuration objects. Adding a new connector type requires implementing a single `SourceConnector` trait/interface — no changes to the core knowledge object model.

## 5. Sync Protocol

### 5.1 Authoritative Ownership

| Domain | Authoritative Plane | Sync Direction |
|--------|-------------------|----------------|
| Identity & membership | Web (Stagecraft) | Web → Desktop |
| Policy bundles | Web (Stagecraft) | Web → Desktop |
| Audit trail | Web (Stagecraft) | Desktop → Web |
| Knowledge objects | Web (Stagecraft) | Web → Desktop (metadata only) |
| Pipeline execution state | Desktop (OPC) | Desktop → Web |
| Artifact content | Desktop (OPC) | Desktop → Web (hashes + upload) |
| Checkpoint state | Desktop (OPC) | Local only |
| Local git state | Desktop (OPC) | Local only |

### 5.2 Communication

```
┌──────────────┐                           ┌──────────────┐
│  STAGECRAFT  │                           │     OPC      │
│  (web plane) │                           │  (desktop)   │
│              │                           │              │
│              │──── WebSocket ────────────→│              │
│              │     push: policy change,   │              │
│              │     gate approval,         │              │
│              │     deploy status,         │              │
│              │     knowledge object ready │              │
│              │                           │              │
│              │←── HTTP POST ─────────────│              │
│              │    push: stage complete,   │              │
│              │    token spend,            │              │
│              │    artifact hash,          │              │
│              │    audit events            │              │
└──────────────┘                           └──────────────┘
```

A WebSocket relay on Stagecraft pushes events to connected OPC instances, scoped by workspace. OPC pushes state updates to Stagecraft via HTTP POST (fire-and-forget, best-effort — failures never block local execution).

## 6. Identity Consolidation

```
GitHub App Installation ──→ Organization (trust anchor)
GitHub OAuth             ──→ User identity + org membership
Rauthy                   ──→ Session tokens (JWT) for both planes
                              M2M tokens for OPC ↔ Stagecraft
```

One identity, two sessions:
- **Web session:** Rauthy-issued JWT in browser cookie, workspace-scoped
- **Desktop session:** Rauthy-issued JWT in OS keychain, workspace-scoped
- Both obtained via GitHub OAuth (web redirects to browser, desktop uses `opc://` deep-link callback)

Password-based auth is removed once Rauthy is fully wired (spec 080).

## 7. Shared Component Architecture

```
packages/
  @opc/ui/                ← shared React component library (exists)
  @opc/types/             ← shared TypeScript types (exists)
  @opc/workspace-sdk/     ← NEW: workspace domain client
    ├── workspace.ts       — Workspace, Project, KnowledgeObject types
    ├── knowledge.ts       — Knowledge intake types and state machine
    ├── factory.ts         — Factory pipeline state machine
    ├── sync.ts            — WebSocket + HTTP sync protocol
    └── auth.ts            — Token management (Rauthy JWT)
```

`@opc/workspace-sdk` is consumed by both the Stagecraft web UI and the OPC React frontend, ensuring identical domain models and state rendering across planes.

## 8. Implementation Phases

### Phase 1: Workspace Foundation

- Add `workspaces` table to Stagecraft schema
- Workspace CRUD endpoints
- Replace `DEFAULT_ORG_ID` with workspace-scoped queries across all Stagecraft endpoints
- Add workspace sync to OPC's `StagecraftClient`
- Create `@opc/workspace-sdk` package with core types

### Phase 2: Knowledge Intake Domain

- Add `knowledge_objects`, `source_connectors`, `document_bindings` tables
- Knowledge object CRUD + upload endpoint
- S3-compatible object store integration (MinIO for local dev, S3/Azure Blob for prod)
- Extraction pipeline (OCR, text extraction, classification)
- Provenance tracking
- Factory pipeline wiring: stage 0 pre-flight reads selected knowledge objects

### Phase 3: Web UI + Sync Channel

- Stagecraft web UI (React SPA sharing `@opc/ui`)
- WebSocket relay on Stagecraft (workspace-scoped event channel)
- OPC WebSocket client for real-time updates
- Factory pipeline dashboard (web)
- Knowledge object browser and selection UI (web)
- Deploy status and promotion UI (web)

### Phase 4: Connector Framework

- `SourceConnector` trait/interface with pluggable implementations
- SharePoint Online connector (Microsoft Graph API, OAuth2, delta sync)
- Upload connector (finalise as the reference implementation)
- Connector configuration UI in workspace settings
- Scheduled sync with change detection

### Phase 5: Identity + Governance Hardening

- Remove password auth from Stagecraft
- OIDC JWT enforcement on all seams (A, B, C, D) per spec 082
- OS keychain storage for desktop session
- Unified RBAC model in `@opc/workspace-sdk`

## 9. Database Schema Additions (Stagecraft)

```sql
-- Workspace: operational container scoped to a GitHub org
CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES organizations(id),
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    object_store_bucket TEXT NOT NULL,  -- S3-compatible bucket for this workspace
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(org_id, slug)
);

-- Source connector: external knowledge source configuration
CREATE TABLE source_connectors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    type TEXT NOT NULL,          -- 'upload' | 'sharepoint' | 's3' | 'azure-blob' | 'gcs'
    name TEXT NOT NULL,
    config_encrypted JSONB,      -- connector-specific config (credentials encrypted)
    sync_schedule TEXT,           -- cron expression for scheduled sync
    status TEXT NOT NULL DEFAULT 'active',
    last_synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Knowledge object: canonical normalised document in workspace store
CREATE TABLE knowledge_objects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    connector_id UUID REFERENCES source_connectors(id),
    storage_key TEXT NOT NULL,    -- S3 object key within workspace bucket
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    content_hash TEXT NOT NULL,   -- SHA-256 of content
    state TEXT NOT NULL DEFAULT 'imported',  -- imported|extracting|extracted|classified|available
    extraction_output JSONB,     -- structured extraction result
    classification JSONB,        -- platform concern tags
    provenance JSONB NOT NULL,   -- { source_type, source_uri, imported_at, version_id }
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Document binding: links knowledge objects to projects
CREATE TABLE document_bindings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id),
    knowledge_object_id UUID NOT NULL REFERENCES knowledge_objects(id),
    bound_by UUID NOT NULL REFERENCES users(id),
    bound_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(project_id, knowledge_object_id)
);
```

## 10. Non-Functional Requirements

- **NF-001:** Knowledge objects must be immutable once in state `available`. Updates create new versions with provenance linking to the prior version.
- **NF-002:** The object store must be S3-compatible (MinIO for local dev, any S3-compatible provider for production).
- **NF-003:** Adding a new connector type requires implementing one trait/interface and registering it — no changes to the knowledge object model or factory integration.
- **NF-004:** OPC must function fully offline. Platform connectivity is optional for all execution operations. Sync happens opportunistically when connectivity is available.
- **NF-005:** The WebSocket relay must be workspace-scoped. An OPC instance only receives events for workspaces it is authenticated to.
- **NF-006:** Factory policy shards are workspace-scoped and travel with the pipeline — they must not require live platform connectivity during execution.

## 11. End-State Model Summary

```
GitHub org          = trust anchor
Workspace           = operational container (identity, governance,
                      collaboration, knowledge intake)
Project             = product/code unit (repos + environment bindings)
Knowledge store     = canonical workspace document substrate
Factory             = execution artifact that transforms workspace
                      knowledge into project output
Desktop (OPC)       = execution plane
Web (stagecraft.ing)= governance and management plane
```

The factory is not merely turning prompts into code. It is operating over a governed body of workspace knowledge.
