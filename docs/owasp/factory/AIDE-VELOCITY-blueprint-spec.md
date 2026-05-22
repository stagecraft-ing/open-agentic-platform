# AIDE-VELOCITY — Architectural Blueprint

> **Purpose of this document.** Impartial architectural blueprint of the
> AIDE-VELOCITY project (a sibling system to OAP), produced for the
> Open Agentic Platform team. The goal is *understanding for
> reimplementation* on the OAP/OPC stack — not code extraction. The
> companion document for the developer-side cockpit is
> [`AIDE-VELOCITY-HARNESS-blueprint-spec.md`](./AIDE-VELOCITY-HARNESS-blueprint-spec.md).
>
> **Source artifacts.** `/Users/bart/Dev1/AIDE-VELOCITY` —
> README.md (119 KB primary narrative), `server/`, `client/`,
> `server/openapi.yaml`, 68 migration files, `.github/workflows/`,
> `.claude/`, and ~50 commits of feature history.
>
> **Provenance.** `github.com/GovAlta-EMU/AIDE-VELOCITY` —
> Ministry of Technology and Innovation, Government of Alberta.
> **Note:** the local git remote contains an embedded GitHub PAT
> (`ghp_…`); that token is treated as leaked and is intentionally
> not reproduced in this document. Rotate it.

---

## 1. Elevator pitch

**Velo** ("Velocity") is a full-stack, AI-native project intelligence
and delivery governance platform built for the Government of
Alberta's Ministry of Technology and Innovation. It tracks
approximately **334 technology projects**, **1,269 CMDB
applications**, and **1,042 contracts** across **29 provincial
ministries**, exposing every operation through a fully documented
OpenAPI surface so both human users (via a Vue 3 frontend) and AI
agents (via API-key authentication) can create, update, and audit
project artifacts interchangeably.

Its defining feature is a **chess-clock 8-step state machine** —
the *Velocity Engine* — in which each project module advances
through requirements → planning → architecture → prototyping →
development → user testing → user acceptance → deployment, with
human and AI turns recorded as an immutable ledger, scored for
alignment and speed, and broadcast in real time over SSE. Velo is
deliberately self-referential: it tracks AI-built projects and
provides the exact APIs those AI agents need to report their own
progress back into the system.

---

## 2. Provenance and ownership

- **Repository:** `github.com/GovAlta-EMU/AIDE-VELOCITY` (GovAlta-EMU
  organization — likely the Enterprise Modernization Unit / Ministry
  of Technology and Innovation).
- **Owner declaration:** README.md line 6 — *"Ministry of Technology
  and Innovation, Government of Alberta."*
