---
id: "112-factory-project-lifecycle"
slug: factory-project-lifecycle
title: Factory Project Lifecycle — Create, Import, Open
status: draft
implementation: pending
owner: bart
created: "2026-04-22"
summary: >
  Defines the contract-anchored lifecycle for factory-produced projects across
  three entry points: (1) OPC opens a local folder and recognises it as a
  factory project via ACP pipeline-state conformance; (2) Stagecraft creates
  a new project by scaffolding an adapter and pushing to GitHub; (3)
  Stagecraft imports an existing GitHub repo and registers it in the
  workspace. Anchors detection on the ACP contract layer (spec 074) and
  extends the translator (spec 108) to bridge legacy
  `goa-software-factory`-shaped manifests. Absorbs the scaffold/push
  capability currently in the external `template-distributor` repo into
  stagecraft; template-distributor is discontinued as a separate service.
depends_on:
  - "074"  # factory-ingestion (ACP contracts — Build Spec, Adapter Manifest, Pipeline State, Verification)
  - "075"  # factory-workflow-engine (engine that advances pipeline state)
  - "087"  # unified-workspace-architecture (duplex channel, workspace-as-atom)
  - "094"  # unified-artifact-store (where emitted artifacts land)
  - "108"  # factory-as-platform-feature (translator, factory_adapters/contracts/processes tables)
  - "110"  # stagecraft-to-opc-factory-trigger (run dispatch envelope)
  - "111"  # org-agent-catalog-sync (establishes the workspace-scoped sync pattern reused here)
implements:
  - path: crates/factory-contracts/schemas/
  - path: crates/factory-project-detect/
  - path: platform/services/stagecraft/api/factory/translator.ts
  - path: platform/services/stagecraft/api/projects/create.ts
  - path: platform/services/stagecraft/api/projects/import.ts
  - path: platform/services/stagecraft/api/projects/scaffold/
  - path: platform/services/stagecraft/web/app/routes/app.projects.new.tsx
  - path: platform/services/stagecraft/web/app/routes/app.projects.import.tsx
  - path: apps/desktop/src-tauri/src/commands/factory_project.rs
  - path: apps/desktop/src/routes/factory/ProjectCockpit.tsx
---

# 112 — Factory Project Lifecycle — Create, Import, Open

## 1. Problem

Spec 108 made Factory a first-class platform feature (adapters / contracts /
processes as stagecraft-owned entities). Spec 110 wired stagecraft-initiated
run dispatch to OPC. Spec 111 established the workspace-scoped catalog
pattern for agents. What none of these specs answer:

**How does a project enter the workspace in the first place?**

Today there are three de-facto paths, none governed:

1. **Hand-rolled local clone.** A user `git clone`s a repo into some folder,
   opens it in OPC, and OPC has no way to recognise whether the repo is
   factory-produced, template-scaffolded, or unrelated. The "Factory" route
   in OPC (spec 076) assumes state it cannot detect.

2. **External `template-distributor` service.** A separate Express app in
   `GovAlta-Pronghorn/template-distributor` provides an ad-hoc web UI for
   cloning the `template` repo, applying a profile, creating a GitHub repo,
   and pushing. It is not integrated with stagecraft — no workspace binding,
   no policy gate, no audit, no adapter identity beyond the copied
   `template.json`.

3. **`goa-software-factory` 5-stage pipeline.** The upstream AI factory
   (`GovAlta-EMU/the_factory`, mirrored at `GovAlta-Pronghorn/goa-software-factory`)
   produces `requirements/` + code directly into a template-scaffolded repo.
   Its output shape — `requirements/audit/factory-manifest.json` (5 stages),
   `requirements/audit/working-state.json`, ad-hoc `requirements/{ui,api}/build-spec.json`
   — predates the ACP contract layer and does not conform to
   `pipeline-state.schema.yaml` or `build-spec.schema.yaml`.

Symptoms:

- Opening `/Users/bart/Dev2/cfs-womens-shelter-funding-portal` in OPC shows
  no factory cockpit even though it is a fully-produced factory project.
- Stagecraft cannot list the projects it owns in a workspace until they are
  manually registered.
- A user cannot click "Import Project" in stagecraft and have the platform
  detect factory state, bind it to an adapter row, and make it available for
  reconciliation.
- `template-distributor` duplicates OAuth, workspace context, and org
  identity that stagecraft already owns, and emits scaffolds that are not
  ACP-native (no `.factory/pipeline-state.json` seeded at generation time).

## 2. Decision

