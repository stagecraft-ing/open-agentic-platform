# Developer Guide

## Prerequisites

| Tool | Install | Notes |
|------|---------|-------|
| Rust | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | Spec compiler and crates |
| Node.js >=18 | `brew install node` | |
| pnpm | `brew install pnpm` | Root workspace only |
| bun | `brew install bun` | |
| Docker | [docker.com](https://docker.com) | Required for platform services |
| Encore CLI | `brew install encoredev/tap/encore` | Only needed for stagecraft |

Run `make check-deps` to verify the core tools are installed.

**Alternative:** Open in VS Code with the [devcontainer](.devcontainer/devcontainer.json) for a pre-configured environment.

## Quick Start

```bash
make setup          # one-time: install deps, build spec compiler, compile registry
make dev            # desktop app only (Tauri + Vite, hot-reload)
make dev-platform   # platform services (stagecraft + deployd-api, background)
make dev-all        # desktop + platform services
make stop           # kill background platform services
```

## Admin Bootstrap

> **There is no default admin password.** The `BOOTSTRAP_ADMIN_EMAIL` environment variable
> does not seed a user account. It flags an email address so that the first **signup** with
> that email is auto-promoted to the `admin` role. You choose the password yourself during
> registration.

How to create your admin account:

1. Start stagecraft: `make dev-stagecraft` (or `make dev-platform`)
   - The npm script already sets `BOOTSTRAP_ADMIN_EMAIL=admin@example.com`
2. Open http://localhost:4000 and click **Sign Up**
3. Enter `admin@example.com`, pick a name and password
4. Your account is created with `role=admin`
5. All subsequent signups get `role=user`

To use a different admin email, override the env var:

```bash
BOOTSTRAP_ADMIN_EMAIL=you@company.com npm run encore
```

See [`platform/services/stagecraft/api/auth/auth.ts:120-122`](platform/services/stagecraft/api/auth/auth.ts) for the bootstrap logic.

## Platform Services

| Service | Stack | Port | URL |
|---------|-------|------|-----|
| stagecraft | Encore.ts, Drizzle ORM | 4000 | http://localhost:4000 |
| deployd-api | Express.js | 8080 | http://localhost:8080 |
| github-app | Probot | 3000 | http://localhost:3000 |

Stagecraft also exposes the Encore development dashboard at http://localhost:9400.

## Local Kubernetes Cluster

For staging-fidelity testing with k3d:

```bash
cp platform/infra/local/.env.example platform/infra/local/.env
# Edit .env if needed (defaults work for local dev)
make k8s-up    # bootstrap k3d cluster + deploy all services
make k8s-down  # tear down
```

## Package Managers

- **Root workspace** uses **pnpm** (`apps/desktop/`, tools, crates)
- **Platform services** use **npm** (each has its own `package-lock.json`)

Platform services are excluded from `pnpm-workspace.yaml`. Do not run `pnpm install` inside `platform/services/*`.

## Further Reading

- [README.md](README.md) — architecture overview and system vision
- [platform/CLAUDE.md](platform/CLAUDE.md) — platform layer technical reference
- [platform/services/stagecraft/README.md](platform/services/stagecraft/README.md) — full stagecraft service docs
- [apps/desktop/README.md](apps/desktop/README.md) — OPC desktop app