- **Version at audit time:** v5.1; `server/openapi.yaml` reports v4.1.0
  in the README banner. (A version skew between the README banner and
  the live spec file is the kind of drift the project's own
  `check-docs-sync.mjs` is designed to catch — flag as open question
  #5 in §17.)
- **Deployment target:** Azure App Service. Resource group
  `pronghorn-dev-velocity`, app name `goa-cc-velocity-app-001`
  (see `.github/workflows/deploy-dev.yml`). The commit
  *"fix(rate-limit): strip client port from req.ip on Azure ARR"*
  confirms it sits behind Azure Application Request Routing.
- **Licensing:** Not explicitly stated in inspected artifacts. Likely
  Crown copyright. No `LICENSE` file at repo root.
- **Contributor signals:** Disciplined conventional-commits style; no
  CODEOWNERS / contributor guides observed. Looks single- or
  small-team authored.
- **Sibling systems** (per `docs/AI-PROCESSING.md`): the same AI
  processing framework reportedly powers two other Alberta tools —
  *Committee of Supply* (legislative budget research) and *TRACE Data
  Insights* (contract intelligence). These are sibling applications,
  not components of this repo.

---

## 3. Problem domain and user model

**Domain.** Government of Alberta IT project portfolio oversight.
Hundreds of technology projects are running concurrently across
dozens of ministries; humans cannot track them all in real time. AI
agents need to participate as **first-class actors** alongside
humans — not as passive tools, but as turn-taking collaborators who
read status, do work, upload artifacts, and advance workflow steps.

### Roles (README §5.5)

| Role          | Persona                                                          |
| ------------- | ---------------------------------------------------------------- |
| `user`        | Read-only viewer — ministry stakeholders, executives             |
| `runner`      | **Active participant** — AI agents AND human developers          |
| `project_lead`| Project steward — creates/manages projects, teams, budgets       |
| `admin`       | Platform operator — user management, system settings             |

The notable design decision: `runner` is shared between human
contributors and AI agents. There is no role-level distinction
between an AI session and a human session — attribution comes from
the **auth source** (`session` vs `api_key`) carried in every
audit row.

### Major user flows

- Portfolio dashboard (334+ projects by phase, risk, ministry)
- Velocity board (chess-clock 8-step human-AI turn-taking)
- Deep audit (LLM-driven code review of GitHub repos)
- AI chat (multi-provider streaming — OpenAI, Claude, Gemini, Grok)
- Leaderboard (gamified, complexity-weighted points)
- Challenges (mini-project competitions with cloning-based parallel
  acceptance)
- Admin (user management, pre-registration, SSE session
  introspection)

### What "velocity" actually means here

The term derives from the chess-clock metaphor. The system measures
how fast human-AI pairs can move work through each stage while
penalizing **loopbacks** (repeated rejection cycles) and rewarding
**alignment** (both human and AI sign-offs before completion).
Complexity-weighted scoring means hard modules earn more points.
*Velocity* is both a delivery-speed metric and a gamification signal.

---

## 4. Top-level topology

The system is a **three-tier monolith** deployed to a single Azure
App Service, with PostgreSQL as the only external stateful service.
There is **no microservices split, no message bus, and no Redis
tier**.

```
Browser (Vue 3 SPA)
  └─ Axios HTTP + Socket.io WS
        │
        ▼
Azure ARR (reverse proxy, strips client port)
        │
        ▼
Express.js server (Node 22; single process or multi-instance plan)
  ├── REST API     /api/v1/*    (27 route files, 22 controllers)
  ├── WebSocket    /ai          (Socket.io, AI chat streaming)
  ├── SSE          /velocity/stream     (VelocityStreamManager + cross-instance bus)
  ├── SSE          /notifications/stream
  ├── SSE          /deep-audit/{id}/stream
  └── SSE          /sharepoint/audit/*
        │
        ▼
PostgreSQL (single DB, pg pool, LISTEN/NOTIFY)
  ├── Core (user, auth, audit_log)
  ├── Project domain (project, module, budget, …)
  ├── Velocity Engine (module_velocity, velocity_turn, velocity_idempotency)
  ├── Gamification (user_points, leaderboard MV, cheating_violation)
  ├── SharePoint mapping (sharepoint_folder, ai_processing_job)
  └── LISTEN/NOTIFY channel: velocity_events (cross-instance fan-out)
        │
        ▼
External services
  ├── Google OAuth 2.0           (Passport.js)
  ├── Microsoft OIDC / Azure AD  (Passport + openid-client)
  ├── Microsoft Graph API        (SharePoint Online, @azure/identity)
  ├── GitHub API                 (Octokit, per-user AES-256 PAT)
  ├── OpenAI / Anthropic / Gemini / Grok  (multi-provider AI)

AI agents (external — AIDE-VELOCITY-HARNESS or bespoke scripts)
  └─ REST + SSE via X-API-Key header
```

When the App Service plan runs multiple instances, **cross-instance
SSE fan-out** is handled via PostgreSQL `LISTEN/NOTIFY` on a
`velocity_events` channel (`server/src/sse/cross-instance-bus.ts`).
Each process parks a dedicated `LISTEN` connection and fires
`pg_notify` on every broadcast. A per-boot `originId` UUID on each
instance prevents echo storms — instances skip re-broadcasting
notifications they originated.

---

## 5. Tech stack (per layer)

### Frontend (`client/`)

| Concern              | Technology                                          |
| -------------------- | --------------------------------------------------- |
| Framework            | Vue 3 + TypeScript (Composition API)                |
| Build                | Vite 7                                              |
| UI                   | PrimeVue 4.x (Aura theme) + Tailwind CSS 4.x        |
| State                | Pinia 3                                             |
| Routing              | Vue Router 4 (lazy-loaded, ~21 routes)              |
| Charts               | Chart.js + vue-chartjs                              |
| Dependency graph     | @vue-flow/core                                      |
| Maps                 | Leaflet + @vue-leaflet                              |
| Forms                | FormKit + AJV (dynamic JSON Schema)                 |
| HTTP                 | Axios (refresh interceptor, CSRF injection)         |
| Rich-text edit       | CodeMirror 6 (JSON/YAML/SQL/MD/HTML/CSS/JS)         |
| Markdown render      | marked + js-yaml                                    |
| XSS                  | DOMPurify                                           |
| PWA                  | `beforeinstallprompt` + service worker              |
| Auth storage         | httpOnly cookies (no localStorage tokens)           |

### Backend (`server/`)

| Concern        | Technology                                                |
| -------------- | --------------------------------------------------------- |
| Runtime        | Node.js 22                                                |
| Framework      | Express 4 + TypeScript (`tsx watch` dev, `tsc` prod)      |
| WebSocket      | Socket.io 4                                               |
| SSE            | Native Node streams (no SSE library)                      |
| Validation     | Zod (server-side)                                         |
| Auth           | Passport.js (Google OAuth, Microsoft OIDC) + JWT RS256    |
| Database       | PostgreSQL (`pg` pool, raw parameterized SQL — **no ORM**) |
| Logging        | Winston (structured, correlation IDs)                     |
| Security       | Helmet, express-rate-limit, double-submit CSRF            |
| File upload    | Multer (memory + disk streaming)                          |
| Office docs    | pdf-lib, Mammoth, Sharp, ExcelJS, pizzip + xml2js         |
| ZIP            | yauzl + archiver                                          |
| Encryption     | Node `crypto` (AES-256-GCM for PATs)                      |
| Compression    | `compression` (gzip/brotli — **excluded for SSE**)        |

### Infrastructure

| Concern       | Technology                                                |
| ------------- | --------------------------------------------------------- |
| Deploy        | Azure App Service (Node 22 runtime)                       |
| CI/CD         | GitHub Actions (reusable `deploy-azure-webapp-reusable.yml`) |
| Database      | PostgreSQL (`sslmode=require`; comment references Render — see §17 Q3) |
| Auth SSO      | Azure AD (Microsoft OIDC), Google OAuth                   |
| File storage  | SharePoint Online via Microsoft Graph                     |
| Code hosting  | GitHub (GovAlta-EMU)                                      |

**No Redis, no queue service, no container orchestration, no service
mesh.** The AI processing queue is **PostgreSQL-backed** (the
`ai_processing_job` table), not BullMQ/Redis/etc.

---

## 6. Backend deep-dive

### Layered architecture (NFR-6.1)

`routes → controllers → services → models`. Strictly enforced by
directory; each layer is a separate folder.

### Route inventory

All routes mounted at `/api/v1/`, with `/api/` as a backward-compat
alias.

| Route file                    | Mount             | Representative operations                                                                  |
| ----------------------------- | ----------------- | ------------------------------------------------------------------------------------------ |
| `auth.routes.ts`              | `/auth`           | Google/Microsoft SSO, CSRF token, `GET /me`, refresh, logout                              |
| `project.routes.ts`           | `/projects`       | CRUD + merge + modules + budgets + links + leads + updates + audit + collaboration         |
| `velocity.routes.ts`          | `/velocity`       | Board dashboard, moves, turns, send-back, lock, **stream (SSE)**, guide, CLAUDE.md download |
| `sharepoint.routes.ts`        | `/sharepoint`     | Graph proxy: folders, files, upload, download, move, rename, delete, ZIP, AI queue        |
| `git.routes.ts`               | `/git`            | Repo create, file commit (single + batch), PR create, branch create, extract analytics    |
| `leaderboard.routes.ts`       | `/leaderboard`    | Rankings by period, personal history, project/module contributors, refresh                |
| `challenges.routes.ts`        | `/challenges`     | List, accept, close, pick-winner, legacy claim/complete/unclaim                           |
| `ai.routes.ts`                | `/ai`             | Chat (WebSocket), conversations CRUD, image analysis                                       |
| `admin.routes.ts`             | `/admin`          | Dashboard stats, content CRUD, submission status, **SSE session snapshot**                |
| `user-management.routes.ts`   | `/users`          | Pre-register, role management, enable/disable, lookup                                      |
| `api-key.routes.ts`           | `/api-keys`       | List, create (full key shown once), revoke                                                |
| `settings.routes.ts`          | `/settings`       | GitHub PAT (encrypted), GitHub URL, PAT status                                            |
| `notification.routes.ts`      | `/notifications`  | List, mark-read, SSE stream, broadcast                                                    |
| `health.routes.ts`            | `/health`         | `/live` and `/ready`                                                                       |
| `canvas.routes.ts`            | `/canvas`         | Position persistence, dependency CRUD                                                     |
| `audit.routes.ts`             | (project-scoped)  | Deep audit, export (JSON/MD/DOCX), LLM analyze                                            |

### Middleware chain (15 layers, README §2.4)

`trust proxy → Helmet → compression → static serving → CORS → rate
limit → content-type check → body parsing → cookie parser →
correlation ID → request logging → Passport → route handlers → SPA
fallback → global error handler`

Per-route additions:

- `authenticate` — JWT cookie OR API key
- `authorize(role)` — role membership check
- `csrf` — double-submit HMAC validation (skipped for API-key auth)
- `validate(schema)` — Zod schema validation
- `idempotency` — `Idempotency-Key` header caching (24h)
- `ifMatchVelocityStep` / `ifMatchProject` — `If-Match` revision check
- `velocityWriteGate` / `projectWriteGate` — membership + lock check

### Persistence

**Raw SQL throughout, no ORM.** Models (`server/src/models/`) are
parameterized query files, one per table. Every mutation uses
`$1`/`$2`/… placeholders. Sort columns are whitelisted to prevent
`ORDER BY` injection. Schema is managed via **68 numbered idempotent
SQL migrations** (`001_*.sql` through `068_redemption.sql`).

### Background jobs — "the Reaper"

`server/src/services/reaper.service.ts` is a full-corpus cheat-
detection audit triggered by an admin call. It walks completed
velocity steps looking for gamification abuse. Detection runs as
**single-transaction bulk CTEs** (a refactor from an earlier per-
step JS loop that caused minute-scale latency on hundreds of
completed steps — the commit *"perf(reaper): bulk-CTE refactor —
minutes -> seconds"* captures the change).

Detection rules:

- **`speed_run`** — completed step where all turns were AI-only
- **`no_collaboration`** — step with only one actor type
- **`no_artifact`** — completed step with zero attachments
- **`blank_module`** — minimal turn content + minimal module metadata
- **`project_module_overflow`** — project with 10+ modules
  (soft-deleted included — closing the "farm-then-delete" loophole)
- **`empty_content`** — burst of empty turns (>80% under 50 chars,
  >3 turns)
- **`burst_turns`** — >3 turns within 5 seconds
- **`self_approval`** — approving your own work without a second actor

For each violation: insert a `cheating_violation` row (idempotent
via unique constraint), insert a paired **negative** `user_points`
row sized at `-2 * original` (so net goes from +N to -N), then
refresh the leaderboard materialized view once. An admin
**Redemption** button (migration `068_redemption.sql`) can forgive
negative balances without deleting violation records.

### SSE architecture

Four SSE streams exist (velocity, notifications, deep-audit, sharepoint-
audit). The **velocity stream is the most sophisticated** —
`VelocityStreamManager` in `server/src/sse/velocity-stream.ts`
maintains a `Map<Response, ClientMeta>` of active SSE clients with
**three back-pressure layers**:

1. **Project-scoped subscriptions.** Clients pass `?projects=<id>,
   <id>` on connect; the manager skips uninterested clients at
   broadcast time. Reduces fan-out by 1–2 orders of magnitude for
   agents watching one project.
2. **Per-client write back-pressure with priority shedding.** Every
   write checks the socket-drain return; slow clients accumulate a
   `lagScore`; over threshold, LOW-priority events (`clients`,
   `sharepoint_ai_*`, `pressure`) are dropped while HIGH-priority
   events (`move`, `note`, `send_back`, `lock`) still flow;
   persistent slowness triggers force-close so the client reconnects
   clean.
3. **Memory-pressure response.** Subscribes to a `memoryPressure`
   utility that monitors process RSS; on AMBER+ a `pressure` event
   broadcasts so well-behaved clients can back off; on RED the
   oldest fraction of clients is evicted.

**Cross-instance fan-out** uses PostgreSQL `LISTEN/NOTIFY`
(`server/src/sse/cross-instance-bus.ts`). A dedicated connection is
checked out of the pool on startup and permanently parked on
`LISTEN velocity_events`. Every `publish()` fires
`pg_notify('velocity_events', JSON.stringify({originId, event,
data}))` in addition to local fan-out. The 8000-byte NOTIFY payload
limit is handled by dropping oversized payloads with a warn (local
delivery still works). Reconnect uses capped exponential backoff
(1s–30s). `SSE_DISABLE_CROSS_INSTANCE=true` disables it for local
dev.

The admin endpoint `GET /admin/sse-sessions` returns a **live snapshot
of all connected SSE clients**, grouped by API key / user / IP —
operationally invaluable for identifying runaway agents.

**Per-identity concurrency caps** (commit *"feat(velocity-sse): per-
identity concurrency caps to stop abuse patterns"*) prevent any
single user or API key from holding excessive simultaneous SSE
connections.

---

## 7. Frontend deep-dive

### Route map (21 routes, all lazy-loaded)

| Path                       | View                  | Purpose                                                              |
| -------------------------- | --------------------- | -------------------------------------------------------------------- |
| `/`                        | `DashboardView`       | Live counts, phase doughnut, ministry bar, upcoming go-lives, past-due |
| `/projects`                | `ProjectsCardView`    | Card/table grid, multi-filter, pagination (24/page)                  |
| `/projects/:id`            | `ProjectDetailView`   | Full project CRUD, sub-resources, risk banner, audit log             |
| `/projects/:id/cluster`    | `ClusterView`         | Side-by-side table of all versions in a clone lineage                |
| `/gantt`                   | `GanttView`           | Zoomable Gantt (24–200 px/month), today marker, phase colors         |
| `/canvas`                  | `CanvasView`          | Vue Flow drag-and-drop dependency graph                              |
| `/heatmap`                 | `HeatmapView`         | Ministry × month activity density grid                               |
| `/heatmap/:ministry`       | `HeatmapMinistryView` | Project × month drill-down                                           |
| `/at-risk`                 | `AtRiskView`          | Risk summary cards, 9-box grid                                       |
| `/leads`                   | `LeadsView`           | Lead grouping, people directory with CRUD + merge                    |
| `/velocity`                | `VelocityView`        | **8-step chess-clock board**, SSE live updates, gameplay guide, ApiPlaybook |
| `/duplicates`              | `DuplicatesView`      | Fuzzy-match pairs, directional merge                                 |
| `/applications`            | `ApplicationsView`    | CMDB registry                                                        |
| `/contracts`               | `ContractsView`       | Contract Gantt + table, expiry tracking                              |
| `/leaderboard`             | `LeaderboardView`     | Points board by period                                               |
| `/challenges`              | `ChallengesView`      | Challenge acceptance, close, pick-winner                             |
| `/settings`                | `SettingsView`        | API keys, GitHub PAT/URL                                             |
| `/admin/users`             | `AdminUsersView`      | Pre-register, roles, enable/disable                                  |
| `/login`                   | `LoginView`           | Google/Microsoft SSO                                                 |
| `/auth/callback`           | `AuthCallbackView`    | OAuth redirect                                                       |
| `/:pathMatch(.*)*`         | `NotFoundPage`        | 404                                                                  |

### State model

Pinia, two main stores:

- `auth.ts` — user identity, token state, idle timeout tracking
- `projects.ts` — project list with shared `assessRisk()` function

**No server-state caching library** (no TanStack Query equivalent);
data is fetched per-view with some in-component caching.

### Real-time integration

SSE consumed in `VelocityView.vue` via plain `EventSource`. Events
patch local board state in place (no full refetch). The
`?projects=…` filter is set at connection time. A live client-count
badge in the nav shows `clients` events from SSE.

### Notable UI features

- CodeMirror 6 for rich inline file editing in SharePoint panel
  (full language-pack set: JSON, YAML, SQL, Markdown, HTML, CSS, JS)
- Speech-to-text for project updates (`webkitSpeechRecognition`,
  en-CA, continuous mode) — see §17 Q7 (privacy)
- 5-theme system (light/dark/warm/ocean/forest), persisted, propagated
  to all Chart.js instances via `useTheme().chartColors`
- PWA support with install prompt + service-worker update detection
- Shared `assessRisk()` function across all risk-displaying views
- Axios interceptor queues concurrent requests during token refresh
  (race-free)

---

## 8. AI / agentic surface

### Design philosophy

> *"The application is self-referential by design: it tracks
> AI-built projects and exposes APIs so those AI agents can push
> status updates and velocity moves back into Velo."*  
> — README §1, line 54

### Agent authentication

User-level API keys generated via `POST /api-keys`. Full key shown
once; SHA-256 hash stored. Sent as `X-API-Key: velo_xxx` or
`Authorization: Bearer velo_xxx`. **Role enforcement is identical
to browser sessions** — the API key carries its owning user's
roles. The only differentiator at audit time is the `auth_source`
field (`session` vs `api_key`) plus the `api_key_id`.

### Dynamic AI Playbook (`ApiPlaybook.vue`)

Per commit *"feat(velocity-ui): dynamic AI playbook driven from
openapi.yaml"*: the component fetches `GET /api/v1/docs` at
runtime, parses the raw YAML with `js-yaml`, and renders a
filterable, expandable endpoint browser grouped by OpenAPI tags,
with HTTP-method badges, parameter tables, role requirements, and
response schemas. It is embedded in `VelocityView.vue` so that
agents reading the board can fetch a live endpoint reference that
**cannot drift** from the actual server.

### `CLAUDE.md` as agent runtime document

`server/src/static/CLAUDE.md` is served at `GET /velocity/claude-md`.
It is a prescriptive playbook for AI agents covering:

1. Authentication setup
2. The 7-step typical agent loop: survey → read → understand → work
   → deliver → advance → repeat
3. Cloning and cluster semantics
4. Membership/lock handling
5. The `If-Match` + `Idempotency-Key` "golden recipe" with curl
   examples
6. The full 8-step velocity game rules
7. Scoring mechanics
8. Raise-hand vs blocked semantics
9. Send-back strategy
10. SharePoint artifact management
11. GitHub operations
12. Challenge acceptance patterns

The document **deliberately does not enumerate endpoints** — it
points agents to `GET /api/v1/docs` for that, ensuring the two
sources never conflict.

### Multi-provider AI (`server/src/services/ai-providers/`)

A provider factory selects from four providers based on `AI_PROVIDER`
env var, overridable per request:

- `openai.provider.ts` — default `gpt-4o-mini`, streaming
- `claude.provider.ts` — Anthropic API, chat + vision
- `gemini.provider.ts` — Google AI, chat + vision
- `grok.provider.ts` — xAI

Vision (PDF/DOCX/PPTX/image) uses Claude and Gemini only.

### 5-phase deep audit pipeline (README §9.3)

1. **Discovery** — full GitHub file tree scan
2. **Selection** — AI selects up to `maxFiles` (10–500, default 200)
3. **Loading** — fetch selected files within `maxContentKB`
   (100–2000 KB, default 500)
4. **Analysis** — batched LLM analysis (quality / tests / security /
   completeness)
5. **Consolidation** — merge batch results into final report

PAT injected from user settings; falls back to env `GITHUB_PAT`.
Returns 202; progress streamed via SSE.

### AI processing pipeline (§12)

A PostgreSQL-backed queue that processes SharePoint files into
Markdown shadow files prefixed `__AI__`. Files changed since last
processing are detected via cTag comparison (unchanged files
skipped). Supports PDF (per-page via pdf-lib → vision OCR), DOCX
(Mammoth + image fallback), PPTX (text + per-slide images), XLSX/CSV
(ExcelJS table → Markdown), and raw images. Sub-jobs track each
vision API call with retry (max 3, exponential backoff). A unique
partial index prevents double-enqueuing.

### Agent concurrency design (§15)

Two cooperative HTTP-header mechanisms make multi-agent writes safe:

- **`If-Match: <revision>`** — optimistic concurrency via DB-trigger-
  maintained integer counters on `project` and `module_velocity`.
  Mismatch returns `412 PRECONDITION_FAILED` with `currentRevision`
  in the error detail; agents can refetch and decide whether intent
  still applies.
- **`Idempotency-Key: <uuid>`** — 24-hour cached replay of 2xx
  responses. Same key + same body → replayed; same key + different
  body → `422 IDEMPOTENCY_KEY_REUSED`. Makes network retries safe
  without duplicate side effects.

---

## 9. Data and persistence

### Schema layers

**68 SQL migrations, numbered and idempotent.** Three layers:

- **Platform foundation (001–018)** — authentication, notifications,
  forms, service catalog, AI chat (the "portal scaffold" Velo
  extends)
- **Project domain (019–038)** — projects, modules, budgets, people,
  CMDB applications, contracts
- **Feature layers (039–068)** — velocity engine, deep audit,
  SharePoint mapping, roles, gamification, collaboration/concurrency,
  challenge acceptance, cheat detection, redemption

### Design decisions

- No ORM. All SQL raw + parameterized.
- `is_deleted` soft-delete pattern; partial unique indexes excluding
  deleted rows (migration 055).
- JSONB for `audit_old_data`/`audit_new_data` and embedded parent
  IDs (orphan resilience).
- DB triggers for `set_updated_at()`, `bump_project_revision`,
  `bump_step_revision` (migration 062) — **service code never
  manually bumps revision counters**.
- `velocity_idempotency` with 24-hour TTL and hourly cleanup
  (`cleanupExpiredIdempotency()` in `server.ts`).
- `leaderboard` as a PostgreSQL materialized view refreshed via
  `refresh_leaderboard()` SQL function.
- `cheating_violation` unique constraint on `(fk_cv_user,
  fk_cv_step_id, violation_type)` — Reaper runs are idempotent.
- Migrations 064–068 are all gamification-integrity hardening —
  iterative anti-abuse evolution.
- Migrations 060–062 (collaboration/concurrency) add 10+ columns to
  `project`, the `project_member` junction table, revision counters,
  and DB-trigger enforcement — a significant schema evolution post-
  initial gamification work.

### Performance strategy

11 targeted indexes in migration 033 (partial on `is_deleted=false`,
composite, JSONB, pg_trgm for typeahead, descending for pagination).
The `project` list query avoids `GROUP BY` in favour of correlated
subqueries to preserve index usage (NFR-1.3). DB pool default
recently raised from 20 → 50 (commit *"chore(db): raise DB_POOL_MAX
default 20 -> 50"*) after SSE thundering-herd exhausted the pool
(commit *"fix(velocity): stop SSE thundering-herd that exhausted DB
pool"*).

---

## 10. Security and governance posture

### Authentication

RS256 JWTs (asymmetric, separate access and refresh keypairs via
`generate-keys.ts`). Access token: 15-minute expiry in httpOnly
cookie. Refresh token: 7-day expiry, SHA-256 hash stored in
`refresh_token` table with rotation on every refresh. Cross-provider
account linking by email.

### CSRF

Stateless double-submit cookie with HMAC verification. Client fetches
token from `GET /auth/csrf`; sends in `X-CSRF-Token`; server validates
via timing-safe comparison. Bypassed for API-key auth. Applied to
POST/PUT/PATCH/DELETE.

### Rate limiting (three tiers — in-memory per process)

- API: 200 req / 15 min
- Auth: 30 req / 15 min
- AI: 60 messages / hour

> **Scaling limitation acknowledged in NFR-5.4:** rate limits are
> per-process, so multi-instance App Service plans bypass per-user
> intent. Redis-backed limiting is noted as future work.

The commit *"fix(rate-limit): strip client port from req.ip on
Azure ARR"* documents an interesting Azure ARR gotcha: ARR appends
a port suffix to `req.ip` that was causing rate limits to count each
unique port as a distinct client.

### Secrets handling

- GitHub PATs encrypted at rest with **AES-256-GCM**
  (`user_github_pat_encrypted` + `user_github_pat_iv`), decrypted
  only at API-call injection time
- API keys SHA-256 hashed
- Refresh tokens SHA-256 hashed
- `CSRF_SECRET` minimum 32 chars enforced at startup
- All `.env` files gitignored

### Input validation

Zod everywhere server-side. AJV for dynamic JSON-Schema form
submissions. Parameterized SQL throughout. Sort columns whitelisted.
URL inputs reject non-`http(s):` protocols. Date max `9999-12-31`
enforced client + server. DOMPurify on the frontend.

### Security ESLint scan (⚠️ broken)

`eslint.security-scan.config.mjs` loads `eslint-plugin-security` from
a **hardcoded absolute Windows path** (
`file://C:/dev/pronghorn-red/hyper-factory-zone/velo/security/blueteam/
node_modules/eslint-plugin-security/index.js`). The config exists
in intent but cannot run on any machine other than the original
developer's. This is an operational gap, not a code-quality one —
the rule set itself is comprehensive: `detect-eval-with-expression`,
`detect-non-literal-fs-filename`, `detect-non-literal-regexp`,
`detect-non-literal-require`, `detect-object-injection`,
`detect-possible-timing-attacks`, `detect-unsafe-regex`,
`detect-buffer-noassert`, `detect-child-process`,
`detect-disable-mustache-escape`, `detect-no-csrf-before-method-
override`, `detect-pseudoRandomBytes`.

### Anti-cheat / audit trail

`audit_log` is **immutable** (no DELETE endpoint). Every mutation
across all entities is logged with JSONB diff (old + new), auth-source
attribution, and API-key ID. The Reaper provides active anti-cheat
for the gamification system. Auth events (login, logout, role
changes) are audited.

### IDOR prevention

All sub-resource queries verify parent ownership
(`audit.sub_resource_delete_queries`, NFR-2.10).

### Collaboration security model

The **admin role does NOT bypass** the velocity write gate — admins
must join projects as members to make velocity moves. Deliberate per
the `velocity-write-gate.ts` comment ("admins are platform operators,
not players"). The bypass was explicitly reverted: commit *"fix(velocity):
admin role no longer bypasses velocity membership gate"*.

---

## 11. Operational shape

### Boot sequence (`server/src/server.ts`)

1. Validate environment with Zod (startup fails loudly on
   misconfiguration)
2. Test DB connection
3. Start cross-instance bus
4. Attach Socket.io to HTTP server
5. Start AI processing queue drain interval (5s)
6. Start idempotency cleanup interval (hourly)
7. Listen on PORT (default 3001 in dev)

### Graceful shutdown

Configurable `SHUTDOWN_TIMEOUT_MS` (default 30s). Order: WebSocket →
HTTP server → DB pool. Handlers for `unhandledRejection` and
`uncaughtException`.

### Static serving

When `SERVE_CLIENT=true`, Express serves the bundled Vue SPA from
`client/dist`. Enables single-dyno deployment. SPA fallback: non-API
GET routes serve `index.html`.

### CI/CD

GitHub Actions with a reusable workflow
(`deploy-azure-webapp-reusable.yml`) parameterized by app name,
resource group, environment, Node version, frontend/backend
directories, build command, and dist dir. Triggers on push (excluding
`*.md`, `.gitignore`, `docs/**`). Uses `AZURE_CREDENTIALS` secret. A
separate `check-docs-sync.yml` workflow enforces documentation
parity.

### Docs-sync CI

`scripts/check-docs-sync.mjs` is a Node script run in CI that
mechanically detects drift between:

- Express routes ↔ OpenAPI spec
- OpenAPI spec ↔ README citations
- SSE broadcast calls ↔ documented event lists
- README migration counts ↔ actual migration files
- Custom error codes ↔ documentation

Exits non-zero on drift. A Claude Code `.claude/settings.json` hook
(`PostToolUse` on Edit/Write, plus `Stop`) also runs a lightweight
version of this check after every agent file modification — drift
prevention at agent-decision time, not just PR time.

### Observability

- Winston structured logging with correlation IDs (UUID per request)
- Request log with duration; `>1s` logged at WARN
- `GET /health/live` and `GET /health/ready` (DB readiness)
- SSE heartbeat every 30s
- `GET /admin/sse-sessions` — live snapshot of connected SSE clients

### Scaling story

Single Node process + PostgreSQL. The cross-instance SSE bus solves
multi-instance fan-out. Rate limiting is explicitly per-process.
The AI processing queue is PostgreSQL-backed (survives restarts) but
the buffer map for sub-job binary data is in-memory (lost on restart;
triggers re-split — note potential vision-API cost spike under
load).

---

## 12. Extension points

### `.claude/` AI workflow integration

- **`.claude/settings.json`** — PostToolUse hook on Edit/Write runs
  `check-docs-sync.mjs --migrations-only` for fast feedback. Stop
  hook runs the full docs-sync check before Claude marks the
  conversation complete.
- **`.claude/skills/sync-docs/SKILL.md`** — a structured skill that
  drives `check-docs-sync.mjs` step-by-step: run check, read sources,
  build patches, show diffs, apply, verify zero drift.

### Public agent-facing endpoints

- **`GET /velocity/claude-md`** — serves the AI-agent operations
  manual. External agents fetch it at startup.
- **`GET /api/v1/docs`** — serves `server/openapi.yaml` as the
  always-current API spec. Both the in-app `ApiPlaybook.vue` and
  external agents consume it.

### Provider plug-in

`server/src/services/ai-providers/provider-factory.ts` is pluggable
via `AI_PROVIDER` env var; new providers implement
`ai-provider.interface.ts`.

### Velocity gameplay guide

`GET /velocity/guide` — downloadable Markdown with the full game
mechanics. For human onboarding *and* AI bootstrap.

### API-key surface

`/api-keys` + `X-API-Key` header is the primary integration surface
for the harness, custom scripts, CI pipelines.

### The `.velocity` file ⚠️

A 45-byte file at repo root containing a raw API key value
(prefix `velo_`). Presumably the default/development key used by the
harness or local automation. It is **not gitignored** in the
inspected working tree. If the key is valid against any live
deployment, this is a secret-leakage exposure surface. Treat as a
finding to remediate (move to `.env.example` template + gitignore;
rotate key; do not commit value).

---

## 13. Contracts and API surface

### OpenAPI

`server/openapi.yaml` is the named source of truth, served at
`GET /api/v1/docs`. `check-docs-sync.mjs` enforces bidirectional
parity between the spec and the actual route files.

### Response envelope (consistent across all endpoints)

- Success: `{ success: true, data: {...} }`
- Paginated: `{ success: true, data: [...], pagination: { page, limit, total, totalPages } }`
- Error: `{ success: false, error: { code, message, details, url, method, timestamp } }`

### Field convention

**Input camelCase, output snake_case** (DB column names). Controllers
bridge via `mapBodyToDb()` and empty-string-to-null coercion. This
is unusual and leaks DB column names to the API surface (see §15).

### Consumers

AI agents (the harness, custom scripts), the Vue SPA, and any
external system with an API key. **All three use identical
endpoints with identical behaviour** — no separate "public" vs
"internal" API split.

### Backward compatibility

`/api/` alias for `/api/v1/` (NFR-8.4).

### Static fallback

`client/src/data/projects.json` is a hardcoded fallback for when the
API is unreachable — offline-first resilience was a consideration.

---

## 14. Relationship to AIDE-VELOCITY-HARNESS

The harness is referenced **inside this repo only once** in code:
`server/src/middleware/velocity-write-gate.ts` line 11.

> *"With the harness running many `velo-listener.js` processes that
> fan in via different API keys, listeners were stepping on each
> other's projects."*

The key phrase is *"many `velo-listener.js` processes that fan in
via different API keys"* — the harness is a **multi-process
orchestrator** that spawns multiple listener agents, each with its
own API key, all connecting to the same Velo instance. This maps
directly to the OAP↔OPC analogy: the harness is the cockpit /
factory layer that **drives** the work; Velo is the platform that
**governs** the state.

Evidence inside Velo of harness-shaped pressure:

- **Three-layer SSE back-pressure**, **per-identity concurrency
  caps**, **project-scoped subscriptions** — all responses to
  harness-scale connection patterns
- **`If-Match` + `Idempotency-Key`** — agent-safety primitives
- **`velocityWriteGate`** — explicit gating after listeners
  collided
- **`.velocity` file** at repo root — likely the harness's default
  API key value
- **DB-pool default 20 → 50** — after thundering-herd SSE reconnects
  exhausted the pool

So the relationship is: **AIDE-VELOCITY-HARNESS is a fleet of API-
key-authenticated Node processes** that watch the Velo SSE stream,
pick up velocity steps assigned to AI, do the actual work (using AI
providers), upload artifacts to SharePoint/GitHub, and make
velocity moves. Velo provides the state machine, the queue
discipline, the anti-cheat enforcement, and the audit trail for that
agent fleet.

---

## 15. Implementation-quality observations

### Genuinely well-engineered

1. **Cross-instance SSE bus** — a clean, production-grade solution
   to a hard distributed-systems problem (fan-out across stateless
   App Service instances **without Redis**). PostgreSQL
   `LISTEN/NOTIFY` as a message bus is clever: zero additional
   infrastructure, careful connection handling (parked dedicated
   client, backoff reconnect, origin-ID dedup).
2. **Three-layer SSE back-pressure** — unusually sophisticated for
   an Express app. Project-scoped subscriptions + priority shedding
   + memory-pressure eviction are production patterns. Source
   comments explain *why*, not just *what*.
3. **Reaper bulk-CTE refactor** — *minutes → seconds*. Good
   operational responsiveness, recorded in the commit log.
4. **`If-Match` + `Idempotency-Key`** — well-designed agent
   concurrency primitives. Advisory-but-safe: clients without
   headers still work; safe clients get deterministic semantics.
   The CLAUDE.md golden recipe with real curl examples *teaches*.
5. **`check-docs-sync.mjs` + Claude hook integration** — a novel
   drift-prevention mechanism. Running mechanical doc-sync checks as
   PostToolUse hooks means the AI agent that adds a feature is
   reminded to update docs before the conversation ends. The same
   "spec-is-truth" philosophy OAP has, implemented at *agent-
   instruction* level rather than *build-system* level.
6. **Collaboration schema (migrations 060–062)** — last-owner
   protection, single-level clone restriction with `CLONE_OF_CLONE`
   error, DB-trigger-enforced revision counters (service code
   *cannot accidentally forget* to bump).

### Awkward or debt-bearing

1. **Security ESLint scan broken by Windows-absolute path** — exists
   in intent, not in practice.
2. **`.velocity` file with raw API key checked into the working
   tree** — standard secret-leakage pattern. Should be gitignored
   + templated.
3. **Rate limiting in-memory per process** — multi-instance plans
   bypass intended limits. Redis is "future work."
4. **No ORM + raw SQL** — performance transparency, zero abstraction
   leakage; cost: 68 migrations + dozens of model files maintained
   by hand, drift caught only by `check-docs-sync.mjs` as the safety
   net.
5. **AI processing buffer map in-memory** — restart triggers re-split;
   under load, possible vision-API cost spike.
6. **`camelCase in, snake_case out`** — leaks DB column names to the
   API surface and forces consumers to track two conventions.
7. **DB pool sizing has been an incident** — 20 → 50 after SSE
   thundering-herd. Pool exhaustion remains a known risk if the
   harness scales further.

---

## 16. Concepts worth porting to OAP/OPC

Capabilities as *concepts*, not code transplants:

1. **Chess-clock state machine with actor tracking** — `current_actor
   ∈ {ai, human}`, turn history with alignment flags, loopback
   counting. OAP's factory pipeline could benefit from per-step
   actor attribution and alignment scoring.
2. **PostgreSQL LISTEN/NOTIFY as cross-instance bus** — zero-
   infrastructure SSE fan-out across Encore service instances for
   events under the 8 KB payload cap. Reuse pattern: dedicated
   connection, origin-ID dedup, exponential backoff.
3. **Three-layer SSE back-pressure** — project-scoped subscriptions
   + priority shedding + memory-pressure eviction. Self-healing for
   any long-lived SSE surface.
4. **API-key-as-agent-identity with identical role enforcement** —
   no privilege escalation/reduction; endpoints behave identically
   for browser and agent; attribution differentiates them.
5. **Optimistic concurrency + idempotency as agent-safety primitives**
   — `If-Match` + `Idempotency-Key` together provide full safety for
   agent-scale racing. DB-trigger-maintained revision counters + 24h
   TTL idempotency table.
6. **Gamification with anti-cheat governance** — cheat rules as
   CTE-based SQL (not application logic), idempotent violation
   tables, paired negative point entries, Redemption capability that
   forgives without destroying the audit trail.
7. **CLAUDE.md as a served runtime document** — well-known endpoint
   (`GET /velocity/claude-md`) cleanly separating *workflow
   recipes* from *endpoint mechanics* (OpenAPI spec). OAP's spec
   spine could generate equivalent agent instruction files during
   the build pipeline.
8. **OpenAPI-driven in-app playbook** — `ApiPlaybook.vue` fetching
   the live spec and rendering a filterable, expandable endpoint
   browser embedded in the app itself. OPC could embed equivalent
   spec-driven tooling.
9. **`check-docs-sync.mjs` as mechanical drift gate** — Node script
   enforcing parity across routes, OpenAPI spec, README citations,
   SSE events, and error codes — wired both into CI *and* Claude
   Code hooks. A lightweight alternative to OAP's spec-compiler for
   projects without a full spec spine.
10. **Challenge system via cloning-based parallel acceptance** — each
    participant gets an independent velocity board, not a shared
    one. Maps directly to OAP's factory concept of parallel agent
    attempts on a spec.
11. **Admin SSE session introspection** — live snapshot of all
    connected SSE clients grouped by API key/user/IP. OPC's cockpit
    could expose equivalent information.

---

## 17. Open questions / gaps

1. **What does the harness look like, precisely?** This repo names
   it only once (in code) and once via the `.velocity` file. See
   the companion blueprint document.
2. **Is the `.velocity` key live?** A `velo_…` value is present in
   `.velocity` at repo root and the file is dated current. If it
   authenticates against any live deployment, it needs immediate
   rotation.
3. **Where is the PostgreSQL database hosted?** Code comments
   reference Render (`sslmode=require`), but the deployment target
   is Azure App Service. Either Azure Database for PostgreSQL
   Flexible Server is used (likely) or the topology is cross-cloud.
4. **What triggers the Reaper?** Admin-triggered endpoint; commit
   log suggests routine use — but no cron / webhook / scheduled job
   is visible in this repo.
5. **OpenAPI version skew.** README banner says `v4.1.0`; spec file
   line 122 says `v3.0.0`. `check-docs-sync.mjs` has a "version
   banner" check that should be catching this.
6. **What is the sibling-systems relationship to "Committee of
   Supply" and "TRACE Data Insights"?** Shared AI-processing
   framework — shared npm packages? Shared backend? Or just
   conceptually similar?
7. **Speech-to-text privacy.** `webkitSpeechRecognition` is Chrome/
   Edge-only and sends audio to Google. For government IT-project
   data, may warrant review.
8. **`ApiDocsPage.vue` exists in views but isn't in the router** —
   dead code, admin-only, or conditionally added?

---

## 18. Synthesis for OAP/OPC reimplementation

If OAP were to absorb the strongest concepts here, the high-leverage
moves are (in rough order of value):

1. Adopt **the chess-clock 8-step state machine with actor-tagged
   turn history** as a factory pipeline primitive in OAP.
2. Add **`If-Match` + `Idempotency-Key`** to every multi-agent
   write surface in stagecraft.
3. Adopt **PostgreSQL `LISTEN/NOTIFY` as cross-instance SSE bus**
   in any Encore service that needs real-time fan-out.
4. Build **`/api/v1/docs`-style live OpenAPI** plus an embedded
   playbook viewer into OPC.
5. Implement the **"CLAUDE.md served from the platform"** pattern —
   OPC and external agents fetch their behavioural manual from
   stagecraft rather than hard-coding it.
6. Mirror **`check-docs-sync.mjs` + PostToolUse Claude hook**
   integration as an OAP rule — drift-prevention at *agent-decision*
   time.
7. Replicate the **three-layer SSE back-pressure** pattern (priority
   shedding + memory-pressure eviction) wherever long-lived
   real-time streams exist in OAP.
8. Adopt **`audit_log` with `auth_source` attribution** — humans and
   agents share endpoints; only attribution differs.

What the OAP/OPC stack can do **better** than Velo:

- **Spec spine** provides authored truth that Velo approximates with
  OpenAPI + README + CLAUDE.md drift scripts. OAP's compiler can be
  the source for all three.
- **Encore.ts service boundaries** would let OAP separate concerns
  (auth, gamification, ai-processing, SSE) into independently scaled
  services rather than a single Express monolith.
- **Tauri-based OPC cockpit** can render the playbook + dashboards
  with file-system access and persistent state that the Velo Vue
  SPA cannot.
- **Rust crates for orchestrator, policy-kernel, tool-registry**
  give type-safe, testable foundations that the Velo `service →
  model` JS layers approximate informally.
