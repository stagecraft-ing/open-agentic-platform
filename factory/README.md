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
