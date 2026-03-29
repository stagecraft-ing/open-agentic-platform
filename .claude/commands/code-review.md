---
description: Multi-aspect code review using parallel sub-agents with adaptive agent selection
allowed-tools: Task, Bash(git status:*), Bash(git diff:*), Bash(git log:*), Bash(git show:*)
argument-hint: "[scope] - e.g., \"recent changes\", \"src/components\", \"crates/agent\", \"PR #42\""
---
# Code Review

## Gather Context

!`git status --short && echo "---DIFF-STAT---" && git diff --stat && echo "---LOG---" && git log --oneline -10`

Determine the base branch:
```bash
MAIN_WORKTREE=$(git worktree list | head -1 | awk '{print $1}'); CURRENT_DIR=$(git rev-parse --show-toplevel); if [ "$MAIN_WORKTREE" != "$CURRENT_DIR" ]; then BASE=$(git -C "$MAIN_WORKTREE" branch --show-current); else BASE="main"; fi; echo "Base: $BASE" && git diff $BASE...HEAD --stat && git diff HEAD --stat
```

## Pre-Review Analysis

Before launching agents, analyze the changes to determine scope and strategy.

### 1. Classify Changed Files

Examine all changed files (committed + uncommitted) and classify them:

| Category | Patterns |
|----------|----------|
| Rust source | `crates/**/*.rs`, `Cargo.toml`, `Cargo.lock` |
| TypeScript/JS source | `packages/**/*.ts`, `apps/**/*.{ts,tsx}`, `tools/**/*.ts` |
| Config/Build | `*.json`, `*.yaml`, `*.toml`, `pnpm-*`, `build/**` |
| Specs/Docs | `specs/**`, `docs/**`, `*.md` |
| Tests | `*test*`, `*spec*`, `tests/` |
| CI/Scripts | `.github/**`, `scripts/**` |

### 2. Determine Agent Set

Based on file classification and **$ARGUMENTS**, select which agents to launch:

- **Docs/specs only** --> Documentation Review agent only
- **Tests only** --> Testing Quality + Code Quality agents
- **Config/CI only** --> Security + Architecture agents
- **Rust source** --> All agents (include Rust-specific focus)
- **TypeScript source** --> All agents (include TS-specific focus)
- **Mixed changes** --> All agents relevant to file types present
- **Broad scope or explicit request** --> All 6 agents

### 3. Shared Context Block

Compose a CONTEXT block to pass to every agent:

```
CONTEXT:
- Repository: open-agentic-platform (polyglot monorepo: Rust crates, TS packages, desktop app)
- Review scope: [what $ARGUMENTS resolved to]
- Changed areas: [list of crates/packages/apps affected]
- Risk assessment: [low/medium/high based on scope and affected areas]
- Key integration points: [any cross-crate or cross-package boundaries touched]
```

## Parallel Agent Dispatch

Launch the selected agents concurrently using the Task tool. Each agent receives the diff content and shared context block.

### Agent 1: Architecture and Design

```
Review architecture and design patterns in: $ARGUMENTS

{CONTEXT block}

Focus areas:
- Module organization and separation of concerns
- Dependency direction (do crates/packages depend on each other correctly?)
- Abstraction levels and API surface design
- Consistency with existing patterns in the monorepo
- Workspace structure (Cargo workspace, pnpm workspace) coherence

THINK END-TO-END:
- Trace how this change affects dependent crates/packages
- Map data and control flow across module boundaries
- Identify what breaks if components fail or interfaces change
- Consider whether public API changes cascade correctly
```

### Agent 2: Code Quality

```
Review code quality and maintainability in: $ARGUMENTS

{CONTEXT block}

Focus areas:
- Readability and naming conventions
- Code complexity and cognitive load
- DRY violations and missed abstractions
- Idiomatic patterns (Rust idioms for .rs, TS idioms for .ts)
- Error handling consistency (Result/Option in Rust, error types in TS)
- Dead code, unused imports, leftover debug statements
- Type safety (no unnecessary `any` in TS, no unnecessary `unwrap()` in Rust)
```

### Agent 3: Security and Dependencies

