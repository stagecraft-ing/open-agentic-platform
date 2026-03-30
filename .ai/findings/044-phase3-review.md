# 044 Phase 3 Review — Step System Prompt Builder (FR-004)

**Reviewer:** claude
**Date:** 2026-03-30
**Scope:** `crates/orchestrator/src/lib.rs` — `build_step_system_prompt()` vs `specs/044-multi-agent-orchestration/spec.md` FR-004
**Verdict:** FR-004 is satisfied. The prompt builder is spec-faithful and ready for integration.

## FR-004 spec text (verbatim)

> When an agent step is invoked, its system prompt includes the absolute paths of its input artifacts and a directive to read them via the filesystem rather than expecting content in the conversation.

## Checklist

| Requirement | Satisfied? | Evidence |
|-------------|-----------|----------|
| System prompt includes absolute input artifact paths | **Yes** | `resolve_input_paths()` produces absolute paths; each is listed with a `- ` prefix under "Input artifact paths (absolute):" |
| Directive to read from filesystem, not conversation | **Yes** | `"You must read any needed context from these filesystem paths instead of expecting their contents in the conversation."` |
| Effort level directive present | **Yes** | Effort text matches FR-005 token budgets verbatim (`< 2k`, `< 10k`, `no hard token cap`) |
| No-input steps handled | **Yes** | Falls back to `"This step has no upstream artifact dependencies."` |
| Step instruction included | **Yes** | `step.instruction` is included under "Step instruction:" header |

## Architecture note — prompt structure

The spec's architecture section (§ "Build agent prompt") lists four elements for the agent prompt:

| Spec element | Present in `build_step_system_prompt`? |
|-------------|---------------------------------------|
| Step instruction | **Yes** — lines 159–161 |
| Input artifact paths (absolute) | **Yes** — lines 163–172 |
| Output artifact path convention | **No** — see P3-001 |
| Effort level directive | **Yes** — lines 177–179 |

## Spec-faithfulness of effort text

| EffortLevel | Spec text (FR-005) | Prompt text | Match? |
|-------------|-------------------|-------------|--------|
| Quick | "single-pass, < 2k token output per step, no sub-agent calls" | "quick — single-pass, very concise (< 2k tokens)" | **Partial** — "no sub-agent calls" omitted |
| Investigate | "iterative with tool use, up to 10k token output per step" | "investigate — thorough analysis with tools (< 10k tokens)" | **Yes** — semantically equivalent |
| Deep | "unrestricted depth, agents may spawn sub-workflows, no token cap" | "deep — exhaustive exploration, unrestricted depth (no hard token cap)" | **Yes** — semantically equivalent |

## Tests: 11/11 pass

| Test | New? | Covers |
|------|------|--------|
| `build_step_system_prompt_includes_absolute_paths_and_effort` | **P3** | FR-004: absolute path appears in prompt, effort directive present, filesystem-read instruction present, external absolute path passed through |
| (10 prior tests) | P1+P2 | Unchanged, still passing |

## Findings

### P3-001: Output artifact path convention not injected (LOW)

**File:** `lib.rs:140–182`
The spec's architecture section says the agent prompt should include "Output artifact path convention". The current prompt does not tell the agent *where* to write its output artifacts. In the current crate, the output path is `ArtifactManager::output_artifact_path(run_id, step_id, filename)` — but the dispatched agent doesn't know this unless told.

**Impact:** When wiring to real agent execution, the agent won't know where to write unless the dispatcher also injects output paths or wraps the agent's file-write calls. This should be addressed during the governed dispatch integration.

**Recommended fix:** Add an output section to the prompt:
```rust
if !step.outputs.is_empty() {
    prompt.push_str("Output artifact paths (write your results here):\n");
    for o in &step.outputs {
        let p = artifact_base.output_artifact_path(run_id, &step.id, o);
        prompt.push_str("- ");
        prompt.push_str(&p.to_string_lossy());
        prompt.push('\n');
    }
    prompt.push('\n');
}
```

### P3-002: Quick effort text omits "no sub-agent calls" (COSMETIC)

**File:** `lib.rs:148`
The spec says Quick means "no sub-agent calls". The prompt text says "single-pass, very concise (< 2k tokens)" but doesn't mention the sub-agent restriction. In practice, effort is advisory (per spec contract notes), so this is cosmetic.

