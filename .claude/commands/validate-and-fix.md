---
name: validate-and-fix
description: Run the local CI parity check (`make ci`) and automatically fix discovered issues using concurrent agents
allowed-tools: Bash, Agent, Read, Edit, Glob, Grep
---

# Validate and Fix

Run the authoritative local CI parity check and automatically fix discovered issues. This command mirrors every GitHub Actions gate end to end — if `make ci` passes locally, the workflows in `.github/workflows/` will pass too.

## Process

### 1. Run the local CI parity check

Invoke `make ci` from the repo root. The `Makefile` is the **single source of truth** for what CI validates. Do not discover validation commands by grepping `package.json`, `Cargo.toml`, or CLAUDE.md — the Makefile already enumerates every gate the CI workflows enforce.

> **Two modes (spec 134).** `make ci` is the parity-bound mirror of every enforcing GitHub workflow — token-equivalent, conservative, ~90 min on M1 Pro. `make ci-fast` is the parity-exempt local accelerator — same gate set, parallel/sccache/nextest, target ≤ 25 min warm. Use `ci-fast` for the inner loop; run `ci` once before pushing to confirm parity.

`make ci` composes:

- **`make ci-rust`** — per-manifest `cargo check` + `cargo clippy -- -D warnings` + `cargo test` for every isolated Rust workspace. Covers: `axiomregent`, `orchestrator`, `policy-kernel`, `tool-registry`, `skill-factory`, `factory-engine`, `factory-contracts`, `provider-registry`, `agent-frontmatter`, `standards-loader`, `platform/services/deployd-api-rs`. Mirrors `ci-axiomregent.yml`, `ci-crates.yml`, `ci-deployd-api-rs.yml`, `ci-orchestrator.yml`, `ci-policy-kernel.yml`.
- **`make ci-tools`** — spec tool crates (build + test + smoke), registry-consumer's 10 contract subsets, `codebase-indexer check` staleness gate. Mirrors `spec-conformance.yml`.
- **`make ci-desktop`** — `apps/desktop/src-tauri` Rust (custom clippy flags: `-A dead_code -D warnings`), version alignment check (`Cargo.toml` ↔ `package.json`), `pnpm install --frozen-lockfile`, `tsc --noEmit`, vitest. Mirrors `ci-desktop.yml`.
- **`make ci-stagecraft`** — `platform/services/stagecraft`: `npm ci` + `npx tsc --noEmit` + `npm test`. Mirrors `ci-stagecraft.yml`.

Not in `make ci` by default (run on demand):
- `make ci-cross` — axiomregent cross-target matrix (requires `rustup target add` per triple). Mirrors `build-axiomregent.yml`.

**If a check is missing, add it to the Makefile and the relevant workflow in the same change.** Never introduce a new validation via a one-off script under `scripts/` — that directory is being retired in favour of Rust binaries invoked from the Makefile (spec 105).

After edits to either the Makefile `ci*` targets or any workflow under `.github/workflows/`, run `make ci-parity` as a pre-PR check. It asserts that every `run:` block in an enforcing workflow has a mirror in the Makefile; drift is a workflow violation (spec 104). The check is also enforced by `.github/workflows/ci-parity.yml`, so a PR that skips the local run will still be caught in CI.

Capture full output — file paths, line numbers, error messages. Categorize findings:

- **CRITICAL** — security issues, breaking changes, data loss risk
- **HIGH** — functionality bugs, test failures, build breaks, staleness gate failures, version mismatches
- **MEDIUM** — clippy warnings, code quality, documentation gaps
- **LOW** — formatting, minor optimizations

### 2. Strategic Fix Execution

#### Phase 1 — Safe Quick Wins
- Start with LOW and MEDIUM priority fixes that can't break anything
- Verify each fix by re-running the narrowest affected subtarget (e.g. `make ci-tools`)

#### Phase 2 — Functionality Fixes
- Address HIGH priority issues one at a time
- Run the affected `make ci-<sub>` after each fix to confirm no regressions

#### Phase 3 — Critical Issues
- Handle CRITICAL issues with explicit user confirmation
- Provide detailed plan before executing

#### Phase 4 — Verification
- Re-run the full `make ci` composite to confirm end-to-end parity
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
- If a phase fails, diagnose and provide specific next steps

### 4. Parallel Execution

Launch multiple agents concurrently for independent, parallelizable tasks:
- **CRITICAL**: Include multiple Agent tool calls in a SINGLE message ONLY when tasks can be done in parallel
- Parallelizable: fixes in different manifests (one agent per failing crate), independent test suites, non-overlapping components
- Sequential: shared-interface changes across crates, ordered phases, anything mutating a cross-crate contract
- Each parallel agent must have non-overlapping file responsibilities
- Each agent verifies its fix by re-running the relevant `make ci-<sub>` target before reporting complete

### 5. Final Verification

After all agents complete:
- Re-run `make ci` to confirm 100% CI parity
- Confirm no new issues were introduced by fixes
- Report any remaining manual fixes needed with specific instructions
- Summary: `Fixed X/Y issues, Z require manual intervention — CI parity: {PASS|FAIL}`