```
Perform security and dependency analysis of: $ARGUMENTS

{CONTEXT block}

Focus areas:
- Input validation and sanitization
- Injection vulnerabilities (SQL, command, path traversal)
- Secrets management (no hardcoded keys, tokens, or credentials)
- Authentication and authorization gaps
- Dependency changes: new packages, version bumps, removal
- Supply chain considerations for new dependencies
- Unsafe code blocks in Rust (are they justified and sound?)
- File system and process spawning safety

CONSIDER ALTERNATIVE ATTACK VECTORS:
- Beyond obvious vulnerabilities, what other attack surfaces exist?
- What assumptions about trust boundaries could be violated?
- Could new dependencies introduce transitive risks?
```

### Agent 4: Performance and Scalability

```
Analyze performance and scalability in: $ARGUMENTS

{CONTEXT block}

Focus areas:
- Algorithm complexity and hot paths
- Memory allocation patterns (unnecessary clones in Rust, large objects in TS)
- Async patterns and potential deadlocks or resource starvation
- I/O efficiency (file operations, network calls, database queries)
- Caching opportunities and cache invalidation
- Resource cleanup (Drop in Rust, cleanup in TS)
- Concurrency correctness (Send/Sync in Rust, race conditions)
```

### Agent 5: Testing Quality

```
Review test quality and coverage for: $ARGUMENTS

{CONTEXT block}

Focus areas:
- Are new code paths covered by tests?
- Test isolation and determinism
- Edge cases and error paths tested
- Meaningful assertions (not just "it doesn't crash")
- Mock vs real dependency balance
- Test naming and readability
- Integration test coverage for cross-boundary changes
- Are there regression tests for bug fixes?
```

### Agent 6: Documentation and API Surface

```
Review documentation and API design for: $ARGUMENTS

{CONTEXT block}

Focus areas:
- Public API documentation (doc comments in Rust, JSDoc/TSDoc in TS)
- Breaking changes to existing APIs or interfaces
- README and docs/ updates needed for new features
- Spec file consistency (if specs/ are affected)
- Code comments for non-obvious logic
- Migration guidance if interfaces changed
- Error messages are clear and actionable
```

## Post-Review Consolidation

After all agents complete, synthesize findings with cross-cutting analysis:

### Cross-Pattern Analysis
- **Competing solutions**: Do findings from different agents conflict?
- **Root causes**: Is the same underlying issue showing up across multiple agents?
- **Intentional trade-offs**: Are apparent "problems" actually deliberate design decisions?
- **Cascading effects**: Do fixes in one area create issues in another?

### Deduplicate and Prioritize
- Merge overlapping findings from different agents
- Remove false positives and theoretical-only issues
- Weight severity by actual impact in context of the change

## Final Report Format

```
## Code Review Report

### Scope
- Target: [files/directories reviewed]
- Base: [base branch] | Head: [current branch or working tree]
- Files changed: [count] | Lines: +[added] / -[removed]
- Agents used: [list which of the 6 were launched and why others were skipped]

### Executive Summary
[2-3 sentences: overall assessment, key strengths, and most important issues]

### CRITICAL (must fix before merge)
1. [SECURITY|ARCHITECTURE|BUG] **Issue title**
   - File: `path/to/file:line`
   - Impact: [what breaks or is at risk]
   - Fix: [specific recommendation or code example]

### HIGH (strongly recommended)
1. [Category] **Issue title**
   - File: `path/to/file:line`
   - Impact: [description]
   - Recommendation: [what to do]

### MEDIUM (should address)
1. [Category] **Issue title** - `file:line`
   Suggestion: [brief recommendation]

### LOW (nice to have)
- [Category] Issue description - `file:line`

### Quality Scorecard
(Only include rows for aspects that were actually reviewed)

| Aspect | Score | Notes |
|--------|-------|-------|
| Architecture | X/10 | [assessment] |
| Code Quality | X/10 | [assessment] |
| Security | X/10 | [assessment] |
| Performance | X/10 | [assessment] |
| Testing | X/10 | [assessment] |
| Documentation | X/10 | [assessment] |

### Strengths
- [Positive patterns worth preserving, with evidence]

### File-by-File Summary
| File | Changes | Key Findings |
|------|---------|--------------|
| `path/to/file` | [what changed] | [issues or "clean"] |

### Recommended Actions
1. [Actionable item from findings]
2. [Next actionable item]
3. ...

[If no actions needed: "No blocking issues found -- branch is ready for merge."]
```

**To proceed:** Reply with the numbers of actions you want taken (e.g., "1, 3, 5" or "all").

---

**This is a read-only review. No files will be modified unless you explicitly request it.**