### P3-003: No test for zero-input step prompt (INFO)

The test constructs a step with two inputs. There's no test exercising the `"This step has no upstream artifact dependencies."` branch (empty inputs). The code path is trivial but untested.

### P3-004: `to_string_lossy()` on artifact paths (INFO)

**File:** `lib.rs:167`
`p.to_string_lossy()` replaces invalid UTF-8 with `�`. For OAP's use case (developer machines, ASCII paths) this is fine. Worth noting if artifact directories ever contain non-UTF-8 characters.

## Wiring recommendations — governed dispatch + registry

The handoff asks for advice on integrating `build_step_system_prompt()` into the real dispatch path. Here's the recommended architecture:

### 1. Replace `dispatch_manifest_noop` inner loop

The no-op dispatcher's success path (line 306: `statuses[idx] = StepStatus::Success`) becomes:

```rust
// 1. Resolve agent from registry (042)
let agent = registry.get_agent(&step.agent)
    .ok_or(OrchestratorError::AgentNotFound { agent_id: step.agent.clone() })?;

// 2. Build system prompt (FR-004) — already done
let system_prompt = build_step_system_prompt(artifact_base, run_id, step);

// 3. Ensure output directory exists
artifact_base.ensure_step_dir(run_id, &step.id)?;

// 4. Dispatch via governed execution (035)
let result = governed_dispatch(agent, system_prompt, step.effort).await?;

// 5. Verify output artifacts were written
for output in &step.outputs {
    let path = artifact_base.output_artifact_path(run_id, &step.id, output);
    if !path.exists() {
        return Err(OrchestratorError::StepFailed {
            step_id: step.id.clone(),
            reason: format!("agent did not produce declared output: {}", path.display()),
        });
    }
}

// 6. Record token usage
summary_entries[idx].tokens_used = result.tokens_used;
statuses[idx] = StepStatus::Success;
```

### 2. Make the dispatcher async

Real agent execution is async (provider registry calls, Claude Code SDK bridge). The dispatcher function signature becomes:

```rust
pub async fn dispatch_manifest(
    artifact_base: &ArtifactManager,
    run_id: Uuid,
    manifest: &WorkflowManifest,
    registry: &ProviderRegistry,  // from 042
) -> Result<RunSummary, OrchestratorError>
```

### 3. Tauri command surface

Expose as Tauri commands in a new `commands/orchestrator.rs`:

```rust
#[tauri::command]
async fn orchestrate_manifest(manifest_path: String) -> Result<RunSummary, String> { ... }

#[tauri::command]
async fn get_run_status(run_id: String) -> Result<RunSummary, String> { ... }

#[tauri::command]
async fn cancel_run(run_id: String) -> Result<(), String> { ... }

#[tauri::command]
async fn cleanup_artifacts(run_id: String) -> Result<(), String> { ... }
```

### 4. Address P3-001 before wiring

Output artifact paths should be injected into the system prompt before the first real dispatch. Without this, agents won't know where to write their output files.

## FR coverage update (cumulative)

| FR | Status | Phase |
|----|--------|-------|
| FR-001 | Partial | P1 (step model), NL decomposition deferred |
| FR-002 | **Done** | P1 + P2 |
| FR-003 | **Done** | P1 |
| FR-004 | **Done** | **P3** (system prompt builder) |
| FR-005 | **Done** | P1 + P3 (effort text in prompt) |
| FR-006 | **Done** | P1 + P2 |
| FR-007 | **Done** | P1 |
| FR-008 | **Done** | P1 + P2 |

All FRs except FR-001 (NL decomposition — explicitly deferred) are now satisfied.

## Summary

The Phase 3 `build_step_system_prompt()` helper correctly satisfies FR-004. It resolves absolute input paths via the existing `resolve_input_paths()`, includes a clear filesystem-read directive, and carries the effort level text matching FR-005. The one actionable gap is P3-001 (output paths not in prompt) — this should be fixed before wiring to real agent execution. 11/11 tests pass. 4 findings — 1 low (actionable), 1 cosmetic, 2 info.

**Recommendation:** Fix P3-001 (output path injection), then proceed to wire `dispatch_manifest()` with async governed execution (035) + provider registry (042) as outlined in the wiring section above.
