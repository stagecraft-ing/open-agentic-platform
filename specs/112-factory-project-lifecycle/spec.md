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

### 1.0 Provenance and end-to-end model

Two upstream repositories produce every project we Import:

- `GovAlta-Pronghorn/goa-software-factory` — the 5-stage AI factory (mirror
  of `GovAlta-EMU/the_factory`).
- `GovAlta-Pronghorn/template` — the scaffold template the factory writes
  into.

The same two upstreams also feed spec 108's **factory sync** that
populates the org's `factory_adapters` / `factory_contracts` /
`factory_processes` rows. Import and ACP are therefore not parallel
pipelines — they are the same upstream projected twice: once as a
fully-produced Stage-5 project (the Import target, e.g.
`GovAlta-Pronghorn/cfs-emergency-family-violence-services-funding-request-portal`),
once as the Adapters/Contracts/Processes (the modular flow the imported
project is re-driven through after import). The lifecycle terminus is a
deployd-api deployment via a future ACP "deploy" stage — out of scope for
this spec (§11).

### 1.1 What none of the prior specs answer

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
   installation); clone server-side, detect, translate legacy → ACP,
   register; OPC claims it on local open. **Scope bound (MVP):** Import
   accepts only fully-executed legacy projects — `LegacyProduced` with
   `legacy_complete == true` (all 5 `goa-software-factory` stages marked
   complete). ACP-native (L2) detection is retained for forward
   compatibility but Import does not exercise it as a primary branch in
   this spec; no upstream ACP producers exist in the wild yet (the only
   path to an L2 repo today is re-cloning a project Create just produced
   — see §11). Scaffold-only, in-progress legacy, and non-factory repos
   are rejected (§6.2).

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
    pub legacy_complete: Option<bool>,           // L1 only — true iff all 5 legacy stages marked complete
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

For `LegacyProduced`, the crate also sets `legacy_complete = Some(true)`
iff every stage key in `factory-manifest.json` reports a terminal
completion status (schema: `completed` with a non-null
`completedAt`); otherwise `Some(false)`. Detection reports the state
truthfully; Import enforces the policy gate (§6.2).

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
  - **Run next ACP stage** *(headline for imported L1 projects)* —
    advance pipeline from the current cursor. For a freshly-imported
    project, this is the first action the cockpit invites; it begins
    the modular ACP flow from the earliest non-completed translated
    stage and progresses toward the deployd-api terminus (§11).
  - *Reconcile* — spec-088-style drift reconciliation, incremental re-run
    from earliest dirty stage.
  - *Run Stage N* — explicit per-stage advance (power-user form of
    Run-next, retained for diagnosis and out-of-order replay).
  - *Re-extract artifacts* — run the prestart extractor on `.artifacts/raw/`
    (see §4.3); no-op when raw inputs are absent (common for imported
    legacy projects — see §10 risk).
  - *Register with workspace* — if the project is not yet bound to a
    stagecraft `projects` row, create the binding and dual-write. Edge
    case — Import already binds; relevant only for OPC Open of a
    never-imported repo.

### 4.3 Artifact extraction as a birth-time step

The existing `.artifacts/extract_artifacts.py` script (raw → extracted)
is replaced by a Rust binary `crates/factory-artifacts/src/bin/extract.rs`
(matching the scripts-to-binaries direction of spec 105) and invoked
**at project birth on stagecraft** (§5.2), not as an OPC pre-flight step.
Rationale: the extractor needs only the uploaded raw documents and
produces deterministic output; running it at Create time means the
repo's first commit already contains `.artifacts/extracted/`, so the
cockpit and the ACP engine never see a repo in a "raw-but-not-extracted"
intermediate state.

Storage split:

- **Raw uploads** live in stagecraft's workspace-scoped bucket
  (audit-durable, re-runnable). Binary originals do not enter git.
- **Extracted outputs** live in the repo under `.artifacts/extracted/`
  and travel with the code. An `.artifacts/extracted/manifest.json`
  records the bucket-object-id → extracted-file mapping so Re-extract
  (§4.2) can re-run deterministically from bucket state.

The cockpit's **Re-extract** action runs the same binary locally on OPC
against any raw content added post-birth (e.g. a new business doc
dropped into a local `.artifacts/raw/` the user creates by hand) and
commits the updated `.artifacts/extracted/` entries.

