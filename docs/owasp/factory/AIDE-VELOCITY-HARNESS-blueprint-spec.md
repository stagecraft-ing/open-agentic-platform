# AIDE-VELOCITY-HARNESS — Architectural Blueprint

> **Purpose of this document.** Impartial architectural blueprint of
> the AIDE-VELOCITY-HARNESS project — the developer-side cockpit
> that pairs with AIDE-VELOCITY in a relationship analogous to
> OPC↔OAP. Produced for the Open Agentic Platform team to inform
> reimplementation of equivalent capabilities on OAP/OPC. The
> companion document for the platform side is
> [`AIDE-VELOCITY-blueprint-spec.md`](./AIDE-VELOCITY-blueprint-spec.md).
>
> **Source artifacts.** `/Users/bart/Dev1/AIDE-VELOCITY-HARNESS` —
> `CLAUDE.md` (17 KB skill/guide index), `README.md` (8.8 KB),
> `harness.html` (82 KB single-file visual manual), `velo-listener.js`
> (32 KB Node daemon, 695 lines), `template/`, `velocity/`,
> `.claude/` (skills, scripts, hooks, references, standards),
> `.claude-plugin/`, ~25 commits of feature history.
>
> **Provenance.** `github.com/GovAlta-EMU/AIDE-VELOCITY-HARNESS` —
> Government of Alberta, Ministry of Technology and Innovation.
> License: **CC0 1.0 Universal** (public domain dedication).
> **Note:** the local git remote contains an embedded GitHub PAT
> (`ghp_…`); treated as leaked, not reproduced here. Rotate it.

---

## 1. Elevator pitch

**AIDE-VELOCITY-HARNESS** is a Claude Code workspace harness that
embeds two complementary systems:

1. **Velocity engine integration** — an 8-step AI-human project
   management game board backed by a SaaS platform (AIDE-VELOCITY)
   with SharePoint artifact storage, GitHub integration, and
   SSE-based event streaming.
2. **Velocity build framework** — opinionated project templates,
   sequential build guides (`v1-requirements` through `v8-deployment`),
   cross-cutting standards, and a **dual-track security pipeline**
   (blueteam + redteam).

The harness is instantiated by cloning the repository and opening
Claude Code in the working directory; every skill, hook, gate, and
template auto-discovers. Its primary users are enterprise software
developers at Alberta government ministries working on public-facing
Vue + Node/Go applications, operating inside a managed Velocity
board where AI agents are assigned steps by human project leads and
must produce structured, gate-verified deliverables before
advancing the board.

The problem it solves is the **fundamental AI reliability gap**:
Claude Code, left uninstructed, will hallucinate completeness,
silently defer must-have features, and produce documentation that
diverges from code. The harness replaces prose obligations with
**mechanical gate scripts** that block the conversation from closing
unless every mandatory check passes.

---

## 2. Provenance and ownership

- **Repository:** `github.com/GovAlta-EMU/AIDE-VELOCITY-HARNESS`
- **License:** CC0 1.0 Universal — full public domain dedication, no
  restrictions
- **Contributors visible in git log:** primary harness author plus
  PR merges from `aide-vh-chris-wright_goa` and
  `Janak-Alford_goa-patch-1`
- **Cadence:** steady — ~25 commits covering security codification,
  template improvements, gate hardening, SSE stability fixes
- **Production Velocity endpoint** (from `.env.example` line 1):
  `https://goa-cc-velocity-app-001.azurewebsites.net/api/v1` —
  consistent with the Alberta Azure deployment of AIDE-VELOCITY
- **Build credentials referenced**: `VELO_BASE_URL`, `VELO_API_KEY`,
  `VELO_PROJECT_ID` (the `.env.example` is the configuration
  surface)

---

## 3. Problem domain and user model

The target user is an **enterprise application developer inside a
government bureaucracy** subject to formal SDLC governance
(SharePoint project artifacts, sign-off gates, audit trails,
multi-stakeholder UAT) while using Claude Code as their primary
development agent.

### The optimization loop

1. A human project lead (PM, product owner, senior dev) moves a
   board step to an `ai_*` status — **the trigger**.
2. Claude Code (autonomously via `velo-listener.js`, or manually via
   a slash command) picks up that board signal, reads prior turn
   history, pulls SharePoint inputs, and executes the matching
   `v<N>` skill.
3. The skill produces a structured markdown deliverable with a
   **mandatory 7-section skeleton**.
4. Gate scripts verify the deliverable against the actual codebase
   before a board advance is allowed.
5. The step is uploaded to SharePoint, the board is advanced with
   `If-Match` + `Idempotency-Key` headers (concurrency-safe), and
   the human reviews.
6. If accepted → advance to next step; if not → human triggers a
   send-back.

### Audiences (`harnessData.meta.audience`, `harness.html` ~line 68)

Three explicit audiences:

- **Layperson** — skim the flowchart
- **AI expert** — evaluate rigor
- **Claude itself** — complete inventory + rules for extending

A secondary user persona is the **solo developer** building non-
Velocity GoA apps who wants the template + build guides + security
framework without the board integration.

---

## 4. Top-level topology

Five moving parts:

