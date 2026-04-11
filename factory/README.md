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