Legacy prompt files (`prestart-prompt.txt`, `start-prompt.txt`,
`reconciliation-prompt.txt`) are **not generated** by the Create path
and are **not consumed** by the ACP engine. The `factory/` ACP
specification is the sole execution target (§11 Non-Goals). Imported
legacy projects retain their copies as historical artifacts; the engine
ignores them.

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
- **Seed inputs (optional)** — multifile upload of business documents
  (individual files, or a `.artifacts/raw/`-shaped directory archive).
  Raw uploads land in the workspace-scoped bucket; stagecraft runs the
  extractor (§4.3) server-side; extracted outputs are written into
  `.artifacts/extracted/` in the generated tree and committed in commit
  #1. Binary originals remain in the bucket, not in git.

### 5.2 Backend — `api/projects/create.ts`

Flow:

1. Validate form against `build-spec.schema.yaml` `project` + `auth`
   sub-schemas (workspace-scoped policy bundle enforces org rules).
2. Resolve the adapter's scaffold source (e.g. a git ref in the `template`
   repo, pinned by the `factory_adapters.source_sha` column) and clone
   into a per-request temp dir.
3. Run the adapter's scaffold entry point with the chosen variant and
   module profile. **Runtime scope (MVP):** stagecraft executes only
   Node-24 entry points shaped like the `template` repo's
   `scripts/setup-app.ts` / `scripts/setup-dual-app.ts` (see §5.3 for
   the concrete absorbed surface). Adapters declaring a
   `scaffold.runtime` other than `node-24` are not Create-eligible via
   the web UI; a later spec may lift this by dispatching non-Node-24
   scaffolds to OPC over the spec 110 envelope.
4. **Seed ACP state**: write `.factory/pipeline-state.json` at L0 shape
   per §3.5, with `pipeline.adapter` populated from the `factory_adapters`
   row. Stagecraft is the sole author of this file; OPC never writes
   commit #1.
5. **Extract seed inputs (if any)**: store raw uploads in the
   workspace-scoped bucket, invoke `factory-artifacts extract` against
   them server-side, and write the extractor's output under
   `.artifacts/extracted/` in the generated tree (plus an
   `.artifacts/extracted/manifest.json` mapping bucket object ids to
   extracted files). Raw binaries are **not** written into the repo.
6. Create the GitHub repo via the org's App installation and push the
   generated tree with a single initial commit. Commit author is the
   stagecraft service identity; co-author is the user.
7. Insert rows into `projects` and `project_repos` (spec 108) with
   `factory_adapter_id` pointing at the adapter used. Emit a
   `project.created` audit event.
8. Return `{ project_id, repo_url, clone_url }` for the UI to link to.

### 5.3 What is absorbed from template-distributor

Stagecraft's server-side scaffold scope is **exactly** the six operations
that `template-distributor/src/server.ts` performs today. Line references
point at the canonical implementation at the time this spec was drafted;
the absorbing PR rewrites these in stagecraft idioms, it does not import
the code.

| # | Template-distributor capability | Reference | Lands in stagecraft as |
|---|---|---|---|
| 1 | Template cache clone + `npm install` + upstream-SHA refresh | `ensureTemplateCache`, server.ts:329-375 | `api/projects/scaffold/templateCache.ts` (per-workspace cache, keyed on `factory_adapters.source_sha`) |
| 2 | Profile prebuilds (minimal/public/internal/dual) | `ensurePrebuilts`, server.ts:377-446 | `api/projects/scaffold/prebuilds.ts` (warm-on-startup; declared profile set comes from the adapter manifest §8) |
| 3 | Per-request scaffold: copy prebuilt + run `setup-*.ts` + `add-module.ts` for extras | server.ts:613-760 | `api/projects/scaffold/runAdapterScaffold.ts` (Node-24 subprocess in a per-request temp dir, concurrency-bounded) |
| 4 | Create GitHub repo + grant team admin (with retry) | `createRepo` + `teams.addOrUpdateRepoPermissionsInOrg`, server.ts:865-897 | `api/projects/scaffold/githubRepoCreate.ts` |
| 5 | Initial git commit + authed push to `main` | server.ts:899-927 | `api/projects/scaffold/githubPushInitial.ts` — uses the existing App installation token via `authedGitUrl` pattern (server.ts:285-300) |
| 6 | Post-push cleanup of server-side working tree | server.ts:929-931 | Inlined in the scaffold handler; temp dir is dropped after successful push |

**Net additions** (stagecraft-owned, not in template-distributor):

- L0 `.factory/pipeline-state.json` seed written into the tree before
  commit #1 (§5.2 step 4).
- Server-side artifact extraction from bucket uploads into
  `.artifacts/extracted/` (§5.2 step 5).
- `scaffold_jobs` table replacing the Express app's in-memory map
  (concurrency-safe, multi-tenant, audit-traceable).
- `oap://` deep-link on the success response (§5.4).

**Net drops:**

