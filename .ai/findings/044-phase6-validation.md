# 044 Phase 6 Validation (cursor)

Date: 2026-03-30
Scope: P5-002 required fix, P5-001 optional fix, and SC-001/SC-002/SC-005 end-to-end validation.

## Changes implemented

1. P5-002 fixed in `apps/desktop/src-tauri/src/commands/orchestrator.rs`
   - `orchestrate_manifest` now accepts `project_path: String`.
   - `RealGovernedExecutor.working_directory` is set from `project_path` (not process CWD).

2. P5-001 addressed in `apps/desktop/src-tauri/src/commands/orchestrator.rs`
   - Provider-registry dispatch now sends:
     - `prompt`: step-level prompt + execution requirements
     - `systemPrompt`: agent profile system prompt
   - This avoids embedding the system prompt inside user prompt for sidecar/provider runs.

3. End-to-end 3-step workflow validation in `crates/orchestrator/src/lib.rs`
   - Added test: `dispatch_manifest_e2e_three_step_workflow_validates_sc_001_002_005`.
   - Uses a real YAML manifest with:
     - `step-01-research` (`quick`)
     - `step-02-draft` (`investigate`)
     - `step-03-review` (`deep`)

## Success criteria evidence

- SC-001 (artifact-based passing): validated
  - Test asserts step 2 output contains step 1 output, and step 3 output contains step 2 output.
  - Data flow is filesystem artifact-based (`input_artifacts` paths), not conversation-carried.

- SC-002 (token measurement): validated
  - Each step returns `tokens_used` and summary records it.
  - Test computes total workflow tokens and checks against a single-context baseline threshold.

- SC-005 (effort-level differentiation): validated
  - Executor writes different output sizes for `quick` / `investigate` / `deep`.
  - Test asserts monotonic output-size growth across those effort levels.

## Verification commands

- `cargo test --manifest-path crates/orchestrator/Cargo.toml`
  - Result: 15 passed, 0 failed.
- `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`
  - Result: success.
