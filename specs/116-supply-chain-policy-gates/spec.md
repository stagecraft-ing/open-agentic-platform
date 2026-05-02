---
id: "116-supply-chain-policy-gates"
slug: supply-chain-policy-gates
title: Supply-Chain Policy Gates — cargo-deny + dependency audit in CI
status: approved
implementation: pending
owner: bart
created: "2026-04-28"
approved: "2026-05-02"
kind: governance
risk: medium
depends_on:
  - "000"  # bootstrap-spec-system
  - "047"  # governance-control-plane (policy posture precedent)
  - "104"  # makefile-ci-parity-contract (Makefile mirror requirement)
code_aliases: ["SUPPLY_CHAIN_POLICY"]
implements:
  - path: deny.toml
  - path: .github/workflows/ci-supply-chain.yml
  - path: Makefile
summary: >
  Add governed supply-chain gates to CI. cargo-deny enforces a policy bundle
  for advisories, licenses, and banned crates across every Rust manifest in
  the repo; pnpm/npm audit covers the JavaScript surface (apps/desktop +
  platform/services/stagecraft). Posture is staged: warn-only for the first
  30 days after merge, then promoted to a blocking gate. The gate is mirrored
  in the Makefile under a new `ci-supply-chain` target, composed into
  `make ci`, and validated by ci-parity-check.
---

# 116 — Supply-Chain Policy Gates

## 1. Problem Statement

The CI surface today (10 workflows, 4 composite actions, full SHA pinning on
every external action) is unusually disciplined for a pre-alpha repo, but
contains zero dependency-level supply-chain enforcement:

- No `cargo audit` or `cargo deny` step runs against any of the 30 Rust
  crates inventoried in `build/codebase-index/index.json`.
- No `pnpm audit` or `npm audit` step runs against the 24 npm packages
  (notably `@opc/desktop` and `platform/services/stagecraft`).
- No license policy is asserted — a contributor can pull in a GPL-3.0 crate
  via transitive dependency without any signal in CI.
- No banned-crate registry exists. If we standardise on, say, `tracing` over
  `log`, nothing prevents a future change from re-introducing `log` directly.

For a project whose stated framing is the **governed operating system for
AI-native software delivery** (README.md), the absence of supply-chain
governance on the platform's own dependencies is conspicuous. It is also
operationally cheap to fix: cargo-deny is a single binary, pnpm audit is
already shipped with the package manager, and the policy file (`deny.toml`)
is markdown-friendly TOML that fits the project's authoring conventions.

This spec adds those gates as a first-class workflow, mirrored in the
Makefile per spec 104.

## 2. Goals

- **One advisory database, one license policy, one ban list.** Authoritative
  policy lives at `deny.toml` at repo root. Hand-edited TOML is the source of
  truth; cargo-deny is the consumer.
- **Coverage parity with CI inventory.** Every Rust manifest enumerated in
  `Makefile` `CI_RUST_MANIFESTS` plus all tool manifests are scanned. No
  manifest is silently excluded.
- **JavaScript audit at the package-manager layer.** `pnpm audit` is run for
  the pnpm workspace (apps/desktop + packages/) and `npm audit` for
  `platform/services/stagecraft`. Both scoped to `--audit-level=high` (warn
  on moderate, block on high/critical).