- The standalone web UI (`template-distributor/public/*`) — stagecraft
  `/app/projects/new` owns the UX.
- OAuth login / session middleware (template-distributor server.ts:90-512) —
  stagecraft already owns identity.
- ZIP download (`/api/download-project`, server.ts:789-825) — stagecraft
  does not offer a "download tree" path; the GitHub repo is the handoff.

OAP does not import code from `template-distributor` — the absorbing
PR rewrites the six operations above in stagecraft idioms. The
external repo lives outside this project's control; §9 Phase 8
describes what OAP retires on its side, not the upstream repo's fate.

### 5.4 Stagecraft–OPC boundary

This spec establishes a crisp temporal boundary between the two planes:

- **Stagecraft owns repo birth.** Template clone/cache, profile prebuild,
  module composition, Node-24 adapter scaffold, L0 pipeline-state seed,
  server-side artifact extraction, GitHub repo creation with team
  grants, initial commit + authed push. Post-push, stagecraft deletes
  its working copy.
- **OPC owns everything else.** Open, claim, cockpit actions (Run Stage
  N, Reconcile, Re-extract, Register-with-workspace), all ACP engine
  runs, all writes to `.factory/pipeline-state.json` after commit #1.
  Dispatch from stagecraft uses the spec 110 envelope
  (`factory.run.request` / `factory.run.ack`).

Consequence: the **GitHub repo is the source-of-truth handoff**. After
push, stagecraft returns `{ project_id, repo_url, clone_url,
oap_deep_link }`. The `oap_deep_link` is an
`oap://project/open?url=<clone_url>&project_id=<id>` URI that, when
clicked on a machine with OPC installed, clones the repo locally and
activates the Factory Cockpit (§4). The success page renders (a) the
GitHub URL, (b) the deep link, and (c) an "install OPC" affordance
visible when the user agent is not OPC and the deep link fails to
resolve.

No ongoing lifecycle operation executes on stagecraft. "Birth on
stagecraft, life on OPC" is the invariant; a future spec may dispatch
non-Node-24 births to OPC but will not move post-birth execution onto
stagecraft.

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
   - **NotFactory**: reject with "not a factory project". Import is for
     factory-produced repos only. Adopting an unrelated repo as a
     factory project belongs to a future "Adopt" spec with its own UX
     and policy gates.
   - **ScaffoldOnly**: reject. Scaffold-only is a transient birth state
     owned by Create (§5); importing an unrun scaffold has nothing to
     translate or register meaningfully. The user should Create fresh
     or run the legacy pipeline upstream to completion before
     importing.
   - **LegacyProduced**: enforce `legacy_complete == true` (all 5
     `goa-software-factory` stages marked complete in
     `factory-manifest.json`). If incomplete, reject with a message
     identifying which stages are incomplete and instructing the user
     to finish the legacy run upstream before re-importing. If
     complete, run `translateLegacyManifest` (§3.4), preview the
     translated pipeline-state to the user, and on confirm open a PR
     against the source repo adding `.factory/pipeline-state.json`
     alongside the legacy manifest files (which stay — they are never
     deleted by this flow).
   - **AcpProduced**: *(forward-compat path; not the primary Import shape
     — see §11.)* validate pipeline-state schema version, confirm adapter
     binding, register without translation.
5. Insert `projects` / `project_repos` rows. Emit `project.imported`
   audit event including the detection level and (for L1) the translator
   version.
6. Return `{ project_id, detection_level, deep_link }` where `deep_link`
   is an `oap://` URI the user can click to hand off to OPC (which clones
   locally and surfaces the Factory Cockpit).

### 6.3 Open-in-OPC handoff

Once an Import succeeds, the stagecraft project menu surfaces an
**Open in OPC** action. Clicking it does not move data over the wire —
it triggers a resolution on OPC against state stagecraft already owns.
The same handoff is what Create's deep link (§5.4) drives; Import simply
reuses it once the imported project has a `projects` row and a
translated `.factory/pipeline-state.json` in the source repo.

1. **Deep link.** Stagecraft emits
   `oap://project/open?project_id=<id>&workspace_id=<ws>&clone_url=<url>`.
   The success page renders this link plus an "install OPC" affordance
   for users without OPC installed.

2. **Local clone.** OPC, on activation, clones `clone_url` locally if
   not already present. The cloned repo carries
   `.factory/pipeline-state.json` written by the Import PR (§6.2 step 4).