```
                ┌──────────────────────────────────────────────┐
                │  AIDE-VELOCITY (SaaS, Azure App Service)     │
                │  goa-cc-velocity-app-001.azurewebsites.net   │
                │                                              │
                │   REST /api/v1/*        (X-API-Key auth)     │
                │   SSE  /velocity/stream                      │
                │   GET  /api/v1/docs     (live OpenAPI 3.0)   │
                │   GET  /velocity/claude-md (agent manual)    │
                │   Injects SharePoint + GitHub creds          │
                └─────────────────┬────────────────────────────┘
                                  │
                                  │ SSE + REST
                                  ▼
                ┌──────────────────────────────────────────────┐
                │  velo-listener.js  (long-running Node daemon) │
                │  - SSE subscription with ?projects= filter   │
                │  - Spawns `claude --session-id <uuid>`       │
                │    per project on activation events          │
                └─────────────────┬────────────────────────────┘
                                  │ spawn child process
                                  ▼
                ┌──────────────────────────────────────────────┐
                │  Claude Code (CLI)                           │
                │  Working dir: harness root                   │
                │  Auto-loads: CLAUDE.md → all skills,         │
                │              standards, references, hooks   │
                │                                              │
                │  PostToolUse: drift-check (cheap)            │
                │  Stop chain: 6 mandatory gates (blocking)    │
                └─────────────────┬────────────────────────────┘
                                  │
                                  ▼
                ┌──────────────────────────────────────────────┐
                │  template/   →  copied to ./app/ by /build   │
                │  velocity/   →  per-step inputs + outputs    │
                │  harness.html → human + AI visual manual     │
                └──────────────────────────────────────────────┘
```

**Narrative flow.** Human moves board → SSE event → listener →
`claude` subprocess → CLAUDE.md auto-loads → matching `v<N>`
`SKILL.md` → SharePoint + GitHub calls via Velo → structured output
written to `velocity/<step>/output/` → Stop-hook gates fire → board
advance if all gates pass → human reviews.

---

## 5. Tech stack (per layer)

### Harness scripts / listener

- **Pure Node.js (≥18), CommonJS** for `velo-listener.js`
- **Zero npm dependencies** for harness-level scripts (explicit in
  CLAUDE.md line 65)
- Built-ins only: `child_process.spawn`, `http`/`https`, `fs`,
  `crypto`, `path`

### Security framework

- **Blueteam (defensive):** Node + external npm — `secretlint`,
  `@typescript-eslint/parser`, ESLint security plugins, `npm audit`.
  Lives under `.claude/security/blueteam/`.
- **Redteam (offensive):** Node + `@anthropic-ai/claude-agent-sdk`
  (orchestrator in `pipeline/claude_sdk.js`). Optional recon tools
  (nmap, semgrep, trufflehog, osv-scanner, OWASP ZAP) with
  Node-native fallbacks (OSV.dev API, built-in regex patterns) so
  `apt`/`brew` installs are not strictly required.

### Templates

- **Frontend:** Vue 3 + Vite + TypeScript
- **Backend:** Express + TypeScript + PostgreSQL
- **GoA overlay:** `@abgov/web-components` (Government of Alberta
  Design System) as an additive layer

### `harness.html`

- **Vanilla browser-side Vue 3** (loaded from `unpkg.com/vue@3` CDN)
- Tailwind CSS (Play CDN)
- **No build step**; all data embedded in the `harnessData` JS object
- SVG flowcharts drawn inline

### Velocity backend contract

- REST/JSON over HTTPS
- SSE for real-time
- `X-API-Key` auth
- `If-Match` + `Idempotency-Key` headers for concurrency

### CI

- **Dependabot only** (`.github/dependabot.yml`); **no GitHub
  Actions workflows yet** (the dependabot config anticipates them
  with a scoped entry).
- No automated test runner at the harness repo level; tests live
  per-template and per-security-framework.

### Storage

- Files on disk: `velocity/`, `.claude/state/`
- Session state: `.claude/state/listener-sessions.json` (per-project
  stable UUIDs)
- Gate audit trail: `.claude/state/gate-state.jsonl` (append-only,
  gitignored)
- **No database at the harness level** — all persistent project
  state lives in the Velocity backend and SharePoint.

---

## 6. The HARNESS itself (`harness.html`)

An **82 KB single-file static web application** — the "visual
instruction manual" (`README.md` line 10). It serves **three roles
simultaneously**:

1. **Human documentation.** Flowcharts, skill cards, decision trees,
   eval matrices, workspace file index, extension instructions —
   all rendered interactively in browser.
2. **AI-readable inventory.** The `harnessData` JavaScript object
   (lines 63–317) is structured JSON embedded in the page. The
   comment at line 54: *"An AI agent reading this HTML can extract
   the harness inventory directly from the harnessData object."*
   This is a deliberate design choice — the file is **both human UI
   and machine-parseable source of truth**.
3. **Markdown export.** A button (line 398) triggers
   `exportMarkdown()` (line 1030) which serializes `harnessData` to
   a GitHub-flavored markdown document and triggers a browser
   download.

The page has **no server, no fetch calls, no external API calls at
runtime** (only CDN loads for Vue and Tailwind). All content is
static in `harnessData`. Vue 3 renders: audience, rules, the 8-step
SVG flowchart, evals matrix (SVG), skills cards with phase
filtering, standards table with search, guides, references,
templates, workspace file index, extend instructions.

**Section filters** (added commit `315a241`) allow filtering skills
by phase (Discovery / Build / Verify / Ship / General) and filtering
evals by step.

### Key invariant

`harness.html` and `CLAUDE.md` are required to stay in sync. The
Stop-hook `check-harness-sync.mjs` enforces that every script under
`.claude/scripts/` is indexed in `CLAUDE.md`; the extension
instructions (CLAUDE.md line 198–204) require updating
`harness.html`'s `harnessData` arrays when adding new skills.
`check-harness-consistency.mjs` catches divergence.

### Weakness