Define a single, contract-anchored lifecycle with three entry points. All
three converge on the same state representation: a
`pipeline-state.schema.yaml`-conformant document at
`<repo-root>/.factory/pipeline-state.json`, with a workspace-scoped
`projects` / `project_repos` row in stagecraft (spec 108) linking to the
`factory_adapters` and `factory_processes` rows used to produce it.

### 2.1 Three entry points

1. **OPC Open** — open a local folder; detect via ACP conformance; surface
   the Factory Cockpit if positive.
2. **Stagecraft Create** — pick an adapter, scaffold a new repo
   ACP-natively, push to GitHub, register; OPC claims it when the user
   opens the local checkout.
3. **Stagecraft Import** — paste a GitHub URL (or pick from App
   installation); clone server-side, detect, translate legacy → ACP if
   needed, register; OPC claims it on local open.

### 2.2 Template-distributor discontinued

The external `template-distributor` service is retired. Its capabilities —
clone the `template` repo, apply a profile via `template/scripts/setup-*.ts`,
create a GitHub repo, push — are absorbed into stagecraft under
`api/projects/scaffold/`. Stagecraft already owns the GitHub App, OAuth
flow, org identity, and workspace context this code needs. No new OAuth,
no new UI.

## 3. Contracts

### 3.1 Canonical state location

```
<repo-root>/.factory/pipeline-state.json   — conformant to pipeline-state.schema.yaml (spec 074)
```

This file is the single authoritative marker that a directory is a factory
project. It is committed to the repo (it travels with the code). OPC,
stagecraft, and the factory engine all read and write it through the
consumer crate (§3.3) — never via ad-hoc JSON parsing (per
`.claude/rules/governed-artifact-reads.md`).

### 3.2 ACP schemas (from spec 074, vendored)

```
crates/factory-contracts/schemas/
  adapter-manifest.schema.yaml
  build-spec.schema.yaml
  pipeline-state.schema.yaml
  verification.schema.yaml
```

Schemas are compiled into Rust types with a compile-time `SCHEMA_VERSION`
const per the existing build-time schema rule. At runtime, stagecraft
mirrors these schemas into the `factory_contracts` table (spec 108 §3)
per-org for policy-scoped lookups. The schemas themselves are the source
of truth; the DB mirror is an org-scoped projection.

### 3.3 Detection crate

New crate `crates/factory-project-detect/`:

```rust
pub enum DetectionLevel {
    NotFactory,
    ScaffoldOnly,      // L0 — template scaffolded, no pipeline run yet
    LegacyProduced,    // L1 — goa-software-factory 5-stage manifest, needs translation
    AcpProduced,       // L2 — pipeline-state.json present and conformant
}

pub struct FactoryProject {
    pub level: DetectionLevel,
    pub pipeline_state: Option<PipelineState>,   // Some for L2; translated value for L1
    pub adapter_ref: Option<AdapterRef>,         // name + version from pipeline-state.adapter
    pub legacy_manifest: Option<serde_json::Value>, // L1 only — raw factory-manifest.json
}

pub fn detect(repo_root: &Path) -> FactoryProject;
```

Detection logic:

| Files present | Level |
|---|---|
| `.factory/pipeline-state.json` (schema-conformant) | AcpProduced |
| `requirements/audit/factory-manifest.json` + `requirements/audit/working-state.json` | LegacyProduced |
| Scaffold-only signals (e.g. `template.json` with `templateName`, no pipeline-state, no legacy manifest) | ScaffoldOnly |
| None of the above | NotFactory |

