---
id: adapter-handoff
name: Adapter Handoff & Scaffolding
sequence: 6
inputs:
  - .factory/build-spec.yaml (complete Build Specification)
  - .factory/adapter-manifest.yaml (resolved adapter)
outputs:
  - Complete scaffolded application
  - .factory/pipeline-state.json (updated with scaffolding progress)
gate: scaffolding_gates + final_validation (from verification contract)
---

# Stage 6: Adapter Handoff & Scaffolding

The Build Specification is complete. Hand off to the adapter for code generation.

This stage is orchestrated by the process layer but executed by adapter-specific agents. The process layer:
- Controls sequencing (data → API → UI → configure → trim)
- Runs the verification harness after each feature
- Manages retries on failure
- Updates pipeline state

## Execution Sequence

### 6a. Initialize Project

1. Copy adapter scaffold to project root (or run adapter's setup commands)
2. Run `adapter.commands.install` to install dependencies
3. Verify project compiles with `adapter.commands.compile`

### 6b. Data Scaffolding

For each entity in `build_spec.data_model.entities`:

1. Invoke adapter's `data_scaffolder` agent with:
   - The entity definition from Build Spec
   - The adapter's `patterns.data.migration` pattern file
   - The adapter's directory conventions
2. Agent generates migration/DDL + type definitions
3. Verification harness confirms files exist

Update pipeline state: mark each entity as completed.

### 6c. API Scaffolding

For each operation in `build_spec.api.resources[].operations`:

1. Invoke adapter's `api_scaffolder` agent with:
   - ONE operation definition from Build Spec
   - The adapter's relevant pattern files (service, controller, route, test)
   - The adapter's directory conventions
   - The stack assignment (for dual variant)
2. Agent generates service + controller + route + test
3. Verification harness runs:
   - `adapter.commands.compile` — must pass
   - `adapter.commands.test` — must pass
4. If verification fails:
   - Feed error output back to agent
   - Retry (up to `retry.max_retries`)
   - If all retries fail: mark as failed, flag for human review, continue to next operation

Update pipeline state: mark each operation as completed/failed.

### 6d. UI Scaffolding

For each page in `build_spec.ui.pages`:

1. Invoke adapter's `ui_scaffolder` agent with:
   - ONE page definition from Build Spec
   - The adapter's page-type pattern file (e.g., `patterns.page_types.list`)
   - The adapter's ui patterns (view, state, route, test)
   - The adapter's directory conventions
2. Agent generates view + state + route config + test
3. Verification harness runs compile + test
4. Retry on failure (same policy as API)

Update pipeline state: mark each page as completed/failed.

### 6e. Configure

Invoke adapter's `configurer` agent with:
- Build Spec project identity
- Build Spec auth configuration
- Adapter manifest (for env var patterns)

Agent applies: project naming, environment files, auth wiring.

### 6f. Trim

Invoke adapter's `trimmer` agent with:
- List of generated files (from pipeline state)
- Adapter scaffold file list
- Build Spec variant

Agent removes: unused scaffold artifacts, template examples, irrelevant modules.

### 6g. Final Validation

Run all `final_validation` checks from the verification contract:
- Process checks: UC-to-code mapping, TC coverage, placeholder scan
- Adapter checks: full build, all tests, lint, type check, format, invariants

If all pass: mark pipeline as `completed`. Generate final manifest.
If any error-severity check fails: mark pipeline as `failed`. Report failures.