At 82 KB, `harness.html` is **not auto-generated** — it is hand-
maintained in parallel with `CLAUDE.md`. The gates enforce that
they agree, but the maintenance burden of keeping two documents
synchronized (one markdown, one JSON-embedded-in-HTML) is non-trivial
and is acknowledged in the extension instructions.

---

## 7. The LISTENER (`velo-listener.js`)

A **695-line single-file Node.js daemon**. Carefully designed.

### What it listens to

SSE stream at `$VELO_BASE_URL/../api/v1/velocity/stream`. The server
supports `?projects=<id>,<id>` query filtering, used by the
listener to minimize bytes received when scoped to known watched
projects (lines 129–132).

### Event taxonomy (~16 types, 2 categories)

- **Activation-worthy** — `move` to `ai_*` status, `send_back`,
  `note` targeting AI → spawn `claude` subprocess.
- **Inform-only** — `version_created`, `challenge_accepted`,
  `challenge_closed`, `lock_acquired`, `lock_released`,
  `module_created`, `sharepoint_ai_shadow_created`,
  `ownership_transferred`, `project_*`, `module_*` → logged and
  cached but no spawn.

### Project scoping ("self-quench") — commit `bdca6ec`

Without `VELO_PROJECT_ID` set and `VELO_LISTEN_ALL=true` absent,
**the listener refuses to start** (lines 110–120). Prevents agent
collision on shared platforms — multiple developers running
unscoped listeners would race to claim the same `ai_*` steps. The
error message is explicit.

### Session model

Each project gets a **stable UUID** stored in
`.claude/state/listener-sessions.json`. Every Claude activation for
a project resumes the same session via `claude --session-id <uuid>`.
**Intentional**: context accumulated during one board event is
available during the next, giving the agent persistent memory of
what it built for that project.

### Concurrency model

- **One Claude subprocess per project at a time.**
- Global cap `MAX_CONCURRENT_CLAUDES` (default 3) — prevents OOM when
  many watched projects activate simultaneously.
- Events arriving while a project's Claude is running are queued in
  `pendingByProject` and drained on subprocess exit (lines 590–598).

### Reconnect / resilience

- Exponential backoff (min 30 s, max 300 s per attempt, line 335)
- Respects `Retry-After` headers from the server (handles 429/503)
- **90-second heartbeat watchdog** (lines 654–662) forces reconnect
  if no SSE traffic — defends against half-open TCP connections that
  the Node http client doesn't detect

### Self-quench on subscription changes

When `WATCH_CLUSTERS=true` or `WATCH_CHALLENGES=true` causes new
projects to be auto-watched, a 3-second debounced timer reconnects
the SSE stream with the updated `?projects=` filter — avoiding
reconnect storms on each individual auto-watch event (lines 358–
368).

### Dry-run mode

`LISTENER_DRY_RUN=true` (in `.env.example` line 8) logs what Claude
would be invoked with but does not spawn — safe for testing event
routing.

### Startup poll

On startup, the listener calls `GET /velocity` to find pre-existing
`ai_*` assignments before subscribing to the stream, ensuring no
work is missed during the window between harness launch and SSE
connect (lines 622–651).

### The prompt sent to Claude (lines 600–619)

A structured 4-step prompt: **read the board → read turn history →
match to a `v<N>` skill → run `check-step-gates` before board
advance**. Module ID, step name, new status, actor, and human
message from the event data are appended as context.

### Weakness

`--dangerously-skip-permissions` is the **default** unless
`CLAUDE_ALLOWED_TOOLS` is set (line 552). This gives the autonomous
agent unrestricted tool access. The `CLAUDE_ALLOWED_TOOLS` env var
lets the operator substitute an explicit allowlist for safer
operation in shared environments, but enumerating every tool the
agent needs is a non-trivial configuration task. **For a system
designed for enterprise government use with audit requirements, this
is a significant operational risk** that the documentation calls
out but doesn't resolve.

---

## 8. The TEMPLATE

Three templates under `template/`:

### `template/generic/client/`

Vue 3 + PrimeVue + Tailwind + Pinia + FormKit. Non-GoA frontend.
Includes PWA setup (manifest, service worker), error boundary
components, a Leaflet-based map view, layout shell with navbar/
footer/theme switcher, PWA install prompt. Standard Vite config.

### `template/goa-public/client/`

Vue 3 + GoA Design System (`@abgov/web-components`) + FormKit GoA
theme. Adds government-branded web components, specific CSP rules,
mobile nav workarounds, and WCAG patterns documented in
`goa-overlay.md`.

### `template/goa-public/server/`

The most substantive template. Express + TypeScript + PostgreSQL.
Contains: `src/app.ts`, `src/server.ts`, `src/config/`,
`src/middleware/`, `src/routes/`, `src/controllers/`,
`src/services/`, `src/models/`, `src/validators/`, `src/utils/`,
`src/sse/`, `src/types/`, `websocket/`, a `migrations/` directory
with numbered SQL files, `scripts/` for DB management, a
`Dockerfile`, and `__tests__/`.

### Rule

Templates are **never edited in place** — they are copied by `/build`
into `./app/` and customized there. CLAUDE.md rule #1: *"Always
copy from `./template/` via `/build`; never scaffold by hand."*
Mechanically enforced: `/v5-development` hard-blocks if `./app/`
doesn't exist or has no `package.json`.

### Intentional vulnerable test fixtures

The template ships with intentionally vulnerable test fixture apps
under `.claude/security/blueteam/tests/integration/fixtures/` —
**not for production use**. `dependabot.yml` explicitly excludes them
(they pin `jsonwebtoken@8.5.1` + `CVE-2022-23529` by design for
blueteam regression testing).

---

## 9. The `velocity/` subtree