The crate is consumed by OPC (via a Tauri command) and by stagecraft (via
a Node addon or by shelling the crate's CLI bin — see §6.2). Both paths
read through the crate — no component parses the JSON directly.

### 3.4 Legacy translation

Legacy detection (L1) routes through stagecraft's existing
`platform/services/stagecraft/api/factory/translator.ts` (spec 108),
extended with:

```ts
export function translateLegacyManifest(
  legacy: GoaSoftwareFactoryManifest,    // 5-stage shape
  workingState: GoaWorkingState,
  orgAdapters: FactoryAdapter[],
): PipelineState;                        // pipeline-state.schema.yaml-conformant
```

Stage remapping table (legacy → ACP):

| Legacy stage | ACP stage id |
|---|---|
| `stage1_businessRequirements` | `business-requirements` |
| `stage2_serviceRequirements` | `service-requirements` |
| `stage3_databaseDesign` | `data-model` |
| `stage4_apiControllers` | `api-specification` |
| `stage5_clientInterface` | `ui-specification` |
| *(implicit)* | `pre-flight` (synthesised as `completed`) |
| *(implicit)* | `adapter-handoff` (synthesised from `fileOwnership` if present) |

`build-spec` production is deferred: legacy `requirements/{ui,api}/build-spec.json`
stays in place as informational artifacts until the project's next factory
run emits a unified ACP-conformant Build Spec. Detector does not demand
Build Spec conformance at L1.

### 3.5 Adapter identity

Scaffold-only projects (L0) must carry a minimal adapter reference so the
lifecycle can advance. The scaffold writer (§4.2) writes:

```json
// .factory/pipeline-state.json (L0 seed)
{
  "schema_version": "1.0.0",
  "pipeline": {
    "id": "<uuid>",
    "factory_version": "<semver>",
    "started_at": null,
    "status": "pending",
    "adapter": { "name": "aim-vue-node", "version": "3.0.0", "source_sha": "..." },
    "build_spec": { "path": null, "hash": null }
  },
  "stages": {}
}
```

`template.json` is **not** the adapter identity. It stays as a local
scaffold config (module inventory, `fileOwnership`) consumed by the
template's own `add-module.ts` / `remove-module.ts` tooling. Detection
does not read it except as a hint when `.factory/pipeline-state.json` is
absent.

## 4. OPC Open Path

### 4.1 Detection on folder open

When OPC opens a directory (via existing recents menu, File → Open, or
workspace sync from stagecraft):

1. `apps/desktop/src-tauri/src/commands/factory_project.rs` invokes
   `factory_project_detect.detect(path)` from the crate.
2. If `NotFactory`: no cockpit; standard editor view only.
3. If `ScaffoldOnly` / `LegacyProduced` / `AcpProduced`: surface the
   **Factory Cockpit** panel at `apps/desktop/src/routes/factory/ProjectCockpit.tsx`.

### 4.2 Factory Cockpit

The cockpit reads pipeline-state (translated for L1) and shows:

- **Pipeline status** — running / paused / completed / failed / cancelled.
- **Stage timeline** — the 7 ACP stages with per-stage status, duration,
  artifact count. For L1 projects, stages are rendered from the translated
  view with a "legacy" badge; the two synthesised stages (`pre-flight`,
  `adapter-handoff`) are marked as such.
- **Adapter identity** — name + version from `pipeline.adapter`; links to
  the `factory_adapters` row in stagecraft (via workspace context).
- **Drift indicator** — compares `pipeline.build_spec.hash` against the
  current Build Spec on disk (if present) and the adapter `source_sha`
  against the workspace's current adapter version in `factory_adapters`.
- **Actions** (each dispatches to the factory engine via the existing
  spec 110 envelope):
  - *Run Stage N* — advance pipeline from current cursor.
  - *Reconcile* — spec-088-style drift reconciliation, incremental re-run
    from earliest dirty stage.
  - *Re-extract artifacts* — run the prestart extractor on `.artifacts/raw/`
    (see §4.3).
  - *Register with workspace* — if the project is not yet bound to a
    stagecraft `projects` row, create the binding and dual-write.

### 4.3 Artifact extraction as an ACP stage

The existing `.artifacts/extract_artifacts.py` script (raw → extracted) is
re-cast as a **pre-flight sub-step**, not a user-run Python script. A new
Rust binary `crates/factory-artifacts/src/bin/extract.rs` replaces the
Python version (matching the scripts-to-binaries direction of spec 105).
The pre-flight stage in pipeline-state gains an `artifact_extraction`
sub-artifact entry once it runs. `prestart-prompt.txt`, `start-prompt.txt`,
`reconciliation-prompt.txt` (currently hand-authored per project) are
retired; the corresponding behaviour is driven by the cockpit actions.

## 5. Stagecraft Create Path

### 5.1 UI — `/app/projects/new`

Form fields:

- **Workspace** (pre-selected from context).
- **Adapter** — dropdown of `factory_adapters` rows for the current org.
  Each shows name, version, and a short capability summary from the
  adapter manifest (dual-stack, auth methods, etc.).
- **Variant** — enum from `build-spec.schema.yaml` `project.variant`:
  `single-public`, `single-internal`, `dual`. The adapter manifest's
  `capabilities` gates which variants are offered (e.g. an adapter without
  `dual_stack: true` hides the `dual` option).
- **Project identity** — name (slug), display name, description, GitHub
  owner (from the user's App installations; private-by-default).
- **Optional seed inputs** — upload business documents into
  `.artifacts/raw/` at generation time so pre-flight has something to
  extract on first run.

### 5.2 Backend — `api/projects/create.ts`

Flow:

1. Validate form against `build-spec.schema.yaml` `project` + `auth`
   sub-schemas (workspace-scoped policy bundle enforces org rules).
2. Resolve the adapter's scaffold source (e.g. a git ref in the `template`
   repo, pinned by the `factory_adapters.source_sha` column) and clone
   into a per-request temp dir.
3. Run the adapter's scaffold entry point
   (`<adapter>/scaffold/scripts/setup.ts` or equivalent — resolved from
   the adapter manifest's `scaffold.entry_point` field, added in §8) with
   the chosen variant and module profile.
4. **Seed ACP state**: write `.factory/pipeline-state.json` at L0 shape
   per §3.5, with `pipeline.adapter` populated from the `factory_adapters`
   row.
5. If the user supplied seed inputs, write them under `.artifacts/raw/`.
6. Create the GitHub repo via the org's App installation and push the
   generated tree with a single initial commit. Commit author is the
   stagecraft service identity; co-author is the user.
7. Insert rows into `projects` and `project_repos` (spec 108) with
   `factory_adapter_id` pointing at the adapter used. Emit a
   `project.created` audit event.
8. Return `{ project_id, repo_url, clone_url }` for the UI to link to.

### 5.3 What is absorbed from template-distributor

Lift, discard the rest:

| Template-distributor capability | Where it lands in stagecraft |
|---|---|
| GitHub App OAuth | Already exists in stagecraft — no change |
| Template repo clone + cache | `api/projects/scaffold/templateCache.ts` (per-workspace cache, keyed on `factory_adapters.source_sha`) |
| Pre-build profiles (minimal/public/internal/dual) | Dropped — profiles are declared by the adapter manifest, not baked binaries |
| Apply profile via `setup-app.ts` / `setup-dual-app.ts` | `api/projects/scaffold/runAdapterScaffold.ts` — invokes the adapter's declared entry point in a sandboxed Node subprocess |
| Create GitHub repo (Octokit) | `api/projects/scaffold/githubRepoCreate.ts` |
| Push generated tree | `api/projects/scaffold/githubPushInitial.ts` |
| In-memory generation state tracking | Replaced with a `scaffold_jobs` table (UUID, status, stream of log events) |
| Standalone web UI | Dropped — stagecraft's `/app/projects/new` owns the UX |

The external `template-distributor` repo is archived; no code is imported
directly. This spec's implementation phase re-writes the ~10 discrete
operations above in TypeScript idioms that match the stagecraft codebase
and respect its policy/audit fabric.

## 6. Stagecraft Import Path

### 6.1 UI — `/app/projects/import`

Form fields:

- **Workspace** (pre-selected).
- **Source** — either (a) pick from GitHub App installation repos, or (b)
  paste a GitHub URL (the server validates App access before proceeding).
- **Import mode** — auto-detected after clone; form confirms the detected
  level (§3.3) and, for L1, shows the translator's preview (which ACP
  stages the legacy manifest will become).

### 6.2 Backend — `api/projects/import.ts`

Flow:

1. Validate App installation access to the target repo.
2. Clone into a per-request temp dir at the default branch HEAD.
3. Invoke the detection crate (via a small Rust CLI bin
   `factory-project-detect inspect <path> --json`) and parse its
   structured output. This is a governed consumer read per
   `.claude/rules/governed-artifact-reads.md` — no ad-hoc JSON parsing on
   the Node side.
4. Branch on detection level:
   - **NotFactory**: show "not a factory project" with an explicit
     "Adopt" confirmation. Adopting writes an L0 `.factory/pipeline-state.json`
     with a placeholder adapter the user must then pick; opens a PR
     against the source repo (does not push to `main` directly).
   - **ScaffoldOnly**: confirm the adapter match against
     `factory_adapters` rows. If a match is found, register directly. If
     not, prompt the user to select or reject.
   - **LegacyProduced**: run `translateLegacyManifest` (§3.4). Preview
     the translated pipeline-state to the user. On confirm, open a PR that
     adds `.factory/pipeline-state.json` alongside the legacy manifest
     files (which stay — they are never deleted by this flow).
   - **AcpProduced**: validate pipeline-state schema version, confirm
     adapter binding, register.
5. Insert `projects` / `project_repos` rows. Emit `project.imported`
   audit event including the detection level and (for L1) the translator
   version.
6. Return `{ project_id, detection_level, deep_link }` where `deep_link`
   is an `oap://` URI the user can click to hand off to OPC (which clones
   locally and surfaces the Factory Cockpit).

## 7. State Authority and Sync

This spec inherits the workspace-as-atom authority model from spec 087:

- **Authoritative state** for project identity, adapter binding, audit
  trail: stagecraft Postgres (`projects`, `project_repos`,
  `factory_adapters`, `scaffold_jobs`, `audit_events`).
- **Authoritative state** for pipeline progress: `.factory/pipeline-state.json`
  in the repo working tree (committed). Stagecraft caches the latest
  known state per-project for list views, but on conflict the repo wins.
- **OPC** sees projects via the duplex channel sync (reuses spec 111's
  catalog envelope pattern). A new `ServerEnvelope::project.catalog.upsert`
  variant carries the `projects` row to connected desktops. OPC displays
  the list in a "Projects" panel and lets the user open any entry; opening
  clones the repo locally if not present and activates the cockpit.

No new duplex envelope variants for scaffold jobs — stagecraft's existing
job-stream SSE (or the spec 109 PubSub pattern) is used for create/import
progress.

## 8. Adapter Manifest Extension

To support §5.2, `adapter-manifest.schema.yaml` gains:

```yaml
scaffold:
  entry_point: string          # Relative path to scaffold entry script (e.g. "scripts/setup.ts")
  runtime: string              # Execution runtime (e.g. "node-24", "deno-2")
  args_schema: object          # JSON schema for --args accepted by the entry point
  profiles:                    # Declared variant/profile combinations
    - name: string             # e.g. "dual-saml-postgres"
      variant: string          # Matches build-spec project.variant
      modules: [string]        # Module names activated for this profile
      default: boolean
  emits:                       # What scaffold produces, relative to project root
    - path: string             # e.g. "template.json", "apps/", ".factory/pipeline-state.json"
```

Existing manifest `build_commands` / `validation` / `directory_conventions`
sections are unchanged. This is a backward-compatible addition (new
optional top-level key).

## 9. Implementation Phases

Each phase is independently mergeable and ends in a runnable state.

**Phase 1 — Detection crate.**
- Land `crates/factory-project-detect/` with the detection algorithm and
  a CLI bin for external consumers.
- Unit tests over three fixture repos: AcpProduced, LegacyProduced (use
  cfs-womens-shelter as reference), ScaffoldOnly.
- Exit criteria: `factory-project-detect inspect <path> --json` returns
  correct level for all three fixtures.

**Phase 2 — Translator extension.**
- Extend `platform/services/stagecraft/api/factory/translator.ts` with
  `translateLegacyManifest`.
- Integration test reads the cfs repo via a fixture and verifies the
  output conforms to `pipeline-state.schema.yaml`.
- Exit criteria: translator round-trips cfs manifest → pipeline-state
  without loss of stage completion timestamps or artifact references.

**Phase 3 — OPC Open path.**
- Wire `apps/desktop/src-tauri/src/commands/factory_project.rs` to invoke
  the detection crate.
- Build `apps/desktop/src/routes/factory/ProjectCockpit.tsx` with the
  timeline and action buttons. Actions dispatch via the existing spec 110
  envelope.
- Exit criteria: opening cfs-womens-shelter in OPC shows a populated
  cockpit.

**Phase 4 — Adapter manifest scaffold extension.**
- Extend `adapter-manifest.schema.yaml` per §8.
- Update the `aim-vue-node` adapter manifest to declare `scaffold.*`
  fields pointing at its `setup-app.ts` / `setup-dual-app.ts`.
- Exit criteria: `registry-consumer show aim-vue-node` (or equivalent)
  shows the new scaffold block and validates.

**Phase 5 — Stagecraft Create.**
- Land `api/projects/create.ts`, `api/projects/scaffold/*`, and the
  `/app/projects/new` route.
- Absorb the six template-distributor operations listed in §5.3.
- Exit criteria: creating a new project via the web UI produces a repo
  whose HEAD contains `.factory/pipeline-state.json` (L0 seed) and is
  registered in `projects`.

**Phase 6 — Stagecraft Import.**
- Land `api/projects/import.ts` and the `/app/projects/import` route.
- Exit criteria: importing cfs-womens-shelter via the web UI results in a
  `projects` row with `detection_level = "legacy_produced"` and a PR
  opened against the cfs repo adding `.factory/pipeline-state.json`.

**Phase 7 — Workspace sync and OPC project list.**
- Add `project.catalog.upsert` envelope variant; reuse the spec 111 sync
  pattern.
- Add a "Projects" panel in OPC showing workspace projects with local
  clone state.
- Exit criteria: creating or importing in stagecraft updates a connected
  OPC's project list without a restart.

**Phase 8 — template-distributor retirement.**
- Archive the external `template-distributor` GitHub repo.
- Remove any remaining references from docs.
- Exit criteria: no repo-level or doc-level references to
  template-distributor remain; the "Create Project" path is accessible
  only via stagecraft.

**Phase 9 — Legacy prompt-file retirement.**
- Remove `prestart-prompt.txt`, `start-prompt.txt`,
  `reconciliation-prompt.txt` from factory-produced template outputs.
- The cockpit actions (§4.2) now drive the equivalent behaviour.
- Existing projects keep their copies until their next reconciliation run.
- Exit criteria: newly created projects (post Phase 5) do not contain
  these three files.

## 10. Risks and Open Questions

- **Build Spec unification** (§3.4). Legacy projects carry split
  `requirements/{ui,api}/build-spec.json` that are not schema-conformant.
  This spec defers unification to a later spec. Risk: the unified Build
  Spec emitted by the ACP pipeline may diverge enough from the legacy
  split artifacts that reconciliation cannot treat them as equivalent.
  Mitigation: the translator preserves the legacy files verbatim; the
  next factory run emits a new conformant Build Spec alongside them; the
  cockpit marks the legacy files as historical.

- **Adapter scaffold entry-point portability.** §5.2 step 3 invokes the
  adapter's declared scaffold entry point in a stagecraft-side Node
  subprocess. The `aim-vue-node` adapter uses Node 24; other adapters
  (e.g. `rust-axum`) need different runtimes. Stagecraft must gate
  scaffold execution by workspace policy (`scaffold.runtime` must be on
  an allowlist) and consider whether to shift this to an OPC-side
  execution (reuses the spec 110 "stagecraft orchestrates, OPC executes"
  invariant). Decision deferred to Phase 5 start.

- **Detection crate embedding in stagecraft.** §6.2 step 3 proposes a CLI
  invocation of the detection crate from Node. Alternative: build a
  stagecraft-owned Node-native detector that reads the same YAML schemas.
  The CLI approach preserves one source of truth and matches spec 105;
  the Node rewrite risks drift. Recommendation: CLI for MVP, revisit if
  latency is a problem.

- **`.factory/` vs. existing `factory/` confusion.** Spec 108 deletes the
  repo-root `factory/` directory. This spec introduces a **per-project**
  `.factory/` directory with one file (`pipeline-state.json`). The
  leading dot, the different location (inside a user project, not the
  platform repo), and the single file make collision unlikely; Phase 1
  tests will include a fixture that validates detection against a repo
  that contains neither.

- **Legacy projects without `.artifacts/raw/`.** cfs-womens-shelter has
  `.artifacts/raw/` with business documents. Not every legacy project
  will — some were produced from transcripts pasted into a prompt.
  Detection does not require `.artifacts/`; the cockpit's "Re-extract"
  action becomes a no-op when `.artifacts/raw/` is absent.

## 11. Non-Goals

- **Multi-repo / monorepo projects.** One project = one repo in this
  spec. Multi-repo coordination is deferred.
- **Local-only projects (no GitHub).** Create and Import both assume a
  GitHub remote. A "pure local" mode is plausible but not in scope.
- **Cross-adapter migration.** Moving a project from one adapter to
  another (e.g. `aim-vue-node` → `rust-axum`) is not addressed. The
  pipeline-state's `adapter` field is informational; migration would be
  a separate spec.
- **Factory run execution changes.** Runs continue to dispatch per spec
  110. This spec only adds lifecycle entry points, not runtime behaviour.

## 12. Glossary

- **ACP** — Adapter / Contract / Process, the three-layer model defined
  in `factory/README.md` and spec 074.
- **L0 / L1 / L2** — Detection levels: ScaffoldOnly / LegacyProduced /
  AcpProduced.
- **Legacy factory manifest** — `requirements/audit/factory-manifest.json`
  produced by `goa-software-factory` (5 stages).
- **ACP pipeline-state** — `.factory/pipeline-state.json` conformant to
  `pipeline-state.schema.yaml` (7 stages).
- **Scaffold** — the act of materialising an adapter's base template into
  a new empty project directory. Does not run the factory pipeline.
- **Produce** — the act of running the factory pipeline against a
  scaffolded project, advancing stage state and emitting artifacts.
