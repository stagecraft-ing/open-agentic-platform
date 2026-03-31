# 049 Permission System — phased implementation plan

> **Non-authoritative.** Planning scratch for agent coordination only. Canonical contract remains `specs/049-permission-system/spec.md`.

## Goal

Implement a deterministic permission system for tool calls with wildcard matching, persistent grant/deny memory, scoped storage (session/project/global), and a `canUseTool` handler that supports interactive and non-interactive execution modes.

## Pre-implementation decisions (P-001 to P-006)

- **P-001 (store ownership and pathing):** Canonical persistence for this feature lands in a new TypeScript package at `packages/permission-system/` with project store at `<repo>/.claude/permissions.json` and global store at `~/.claude/permissions.json`. Session scope remains in-memory only.
- **P-002 (pattern canonical form):** Normalize permission patterns to the explicit `Tool(argPattern)` format before persistence to keep matching deterministic and human-editable (NF-002, NF-003).
- **P-003 (precedence semantics):** Enforce hard order: non-interactive gate -> bypass -> disallowed -> remembered allow -> prompt, with disallowed precedence over allow at equal specificity (FR-002, FR-007).
- **P-004 (matching target extraction):** Resolve a stable primary argument string per tool for pattern matching (for `Bash`, command line first; for file tools, path first; for MCP, `server:tool`) and document fallback rules.
- **P-005 (scope merge strategy):** Runtime lookup uses merged view `session + project + global`, but mutations write only to selected scope. Revoke can target a specific scope or all matching scopes.
- **P-006 (integration seam):** Integrate through existing governed execution path by injecting permission evaluator as the `canUseTool` callback contract, keeping UI prompt transport abstract behind a callback interface.

## Implementation slices

### Phase 1 — Types and wildcard matcher (FR-003, NF-003)

Deliverables:

- `packages/permission-system/src/types.ts` with `PermissionEntry`, `PermissionScope`, `PermissionDecision`, and evaluator I/O contracts.
- `packages/permission-system/src/pattern.ts` parser/matcher supporting `*` and `**` in tool and argument positions.
- Deterministic normalization helpers for stored patterns.

Validation:

- Unit tests for wildcard behavior (`*` single segment, `**` recursive), deterministic repeated matches, and invalid pattern diagnostics.
- SC-004 fixture coverage for `Read(/Users/me/**)` include/exclude behavior.

### Phase 2 — JSON permission store (FR-005, FR-006, NF-002)

Deliverables:

- `packages/permission-system/src/store.ts` with `list`, `upsert`, `revoke`, `clearExpired`, and scoped load/save.
- Schema versioning (`version: 1`) and readable pretty JSON persistence.
- Atomic write strategy (temp file + rename) to avoid partial writes.

Validation:

- Unit tests for project/global path resolution, create-on-first-write, revoke by exact pattern, and expired-entry clearing.
- Round-trip tests proving hand-edited JSON remains parseable.

### Phase 3 — Layered evaluator and prompt contract (FR-001, FR-002, FR-004, FR-007)

Deliverables:

- `packages/permission-system/src/evaluator.ts` implementing ordered layer evaluation and decision rationale.
- `packages/permission-system/src/defaults.ts` for baseline bypass/disallowed lists.
- `packages/permission-system/src/prompt.ts` callback interface for `Allow once`, `Allow & remember`, `Deny`.

Validation:

- SC-001 bypass short-circuit and SC-002 disallowed-over-allow behavior.
- SC-003 remember flow persists grant then suppresses subsequent prompts.
- Tests for ambiguous overlaps and explicit precedence guarantees.

### Phase 4 — canUseTool hook integration (FR-001, FR-004)

Deliverables:

- `packages/permission-system/src/index.ts` public `createPermissionHandler(...)` API returning SDK-compatible `canUseTool`.
- Wiring in governed execution/bridge path to call the handler for every tool invocation.
- Clear blocked decision payload surface for caller UI/error reporting.

Validation:

- Integration tests showing every tool call routes through permission handler.
- Regression tests ensuring denied results block tool execution and return clear reason.

### Phase 5 — CLI permissions management (FR-008, SC-005, SC-006)

Deliverables:

- `packages/permission-system/src/cli.ts` commands:
  - `permissions list`
  - `permissions revoke <pattern>`
  - `permissions clear --expired`
- Output formatting includes pattern, decision, scope, timestamps.

Validation:

- SC-005 list output fixtures include scope + timestamps.
- SC-006 revoke fixture re-prompts on next matching call.

### Phase 6 — Non-interactive policy + verification (FR-009, NF-001)

Deliverables:

- Non-interactive mode behavior with configurable default (`deny-all` or `allow-from-list`).
- Benchmark/test harness for p99 overhead target (<2ms at 500 entries).
- `specs/049-permission-system/execution/verification.md` with SC/NF command evidence.

Validation:

- Tests for non-interactive deny-all and allow-list flows.
- Performance results captured in verification doc with dataset size and environment notes.