Per-module working directory for a live Velocity project. Structure
mirrors the 8-step lifecycle:

```
velocity/
├── _audit/                              # Post-execution audit artifacts
│   ├── module-1-critical-audit.md       # Self-critical AI audit after v1-v8 run
│   ├── receipts/v{1-8}-*-receipt.md     # Per-step completion receipts
│   └── *.mjs                            # upload-receipts, walk-steps utilities
├── v1-requirements/
│   ├── inputs/                          # Downloaded SharePoint docs
│   └── output/requirements.md           # Structured FR/NFR deliverable
├── v2-planning/
│   ├── inputs/requirements.md           # Copied from v1 output
│   └── output/plan.md
├── v3-architecture/
│   ├── inputs/{requirements.md, plan.md}
│   └── output/{architecture.md, adrs/ADR-001..008.md}
├── v4-prototyping/
│   ├── inputs/architecture.md
│   └── output/prototype-report.md
├── v5-development/output/development-report.md
├── v6-user-testing/output/test-plan.md
├── v7-user-acceptance/output/{uat-script.md, sign-off.md}
└── v8-deployment/output/{runbook.md, release-notes.md}
```

### A remarkable artifact: `module-1-critical-audit.md`

A **self-critical post-execution audit** written by an AI agent
after running a live v1–v8 cycle on the "Alberta Emergency Services
Interactive Map" project (dated 2026-05-08). Refreshingly honest:

- 50 % delivery rate on must-have FRs (7 of 16 fully delivered, 8
  deferred entirely)
- `/sync-docs` was never invoked during v5
- `/blueteam` was never run as a pre-deploy gate
- v4-prototyping was *"a paper exercise, not actual"*

This is the harness's **own production evidence** of exactly the
failure modes its gate system is designed to prevent — and the
honesty of capturing it (rather than burying it) is unusual.

---

## 10. Claude Code surface

### Skills (`.claude/skills/`) — 13 total

Each is a directory with `SKILL.md`, optional `methodology.md`,
optional `output-template.md`, optional `examples/`.

| Skill              | Slash                | Purpose                                                          |
| ------------------ | -------------------- | ---------------------------------------------------------------- |
| `v1-requirements`  | `/v1-requirements`   | Pull SharePoint docs → structured FR/NFR/modules                 |
| `v2-planning`      | `/v2-planning`       | Decompose to 0.5–3d tasks, critical path                         |
| `v3-architecture`  | `/v3-architecture`   | Components, data model, API sketch, ADRs, threat model           |
| `v4-prototyping`   | `/v4-prototyping`    | Tracer-bullet validation slice                                   |
| `v5-development`   | `/v5-development`    | Production code via `/build` + guides                            |
| `v6-user-testing`  | `/v6-user-testing`   | Test plan + E2E suite + issue triage                             |
| `v7-user-acceptance` | `/v7-user-acceptance` | UAT script + stakeholder sign-off                                |
| `v8-deployment`    | `/v8-deployment`     | GCP Cloud Run or Docker + runbook                                |
| `velocity`         | `/velocity`          | Poll board, find AI-assigned work                                |
| `build`            | `/build`             | Scaffold template into `./app/`                                  |
| `sync-docs`        | `/sync-docs`         | Drift audit README / OpenAPI / CLAUDE.md / code                  |
| `blueteam`         | `/blueteam`          | OWASP ASVS L2 defensive assessment                               |
| `redteam`          | `/redteam`           | Offensive 4-phase pipeline                                       |

### Settings / hooks

`.claude/settings.json` is identical to `.claude/harness-hooks.json`.
Two hook types:

**PostToolUse (`Edit|Write`)** — runs
`check-docs-sync.mjs --migrations-only`, filtering for migration
count drift. Cheap; runs after every file edit to surface stale
README migration counts immediately.

**Stop (six hooks, all mandatory, in order)** — every one blocks the
session from closing on non-zero exit:

1. `check-docs-sync.mjs` — full drift audit across README / OpenAPI
   / CLAUDE.md / migrations / routes
2. `validate-active-steps.mjs --quiet` — lint every
   `velocity/v?-*/output/*.md` against its step's
   `output-template.md`; no-ops if no velocity outputs exist
3. `detect-active-step.mjs` + `check-step-gates.mjs <step>` —
   identify the in-flight velocity step and run its full gate set
   (FR coverage, NFR coverage, migration idempotency, …)
4. `check-security-gate.mjs` — hard-block if blueteam scan is
   missing, >24 h stale, or has CRITICAL findings
5. `check-harness-sync.mjs` — hard-block if any
   `.claude/scripts/*.mjs` is not indexed in `CLAUDE.md`
6. **`check-finding-codified.mjs`** — hard-block if any CRITICAL /
   HIGH finding from redteam PoC or blueteam scan has no
   corresponding entry in `02-security.md`

### Scripts (`.claude/scripts/`)

**20 files, all Node ESM `.mjs`.** Consistent pattern: read artifacts
from disk, derive facts, emit pass/fail with specific evidence on
failure, exit non-zero to block the Stop chain.

### `.claude-plugin/plugin.json`

A forward-compatibility stub only. Documents the migration path to
a proper Claude Code plugin layout. Currently the harness uses
clone-and-use distribution.

### References (`.claude/references/`)

| File                              | Purpose                                                       |
| --------------------------------- | ------------------------------------------------------------- |
| `velocity-api.md`                 | Prescriptive agent guide, mirrored from `GET /velocity/claude-md`. Does **not** enumerate endpoints — those live in the live OpenAPI. Carries workflow patterns, behavioral rules, multi-step recipes. |
| `goa-overlay.md`                  | GoA Design System patterns, CSP, mobile nav workarounds       |
| `security-blueteam.md`            | Pointer into the blueteam framework                           |
| `security-redteam.md`             | Pointer into the redteam framework                            |
| `velocity-board-transitions.md`   | Board state-machine documentation                             |

