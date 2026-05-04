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

- `create.ts` (spec 112 §5) — `POST /api/projects/factory-create`. ACP-native; writes commit #1 with a `.factory/pipeline-state.json` L0 seed, links the project to a `factory_adapters` row, auto-provisions a `kind=development` environments row (Phase 7), and returns an `opc://` deep link. Pre-flight checks (warmup readiness, adapter present, upstream PAT configured, spec 112 §10 runtime gate) raise `APIError.failedPrecondition` with the actual cause — never the Encore-wrapped generic 500.
- `scaffoldReadiness.ts` (spec 112 Phase 5) — `GET /api/projects/scaffold-readiness`. Per-org readiness verdict for the Create form: `{ ready, step, progress, hasFactoryAdapter, hasUpstreamPat, canCreate, blocker }`. The web UI renders a banner per-blocker and disables submit until `canCreate` flips.
- `import.ts` (spec 112 §6) — `POST /api/projects/factory-import`. Clones the repo, shells the `factory-project-detect` CLI for a governed detection read, branches on the level (reject / translate / register), and emits a `project.imported` audit event.
- `clone.ts` (spec 113) — `POST /api/projects/{sourceProjectId}/clone`. Mirror-clones a source project's primary repo into the caller's current OAP org installation, registers a new project bound to that repo, hydrates raw artefacts via the same `registerRawArtifactsFromRepo` path as import, and emits a `project.cloned` audit event. Default-vs-user-typed name semantics resolve collisions per FR-029/FR-030; rollback deletes the destination repo on any post-create failure.
- `cloneAvailability.ts` (spec 113) — `GET /api/projects/clone/check-availability`. Read-only, idempotent verdict for the Clone dialog's debounced field checks.
- `scaffold/` — the absorbed scaffold subflow:
  - `templateCache.ts` — clones the upstream template into `${STAGECRAFT_WORKSPACE_DIR}/_template-cache`, runs `npm install`, persists upstream SHA in `.template-commit`. Materialises `_prebuilt-{minimal,public,internal,dual}` via `tsx scripts/setup-{app,dual-app}.ts`, persists prebuild SHA in `.prebuilt-commit`. Module-scoped `initStatus` drives the readiness endpoint.
  - `scheduler.ts` — Encore `CronJob("scaffold-warmup-refresher", every: "30m")` plus a fire-and-forget warmup at module load. Resolves `(templateRemote, branch, PAT)` from the first eligible org (adapter declaring `template_remote` + configured upstream PAT).
  - `perRequestScaffold.ts` — copies the chosen prebuilt tree into a per-request temp dir, runs `tsx add-module.ts <id>` for each user-selected extra, refreshes the lockfile via `npm install --package-lock-only`, writes `.factory/pipeline-state.json`.
  - `gitInitAndPush.ts` — `git init -b <branch>` → `add` → `commit` → token-injected push, then `git remote set-url origin <bare>` so the token does not survive in `.git/config`. Subprocess output is token-redacted before any error surface.
  - `githubRepoCreate.ts` — wraps `createGitHubRepo` with `autoInit: false` so commit #1 is the scaffold tree, not an auto-generated README.
  - `moduleCatalog.ts` — pure data + helpers: `MODULE_CATALOG`, `PROFILE_MODULES`, `INSTALL_ORDER`, `PRESETS`, `pickProfileFromModules(variant, modules)`, `extrasFor(profile, selected)`, `detectProfile(modules)`, `isKnownModule(id)`. Mirrors `template-distributor/src/server.ts:108-232`.
  - `seedPipelineState.ts`, `deepLink.ts`, `artifactExtract.ts`, `types.ts` — pure helpers consumed by `create.ts`.

The `template-distributor` external service is retired — all scaffold work for newly-created factory projects happens in-process here under the org's existing GitHub App installation, backed by the workspace PVC declared in `platform/charts/stagecraft/templates/workspace-pvc.yaml`.

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
- `STAGECRAFT_FACTORY_RUN_STALE_AFTER_SEC` — `factory_runs` staleness cutoff for the spec 124 sweeper (default 1800, i.e. 30 minutes). Rows in `(queued, running)` whose `last_event_at` is older than this are flipped to `failed` with a `factory.run.swept` audit by `api/factory/runsScheduler.ts` (cron `factory-runs-staleness-sweeper`, every 1m).
