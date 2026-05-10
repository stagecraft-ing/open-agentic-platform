---
description: "Task list for spec 144 — Hiqlite default-features hygiene"
---

# Tasks: Hiqlite default-features hygiene

**Input**: `specs/144-hiqlite-default-features/spec.md` + `plan.md`
**Prerequisites**: `plan.md` (required), `spec.md` (required for §-anchors), `audit.md` + `verifications.md` (provenance)

**Tests**: not authored — this is manifest hygiene. Existing
`cargo check / clippy / test` and the orchestrator CI workflow cover
the affected build paths. The lockfile diff is itself the verification
artifact for the unification fix.

## Format: `[ID] [P?] [Phase] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Phase]**: Maps to `plan.md` phase (P0–P2)
- File paths in descriptions are exact

## Path Conventions

- Orchestrator manifest: `crates/orchestrator/Cargo.toml`
- Axiomregent manifest: `crates/axiomregent/Cargo.toml`
- Workspace lockfile: `crates/Cargo.lock`
- Reference workspace (negative control): `platform/services/deployd-api-rs/Cargo.lock`

---

## Phase 0: Pre-flight verification

**Purpose**: Confirm the audit findings still reproduce on the working
tree before editing anything.

- [ ] T001 [P0] Confirm spec 144 frontmatter compiles cleanly:
      `./tools/spec-compiler/target/release/spec-compiler compile` and
      verify exit 0 + spec 144 appears in `build/spec-registry/registry.json` via
      `./tools/registry-consumer/target/release/registry-consumer show 144-hiqlite-default-features`.
- [ ] T002 [P0] Confirm `crates/orchestrator/Cargo.toml:20` still
      lacks `default-features = false`. Expected line text:
      `hiqlite = { version = "~0.13", features = ["sqlite", "dlock", "listen_notify_local"], optional = true }`.
- [ ] T003 [P0] Confirm `crates/axiomregent/Cargo.toml:38` still
      lists `cache` in the explicit feature list. Expected:
      `hiqlite = { version = "~0.13", default-features = false, features = ["sqlite", "dlock", "listen_notify_local", "cache"] }`.
- [ ] T004 [P0] Confirm `crates/Cargo.lock` contains the
      unified-by-accident transitives:
      `grep -nE '^name = "cron"$|^name = "toml"$|^version = "1.1.2\+spec-1.1.0"$' crates/Cargo.lock`
      and confirm the expected `cron` package and the
      `1.1.2+spec-1.1.0` `toml` entry are present.
- [ ] T005 [P0] Confirm `dlock` is dead in orchestrator source:
      `grep -rnE 'client\.lock\(|hiqlite::Lock' crates/orchestrator/src` →
      expect zero hits.
- [ ] T006 [P0] Confirm `--features distributed` enablement is dead
      across the repo: re-run the searches in `verifications.md` Q2
      across `**/Cargo.toml`, `**/*.yml`, `**/*.yaml`, `**/Dockerfile*`,
      `**/*.sh`, `**/Makefile`, `**/*.mk`, `**/Justfile`, expect zero
      hits relevant to enablement (matches inside `[features]`
      *declarations* are expected and not enablement).
- [ ] T007 [P0] Confirm no other workspace member opts into the
      hiqlite features being dropped:
      `grep -rnE 'features.*=.*\[.*"(backup|s3|toml|cache)"' crates/*/Cargo.toml`
      should return zero hits relevant to hiqlite (other crates may
      list `backup` / `cache` for unrelated deps — eyeball the matches).

**Checkpoint**: All preconditions match `audit.md` + `verifications.md`.
Phase 1 cannot start until this checkpoint clears.

---

## Phase 1: Manifest edits + lockfile regen

**Purpose**: Apply the three-file change.

- [ ] T010 [P1] Edit `crates/orchestrator/Cargo.toml:20` in place to:
      ```toml
      hiqlite = { version = "~0.13", default-features = false, features = ["sqlite", "listen_notify_local", "auto-heal"], optional = true }
      ```
      Three coupled edits: add `default-features = false`, drop
      `dlock`, add `auto-heal`.
- [ ] T011 [P1] Edit `crates/axiomregent/Cargo.toml:38` in place to:
      ```toml
      hiqlite = { version = "~0.13", default-features = false, features = ["sqlite", "dlock", "listen_notify_local", "auto-heal"] }
      ```
      Two coupled edits: drop `cache`, add `auto-heal`.
