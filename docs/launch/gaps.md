# README launch gaps (Phase 5)

> Captured 2026-05-06 for May 12 launch. Each gap names what the new
> README cannot honestly claim, the spec or code path involved, what's
> missing, and what would need to ship before the claim is true.

The new `README.md` does not make any claim listed here. Each gap below is
an item the prompt asked for that the codebase doesn't yet support — the
README is silent or honestly hedged on each.

## G-1 — Adapter status is not codified [severity: medium]

- **What the prompt asked for:** "Name the production-supported adapter
  explicitly. Mark the other three as factory-contract validators."
- **Spec ID:** none. `074-factory-ingestion`, `075-factory-workflow-engine`,
  `108-factory-as-platform-feature`, `139-factory-artifact-substrate` are
  the relevant specs; none carry an `adapter_status` field.
- **Code path:** `platform/services/stagecraft/api/factory/oapNativeAdapters.ts`
  registers all four adapters identically (`aim-vue-node`, `next-prisma`,
  `rust-axum`, `encore-react`). No production/validator distinction in code.
- **What's missing:** machine-readable status field on the adapter manifest.
- **What would need to ship:** a spec amendment (likely on 108 or 139)
  adding `adapter_status: production | validator | roadmap`, with the
  factory_adapters DB table column to match. README would then read from
  the registry.
- **README disposition:** the rewrite names `aim-vue-node` as
  "production-supported (active scaffold target)" with merge-history as
  evidence (specs 138/140/141 all aim-vue-node) and marks the other three
  as "factory-contract validated." Honest hedging; not propagating false
  precision.

## G-2 — Governance certificate is not emitted by the pipeline [severity: critical for May 12]

- **What the prompt asked for:** "Try it — commands that produce a real
  governance certificate and traceability report from a fresh checkout."
- **Spec ID:** `102-governed-excellence` (status: `approved`,
  `implementation: complete`).
- **Code path:**
  - `crates/factory-engine/src/governance_certificate.rs` — schema +
    builder (works, 5 unit tests pass).
  - `crates/factory-engine/src/bin/verify_certificate.rs` — verifier
    binary (works on a JSON fixture).
  - `crates/factory-engine/src/bin/factory_run.rs` — **does not call
    certificate emission.** No `make` target, no `cargo run` from a fresh
    checkout, produces a certificate file.
- **What's missing:** the line in `factory_run.rs` (or its post-run hook
  in the orchestrator) that builds and writes
  `governance-certificate.json` after a successful pipeline run. Plus a
  `make verify-certificate FILE=...` target so the README can demo it.
- **Scope (assessed 2026-05-06):** **small.** The certificate API is
  fully exported from `crates/factory-engine/src/lib.rs` lines 44–46 —
  `CertificateBuilder`, `generate_certificate`, `persist_certificate`,
  `verify_certificate` are public. A test
  `generate_certificate_from_pipeline_state` (line 636 of
  `governance_certificate.rs`) already proves the
  `FactoryPipelineState → GovernanceCertificate` mapping. Wiring is the
  call site, not new logic. Estimated 1 day for someone fluent in the
  engine.
- **What would need to ship before the claim is true:**
  1. Pipeline-end emission wired in `factory_run.rs` — call
     `generate_certificate(...)` + `persist_certificate(&cert, output_dir)`
     at the success path.
  2. A reproducible end-to-end demo path (a small fixture pipeline run
     that produces a certificate).
  3. A `make` entrypoint for both emission and verification.
- **Recommendation (post-investigation):** **option 1** (wire emission)
  is viable as a one-day task. Option 2 (amend spec 102 to `partial`)
  remains the safe fallback if the wiring uncovers integration issues
  the test fixture didn't surface.
- **README disposition:** the "Try it" section demos the **traceability
  report** (`registry-consumer compliance-report --json`) which is real
  end-to-end. The certificate is described as "schema + verifier
  production-ready; pipeline emission is on the spec 102 closure path"
  — accurate to what the code supports today.
- **Recommended action before launch:** either land the pipeline
  emission so the demo can include a certificate, or flip spec 102's
  `implementation` field from `complete` to `partial` (CONST-005: spec
  must match code).

## G-3 — Multi-cloud "procurement optionality" works on Azure only [severity: medium]

- **What the prompt asked for:** "Position as procurement optionality, not
  technical agnosticism. GC runs Azure-heavy, uneven GCP, corner-case AWS,
  with provincial sovereignty constraints."
- **Spec ID:** `072-multi-cloud-k8s-portability` (status: `approved`,
  `implementation: complete`).
- **Code path:**
  - `platform/charts/{deployd-api,rauthy,stagecraft,tenant-hello}` —
    cloud-neutral Helm charts, real.
  - `platform/infra/terraform/envs/dev/{core,cluster}/` — Azure (real).
  - `platform/infra/terraform/modules/{aws,gcp,do}_core/` — modules exist
    but **no `envs/aws-dev/`, `envs/gcp-dev/`, `envs/do-dev/`** exist.
  - `platform/Makefile` `deploy-aws` references `envs/aws-$(ENV)/core`
    which does not exist; would fail.
  - `platform/infra/hetzner/` — separate Helm-direct deployment, real.
- **What's missing:** instantiated `envs/` directories for AWS/GCP/DO and
  validation that the modules apply cleanly.
