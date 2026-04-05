---
id: pre-flight
name: Pre-Flight Checks
sequence: 0
inputs:
  - business_artifacts (raw documents)
  - adapter_name (selected adapter)
outputs:
  - .elucid/pipeline-state.json (initialized)
  - .elucid/adapter-manifest.yaml (resolved copy)
gate: pre_flight (from verification contract)
---

# Pre-Flight Checks

Validate all inputs and confirm the adapter can fulfill the work before any pipeline stage runs.

## Steps

1. **Locate adapter** — Resolve the adapter by name. Read its `manifest.yaml`.
2. **Validate adapter manifest** — Confirm manifest conforms to adapter-manifest schema.
3. **Validate business artifacts** — Confirm at least one readable business document exists in the provided artifacts path.
4. **Initialize pipeline state** — Create `.elucid/pipeline-state.json` with status `running`, adapter identity, and all stages set to `pending`.
5. **Copy adapter manifest** — Write resolved manifest to `.elucid/adapter-manifest.yaml` for downstream stages.

## Capability Validation

Capability validation runs AFTER Stage 2 (when the variant and auth requirements are known), not here. Pre-flight only validates structural readiness.

## Failure Behavior

If any check fails, the pipeline does not start. Report the specific failure and exit.