- [ ] T012 [P1] Regenerate `crates/Cargo.lock`:
      `cargo check --manifest-path crates/orchestrator/Cargo.toml`
      (workspace member) **or**
      `cargo generate-lockfile --manifest-path crates/Cargo.toml`
      (workspace root). Either rewrites the lockfile to reflect the
      new feature posture without recompiling the workspace.
- [ ] T013 [P1] Inspect the lockfile diff:
      `git diff crates/Cargo.lock`. Expected removals: `cron` package
      block; `toml` package at version `1.1.2+spec-1.1.0`. Expected
      retentions: `futures-util` (kept by `listen_notify_local`),
      `cryptr`, `s3-simple`, `deadpool`, `rusqlite`. Halt and
      re-investigate if any other crate is added or removed
      unexpectedly.
- [ ] T014 [P1] Confirm `platform/services/deployd-api-rs/Cargo.lock`
      is **unchanged**: `git diff platform/services/deployd-api-rs/Cargo.lock`
      should be empty (separate workspace; already correct).

**Phase 1 exit:** working tree carries the three-file diff
(`crates/orchestrator/Cargo.toml`, `crates/axiomregent/Cargo.toml`,
`crates/Cargo.lock`) and nothing else.

---

## Phase 2: Verification

**Purpose**: confirm the change is benign on every build path the
crates ship through.

- [ ] T020 [P2] `cargo check --manifest-path crates/orchestrator/Cargo.toml`
      → exit 0.
- [ ] T021 [P2] `cargo clippy --manifest-path crates/orchestrator/Cargo.toml --all-targets -- -D warnings`
      → exit 0 (warnings are errors).
- [ ] T022 [P2] `cargo test --manifest-path crates/orchestrator/Cargo.toml`
      → all tests pass.
- [ ] T023 [P2] `cargo check --manifest-path crates/axiomregent/Cargo.toml`
      → exit 0.
- [ ] T024 [P2] `cargo clippy --manifest-path crates/axiomregent/Cargo.toml --all-targets -- -D warnings`
      → exit 0 (warnings are errors).
- [ ] T025 [P2] `cargo test --manifest-path crates/axiomregent/Cargo.toml`
      → all tests pass.
- [ ] T026 [P2] Smoke-build the desktop crate:
      `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`
      → exit 0. Catches any unexpected feature regression in the
      transitive consumer.
- [ ] T027 [P2] `make ci` (warm, spec 134 / 135 fast-CI loop) → exit 0.
- [ ] T028 [P2] Recompile spec registry + codebase index:
      `./tools/spec-compiler/target/release/spec-compiler compile` and
      `./tools/codebase-indexer/target/release/codebase-indexer compile && render`.
- [ ] T029 [P2] Run the coupling check:
      `./tools/spec-code-coupling-check/target/release/spec-code-coupling-check`
      → no warnings against spec 144's `implements:` list. (AC-7.)
- [ ] T030 [P2] Update spec 144 frontmatter:
      `implementation: complete`, `closed: "<today>"`. Recompile
      registry. Confirm `registry-consumer status-report` reflects
      the change.
- [ ] T031 [P2] Commit + open PR. Title:
      `feat(spec-144): hiqlite default-features hygiene — stop unifying upstream defaults`.

**Phase 2 exit:** AC-1 through AC-7 in `spec.md` §3 pass; PR open.

---

## Acceptance criteria mapping

| AC | Tasks |
|---|---|
| AC-1 (orchestrator manifest) | T010 |
| AC-2 (axiomregent manifest) | T011 |
| AC-3 (lockfile delta) | T012, T013 |
| AC-4 (cargo check / clippy / test on both crates) | T020, T021, T022, T023, T024, T025 |
| AC-5 (CI green) | T027 |
| AC-6 (`make ci` warm green) | T027 |
| AC-7 (coupling gate clean) | T029 |

---

## Quick reference — key file:line anchors

| File | Lines | Phase | Action |
|---|---|---|---|
| `crates/orchestrator/Cargo.toml` | 20 | 1 | add default-features=false; drop dlock; add auto-heal |
| `crates/axiomregent/Cargo.toml` | 38 | 1 | drop redundant cache; add auto-heal |
| `crates/Cargo.lock` | (regenerated) | 1 | drop cron + toml 1.1.2+spec-1.1.0 |
| `platform/services/deployd-api-rs/Cargo.lock` | (must not change) | 1 | negative control — confirm clean diff |