- **Staged enforcement.** First 30 days after merge: gate is warn-only.
  Each audit step inspects the `SUPPLY_CHAIN_BLOCKING` repo variable; when
  unset it soft-fails with a `::warning::` annotation (matching the
  Makefile's `|| true` mirror) so violations surface as advisories rather
  than blocking the workflow. On day 30, flipping the variable to `true`
  promotes the same step to blocking — no code change required. The 30-day
  window exists to triage the initial advisory backlog, not as a permanent
  posture.
- **Local mirror at parity.** A new `make ci-supply-chain` target invokes
  the same checks. `make ci` composes it. `ci-parity-check` validates the
  workflow ↔ Makefile mirror.

## 3. Scope

### In scope

- `deny.toml` with sections: `advisories`, `bans`, `licenses`, `sources`.
- `.github/workflows/ci-supply-chain.yml` (new) running:
  - `cargo deny check` per Rust manifest group.
  - `pnpm audit --audit-level=high` for the pnpm workspace.
  - `npm audit --audit-level=high` for `platform/services/stagecraft`.
- `Makefile`: `ci-supply-chain` target composing all three.
- `make ci` (root composite): adds `ci-supply-chain` to the dependency chain.
- 30-day warn-only window encoded as a comment in both the workflow and the
  spec, with a calendar marker for promotion.
- Documentation entry in `make help`.

### Out of scope

- Renovate/Dependabot-driven version bumps. Spec 116 is the gate; the bump
  cadence is handled separately by the dependabot config landed in Phase 0
  of the parent CI hardening branch.
- SBOM generation — that lives in spec 117.
- Build provenance attestations — also spec 117.
- Per-crate vendoring or `cargo vendor` workflows — out of scope for v1.

## 4. Policy Bundle (deny.toml)

The initial policy is intentionally narrow. It blocks the common foot-guns
without forcing a backlog of false-positives on day one.

```toml
[advisories]
# Pull from the official RustSec database.
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/RustSec/advisory-db"]
# Block any advisory rated yanked, vulnerability, or unmaintained on day 30.
# During the warn-only window these are reported but not blocked.
yanked = "warn"
ignore = []

[bans]
# Block multiple major versions of the same crate transitively. This is the
# single most common foot-gun in Rust workspaces.
multiple-versions = "warn"
wildcards = "deny"
# Crates we have explicitly chosen NOT to use. Empty on day 1; populate via
# follow-up PRs as architectural decisions land.
deny = []
# Skip multiple-version warnings for these crates (use sparingly, document why).
skip = []

[licenses]
# Permissive licenses we accept by default.
allow = [
  "MIT",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "Unicode-DFS-2016",
  "Unicode-3.0",
  "CC0-1.0",
  "Zlib",
  "MPL-2.0",
]
# Hard block. GPL-family is incompatible with the desktop bundle's
# distribution model.
deny = [
  "GPL-1.0",
  "GPL-2.0",
  "GPL-3.0",
  "AGPL-1.0",
  "AGPL-3.0",
  "LGPL-2.0",
  "LGPL-2.1",
  "LGPL-3.0",
]
confidence-threshold = 0.93

[sources]
# Block crates from sources other than crates.io and github.com unless
# explicitly allowlisted (e.g. a vendored grammar fork).
unknown-registry = "deny"
unknown-git = "warn"
allow-git = []
```

The `confidence-threshold` of 0.93 matches cargo-deny's default; lower
values produce false positives on dual-licensed crates.

## 5. CI Workflow Shape

`.github/workflows/ci-supply-chain.yml`:

- Triggers: `pull_request` (path filter on `Cargo.toml`, `Cargo.lock`,
  `package.json`, `pnpm-lock.yaml`, `package-lock.json`, `deny.toml`, the
  workflow itself), `push` on main, `workflow_dispatch`, and a weekly cron
  (`0 12 * * 1`) so advisory-db updates surface even on a quiet week.
- Three independent jobs (`cargo-deny`, `pnpm-audit`, `npm-audit-stagecraft`)
  run in parallel.
- Each audit step reads `BLOCKING: ${{ vars.SUPPLY_CHAIN_BLOCKING }}`. When
  the variable is `true`, a non-zero exit propagates and the job fails;
  otherwise the step soft-fails with a `::warning::` annotation. The
  warn-window flip is a one-line repo-variable change, not a code change.
- The repository has no top-level `Cargo.toml`; the `cargo-deny` step
  iterates the same Rust manifests CI compiles (workspace
  `crates/Cargo.toml` plus standalone tool / desktop / deployd-api
  manifests). The Makefile mirrors the same list via
  `SUPPLY_CHAIN_RUST_MANIFESTS`.
- `cargo-deny` is pinned to `^0.19` (≥ 0.19 is required to parse RustSec
  advisories that use CVSS v4.0 vectors).
- Each job uploads its raw output as a workflow artifact for triage.

Day-30 promotion is a single repo-variable change setting
`SUPPLY_CHAIN_BLOCKING=true`. No workflow edit is required; the existing
step-level guard becomes a strict gate. A clean-up PR may drop the dead
`else` branch of each guard at that point.

## 6. Makefile Shape

```makefile
# Spec 116 — supply-chain policy gates.
# Mirrored in .github/workflows/ci-supply-chain.yml.
ci-supply-chain: ci-supply-chain-cargo ci-supply-chain-pnpm ci-supply-chain-npm
	@echo ""
	@echo "==> ci-supply-chain: all gates passed."

# No top-level Cargo.toml: iterate every Rust manifest CI compiles.
SUPPLY_CHAIN_RUST_MANIFESTS = \
    crates/Cargo.toml \
    platform/services/deployd-api-rs/Cargo.toml \
    apps/desktop/src-tauri/Cargo.toml \
    tools/spec-compiler/Cargo.toml \
    tools/registry-consumer/Cargo.toml \
    tools/spec-lint/Cargo.toml \
    tools/codebase-indexer/Cargo.toml \
    tools/policy-compiler/Cargo.toml \
    tools/adapter-scopes-compiler/Cargo.toml \
    tools/ci-parity-check/Cargo.toml \
    tools/shared/frontmatter/Cargo.toml

ci-supply-chain-cargo:
	@command -v cargo-deny >/dev/null 2>&1 || cargo install cargo-deny --locked --version '^0.19'
	@for m in $(SUPPLY_CHAIN_RUST_MANIFESTS); do \
	    cargo deny --manifest-path $$m check || true; \
	done   # warn-only until 2026-05-28

ci-supply-chain-pnpm:
	pnpm audit --audit-level=high || true

ci-supply-chain-npm:
	cd platform/services/stagecraft && npm audit --audit-level=high || true
```

`ci` is updated to `ci: ci-rust ci-tools ci-desktop ci-stagecraft ci-supply-chain`.

`make help` gains a "Supply chain" section.

## 7. Acceptance Criteria

- **AC-1:** A PR introducing a crate with a known RUSTSEC advisory of
  `severity: high` or `critical` triggers a non-empty `cargo-deny` job
  output. During the 30-day window the job is yellow (warn); after
  promotion it is red (block).
- **AC-2:** A PR adding a GPL-3.0 dependency (direct or transitive) triggers
  a license violation in `cargo-deny`. Same warn/block behaviour as AC-1.
- **AC-3:** A PR introducing a known npm advisory at `--audit-level=high`
  triggers the corresponding `pnpm-audit` or `npm-audit-stagecraft` job.
- **AC-4:** `make ci-supply-chain` runs all three checks locally with the
  same exit-code semantics as CI, modulo the warn-window flag.
- **AC-5:** `make ci-parity` (spec 104) confirms the workflow's `run:`
  blocks are mirrored by the Makefile recipe; deliberate divergence is
  detected.
- **AC-6:** A weekly cron run produces a workflow run even with no source
  changes, ensuring fresh advisory-db reports.

## 8. Risks and Mitigations

- **Risk:** initial advisory backlog creates noise that obscures real
  signals.
  **Mitigation:** the 30-day warn window exists exactly for this. Triage
  cadence: weekly review of the workflow's job-summary output, with
  resolutions tracked in `deny.toml`'s `advisories.ignore` (with reason and
  expiry comment).

