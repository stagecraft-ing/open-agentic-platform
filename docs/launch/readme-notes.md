# README rewrite — investigation notes (Phase 1)

> Working notes captured 2026-05-06. The README rewrite (`README.md`) is
> the deliverable; this file is the audit trail. Section structure mirrors
> the prompt's Phase 1 investigation surface.

## Spec corpus (from registry, not directory traversal)

`./tools/registry-consumer/target/release/registry-consumer status-report --json --nonzero-only`:

- **Total:** 142 specs (000–141)
- **Approved:** 137
- **Draft:** 1 — `137-tenant-environment-access-gates`
- **Superseded:** 4 — `038-titor-tauri-command-wiring`,
  `040-blockoli-semantic-search-wiring`, `044-multi-agent-orchestration`,
  `088-factory-upstream-sync`
- **Latest:** `141-aim-vue-node-source-id-template-name-alignment` (merged
  PR #97)

Recent merge concentration on `136-tenant-hello-demo-service`,
`137-tenant-environment-access-gates`, `140-aim-vue-node-scaffold-source-id-cutover`,
`141-aim-vue-node-source-id-template-name-alignment` confirms two active
narratives: (1) tenant onboarding, (2) aim-vue-node adapter hardening.

## Factory engine — what's wired, what isn't

### Wired

- `crates/factory-engine` — 77 passing tests (per spec 102 §Purpose).
- `crates/factory-contracts` — typed contracts for BuildSpec, AdapterManifest,
  PipelineState, VerificationContract.
- Two-phase pipeline (s0–s5 sequential, s6a–s6g fan-out) per spec 075.
- All four OAP-native adapters registered in
  `platform/services/stagecraft/api/factory/oapNativeAdapters.ts`:
  `aim-vue-node`, `next-prisma`, `rust-axum`, `encore-react`. The TS
  registry treats them as equally first-class.
- Factory-as-platform-feature (spec 108) — adapters live in `factory_adapters`
  DB table; UI at `app.factory.tsx`.
- Factory artifact substrate (spec 139) unified all adapter representations.

### Not wired (or not in the spec spine)

- **No machine-readable adapter status field.** The README needs to call
  `aim-vue-node` production and the other three validators. The codebase
  doesn't carry that distinction. Inferred from merge-history concentration
  (specs 138, 140, 141 all aim-vue-node) but not authoritative. See `gaps.md` G-1.
- **`factory_run.rs` does not emit a governance certificate.**
  `crates/factory-engine/src/governance_certificate.rs` defines the schema,
  `crates/factory-engine/src/bin/verify_certificate.rs` (binary name
  `verify-certificate`) verifies one given a JSON file path, 5 unit tests
  exercise the round-trip and tampering detection. The pipeline does not call
  the emit path. Spec 102 marks `implementation: complete`. See `gaps.md` G-2.

## Identity → policy → execution trust fabric

### Concrete trace (works today)

1. **Identity (Rauthy):** `platform/charts/rauthy/` — Helm chart with HA
   (3 replicas, hiqlite embedded Raft), ingress at `rauthy.localdev.online`.
   Spec 106 (`rauthy-native-oidc-and-membership`, `implementation: complete`,
   merged 2026-04-17). Issues JWTs with custom `oap` scope per FR-002.
2. **Policy gate (deployd-api):** `platform/services/deployd-api-rs/`
   — axum + hiqlite. `src/config.rs` reads `DEPLOYD_REQUIRED_SCOPE`;
   `src/routes.rs` calls `auth::has_scope(&claims, &state.config.required_scope)`
   on every request. Token validation against the OIDC endpoint is real.
3. **Spec-bound execution (factory-engine + policy-kernel):** The
   policy-kernel (spec 047) provides 5-tier settings merge, SHA-256 proof
   chains with standalone verifier, permission runtime with glob matching,
   JSONL audit logger. 40+ tests.

### The gap in the trace

The path **deployd-api scope check → spec-defined permission via
policy-kernel** is not live. The scope check is hardcoded against
`DEPLOYD_REQUIRED_SCOPE`. There is no code that maps an OIDC scope to a
policy-kernel permission tier the way spec 137
(`tenant-environment-access-gates`, status: **draft**) defines.

**README posture:** describe the working trace honestly; do not claim the
scope→spec-permission binding works yet.

## Multi-cloud — what's procurement-real

`platform/charts/`:

- `deployd-api/` — chart present
- `rauthy/` — chart present (production-ready per spec 106)
- `stagecraft/` — chart present
- `tenant-hello/` — chart present (spec 136, demo tenant)

`platform/infra/terraform/envs/`:

- `dev/core/` — Azure resource group, ACR, KeyVault (real)
- `dev/cluster/` — AKS cluster (real)
- **No `aws-dev/`, `gcp-dev/`, `do-dev/` directories.** AWS/GCP/DO Terraform
  modules exist under `platform/infra/terraform/modules/{aws,gcp,do}_core/`
  but no environment instantiates them.

`platform/infra/hetzner/` — separate deployment root, Helm-direct (not
Terraform). Operational but outside the two-tier core/cluster model.

`make deploy-azure | deploy-aws | deploy-hetzner` — `aws` target points at
`envs/aws-$(ENV)/core` which doesn't exist; would fail. `azure` and
`hetzner` work.

**README posture:** "AKS production-deployed; AWS/GCP/DO charts are
cloud-neutral, Terraform modules ready, environments not yet
instantiated; Hetzner K3s operational via standalone path." This is
procurement optionality framed honestly: the Helm and chart layer survives
the move; the IaC layer needs an environment per cloud.

## Governance artifacts

### What emits real machine-truth artifacts today

| Artifact | Producer | Demo-able? |
|---|---|---|
| Spec registry (`build/spec-registry/registry.json`) | `make spec-compile` | Yes |
| Codebase index (`build/codebase-index/index.json`, rendered to `CODEBASE-INDEX.md`) | `make index` / `make index-render` | Yes |
| OWASP ASI 2026 compliance map (JSON) | `registry-consumer compliance-report --framework owasp-asi-2026 --json` | Yes — load-bearing demo |
| Spec/code coupling gate result | `tools/spec-code-coupling-check` | Yes (PR-time) |
| Schema parity walker (Rust ↔ TS contracts) | `make ci-schema-parity` | Yes |
| Supply chain (cargo-deny, pnpm/npm audit) | `make ci-supply-chain` | Yes (blocking from day 0, spec 116) |
| **Governance certificate** | (no end-to-end producer) | **No** — see G-2 |

### Compliance map — actual coverage

`registry-consumer compliance-report --framework owasp-asi-2026 --json`
returns:

- ASI01, ASI03, ASI05, ASI07, ASI09, ASI10 → `102-governed-excellence`
- ASI02, ASI04, ASI06, ASI08 → unmapped

Six of ten ASI controls map to one spec. The CLI works, the corpus is
shallow. README says "OWASP ASI 2026 mappings are queryable" — true and
defensible — without overclaiming density.

## Adapter coverage (the production-vs-validator question)

The user prompt requires the README to "name the production-supported
adapter explicitly. Mark the other three as factory-contract validators."

**Investigation found:**

- All four adapters are registered identically in
  `platform/services/stagecraft/api/factory/oapNativeAdapters.ts`.
- No spec carries an `adapter_status: production | validator` field.
- Recent merges concentrate on `aim-vue-node`: spec 138
  (`stagecraft-create-realised-scaffold`), spec 140 (`aim-vue-node-scaffold-source-id-cutover`),
  spec 141 (`aim-vue-node-source-id-template-name-alignment`).
- This concentration is evidence — not proof — that aim-vue-node is the
  production target.

**README posture (chosen):** name `aim-vue-node` as "production-supported
(active scaffold target)" with merge-history evidence; mark
`next-prisma`, `rust-axum`, `encore-react` as "factory-contract validated"
to honour the user's framing intent without overstating what governed
metadata supports. Flag in `gaps.md` G-1 so a future spec can codify
this.

## What was NOT explored

- Per-spec contents (only frontmatter via `registry-consumer show`).
- Individual adapter source files. The codebase index gave the spec→path
  mapping; that was sufficient.
- Tauri command surface. The desktop app is referenced in the architecture
  diagram (kept from old README); no new claims were made.