---

## 11. Security codification loop ⭐

**This is the most architecturally distinctive element of the
harness.** The loop closes the gap between *"we found a security
issue"* and *"that class of issue can never silently recur."*

### Step 1 — Offensive discovery (redteam)

`/redteam` runs 8 agent skills in order: recon → code analysis →
dependency analysis → SAST → secrets detection → infrastructure
analysis → PoC execution → recommendations. Each skill produces a
JSON deliverable (e.g. `poc_testing_MAC001.json`,
`code_analysis_deliverable_MAC001.json`). The
`pipeline/claude_sdk.js` orchestrator runs the 4 core agents via
`@anthropic-ai/claude-agent-sdk`. Findings carry severity levels
(CRITICAL / HIGH / MEDIUM / LOW).

### Step 2 — Defensive confirmation (blueteam)

`/blueteam` runs 15 main skills + 14 ASVS-chapter sub-skills.
Mandatory deterministic scripts (`security-pipeline.js`) run
`npm audit`, `secretlint`, and ESLint security plugins. Outputs
include `security-scan-results.json`, kill-chain YAML, risk
register, and an HTML overview SPA. A **risk-acceptance mechanism**
allows formally accepting non-Critical findings via inline
`// RISK_ACCEPTED: RA-NNN` annotations + entries in
`.ai/data/risk_acceptances.json` (documented in
`RISK_ACCEPTANCE_GUIDE.md`).

### Step 3 — Codification gate (`check-finding-codified.mjs`) ⭐

**The centerpiece.** Runs as a Stop hook. Reads:

- `app/.ai/reports/poc_testing_MAC001.json` for CRITICAL/HIGH
  redteam findings
- `app/.ai/data/security-scan-results.json` for CRITICAL/HIGH
  blueteam findings

For each finding, checks whether the finding's ID or significant
keywords from its title appear in `.claude/standards/02-security.md`.
**If any CRITICAL/HIGH finding is not mentioned in the standards
file, the session is hard-blocked.**

**The rule:** *Every fixed security finding must produce a harness
standard entry so the same class of mistake can never recur.*

### Step 4 — Standards propagation

`02-security.md` grows over time as findings are codified. Commits
`53fff43` and `3c70d4b` show the pattern in practice: redteam
findings CA-001, CA-008, and MAC002 were codified as rules in
`02-security.md` after being discovered. The commit message
*"docs(harness): codify CA-001 and CA-008 findings in 02-security.md
+ update index"* captures the pattern precisely.

### Step 5 — Mechanical enforcement on future builds

Because `02-security.md` is loaded by every v5-development skill
execution as a mandatory standard (CLAUDE.md skill step 0, item 3),
and because `/sync-docs` checks CLAUDE.md references, **a rule
added to `02-security.md` after redteam finding X will be read by
Claude on every future project build**. The harness treats
`02-security.md` as living institutional memory of past failures.

### Gap in the loop

The codification check uses **keyword matching** (first 3 significant
words of a finding title, or the finding ID) against the standards
file. This is heuristic — a sufficiently paraphrased finding title
could pass without genuine codification. The check is honest about
this (no formal semantic matching); it relies on the AI agent to
write meaningful rule text.

---

## 12. Data flow and state

### On-disk state at the harness root

- `.env` — credentials (gitignored)
- `.claude/state/listener-sessions.json` — project → stable session
  UUID map (gitignored, persisted across listener restarts)
- `.claude/state/sse-events.jsonl` — append-only log of all SSE
  events received (gitignored)
- `.claude/state/gate-state.jsonl` — append-only log of gate fire
  results, timestamped (gitignored)

### On-disk state in project working dirs

- `velocity/<step>/inputs/` — documents downloaded from SharePoint
- `velocity/<step>/output/*.md` — structured deliverables (version-
  controlled)
- `app/.ai/data/security-scan-results.json` — blueteam scan output
- `app/.ai/reports/*.json` — redteam deliverables
- `app/.ai/data/risk_acceptances.json` — formal risk acceptance
  records

### External state (in the Velocity backend)

- Velocity board status for each module step
- SharePoint folder containing all project artifacts
- GitHub repository with committed code (Velocity batch API)
- Turn history per step (`GET /velocity/modules/{moduleId}/steps/
  {stepName}/turns`)
- Project revision counter (for `If-Match`)
- Lock state (for multi-agent coordination)

### Env vars consumed at runtime

`VELO_BASE_URL`, `VELO_API_KEY`, `VELO_PROJECT_ID` (or
`VELO_LISTEN_ALL`), `PROJECT_DIRS`, `WATCH_CLUSTERS`,
`WATCH_CHALLENGES`, `CLAUDE_ALLOWED_TOOLS`, `LISTENER_DRY_RUN`,
`MAX_CONCURRENT_CLAUDES`, `QUIET`.

---

## 13. Contracts to the outside world

### Velocity backend API

The primary external contract. All interactions go through
`$VELO_BASE_URL` with `X-API-Key` auth. Key contract points (from
`velocity-api.md`):

- Always check `GET /projects/{id}/permissions` before writing
  (`canMakeVelocityMoves` boolean)
- Send `If-Match: <project_revision>` + `Idempotency-Key: <UUID>`
  on every write
- **Never hardcode endpoint shapes** — fetch `GET /api/v1/docs` (live
  OpenAPI 3.0) at runtime
