# CLAUDE.md — Platform Layer

The platform layer is the organisational control plane for OAP. It provides identity, deployment orchestration, and audit infrastructure running on Azure Kubernetes Service.

**Imported from:** `stagecraft-ing/platform` (git subtree, 2026-03-31)

## Services

| Service | Stack | Port | Purpose |
|---------|-------|------|---------|
| **stagecraft** | Encore.ts, Drizzle ORM, React Router v7 | 4000 | SaaS platform: auth, admin, uptime monitoring, Slack integration; `api/github/` handles webhook ingestion and token brokering (absorbed from github-app) |
| **deployd-api-rs** | Rust (axum, hiqlite) | 8080 | K8s deployment orchestration with Helm, OIDC JWT auth |
| **tenant-hello** | Express.js | 8080 | Example tenant service |

> **github-app** (Probot) has been absorbed into stagecraft. Webhook handling and PR preview deployment logic now lives in `services/stagecraft/api/github/`.

## Package Manager

Platform services use **npm** (not pnpm). They are excluded from the root `pnpm-workspace.yaml`. Each service has its own `package.json` and `package-lock.json`.

## Database

Stagecraft uses PostgreSQL via Drizzle ORM. Schema is in `services/stagecraft/api/db/schema.ts`:
- `users` — accounts with roles (user/admin), email, password hash
- `sessions` — session tokens with 14-day TTL, user/admin kinds
- `audit_log` — append-only audit trail (actor, action, target, metadata JSONB)
- `organizations` — top-level org (name, slug, GitHub identity)
- `workspaces` — operational container scoped to a GitHub org (org_id, name, slug, object_store_bucket). The workspace is the unit of identity, governance, collaboration, knowledge intake, and factory execution (spec 087).
- `projects` — unit of work within a workspace (workspace_id, org_id, name, slug)
- `project_repos` — GitHub repo links (project_id, github_org, repo_name, default_branch)
- `environments` — deployment targets (project_id, name, kind, k8s_namespace, auto_deploy_branch)
- `project_members` — team access (project_id, user_id, role: viewer/developer/deployer/admin)
- `source_connectors` — external knowledge sources (workspace_id, type: upload/sharepoint/s3/azure-blob/gcs, config, sync schedule) (spec 087 Phase 2)
- `knowledge_objects` — canonical normalised documents in workspace object store (workspace_id, storage_key, filename, mime_type, content_hash, state lifecycle: imported→extracting→extracted→classified→available, provenance JSONB) (spec 087 Phase 2)
- `document_bindings` — links knowledge objects to projects (project_id, knowledge_object_id, bound_by) (spec 087 Phase 2)
- `sync_runs` — connector sync execution history (connector_id, workspace_id, status: running/completed/failed, objects_created/updated/skipped, delta_token for incremental sync) (spec 087 Phase 4)

## Identity

Authentication is handled by **Rauthy** (self-hosted OIDC/OAuth2 provider, Rust-based). Helm chart in `charts/rauthy/`. Uses hiqlite (embedded Raft SQLite) for HA — no external database required. Machine-to-machine auth uses standard OIDC client credentials flow.

## Local Development

```bash
# Stagecraft (requires Encore CLI: https://encore.dev/docs/install)
cd services/stagecraft && npm run start
# → http://localhost:4000 (app), http://localhost:9400 (Encore dashboard)

# Deployd-API (Rust)
cargo build --release --manifest-path services/deployd-api-rs/Cargo.toml
# → http://localhost:8080
```

## Infrastructure

Two-layer Terraform in `infra/terraform/`:
1. **Core** (`envs/dev/core/`) — Azure Resource Group, AKS cluster, ACR, Key Vault
2. **Cluster** (`envs/dev/cluster/`) — Ingress-nginx, cert-manager, CSI secrets provider

```bash
make tf-init      # Init both layers
make tf-apply     # Create Azure resources + deploy
make docker-push  # Build and push container images to ACR
make tf-destroy   # Tear down everything
```

## Helm Charts

- `charts/stagecraft/` — Main SaaS service (ingress: stagecraft.localdev.online)
- `charts/deployd-api/` — Deployment orchestrator
- `charts/rauthy/` — Rauthy OIDC identity provider (ingress: rauthy.localdev.online)

## Key Integration Points with OPC

- **Policy bundle serving** — stagecraft can serve compiled policy bundles to axiomregent over HTTP
- **Audit streaming** — axiomregent can POST audit records to stagecraft's `audit_log` table
- **Permission grants** — stagecraft auth can provide workspace-scoped grants to the desktop app
- **Agent authorization** — stagecraft can validate agent execution against org-level policies

## Policy Rules

```policy
id: SHARD-PLATFORM-infra-guard
description: "Block terraform destroy in platform layer without confirmation"
mode: enforce
scope: domain:platform
gate: destructive_operation
```