3. **Bundle resolution.** OPC binds the local checkout to the workspace
   via the existing duplex channel (spec 087) and resolves four things —
   none of which travel on the wire as a new payload; they are reads
   against state stagecraft already maintains:

   - **Adapter** — the `factory_adapters` row referenced by
     `projects.factory_adapter_id` (set by Import per §6.2 step 5 from
     the translator's adapter resolution, or by Create per §5.2 step 7).
   - **Contracts** — the org's `factory_contracts` rows synced via spec
     108. OPC pulls them through the existing catalog envelope; no
     per-project mirror.
   - **Processes** — the org's `factory_processes` rows, same path.
   - **Agents** — the workspace-scoped agent catalog from spec 111. The
     set of agents bound to this project is the workspace catalog
     filtered by the adapter's declared agent compatibility. Per-project
     agent overrides are out of scope here; a future spec may introduce
     them.

4. **Cockpit activation.** OPC opens Factory Cockpit (§4.2) with the
   L1-translated `pipeline-state.json` and the resolved bundle. The
   cockpit's headline action for a freshly-imported project is **Run
   next ACP stage** (§4.2) — the modular ACP flow begins from the
   earliest non-completed translated stage.

5. **First run dispatch.** When the user clicks Run next ACP stage, the
   cockpit dispatches via the existing spec 110 `factory.run.request`
   envelope. Nothing on the wire changes — the bundle is a resolution
   on the OPC side, not a new envelope payload. Knowledge bundles and
   business docs continue to materialise per spec 110 §2.3.

The bundle is therefore deliberately small on the wire (deep link
identifiers only) and large in OPC (adapter + contracts + processes +
agents pulled from the catalog the workspace already syncs). The
authority invariant from spec 087 holds: the GitHub repo is the
source-of-truth handoff for project content; stagecraft is the
source-of-truth handoff for governance state; OPC composes the two and
runs.

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
- **Sequencing:** depends on spec 108 Phase 2 (schema landed,
  `factory_adapters` table populated by a prior sync run). The
  `projects` row references `factory_adapter_id`, and the adapter
  scaffold source is resolved by joining on this table. Phase 5 must
  not merge before 108 Phase 2 has shipped.
- Land `api/projects/create.ts`, `api/projects/scaffold/*`, and the
  `/app/projects/new` route.
- Absorb the six template-distributor operations listed in §5.3.
- Exit criteria: creating a new project via the web UI produces a
  GitHub repo whose **commit #1** contains (a)
  `.factory/pipeline-state.json` at L0 shape with adapter identity
  resolved from `factory_adapters`, (b) `.artifacts/extracted/`
  populated from server-side extraction when the user supplied seed
  inputs (plus the `manifest.json` bucket mapping), and (c) **no**
  legacy prompt files (`prestart-prompt.txt`, `start-prompt.txt`,
  `reconciliation-prompt.txt`). The API response includes
  `{ project_id, repo_url, clone_url, oap_deep_link }` and the
  `projects` row references the correct `factory_adapter_id`. Raw
  uploads are retrievable from the workspace bucket.

**Phase 6 — Stagecraft Import.**
- Land `api/projects/import.ts` and the `/app/projects/import` route.
- Exit criteria: importing cfs-womens-shelter (fully executed — all 5
  legacy stages marked complete) via the web UI produces a `projects`
  row with `detection_level = "legacy_produced"` and a PR opened
  against the cfs repo adding `.factory/pipeline-state.json`. Importing
  an in-progress legacy project (any stage incomplete) is rejected at
  step 4 with an actionable error naming the incomplete stages.
  Importing a scaffold-only or non-factory repo is rejected. Importing
  an L2 AcpProduced project registers without a PR.

**Phase 7 — Workspace sync and OPC project list.**
- Add `project.catalog.upsert` envelope variant; reuse the spec 111 sync
  pattern.
- Add a "Projects" panel in OPC showing workspace projects with local
  clone state.
- Exit criteria: creating or importing in stagecraft updates a connected
  OPC's project list without a restart.

**Phase 8 — template-distributor retirement (OAP-side).**
- Remove remaining references to `template-distributor` from this
  repo's docs, commands, and agent context files.
- Stop invoking or linking to the external service from stagecraft,
  OPC, or any pipeline. The absorbed scaffold path (§5.3) is the only
  in-platform way to reach the same outcome.
- Exit criteria: no repo-level or doc-level references to
  template-distributor remain in OAP; the "Create Project" path is
  accessible only via stagecraft. The fate of the external GitHub
  repo is not governed by this spec — OAP does not own or control it.

**Phase 9 — Legacy prompt-file retirement.**
- Confirm (already enforced by Phase 5 exit criteria) that
  `prestart-prompt.txt`, `start-prompt.txt`, and
  `reconciliation-prompt.txt` are absent from newly-created project
  trees.
- Update the `template` repo's `scripts/setup-app.ts` /
  `scripts/setup-dual-app.ts` to stop emitting these three files.
- Imported legacy projects retain their copies as historical artifacts;
  the ACP engine does not read them. The `factory/` ACP specification
  is the sole execution target.
- Exit criteria: the `template` repo's setup scripts contain no
  reference to the three prompt files, and a fresh scaffold (whether
  invoked from stagecraft or directly) produces a tree without them.

## 10. Risks and Open Questions

- **Build Spec unification** (§3.4). Legacy projects carry split
  `requirements/{ui,api}/build-spec.json` that are not schema-conformant.
  This spec defers unification to a later spec. Risk: the unified Build
  Spec emitted by the ACP pipeline may diverge enough from the legacy
  split artifacts that reconciliation cannot treat them as equivalent.
  Mitigation: the translator preserves the legacy files verbatim; the
  next factory run emits a new conformant Build Spec alongside them; the
  cockpit marks the legacy files as historical.

- **Adapter scaffold entry-point portability.** *Resolved.*
  Stagecraft-side scaffold is Node-24-only and uses the `template`
  repo's `setup-*.ts` shape (§5.2 step 3, §5.3, §5.4). Adapters
  declaring any other `scaffold.runtime` are not Create-eligible via
  the web UI. Non-Node-24 adapter outputs can still reach the platform
  via Import of a fully-executed repo (§6.2). A future spec may lift
  this bound by dispatching non-Node-24 scaffolds to OPC through the
  spec 110 envelope without disturbing the post-birth invariant.

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

- **Legacy projects without `.artifacts/raw/`.** Not every legacy
  project has raw inputs on disk — some were produced from transcripts
  pasted into a prompt. Import accepts this: detection does not require
  `.artifacts/`, and the cockpit's "Re-extract" action is a no-op when
  `.artifacts/raw/` is absent. Imported projects are historical; new
  extraction is only relevant after the user adds raw content post-
  import.

- **Orphan-repo recovery on partial Create failure.** If
  `githubRepoCreate` succeeds but `githubPushInitial` fails, an empty
  repo is left in the target org. The `scaffold_jobs` row captures the
  partial state; stagecraft must implement an explicit policy —
  preferred: automatic retry of the push (bounded), then on terminal
  failure either delete the orphan repo or mark it `orphaned` with a
  reclaim action in the admin UI. The Express predecessor leaves
  orphans silently; this regression must be closed in Phase 5.

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
- **Partial legacy imports.** Projects with an incomplete
  `factory-manifest.json` (any of the 5 stages not marked complete)
  are not importable. Users must finish the legacy run upstream in
  `goa-software-factory` before importing. Rationale: translation of a
  partial manifest produces a pipeline-state with holes the ACP engine
  cannot reconcile from, and the platform has no interest in resuming
  legacy execution.
- **Single-prompt factory execution.** The legacy
  `prestart-prompt.txt` / `start-prompt.txt` / `reconciliation-prompt.txt`
  flow is not supported on any created or imported project. All
  factory runs use the ACP 7-stage engine via the cockpit (§4.2) and
  the spec 110 envelope. The `factory/` ACP specification is the sole
  execution target; legacy prompt files, if present in imported repos,
  are historical artifacts only.
- **Adopt-unrelated-repo via Import.** Import accepts factory-produced
  repos only (`LegacyProduced` with `legacy_complete == true`, or
  `AcpProduced`). `NotFactory` and `ScaffoldOnly` are rejected.
  Adopting an unrelated repo as a factory project belongs to a future
  "Adopt" spec with its own UX and policy gates.

- **Wide L2 Import.** Detection recognises `AcpProduced` (L2) and §6.2
  registers it without translation, but L2 is not a primary Import
  shape in this spec because no upstream ACP producers exist yet. The
  only current path to an L2 repo is re-cloning a project Create just
  produced — a corner case, not a use case. A future spec will widen
  L2 Import once ACP producers are in the wild (e.g. organisations
  publishing ACP-native templates, or downstream forks of projects
  Create produced).

- **Deployd-api terminal stage.** The lifecycle described here begins
  at Import and ends when the cockpit hands the user a live ACP run
  loop (§4.2, §6.3). The terminal **deploy** stage that pushes a
  promoted build to `deployd-api-rs` infrastructure is referenced by
  §1.0 as the lifecycle terminus but is not specified here — it
  belongs in a follow-up spec that defines (a) the `deploy` ACP stage
  identifier and contract, (b) the deployd-api dispatch envelope, and
  (c) the promotion gate that decides when an imported project is
  deploy-eligible.

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