- The Velocity server injects GitHub + Microsoft Graph credentials
  server-side; **agents never hold these tokens**
- SharePoint paths follow `/{Project}/{Module}/{StepName}/`

### Anthropic Claude API

Used by the redteam pipeline orchestrator (`pipeline/claude_sdk.js`)
via `@anthropic-ai/claude-agent-sdk`. The blueteam and the main
harness use Claude Code **CLI process spawning**, not the API
directly.

### GitHub

The Velocity server exposes GitHub repo management endpoints (create
repo, commit via batch API) transparently to the harness. Harness
skills reference GitHub through Velocity, not directly.

### npm registry

Template + security framework packages (blueteam, redteam). Dependabot
monitors these.

### External recon targets (redteam only)

Public DNS resolvers, CT logs, the target app itself. Intended only
for apps under assessment, not production third-party services.

---

## 14. Relationship to AIDE-VELOCITY

The harness makes extensive reference to AIDE-VELOCITY but **treats
it purely as a service**. Key reference points:

- **`velocity-api.md` lines 5–9:** *"This file mirrors the live agent
  reference served at `GET /velocity/claude-md` (also stored at
  `server/src/static/CLAUDE.md` in the Velo repo)."* The reference
  doc in the harness is a copy of content served by the Velocity
  backend, with explicit instruction to re-sync when the live one
  changes.
- **`.env.example` line 1:**
  `VELO_BASE_URL=https://goa-cc-velocity-app-001.azurewebsites.net/api/v1` —
  the only hard-coded external URL in the entire harness.
- **`velo-listener.js` lines 92, 130:** Constructs SSE URL from
  `VELO_BASE_URL` by replacing the `/api/v1` suffix with
  `/api/v1/velocity/stream`.
- **`velocity-api.md` line 22:** *"The canonical, always-current
  OpenAPI 3.0 spec is served at `$VELO_BASE_URL/api/v1/docs`. Fetch
  it once at startup."* The live API contract — the harness
  explicitly avoids duplicating endpoint schemas.
- **`velo-listener.js` line 626:** `GET /velocity` — startup poll
  for pre-existing AI assignments.

The relationship is **the OPC↔OAP analogy realized**:
AIDE-VELOCITY-HARNESS is the developer-side tool (cockpit, factory
floor) that integrates with the AIDE-VELOCITY platform (control
plane, SaaS backbone). The harness has **no peer dependencies**; it
is entirely passive from AIDE-VELOCITY's perspective (a client,
never a server).

---

## 15. Operational shape

### One-time setup

```bash
git clone <harness> my-project && cd my-project
cp .env.example .env
# Edit .env: set VELO_BASE_URL, VELO_API_KEY, VELO_PROJECT_ID

# Install security framework deps (optional):
cd .claude/security/blueteam && npm install
cd .claude/security/redteam && npm install
```

### Per-project hooks wiring (optional)

```bash
cp .claude/harness-hooks.json <project>/.claude/settings.json
```

### Pre-flight before any v1–v8 skill

```bash
node .claude/scripts/preflight-velocity.mjs <projectId> [moduleId]
```

### Autonomous mode

```bash
node velo-listener.js         # foreground
# or: pm2 start velo-listener.js
```

### Manual skill invocation (in Claude Code)

```
/v1-requirements
/v5-development --module <moduleId>
/blueteam
/sync-docs
```

### Process model

- **One** `velo-listener.js` per developer (or per team, scoped to
  a project)
- **Multiple** Claude Code subprocesses may run concurrently across
  projects (capped by `MAX_CONCURRENT_CLAUDES`)
- The harness itself has **no server process** — entirely client-side

### Update model

`git pull` in the harness root. Templates, skills, gates, and
standards all update by pulling. Child project
`.claude/settings.json` (copied from `harness-hooks.json`) may need
manual merge after significant harness changes.

---

## 16. Extension points

The harness is explicitly designed to grow. In decreasing formality:

1. **New velocity step skills** (`v<N>-<name>/SKILL.md`) — must use
   `v<N>-` prefix, stay under 500 lines, include `methodology.md` +
   `output-template.md`. After creation: update CLAUDE.md skill
   index, update `harnessData.skills` in `harness.html`, run
   `/sync-docs`.
2. **New general skills** (no prefix) — same file requirements,
   no `v<N>-` prefix.
3. **New gate scripts** (`.claude/scripts/check-*.mjs`) — add to
   `check-step-gates.mjs` `GATES_BY_STEP` map for the appropriate
   steps. Wire as a Stop hook in `harness-hooks.json` if it should
   fire globally. **`check-harness-sync.mjs` will hard-block until
   the new script is indexed in CLAUDE.md** — the self-enforcing
   consistency gate.
4. **New templates** (`template/<name>/`) — add
   `{ id, path, stack, use }` entry to `harnessData.templates` in
   `harness.html`.
5. **New standards** (`.claude/standards/<name>.md`) — add entry to
   `harnessData.standards`.
6. **New guides** (`.claude/guides/<area>/<NN>-*.md`) — add entry
   to `harnessData.guides`.
7. **Plugin distribution (future)** — migrate `.claude/skills/` →
   `skills/`, `.claude/scripts/` → `scripts/`,
   `.claude/harness-hooks.json` → `hooks/hooks.json` at harness
   root. Steps documented in `.claude-plugin/MIGRATION_NOTE.md`.

### Why `check-harness-sync.mjs` matters

It is the **key enforcer**: blocks session close if any script in
`.claude/scripts/` is not indexed in `CLAUDE.md`, making it
**mechanically impossible** to add a gate without also documenting
it.

---

## 17. Implementation-quality observations

### Strengths

