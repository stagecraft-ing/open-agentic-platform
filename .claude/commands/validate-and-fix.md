---
name: validate-and-fix
description: Run quality checks and automatically fix issues using concurrent agents
allowed-tools: Bash, Agent, Read, Edit, Glob, Grep
---

# Validate and Fix

Run quality checks and automatically fix discovered issues using parallel execution.

## Process

### 1. Systematic Priority-Based Analysis

#### Command Discovery
Discover what validation commands are available:
1. Check CLAUDE.md/AGENTS.md for documented build/test/lint commands
2. Examine package.json scripts, Cargo.toml, Makefile for available commands
3. Look for common patterns:
   - **Rust**: `cargo clippy`, `cargo check`, `cargo test`, `cargo fmt --check`
   - **TypeScript**: `tsc --noEmit`, `eslint`, `vitest`, `prettier --check`
   - **Specs**: `spec-compiler compile`, `spec-lint`
   - **Build**: `cargo build`, `pnpm build`
4. Check README.md for additional validation instructions

#### Discovery with Immediate Categorization
Run all discovered quality checks in parallel using Bash. Capture full output including file paths, line numbers, and error messages.

Immediately categorize findings by:
- **CRITICAL**: Security issues, breaking changes, data loss risk
- **HIGH**: Functionality bugs, test failures, build breaks
- **MEDIUM**: Code quality, style violations, documentation gaps
- **LOW**: Formatting, minor optimizations

#### Risk Assessment Before Action
- Identify "quick wins" vs. complex fixes
- Map dependencies between issues (fix A before B)
- Flag issues that require manual intervention

### 2. Strategic Fix Execution

#### Phase 1 — Safe Quick Wins
- Start with LOW and MEDIUM priority fixes that can't break anything
- Verify each fix immediately before proceeding

#### Phase 2 — Functionality Fixes
- Address HIGH priority issues one at a time
- Run tests after each fix to ensure no regressions

#### Phase 3 — Critical Issues
- Handle CRITICAL issues with explicit user confirmation
- Provide detailed plan before executing

#### Phase 4 — Verification
- Re-run ALL checks to confirm fixes were successful
- Provide summary of what was fixed vs. what remains

### 3. Comprehensive Error Handling

#### Rollback Capability
- Create git stash checkpoint before ANY changes: `git stash push -m "pre-validate-and-fix"`
- Provide instant rollback procedure if fixes cause issues

#### Partial Success Handling
- Continue execution even if some fixes fail
- Clearly separate successful fixes from failures
- Provide manual fix instructions for unfixable issues

#### Quality Validation
- Accept 100% success in each phase before proceeding
- If phase fails, diagnose and provide specific next steps

### 4. Parallel Execution

Launch multiple agents concurrently for independent, parallelizable tasks:
- **CRITICAL**: Include multiple Agent tool calls in a SINGLE message ONLY when tasks can be done in parallel
- Tasks that depend on each other must be executed sequentially (separate messages)
- Parallelizable: different file fixes, independent test suites, non-overlapping components
- Sequential: tasks with dependencies, shared state modifications, ordered phases
- Each parallel agent should have non-overlapping file responsibilities to avoid conflicts
- Agents working on related files must understand the shared interfaces
- Each agent verifies their fixes work before completing
- Execute phases sequentially: complete Phase 1 before Phase 2, etc.
- Create checkpoint after each successful phase

### 5. Final Verification

After all agents complete:
- Re-run all checks to confirm 100% of fixable issues are resolved
- Confirm no new issues were introduced by fixes
- Report any remaining manual fixes needed with specific instructions
- Provide summary: "Fixed X/Y issues, Z require manual intervention"
