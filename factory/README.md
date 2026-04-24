# Factory

A generic, modular, technology-agnostic software factory framework. Factory separates the **process** of building software (requirements, design, specification) from the **implementation details** (frameworks, languages, patterns) by introducing a formal contract layer between them.

## Architecture

Factory is built on three layers:

```
┌───────────────────────────────────────────────────┐
│                 PROCESS LAYER                     │
│          (universal, tech-agnostic)               │
│                                                   │
│  Pipeline stages that transform business docs     │
│  into structured Build Specifications.            │
│  Never references any framework or language.      │
├───────────────────────────────────────────────────┤
│                CONTRACT LAYER                     │
│           (formal interface schemas)              │
│                                                   │
│  Build Specification — what to build              │
│  Adapter Manifest — what an adapter provides      │
│  Verification Contract — what must pass           │
│  Pipeline State — durable execution state         │
├───────────────────────────────────────────────────┤
│                 ADAPTER LAYER                     │
│          (pluggable, tech-specific)               │
│                                                   │
│  Each adapter implements one tech stack.          │
│  Contains: scaffold, patterns, agents,            │
│  build commands, validation rules.                │
│                                                   │
│  ┌─────────┐ ┌──────────┐ ┌─────────┐            │
│  │Vue+Node │ │Next.js+  │ │Rust+    │  ...       │
│  │+GoA     │ │Prisma    │ │Axum     │            │
│  └─────────┘ └──────────┘ └─────────┘            │
└───────────────────────────────────────────────────┘
```

### Process Layer (`process/`)

Universal pipeline stages that transform raw business documentation into structured, tech-agnostic specifications. The process layer:

- Runs requirement analysis, service design, and data modeling
- Produces a **Build Specification** (structured JSON/YAML) — not code
- Enforces cross-stage consistency and validation gates
- Manages pipeline state for durability and resumability
- Never imports, references, or assumes any specific technology

### Contract Layer (`contract/`)

Formal schemas that define the interface between process and implementation:

- **Build Specification** (`build-spec.schema.yaml`) — The factory's output: what resources exist, what operations are available, what pages display them, what rules govern them. Completely tech-free.
- **Adapter Manifest** (`adapter-manifest.schema.yaml`) — What an adapter declares: its stack, capabilities, supported auth methods, build commands, directory conventions, pattern file locations.
- **Verification Contract** (`verification.schema.yaml`) — What must pass at each gate: capability checks, build/test/lint gates, cross-stage consistency checks. Separates factory-owned checks from adapter-owned checks.
- **Pipeline State** (`pipeline-state.schema.yaml`) — Durable execution state for resumability: stage progress, feature scaffolding status, retry counts, artifact inventory.

### Adapter Layer (`adapters/`)

Pluggable implementations, one per tech stack. Each adapter is self-contained:

```
adapters/{name}/
  manifest.yaml          — Capability declarations
  scaffold/              — Base project template/skeleton
  agents/                — Focused agent prompts (<2K tokens each)
  patterns/              — Code generation patterns (concrete examples)
    api/                 — Backend patterns (service, controller, route, test)
    ui/                  — Frontend patterns (view, state, route, test)
    data/                — Data layer patterns (query, migration)
  validation/            — Architecture rules, invariants
```

Adding a new tech stack = creating a new adapter. The process layer and contract schemas never change.

## Key Principles

1. **The factory never generates code.** It produces structured specifications. Adapters generate code.
2. **Each agent has bounded context.** No agent holds the entire pipeline in memory. Each reads one spec + one pattern + generates one artifact.
3. **Validation is automated, not self-assessed.** The adapter declares build/test/lint commands. A verification harness runs them. No agent validates its own output.
4. **Build-test-fix loops, not one-shot generation.** Each feature is scaffolded, verified, and retried if needed — not batch-generated and hoped to work.
5. **Durable state enables resumability.** Pipeline state is persisted after every successful step. Crash recovery reads state and continues from last checkpoint.
6. **Adapters are self-describing.** The manifest declares capabilities. Pre-flight checks verify the adapter can fulfill the Build Specification before any work begins.

## Directory Structure

```
factory/
├── process/
│   ├── stages/           7 stage definitions (pre-flight through adapter handoff)
│   └── agents/           7 agent prompts (pipeline orchestrator + 5 stage agents + scaffolding orchestrator)
├── contract/
│   ├── schemas/          4 YAML schemas + 5 JSON stage-output schemas
│   └── examples/         Build Spec example, adapter manifest example, stage output examples
├── adapters/
│   ├── aim-vue-node/     Express 5 + Vue 3 (manifest, agents, patterns, invariants)
│   ├── next-prisma/      Next.js 15 + Prisma 5
│   ├── rust-axum/        Axum + HTMX
│   └── encore-react/     Encore.ts + React
└── docs/                 Architecture overview, adapter agent examples
```

