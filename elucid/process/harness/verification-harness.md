---
id: verification-harness
name: Verification Harness Specification
description: >
  Automated verification runner that replaces self-assessed markdown
  checklists with real tool execution. Runs adapter-declared commands,
  checks file existence, validates JSON schemas, and performs cross-stage
  consistency checks.
---

# Verification Harness

The verification harness is the enforcement layer. It runs automatically after every stage and every scaffolded feature. No agent validates its own output.

## Principles

1. **External verification** — the harness runs commands and checks results. The generating agent never assesses its own work.
2. **Fast feedback** — after each feature, run only `feature_verify` commands (not full build). Full validation runs once at the end.
3. **Error forwarding** — on failure, capture stdout/stderr and feed it back to the agent for retry.
4. **Durable results** — write all check results to pipeline state.

## Check Types

### `schema-validation`
Validates a JSON/YAML artifact against a JSON Schema.

```
Input: artifact path, schema path
Action: Parse artifact, validate against schema
Pass: No validation errors
Fail: List of validation errors with paths
```

### `artifact-exists`
Confirms a file exists at the expected path.

```
Input: file path (may include globs)
Action: Check file existence
Pass: File exists and is non-empty
Fail: File missing or empty
```

### `artifact-content`
Checks a condition against parsed artifact content.

```
Input: artifact path, condition expression
Action: Parse JSON/YAML, evaluate condition
Pass: Condition is true
Fail: Condition is false, report actual value
```

### `cross-reference`
Checks that items in one artifact are referenced in another.

```
Input: source artifact, target artifact, mapping rule
Action: Extract IDs from source, check each exists in target
Pass: All source IDs found in target
Fail: List of missing IDs
```

### `command-succeeds`
Runs a shell command and checks exit code.

```
Input: command string, working directory
Action: Execute command, capture stdout/stderr
Pass: Exit code 0
Fail: Non-zero exit code, captured output
Timeout: 120 seconds default, configurable
```

### `grep-absent`
Ensures a pattern does NOT appear in the codebase.

```
Input: regex pattern, scope (directory), optional excludes
Action: grep -rE pattern scope
Pass: No matches
Fail: List of matching files and lines
```

### `grep-present`
Ensures a pattern DOES appear.

```
Input: regex pattern, scope
Action: grep -rE pattern scope
Pass: At least one match
Fail: No matches found
```

### `file-check`
Confirms expected files were created per directory conventions.

```
Input: convention template, placeholder values
Action: Resolve template → file path, check existence
Pass: File exists
Fail: File missing
```

## Execution Points

### After Each Process Stage (1-5)

Run the stage's checks from `verification.schema.yaml → stage_gates`:

```
Stage 1 complete → run S1-001 through S1-004
Stage 2 complete → run S2-001 through S2-003
...
```

All `error` severity checks must pass. `warning` checks are logged but don't block.

### After Each Scaffolded Feature

Run the `scaffolding_gates` from verification contract:

```
API feature scaffolded → run SF-API-001 (compile), SF-API-002 (test), SF-API-003 (files exist)
UI feature scaffolded → run SF-UI-001 (compile), SF-UI-002 (test), SF-UI-003 (files exist)
```

The commands come from `adapter.commands.feature_verify`.

### Retry Protocol

On feature verification failure:
1. Capture the full error output (stdout + stderr)
2. Truncate to 2000 characters if longer (preserve tail, which usually has the error)
3. Feed back to the scaffolding agent as context: "Your generated code failed verification: {error}"
4. Agent regenerates the failing artifact(s)
5. Re-run verification
6. Repeat up to `retry.max_retries` (default: 3)
7. After max retries: mark as failed, log error, continue to next feature

### Final Validation

Run all checks from `verification.schema.yaml → final_validation`:

Process checks:
- UC→code mapping (every UC has at least one generated service/controller)
- TC→test mapping (every TC has at least one test file)
- Entity→migration mapping (every entity has a migration)
- No unfilled `{{PLACEHOLDER}}` patterns in source files

Adapter checks (via declared commands):
- Full build: `adapter.commands.compile`
- All tests: `adapter.commands.test`
- Lint: `adapter.commands.lint`
- Type check: `adapter.commands.type_check`
- Format: `adapter.commands.format_check`
- Invariants: run each invariant from `adapter.validation.invariants`

## Pipeline State Updates

After every check execution, update `.elucid/pipeline-state.json`:

```json
{
  "stages": {
    "business-requirements": {
      "gate": {
        "passed": true,
        "checked_at": "2026-04-04T14:00:00Z",
        "checks": [
          { "id": "S1-001", "passed": true, "message": "" },
          { "id": "S1-002", "passed": true, "message": "" }
        ]
      }
    }
  },
  "scaffolding": {
    "api": {
      "operations_completed": [
        {
          "operation_id": "list-organizations",
          "files_created": ["apps/api-internal/src/services/organization.service.ts"],
          "verified_at": "2026-04-04T14:30:00Z"
        }
      ],
      "operations_failed": [
        {
          "operation_id": "transition-funding-request",
          "error": "TypeError: Cannot read property 'status' of undefined",
          "retries": 3,
          "max_retries": 3
        }
      ]
    }
  }
}
```

## Implementation Notes

The harness can be implemented as:
- A shell script that reads verification.schema.yaml and runs checks
- A Python/Node CLI tool with JSON Schema validation built in
- An LLM agent that reads the contract and executes checks via Bash tool

The contract is implementation-agnostic — it defines WHAT to check, not HOW to run the check tool. Any implementation that satisfies the check types above is valid.
