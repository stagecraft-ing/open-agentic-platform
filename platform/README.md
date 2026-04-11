# Platform — Organisational Control Plane

The platform layer is the organisational control plane for Open Agentic Platform. It provides identity, deployment orchestration, workspace management, knowledge intake, and audit infrastructure.

**Imported from:** `stagecraft-ing/platform` (git subtree, 2026-03-31)

## Services

| Service | Stack | Path | Purpose |
|---------|-------|------|---------|
| **stagecraft** | Encore.ts, Drizzle ORM | `services/stagecraft/` | SaaS platform: auth, admin, monitoring, Slack, GitHub webhooks, Factory lifecycle API |
| **deployd-api-rs** | Rust (axum, hiqlite) | `services/deployd-api-rs/` | K8s deployment orchestration with Helm, OIDC JWT auth |
| **tenant-hello** | Express.js | `services/tenant-hello/` | Example tenant service |

> GitHub webhook handling was absorbed into stagecraft (`services/stagecraft/api/github/`).

## Infrastructure

Two-layer Terraform in `infra/terraform/`:

1. **Core** (`envs/dev/core/`) — Azure Resource Group, AKS cluster, ACR, Key Vault
2. **Cluster** (`envs/dev/cluster/`) — Ingress-nginx, cert-manager, CSI secrets provider

Helm charts in `charts/`: stagecraft, deployd-api, rauthy (OIDC identity provider).

## Local Development

```bash
# Stagecraft (requires Encore CLI)
cd services/stagecraft && npm run start
# → http://localhost:4000 (app), http://localhost:9400 (Encore dashboard)

# Deployd-API (Rust)
cargo build --release --manifest-path services/deployd-api-rs/Cargo.toml
```

## Azure Deployment

```bash
make tf-init      # Init both Terraform layers
make tf-apply     # Create Azure resources + deploy
make docker-push  # Build and push container images to ACR
make tf-destroy   # Tear down everything
```

## Further Reading

- [`CLAUDE.md`](CLAUDE.md) — Platform layer technical reference (database schema, identity, integration points)
- [`services/stagecraft/README.md`](services/stagecraft/README.md) — Stagecraft service documentation
- [`services/stagecraft/CLAUDE.md`](services/stagecraft/CLAUDE.md) — Encore.ts conventions
- [`infra/terraform/MIGRATION.md`](infra/terraform/MIGRATION.md) — Terraform state migration guide

## Specs

- Spec 072 — Multi-cloud K8s portability
- Spec 077 — Stagecraft Factory API
- Spec 087 — Unified Workspace Architecture