> **Verification harness:** implemented in `crates/orchestrator/src/verify.rs`
> (spec 075). Runs adapter-declared commands (compile, test, lint) after each
> scaffolding step with retry on failure.

## Platform Integration

Specs 108 and 112 move Factory from in-tree files into a first-class platform feature. The three-layer architecture above is unchanged; physical storage and execution responsibilities are split:

- **Contract schemas** — canonical compile-time home at `crates/factory-contracts/schemas/` (spec 112 §3.2). Rust consumers embed `SCHEMA_VERSION` as a compile-time const. Stagecraft mirrors the schemas per-org into the `factory_contracts` table for runtime policy lookups (spec 108 §3). The in-tree `contract/schemas/` tree here is superseded by spec 108 Phase 2.
- **Adapters and processes** — persisted per-org in `factory_adapters` and `factory_processes`, synced from upstream by stagecraft (spec 108 §5). The in-tree `adapters/` and `process/` trees here are working areas, not the post-108 governance source of truth.

### Execution boundary

Spec 112 §5.4 makes the stagecraft ↔ OPC split explicit:

- **Stagecraft is the birth tier** — template cache, adapter scaffold execution, L0 pipeline-state seed, server-side artifact extraction, GitHub repo creation, initial commit + push. After push, stagecraft drops its working copy.
- **OPC is the life tier** — everything after commit #1: cockpit actions (Run Stage N, Reconcile, Re-extract), ACP engine runs, all subsequent writes to `.factory/pipeline-state.json`.

**Invariant: birth on stagecraft, life on OPC.** Post-birth execution never moves back to stagecraft.

### Create path runtime (MVP)

Stagecraft-side scaffold execution is Node-24-only, shaped after the `template` repo's `scripts/setup-*.ts` (spec 112 §5.2 step 3). Adapters declaring a `scaffold.runtime` other than `node-24` are not Create-eligible via the web UI in the MVP — their outputs reach the platform through Import of fully-executed repos. A follow-up spec will dispatch non-Node scaffolds to OPC over the spec 110 envelope without disturbing the post-birth invariant.

### Legacy prompt files are retired

Per spec 112 §11 Non-Goals and Phase 9: `prestart-prompt.txt`, `start-prompt.txt`, and `reconciliation-prompt.txt` are no longer part of the factory execution surface. The ACP 7-stage engine is the sole execution target; the `factory/` contract here is its specification. Newly-created projects (spec 112 §5) do not emit these files. Imported legacy projects may carry copies as historical artefacts — they are not read by any adapter, process, or engine in this tree. Removing the files from the upstream `template` repo's `scripts/setup-*.ts` is tracked there, not here — OAP does not own that repo.

## `requirements/` vs. knowledge objects

Every GoA software-factory-produced repo lands two parallel trees that OAP must classify distinctly:

| Repo tree | Origin | OAP storage | Lifecycle | Bound to |
|-----------|--------|-------------|-----------|----------|
| `.artifacts/raw/` | Business source documents uploaded by the caller (BRDs, transcripts, community data, DOCX/PDF/XLSX/PPTX/ZIP) | `knowledge_objects` row per file, workspace object store, state machine `imported → extracting → extracted → classified → available` | Lives beyond any single pipeline run. A second project can bind the same object via `document_bindings`. | Workspace, then project via `document_bindings` |
| `.artifacts/extracted/` | Plain-text derivative of `.artifacts/raw/` produced by the artifact-extract crate | `knowledge_objects.extraction_output.extractedStorageKey` on the same row — **not** a separate row | Regeneratable from raw at any time. | Same object as the raw input |
| `requirements/` | Every numbered factory stage writes into a subdirectory: `requirements/` (Stage 1 BRD + IO schema), `requirements/services/` (Stage 2), `requirements/database/` (Stage 3), `requirements/api/` (Stage 4 build-spec), `requirements/ui/` (Stage 5 build-spec), `requirements/audit/` (the manifest + per-stage validation logs), `requirements/client/` (optional client-documentation stage) | One `factory_artifacts` row per file (spec 082 Phase 3), keyed on `pipeline_id` + `stage_id` | Tied to one pipeline execution. Re-running a stage writes a new pipeline row; the old artifacts remain for audit. | Pipeline run, not directly to the project |

