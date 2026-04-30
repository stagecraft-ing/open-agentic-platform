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

## Factory project scaffold

Project creation and import live under `api/projects/`:

- `create.ts` (spec 112 §5) — `POST /api/projects/factory-create`. ACP-native; writes commit #1 with a `.factory/pipeline-state.json` L0 seed, links the project to a `factory_adapters` row, and returns an `opc://` deep link.
- `import.ts` (spec 112 §6) — `POST /api/projects/factory-import`. Clones the repo, shells the `factory-project-detect` CLI for a governed detection read, branches on the level (reject / translate / register), and emits a `project.imported` audit event.
- `clone.ts` (spec 113) — `POST /api/projects/{sourceProjectId}/clone`. Mirror-clones a source project's primary repo into the caller's current OAP org installation, registers a new project bound to that repo, hydrates raw artefacts via the same `registerRawArtifactsFromRepo` path as import, and emits a `project.cloned` audit event. Default-vs-user-typed name semantics resolve collisions per FR-029/FR-030; rollback deletes the destination repo on any post-create failure.
- `cloneAvailability.ts` (spec 113) — `GET /api/projects/clone/check-availability`. Read-only, idempotent verdict for the Clone dialog's debounced field checks.
- `scaffold/` — the six absorbed operations (template cache, prebuilds, adapter scaffold runner, GitHub repo create, initial push, artefact extraction) plus pure helpers (`deepLink`, `seedPipelineState`, `pickProfile`).

The `template-distributor` external service is retired — all scaffold work for newly-created factory projects happens in-process here under the org's existing GitHub App installation.

## Knowledge extraction pipeline (spec 115)

Replaces the manual click-walk on `imported → extracting → extracted` with an
automatic, agent-aware pipeline. Mirrors the spec 114 clone-pipeline shape
(Topic + Subscription + run-row + CAS + staleness sweeper).

Module map:

- `api/knowledge/extractionEvents.ts` — `KnowledgeExtractionRequestTopic` (PubSub, at-least-once)
- `api/knowledge/extractionCore.ts` — `enqueueExtraction`, `runExtractionWork`, `markRunFailed`, `sweepStaleExtractionRuns`
- `api/knowledge/extractionWorker.ts` — Subscription wrapper, drives `runExtractionWork`
- `api/knowledge/scheduler.ts` — `extraction-staleness-sweeper` cron (60s)
- `api/knowledge/extractionPolicy.ts` — project policy slice resolver (loads `build/policy/projects/{projectId}.json`, 30s cache, deterministic-only fallback). Spec 119 §4.5 renamed the on-disk layout from per-workspace to per-project.
- `api/knowledge/extractionOutput.ts` — Zod-validated `ExtractionOutput` contract (FR-016)
- `api/knowledge/prompts.ts` — versioned prompt registry; `promptFingerprint = sha256(kind|version|system)` (FR-020)
- `api/knowledge/magic.ts` — pure mime-sniff + reconcile (FR-014); `storage.sniffMimeType` is the S3-backed wrapper
- `api/knowledge/extractors/`
  - `dispatch.ts` — registry + `pickExtractor` (FR-011)
  - `types.ts` — `Extractor` interface + `ExtractorError` (FR-015)
  - `deterministic-text.ts`, `deterministic-pdf-embedded.ts`, `deterministic-docx.ts`
  - `agent-base.ts` — Anthropic client + cost gates + `runAgentMessage` with prompt caching (FR-017–FR-021)
  - `agent-pdf-vision.ts`, `agent-image-vision.ts`
  - `agent-cost-helpers.ts` — pure cost estimator + `assertNoTools` (FR-018, FR-021)
  - `index.ts` — registers every extractor cheapest-first; side-effect imported by `extractionCore` and `extractionWorker`

Entry points (the four "what to call from the rest of the codebase" surfaces):

- `enqueueExtraction({ knowledgeObjectId, projectId, reason })` — call from any path that lands a row in `imported` (today: `confirmUpload`, `executeSyncRun`). Spec 119 collapsed the legacy workspace scope into project.
- `runExtractionWork({ extractionRunId })` — invoked by the Subscription; tests can drive it directly with a real DB.
- `pickExtractor(input, policy)` — synchronous; returns `null` when no extractor matches.
- `retryExtraction` endpoint at `POST /api/projects/:projectId/knowledge/objects/:id/retry-extraction` — operator re-enqueue for failed objects.

Env knobs:

- `STAGECRAFT_EXTRACT_STALE_AFTER_SEC` — sweeper cutoff (default 600)
- `STAGECRAFT_EXTRACT_MAX_AUTO_RETRIES` — auto-retry cap before manual Retry needed (default 2)
- `STAGECRAFT_EXTRACT_EAGER_BUFFER_BYTES` — worker eager-load threshold (default 4MB)
- `STAGECRAFT_EXTRACT_PDF_MIN_MEDIAN_CHARS` — embedded-text PDF threshold (default 80)
- `STAGECRAFT_EXTRACT_LEGACY_TRANSITION` — set to `"true"` to re-enable the legacy click-walk endpoint for incident response (FR-027)
- `STAGECRAFT_EXTRACT_POLICY_DIR` — override for compiled policy snapshot dir (default `build/policy/projects`; spec 119 §4.5)
- `STAGECRAFT_EXTRACT_PRICE_*_USD_PER_MTOK` — Anthropic pricing overrides for the cost estimator
- `ANTHROPIC_API_KEY` — required secret; agent extractors fail closed without it