- **Risk:** false-positive license detection on dual-licensed crates causes
  contributor friction.
  **Mitigation:** `confidence-threshold = 0.93` is the cargo-deny default
  and well-tested. Borderline crates are added to a per-crate
  `[licenses.exceptions]` block on a case-by-case basis with documented
  rationale.

- **Risk:** advisory-db rate limits or fetch failures break CI.
  **Mitigation:** cargo-deny caches the db at `~/.cargo/advisory-db`;
  `actions/cache` keys it by date for once-a-day refresh.

## 9. Day-30 Promotion Plan

### 9.1 Lifecycle naming

This spec uses the two-axis lifecycle deliberately: `status: approved` records
that the design is settled and the artefacts (`deny.toml`, the workflow, the
Makefile target, the BLOCKING-var guard) are landed; `implementation: pending`
records that operational enforcement is still inside the staged warn-only
window. The `implementation: complete` milestone is **the day-30 follow-up
PR described below** — flipping the field earlier would assert a posture the
gate does not yet enforce.

### 9.2 Day-30 PR

A calendar entry is set for 2026-05-28. The follow-up PR:

1. Sets repo variable `SUPPLY_CHAIN_BLOCKING=true` (immediately enforces
   blocking — no code change required).
2. Optionally drops the dead `else` branch of each step-level guard now
   that the warn path is unreachable.
3. Updates this spec's `implementation` field to `complete` and changes the
   header note from "warn-only window" to "blocking gate".
4. Closes the calendar marker.

Promotion is contingent on the warn window having generated zero unresolved
advisories. If unresolved items remain, the window is extended to 60 days
and the rationale documented in this spec.