### Decision: `requirements/` is pipeline output, not knowledge input

The task that produced this section (April 2026) asked whether `requirements/` should be:

1. **Governed as factory pipeline output** — bound to a pipeline run, mirrored into `factory_artifacts`.
2. **Lifted into Build Spec resource sections** — re-encoded into `build-spec.schema.yaml`'s `api.resources[]`, `ui.pages[]`, `data_model.entities[]`, etc.

Both of these are true simultaneously, but they address different boundaries and both belong on the pipeline-output side of the line — not the knowledge-input side.

**`requirements/` is factory pipeline output.** Every file under `requirements/` is produced by a factory stage. `requirements/audit/factory-manifest.json` is authoritative: it itemises `outputDirectory` + `artifacts[]` per stage (stage 1 → `requirements/`, stage 2 → `requirements/services/`, stage 3 → `requirements/database/`, stage 4 → `requirements/api/`, stage 5 → `requirements/ui/`, plus `clientDocumentation` → `requirements/client/`, plus `audit` → `requirements/audit/`). Nothing under `requirements/` is uploaded by a human; the factory wrote all of it. That is the exact contract of a pipeline artifact.

**`requirements/ui/build-spec.json` and `requirements/api/build-spec.json` are not the Build Spec.** They are GoA-factory-specific stage-output shapes — keyed on `schemaVersion: "1.0"`, `stage: 4|5`, `service`, `framework`, `apiStacks` — that pre-date the OAP contract. The canonical `factory/contract/schemas/build-spec.schema.yaml` is the post-ACP shape: tech-agnostic, single document, covers all eight resource sections (`project`, `auth`, `data_model`, `business_rules`, `api`, `ui`, `integrations`, `notifications`, `audit`, `security`, `health_checks`, `error_handling`). The legacy per-stage build-spec files map into it but are not themselves conformant.

**What to do with `requirements/` on Import:**

- **Do not register any `requirements/` file as a `knowledge_objects` row.** The Import flow (`platform/services/stagecraft/api/projects/importArtifacts.ts`) walks only `.artifacts/raw/` for that reason.
- **Do mirror `requirements/` into `factory_artifacts`** keyed on the translator-synthesised `pipeline_id`. The legacy-translator already constructs per-stage entries from `requirements/audit/factory-manifest.json` (`translator.ts` → `translateLegacyManifest`). Spec 082 Phase 3 defines the `factory_artifacts` schema with `pipeline_id`, `stage_id`, `artifact_type`, `content_hash`, `storage_path`, `producer_agent`. A follow-up on the Import path should upload each `requirements/**` file into the workspace bucket and land a `factory_artifacts` row per file — this is orthogonal to the `knowledge_objects` work and belongs under spec 082, not spec 087.
- **Do not lift `requirements/{ui,api}/build-spec.json` into the Build Spec schema automatically on Import.** Those legacy documents are evidence of what the factory produced, not a re-usable OAP contract. If a user wants an OAP-conformant Build Spec for an imported project, running the ACP engine against the imported repo is the supported path — stages re-derive the Build Spec from the same bound knowledge objects.

### Why the boundary matters

Getting this wrong in either direction is costly:

- **If `requirements/*.json` is written as knowledge objects**, two projects binding "the same BRD" actually bind two frozen stage-1 outputs from two different pipeline runs. The lifecycle lies — `state=extracted` is meaningless for a file that was never raw — and re-running the pipeline can't update the knowledge set because the schema has no concept of pipeline membership.
- **If `.artifacts/raw/*.docx` is written as pipeline artifacts**, the business source document is pinned to one pipeline run and is unreachable to any second pipeline executed against the same project. The same BRD bound to a redesign effort would need to be re-uploaded and re-hashed, breaking provenance.

The split exists because `.artifacts/raw/` is **evidence the human provided** and `requirements/` is **evidence the factory produced**. They have different audit trails, different re-runnability semantics, and different cross-project sharing rules. Keep them in separate tables.

### Non-goals of this decision

- **Importing `requirements/` into `factory_artifacts` is not implemented here.** The Import endpoint registers raw artifacts only. Spec 082 Phase 3 tracks the artifact-registry Import wiring; until that lands, the translator emits `pipeline_state.stages[].artifacts[].path` entries pointing into `requirements/` and the files themselves remain readable directly from the imported repo.
- **Backfilling a conformant Build Spec from legacy `requirements/{api,ui}/build-spec.json` is not in scope.** If that ever becomes useful (e.g. for side-by-side diffs during migration), it is a dedicated transformer — not a general rule.
