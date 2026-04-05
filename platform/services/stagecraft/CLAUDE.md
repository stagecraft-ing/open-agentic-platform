# CLAUDE.md — Stagecraft (Encore.ts)

Stagecraft is the SaaS control plane built on **Encore.ts**. These conventions apply when working in this directory.

## Framework

- Backend: [Encore.ts](https://encore.dev) — type-safe TypeScript APIs with built-in infra primitives
- ORM: Drizzle (PostgreSQL)
- Frontend: React Router v7 (in `web/`)
- Package manager: **npm** (not pnpm — excluded from root workspace)
- Node.js v20+, ES6+ syntax, `import` only (never `require`)

## Encore.ts Patterns

- Define APIs with `import { api } from "encore.dev/api"` — not raw Express handlers
- Service-to-service calls use `import { serviceName } from "~encore/clients"`
- Auth data accessed via `import { getAuthData } from "~encore/auth"`
- Database access via `new SQLDatabase("name", { migrations: "./migrations" })` from `encore.dev/storage/sqldb`
- PubSub via `new Topic<T>("name", { deliveryGuarantee: "at-least-once" })` from `encore.dev/pubsub`
- Secrets via `secret("SecretName")` from `encore.dev/config`
- Structured logging via `import log from "encore.dev/log"`
- Errors via `APIError` / `ErrCode` from `encore.dev/api`

## Local Dev

```bash
cd platform/services/stagecraft && npm run start
# App: http://localhost:4000 | Encore dashboard: http://localhost:9400
```

## Testing

```bash
encore test          # Recommended — sets up test databases, isolated infra per test
npm test             # Direct vitest without infra setup
```

Test API endpoints by calling them directly as functions. Don't mock Encore infrastructure (databases, PubSub) — use the real thing.

## Reference

For full Encore.ts API reference (APIs, databases, PubSub, streaming, auth, middleware, validation, etc.), see [`docs/encore-ts-reference.md`](docs/encore-ts-reference.md).