- **What would need to ship:** at minimum, an `envs/aws-dev/` with a
  passing `terraform plan` and a CI gate that runs `terraform validate`
  per module per env.
- **README disposition:** the rewrite says "AKS production-deployed (dev
  environment); AWS, GCP, and DigitalOcean Terraform modules are ready,
  environments not yet instantiated; Hetzner K3s operational via a
  separate bootstrap path." This is the truth. No claim of
  drop-in-AWS-deploy is made.

## G-4 — OWASP ASI 2026 compliance coverage is shallow [severity: low]

- **What the prompt asked for:** the README should land for OWASP ASI
  practitioners.
- **Spec ID:** `102-governed-excellence` carries the only `compliance:`
  frontmatter — six controls (ASI01, ASI03, ASI05, ASI07, ASI09, ASI10).
- **Code path:** `tools/registry-consumer` `compliance-report` subcommand
  reads frontmatter and aggregates. Works correctly; the corpus is thin.
- **What's missing:** ASI02, ASI04, ASI06, ASI08 are unmapped. Spec 116
  (supply chain) addresses ASI04 materially but does not declare it in
  frontmatter.
- **What would need to ship:** content authoring — add `compliance:`
  frontmatter to specs that already address ASI02/04/06/08, particularly
  spec 116 (supply chain → ASI04) and any spec covering input/output
  filtering.
- **README disposition:** the rewrite says "OWASP ASI 2026 mappings are
  queryable through `registry-consumer compliance-report`" — true. It does
  not claim full ASI coverage. The actual JSON output is short enough to
  paste in the demo, which is intellectually honest.

## G-5 — Identity → spec-defined permission binding is not live [severity: medium]

- **What the prompt asked for:** "Identity, policy, and collaboration as
  one continuous trust fabric."
- **Spec ID:** `137-tenant-environment-access-gates` (status: **draft**).
- **Code path:**
  - Working: Rauthy issues OIDC tokens with custom `oap` scope (spec 106).
  - Working: `platform/services/deployd-api-rs/src/routes.rs` enforces
    `DEPLOYD_REQUIRED_SCOPE` on every request.
  - **Missing:** code that maps an OIDC scope to a policy-kernel
    permission tier scoped to a tenant environment.
- **What's missing:** spec 137 implementation (currently planning artifacts
  only — `plan.md` and `tasks.md`).
- **What would need to ship:** spec 137 closure — environment-scoped scope
  routing in deployd-api, with the policy-kernel tier as the resolver.
- **README disposition:** the rewrite says "Rauthy-issued OIDC tokens gate
  deployd-api scope checks; spec-scoped tenant permissions are under
  spec 137 (in planning)." Clear about what's wired and what's roadmap.

## G-6 — `CONTRIBUTING.md` did not exist [severity: low — **resolved 2026-05-06**]

- **What the prompt asked for:** "Move the 'Claude-native development'
  section to `CONTRIBUTING.md`."
- **Resolution:** `CONTRIBUTING.md` created at the repo root. The cut
  Claude-native section landed there, expanded with spec-first
  conventions, the CONST-005 adversarial-prompt-refusal rule, the local
  validation matrix, and commit hygiene rules. The README's `Layout`
  table can optionally be updated to reference it; the
  `## How it works` section already implies it via the `.claude/` row.
- **Appendix below preserved for archival** — the source content is now
  in `CONTRIBUTING.md`, but the cut is recorded for traceability.

---

## Appendix: cut content (Claude-native development)

The following section was cut from the README per Phase 4 of the prompt.
It is preserved here so it can be moved into `CONTRIBUTING.md` (or
elsewhere) without re-deriving it.

> ### Claude-native development
>
> This repository ships with first-class [Claude
> Code](https://docs.anthropic.com/en/docs/claude-code) integration in
> `.claude/`:
>
> - **Agents** — `architect`, `explorer`, `implementer`, `reviewer`,
>   `encore-expert`
> - **Commands** — `/init`, `/commit`, `/code-review`, `/review-branch`,
>   `/implement-plan`, `/research`, `/validate-and-fix`, `/cleanup`,
>   `/refactor-claude-md`
> - **Rules** — orchestrator behavioural rules (step ordering, file-based
>   artifact passing, checkpoint discipline, governed artifact reads)
>
> Combined with `CLAUDE.md` (project conventions) and `AGENTS.md` (session
> init protocol), this is the environment the platform is built in.

## Severity summary for May 12 launch

| ID | Gap | Severity | Blocks launch? |
|---|---|---|---|
| G-2 | Certificate not pipeline-emitted | **Critical** | If the launch promises a working certificate demo, **yes**. The README rewrite avoids the promise. If the launch holds itself to spec 102's existing `implementation: complete` claim, **also yes** (CONST-005 drift). |
| G-1 | Adapter status not codified | Medium | No. README hedges. Codify post-launch. |
| G-3 | Multi-cloud envs incomplete | Medium | No. README is honest. Codify post-launch. |
| G-5 | Spec-bound permission gate | Medium | No. README labels spec 137 as roadmap. |
| G-4 | ASI coverage shallow | Low | No. The CLI works; corpus density is content. |
| G-6 | No `CONTRIBUTING.md` | Low | **Resolved 2026-05-06.** |

**Recommended pre-launch decision:** resolve G-2 by either wiring
pipeline emission (preferred) or amending spec 102's `implementation`
field. Both options are CONST-005-compliant. The README rewrite is
neutral on which is chosen.
