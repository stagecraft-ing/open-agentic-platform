# Prompt: Claude — runtime path review

You are assisting on **open-agentic-platform**. Execution truth is recorded in **`specs/.../execution/`** and code — use `.ai/` for analysis drafts only.

## Read first

1. `.ai/handoff/current.md`
2. `specs/032-opc-inspect-governance-wiring-mvp/spec.md` (inspect + git + governance requirements)
3. `specs/032-opc-inspect-governance-wiring-mvp/tasks.md` and `execution/changeset.md`
4. Source areas referenced in the handoff (desktop app, Tauri, MCP client packages as applicable)

## Your job

Map the **actual runtime path** for the current slice:

- Entrypoints → IPC / commands → sidecar / MCP → data returned to UI.
- **Dead paths**, feature flags that never connect, or **partial** integrations.
- **Execution/gating gaps**: what can fail silently vs explicit error/degraded states (per FR-004–FR-006 style requirements).

## Write outputs to

- `.ai/findings/runtime-path.md` (primary)
- Touch `.ai/findings/integration-risks.md` if cross-cutting

## Rules

- Every nontrivial claim should have **evidence** (file path, test name, or command).
- **Non-authoritative** — propose `execution/verification.md` commands for anything you could not run.
- **Update the baton** before commit; list promotion candidates for verification steps.
