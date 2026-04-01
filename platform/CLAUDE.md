# CLAUDE.md — Platform Layer

The platform layer is the organisational control plane for OAP. It provides identity, deployment orchestration, and audit infrastructure running on Azure Kubernetes Service.

**Imported from:** `stagecraft-ing/platform` (git subtree, 2026-03-31)

## Services

| Service | Stack | Port | Purpose |
|---------|-------|------|---------|
| **stagecraft** | Encore.ts, Drizzle ORM, React Router v7 | 4000 | SaaS platform: auth, admin, uptime monitoring, Slack integration |
| **deployd-api** | Express.js, @kubernetes/client-node | 8080 | K8s deployment orchestration with Helm, Logto JWT auth |
| **github-app** | Probot | 3000 | PR preview deployments via GitHub webhooks |
| **tenant-hello** | Express.js | 8080 | Example tenant service |

## Package Manager

Platform services use **npm** (not pnpm). They are excluded from the root `pnpm-workspace.yaml`. Each service has its own `package.json` and `package-lock.json`.

## Database

Stagecraft uses PostgreSQL via Drizzle ORM. Schema is in `services/stagecraft/api/db/schema.ts`:
- `users` — accounts with roles (user/admin), email, password hash
- `sessions` — session tokens with 14-day TTL, user/admin kinds
- `audit_log` — append-only audit trail (actor, action, target, metadata JSONB)

## Identity

Authentication is handled by **Logto** (self-hosted OAuth2/OIDC). Helm chart in `charts/logto/`. Machine-to-machine auth uses Logto client credentials flow.

## Local Development

```bash
# Stagecraft (requires Encore CLI: https://encore.dev/docs/install)
cd services/stagecraft && npm run start
# → http://localhost:4000 (app), http://localhost:9400 (Encore dashboard)

# Deployd-API
cd services/deployd-api && npm run dev
# → http://localhost:8080

# GitHub App
cd services/github-app && npm start
# → http://localhost:3000
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
- `charts/logto/` — Logto identity provider (ingress: logto.localdev.online)

## Key Integration Points with OPC

- **Policy bundle serving** — stagecraft can serve compiled policy bundles to axiomregent over HTTP
- **Audit streaming** — axiomregent can POST audit records to stagecraft's `audit_log` table
- **Permission grants** — stagecraft auth can provide workspace-scoped grants to the desktop app
- **Agent authorization** — stagecraft can validate agent execution against org-level policies