1. **The gate philosophy is sound and consistently applied.** *"All
   gates mechanical, not aspirational"* (CLAUDE.md rule #11) is not
   just a statement — every gate is a real script with a real exit
   code wired into a real hook.
2. **The SSE listener is professionally engineered.** Exponential
   backoff with minimum floors, `Last-Event-ID` replay, `Retry-After`
   header respect, half-open connection detection via heartbeat
   watchdog, subscription update debouncing, self-quench project
   scoping. The fix history (commits `68e5234`, `f364804`, *"Fix SSE
   reconnect storm"*) shows real production debugging happened.
3. **The security codification loop is genuinely novel.** The idea
   that a CRITICAL finding must write itself into a standard rule
   before the session closes creates **institutional memory at the
   harness level**, not just the project level. Every future Claude
   session on any project inherits the lessons from past redteam
   runs.
4. **Honest self-assessment.** `module-1-critical-audit.md` is
   remarkable — an AI-authored self-critique identifying 21 gaps in
   its own v1–v8 run, including admitting it produced a
   *"performative"* prototype and never ran the mandated security
   scans. This kind of honest accounting is rare.
5. **Zero-dependency harness scripts.** `.claude/scripts/*.mjs`
   working anywhere Node ≥18 is available without `npm install`.
6. **The `check-finding-codified` gate is the right idea.** Closing
   the loop from finding → standard rule turns security assessments
   from one-shot audits into incremental policy accumulation.

### Weaknesses and trade-offs

1. **Two parallel documents (`CLAUDE.md` + `harness.html`) must be
   kept in sync by hand.** The gate catches divergence after the
   fact but cannot auto-update `harness.html` when a skill is added
   to `CLAUDE.md`. Growing maintenance burden.
2. **82 KB single-file HTML.** The trade-off (no build step, CDN-only
   deps, works offline once loaded) is understandable, but the file
   size makes git diffs unreadable. A build step generating
   `harness.html` from `CLAUDE.md` would eliminate the sync problem.
3. **32 KB single-file listener.** `velo-listener.js` is CommonJS
   and monolithic. Session management, HTTP helpers, SSE parsing,
   event routing, and Claude spawning are all in one file. Well-
   organized with section comments but would benefit from module
   splitting for testability. **No unit tests for the listener.**
4. **`--dangerously-skip-permissions` default.** The listener
   spawns `claude --dangerously-skip-permissions` unless
   `CLAUDE_ALLOWED_TOOLS` is explicitly set. **For a system designed
   for enterprise government use with audit requirements, this is a
   significant operational risk** that the documentation calls out
   but doesn't resolve.
5. **Keyword-matching codification check.** First-3-significant-words
   heuristics are gameable. A finding titled "Cross-Site Scripting
   via innerHTML" could be "codified" by a standards entry
   mentioning "cross-site scripting" in an unrelated context.
6. **No CI other than Dependabot.** Harness gate philosophy is
   implemented for child projects but the harness repo itself relies
   on developer discipline.
7. **The module-1 audit reveals real gaps in gate coverage at the
   time of the v1–v8 run.** Gates were added reactively after that
   run — the expected trajectory ("when evals fail, build better
   evals") but the gate set should be considered continuously
   evolving rather than complete.
8. **Intentionally vulnerable test fixtures alongside production
   templates.** A developer cloning the harness and running
   `npm audit` from the repo root might be confused by the results
   without reading `FIXTURE_MANIFEST.md`.

---

## 18. Concepts worth porting to OAP/OPC

Ranked by likely value to the OAP/OPC stack:

1. **Mechanical Stop gates wired into Claude Code hooks.** The
   `harness-hooks.json` pattern — PostToolUse cheap drift check +
   Stop chain of blocking validators — is directly applicable to
   OPC/OAP. OAP already has `make pr-prep` and CI coupling gates;
   the analogous pattern here runs those checks **inline during
   Claude Code sessions**, not just at PR time. The
   `check-spec-code-coupling` equivalent could fire as a Stop hook.
2. **Security codification loop: finding → standard rule → gate.**
   OAP has the spec spine as its truth document. A matching pattern
   would be: security finding discovered in `axiomregent` or
   `policy-kernel` → a new rule added to a `standards/security/`
   spec → a gate that hard-blocks CI until the finding class is
   represented. The `check-finding-codified.mjs` pattern could map
   to a `check-security-finding-in-spec.mjs` equivalent.
3. **Stable session UUIDs per project context.** The listener's
   `listener-sessions.json` pattern — one stable UUID per logical
   "project", resumed across all events — is directly applicable to
   OPC's multi-agent orchestration (`crates/orchestrator`).
   Currently OAP dispatches agent workflows per-task; per-project
   session continuity would let agents accumulate context across the
   full lifecycle.
4. **Structured deliverable validation against output templates.**
   Each skill's `output-template.md` with `validate-step-output.mjs`
   checking section presence is analogous to OAP's spec spine
   requirements. The harness runs these checks at Stop time; OAP
   runs them at PR time. Running them earlier (at conversation end)
   would surface gaps sooner.
5. **Self-quench / project scoping for autonomous agents.** The
   `VELO_PROJECT_ID` guard preventing agent collision is directly
   applicable to OAP's orchestrator when multiple agents might
   respond to the same platform event. The `VELO_LISTEN_ALL=true`
   explicit opt-in with a warning message is a good safety pattern.
6. **Evidence strength distinction.** `check-evidence-strength.mjs`
   distinguishes between *"config file exists"* and *"config file is
   actually exercised (call sites present)."* This maps well to
   OAP's spec coupling gate: the gate could distinguish *"spec is
   mentioned in a `[package.metadata.oap]` entry"* (config-level
   citation) vs *"spec concepts appear in actual test assertions"*
   (evidence-level citation).
7. **The dual blue/red security assessment protocol.** OAP's
   policy-kernel and tool-registry implement security decisions but
   may not have the equivalent of a 14-chapter ASVS assessment +
   STRIDE threat model + deterministic scan pipeline. The harness's
   security framework is portable to any Node/Express project
   assessment.
8. **Gate audit trail (`gate-state.jsonl`).** Append-only JSONL
   audit trail of gate fires is a lightweight but useful
   observability pattern for any governed workflow. OAP's
   factory-engine could emit similar structured traces.
9. **`preflight-velocity.mjs` pattern.** A preflight script that
   checks credentials, API reachability, and permissions before any
   long-running operation is a best practice OAP could apply to any
   Encore.ts service startup or Tauri app boot sequence.
10. **Harness.html as AI-readable documentation.** Embedding
    structured JSON in a human-readable HTML page so both humans and
    AI agents can extract the same inventory is applicable to OPC.
    A companion `opc-harness.html` with embedded `harnessData`
    describing available commands, factory stages, and adapter types
    would give both users and agents a navigable reference.

---

## 19. Open questions / gaps

1. **What does AIDE-VELOCITY look like architecturally?** The
   harness treats it as a black-box API. The `velocity-api.md` is
   an agent guide, not an architecture document. (See companion
   blueprint.)
2. **How does the Velocity board "game" scoring work?**
   `velocity-api.md` references scoring and leaderboards; the
   listener handles `challenge_winner_picked` events. Mechanics not
   exposed in this repo.
3. **What is the `challenge` concept?** Challenges appear to be
   competitive clones of projects with `challenge_accepted` creating
   a clone and `challenge_winner_picked` selecting a winner. The
   listener handles these events but business purpose is not
   documented in this repo.
4. **Are the section filters and markdown export features being used
   in practice?** Added in commit `315a241`; no evidence of
   generated markdown outputs in the repo.
5. **How does the Velocity batch commit API work?**
   `/v5-development` references *"committed via batch API"* — the
   batch API shape lives in the live OpenAPI fetched at runtime,
   not in this harness.
6. **What is `sharepoint_ai_shadow_created`?** The listener handles
   this event (line 483) with *"AI shadow ready for project"* but
   the concept of an *AI shadow file* is not documented here.
7. **GoA context auto-detection.** `goa-overlay.md` and `goa/`
   standards are loaded manually when building GoA apps; no
   detection mechanism auto-applies GoA overlays when a GoA-flagged
   project is detected.
8. **Formal test suite at harness scripts level?** Blueteam has unit
   + integration tests. Core `.claude/scripts/*.mjs` don't appear
   to have automated tests — they rely on
   `check-harness-consistency.mjs` and the self-referential Stop
   hook suite.
9. **What is the `MAC001` / `MAC002` naming convention?**
   "MAC = Module Assessment Configuration"? Or a project code? Not
   explained.
10. **Multi-module project layout collision?** The 8-step flow is
    module-scoped (each module has its own board). The listener
    resolves `moduleId → projectId` lazily. But `velocity/` appears
    to be flat (one set of `v1`–`v8` dirs), not per-module. For a
    project with many modules, the layout would collide unless
    module-specific subdirectories are used — not documented.

---

## 20. Synthesis for OAP/OPC reimplementation

The harness's **defining contribution** is the conviction that **prose
obligations cannot be trusted**: every "you must do X" must be
backed by a mechanical gate script that hard-blocks on failure. This
conviction is implemented uniformly via the Claude Code Stop-hook
chain. The closest OAP analogue is the spec-coupling gate (spec 127
amended by 130), but OAP enforces at PR time; the harness enforces
at **conversation end**. The leverage difference is significant —
the harness catches drift before any commit is made.

If OAP/OPC were to absorb the strongest concepts, the high-leverage
moves are:

1. **Add a Stop-hook gate chain to OPC.** Wire `make pr-prep`-class
   checks (codebase indexer, spec coupling, spec lint) as PostToolUse
   and Stop hooks on OPC's Claude Code surface. Convert "you should
   run X" prose into "X runs automatically and blocks otherwise."
2. **Implement the codification gate.** For every security or
   correctness finding generated by `axiomregent`, require a spec
   spine entry under `standards/` before the conversation closes.
3. **Adopt stable per-project session UUIDs in the orchestrator.**
   Persist agent context across factory-engine invocations so each
   project accumulates institutional memory.
4. **Build `harness.html` analogue for OPC.** A single-file HTML
   with embedded `harnessData` JSON, surfaceable inside OPC and
   parseable by AI agents — same dual-audience principle.
5. **Add evidence-strength distinction to the coupling gate.**
   Don't just check that a spec is *mentioned* in a manifest — check
   that spec concepts appear in test assertions.

What the OAP/OPC stack can do **better** than the harness:

- **OAP's spec spine** is *authored* truth (markdown), where the
  harness uses derived truth (gate scripts pointing at convention).
  OAP's compiler emits machine truth from authored truth; the
  harness lacks this layer.
- **Rust crates with type safety** (orchestrator, policy-kernel,
  tool-registry) give compile-time guarantees that the harness's
  zero-dependency Node `.mjs` files cannot.
- **Tauri-based OPC** can provide a native cockpit with file-system
  access, persistent state, and native notifications — features the
  harness's `harness.html` (browser-only, CDN-loaded) cannot offer.
- **OAP's policy kernel + 5-tier settings merge** provides governed
  configuration that the harness handles informally via env vars.
