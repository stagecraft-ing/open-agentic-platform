---
id: "167-born-with-spec-spine-kernel"
slug: born-with-spec-spine-kernel
title: "Born-with spec-spine kernel emission — every produced project ships with a spine"
status: approved
implementation: complete
owner: bart
created: "2026-05-22"
amended: "2026-05-23"
kind: capability
risk: high
depends_on:
  - "000"  # bootstrap-spec-system (the kernel content)
  - "001"  # spec-compiler-mvp
  - "075"  # factory-workflow-engine
  - "120"  # factory-extraction-stage (the spec this spec extends per intent doc §9.8)
  - "127"  # spec-code-coupling-gate (kernel includes coupling gate wiring)
  - "147"  # spec-kind-grammar
  - "165"  # opc-decomposition-pipeline (born-with case routes through this)
code_aliases: ["BORN_WITH_KERNEL", "SPEC_SPINE_KERNEL_EMISSION"]
establishes:
  - unit: { kind: directory, path: crates/factory-engine/src/kernel_emission }
  - unit: { kind: file, path: crates/factory-engine/src/kernel_emission/mod.rs }
  - unit: { kind: file, path: crates/factory-engine/src/kernel_emission/version.rs }
  - unit: { kind: file, path: crates/factory-engine/src/kernel_emission/gather.rs }
  - unit: { kind: file, path: crates/factory-engine/src/kernel_emission/templates.rs }
  - unit: { kind: file, path: crates/factory-engine/src/kernel_emission/adapter_specs.rs }
  - unit: { kind: file, path: crates/factory-engine/src/kernel_emission/emit.rs }
  - unit: { kind: file, path: crates/factory-engine/templates/kernel/tenant-ci.yml.tmpl }
  - unit: { kind: file, path: crates/factory-engine/templates/kernel/tenant.makefile.tmpl }
  - unit: { kind: file, path: crates/factory-engine/tests/kernel_emission_integration.rs }
extends:
  - spec: "120-factory-extraction-stage"
    nature: additive
    unit: { kind: directory, path: crates/factory-engine }
  - spec: "075-factory-workflow-engine"
    nature: additive
    unit: { kind: file, path: crates/factory-engine/src/engine.rs }
  - spec: "075-factory-workflow-engine"
    nature: additive
    unit: { kind: file, path: crates/factory-engine/src/lib.rs }
references:
  - role: decomposition-source
    unit: { kind: file, path: docs/owasp/factory/AIDE-VELOCITY-OAP-INTENT.md }
  - role: kernel-source
    unit: { kind: file, path: specs/000-bootstrap-spec-system/spec.md }
  - role: contract-substrate
    unit: { kind: directory, path: standards/spec }
  - role: gate-substrate
    unit: { kind: file, path: tools/spec-spine/spec-code-coupling-check/Cargo.toml }
summary: >
  When `factory-engine` produces a new project from an
  adapter, the resulting repo MUST ship with a
  pre-populated spec-spine kernel: a copy of
  `specs/000-bootstrap-spec-system/spec.md`, a
  `standards/spec/` directory cloned from OAP's current
  refinement, a compiled `.derived/spec-registry/registry.json`
  from the initial bootstrap, and tenant-side analogues
  of the coupling gate (127/130/133) wired into the
  tenant's CI.

  The kernel is *versioned* and *content-hash-anchored*.
  When OAP refines its own spec-spine (e.g., the
  130/152 relationship-graph work), tenants born earlier
  carry an older kernel. The kernel hash recorded in
  the tenant's repo is the substrate for spec 167's
  companion kernel-update propagation (deferred to
  follow-up spec).

  Spec 167 turns OAP from "a substrate that runs spec-
  spined work" into "a substrate that *produces* spec-
  spined projects by construction." Every project the
  factory emits inherits the discipline as a matter of
  birth.
---

# 167 — Born-with spec-spine kernel emission

## 1. Problem

OAP's spec spine is the load-bearing governance kernel: 160
specs compile deterministically to `registry.json`; drift
between code and spec fails CI; the relationship graph captures
who establishes / extends / refines / supersedes / amends what;
the coupling gate (127/130/133) is the structural defence
against engineered drift.

A tenant project produced by factory-engine today does *not*
inherit this discipline. The intent doc §6 names the gap:

> *"OAP's spec-spine is not only the substrate's internal
> governance — it is a **portable governance kernel** that every
> tenant project should carry inside itself. Two channels:
> Born-with [factory emits the kernel into the scaffold] and
> Retrofit [the same kernel installed after the fact, scanning
> existing code to seed relationships]."*

The retrofit channel is owned by spec 165 (the decomposition
pipeline applied to imported projects). The born-with channel
needs its own spec because the production-time semantics differ:

- The factory adapter knows what it scaffolded — it can seed
  the kernel with richer initial specs than a retrofit can.
- The factory pipeline runs once at project creation; the
  kernel emission is a one-shot, not an iterative refinement.
- The kernel-hash anchoring lets OAP track *which kernel
  version each tenant was born under* so kernel updates can
  propagate cleanly.

Without spec 167, every L2 capability in the convergence work
(state-machine concurrency primitives, governance certificate,
identity scopes, drift gates, sandboxing) has to be re-asserted
per tenant by hand. With it, the substrate ships the discipline
as a kernel and the tenant inherits it as a matter of birth.

## 2. Decision

Add a kernel-emission step to the factory-engine pipeline.
When an adapter produces a project, the produced project's
working tree includes a fully-populated spec-spine kernel
*before* any adapter-specific code lands.

### 2.1 Kernel contents

The emitted kernel includes:

1. **`<project>/specs/000-bootstrap-spec-system/spec.md`** —
   verbatim copy of OAP's current spec 000.
2. **`<project>/standards/spec/`** — directory tree:
   `constitution.md`, `contract.md`, `templates/*`. Verbatim
   copy from OAP's current `standards/spec/`.
3. **`<project>/.derived/spec-registry/registry.json`** —
   pre-compiled from the kernel specs.
4. **`<project>/.kernel-version`** — a marker file recording:
   - The OAP commit SHA of the kernel source.
   - The content hash of the kernel directory.
   - The factory-engine version that emitted the kernel.
   - The adapter version + identity.
5. **Tenant-side gate wiring**:
   - `<project>/.github/workflows/ci-spec-code-coupling.yml`
     (or platform-equivalent CI config) that invokes a
     tenant-resident `spec-code-coupling-check` against the
     tenant's own spec spine.
   - `<project>/Makefile` (or platform-equivalent) target
     `pr-prep` invoking the codebase-indexer + coupling gate.
6. **Adapter-seeded initial specs** — the adapter contributes
   spec drafts capturing what it scaffolded. These specs
   declare appropriate `kind:` per spec 147, claim logical
   units for the scaffolded code per spec 154, and may carry
   `references:` to the adapter's manifest (per spec 161's
   provenance grammar, kind `knowledge` with the adapter
   manifest URI as the source).

### 2.2 Tenant-resident binaries

The kernel emission includes:

- A pre-compiled `spec-compiler` binary for the target
  platform.
- A pre-compiled `spec-code-coupling-check` binary.
- A pre-compiled `codebase-indexer` binary.
- A pre-compiled `spec-lint` binary.

These are emitted under `<project>/tools/spec-spine/` (or a
platform-equivalent location) so the tenant's CI can invoke
them without re-building from source.

Alternative: tenants point at a pinned-version OAP toolchain
distribution. The spec leaves this as an implementation
decision (`vendor-binaries` vs `pinned-toolchain-reference`)
with FR-008 specifying the chosen mode.

### 2.3 Kernel version anchoring

The `<project>/.kernel-version` file is the load-bearing
substrate for propagation. It records:

```yaml
kernel:
  source_commit: <SHA>
  source_hash: <content-hash>
  factory_engine_version: <semver>
  emitted_at: <ISO-8601>
adapter:
  id: <adapter-id>
  version: <semver>
  manifest_hash: <content-hash>
```

Future kernel-update propagation reads this file to decide
whether a tenant is "up-to-date" with current OAP refinements.
The propagation mechanism itself is out of scope for spec 167
(deferred to a follow-up spec named in §6).

## 3. Functional Requirements

- **FR-001** Every project produced by `factory-engine` from
  an adapter includes a populated spec-spine kernel before
  any adapter-specific code lands in the project's working
  tree.
- **FR-002** The kernel includes the four core files /
  directories named in §2.1.1–§2.1.4: spec 000, standards/spec/,
  .derived/spec-registry/registry.json, .kernel-version.
- **FR-003** The kernel includes tenant-side gate wiring (§2.1.5)
  whose default platform target is GitHub Actions; alternate
  CI platforms (GitLab CI, Azure Pipelines) are supported
  via per-adapter overrides.
- **FR-004** The adapter contributes initial spec drafts
  (§2.1.6) capturing the scaffold. Each draft satisfies spec
  147 grammar, spec 154 unit grammar, and (if derived from
  the adapter manifest) spec 161 emission contract.
- **FR-005** Tenant-resident spec-spine binaries (§2.2) are
  included with the project, OR the project's CI references
  a pinned-version OAP toolchain distribution. The chosen
  mode is recorded in `.kernel-version`.
- **FR-006** The `.kernel-version` file records the four
  source fields and four adapter fields (§2.3). The file is
  authoritative; tampering is detected by the future
  propagation mechanism.
- **FR-007** Running `make pr-prep` (or the platform-equivalent
  target) in a born-with project succeeds against the
  emitted kernel content — the coupling gate, lint, and
  index check all pass at project birth.
- **FR-008** A tenant project's CI runs the coupling gate
  against the project's *own* spec spine, not against OAP's.
  The gate uses the tenant-resident or pinned-toolchain
  binaries.
- **FR-009** Re-running the factory pipeline on the same
  adapter + input set produces an identical kernel
  (deterministic emission). Hash-equal `.kernel-version` is
  the verification.

## 4. Success Criteria

- **SC-001** A new project produced from `aim-vue-node` (or
  any production-supported adapter) contains the full
  kernel at the first commit; the project's own coupling
  gate passes against the initial commit.
- **SC-002** The `.kernel-version` file in the produced
  project records the correct source commit, content hash,
  and adapter metadata.
- **SC-003** Running the project's CI workflow on a
  hand-introduced spec/code drift (e.g., a code edit
  without a matching spec edit) fails the coupling gate.
- **SC-004** Two factory runs on the same adapter + input
  produce hash-equal kernels (deterministic emission).
- **SC-005** A born-with project displays its decomposed
  initial specs in stagecraft's Requirements view (spec
  163), each carrying the provenance badge pointing at the
  adapter manifest.

## 5. Scope

### In scope

- The kernel content definition.
- The factory-engine pipeline integration that emits the
  kernel.
- The tenant-side gate wiring template.
- The tenant-resident binary distribution OR pinned-toolchain
  reference (one chosen by FR-005).
- The `.kernel-version` marker file.

### Out of scope (deferred to follow-up spec)

- **Kernel-update propagation cadence and compatibility
  policy.** Intent doc §7 OQ-8 names this as an open
  question. When OAP refines its own spec-spine, how do
  born-earlier tenants opt into the refinement? The
  `.kernel-version` file is the substrate; the propagation
  mechanism is its own spec.
- **Multi-language tenant-resident binaries.** The first
  cut ships binaries for the host platform of the adapter
  run. Cross-compilation matrices are out of scope.
- **Kernel customisation per tenant.** The kernel is OAP's
  canonical kernel; tenants may not edit `specs/000` or
  `standards/spec/`. They may add new specs and (post-birth)
  refine their own corpus normally.
- **Retrofit channel.** Spec 165 owns the retrofit case
  (decomposition pipeline applied to existing projects).
  Spec 167 covers born-with only.

## 6. Cross-references

- **INTENT doc** §6.5, §9.8.
- **Spec 000** — bootstrap-spec-system; the kernel content
  source.
- **Spec 120** — factory-extraction-stage; spec 167
  `extends:` this for the born-with seeding case.
- **Spec 075** — factory-workflow-engine; the pipeline
  that gains the kernel-emission step.
- **Spec 127 / 130 / 133** — coupling gate; tenant CI
  wiring invokes equivalents.
- **Spec 147** — kind grammar; adapter-seeded specs declare
  `kind:`.
- **Spec 154** — logical-unit grammar; adapter-seeded specs
  declare units.
- **Spec 156 / 161** — provenance grammar / emission
  contract; adapter-seeded specs use these to point at the
  adapter manifest.
- **Spec 165** — decomposition pipeline; born-with case
  routes through this for stages 2–6 (xray, semantic,
  callgraph, lineage, synthesis) when adapter content is
  insufficient to seed specs directly.
- **Follow-up — kernel-update propagation spec.** Open
  per intent doc §7 OQ-8.

## 7. Implementation status (2026-05-23)

The kernel-emission contract lands as a library at
`crates/factory-engine/src/kernel_emission/` plus a thin
`FactoryEngine::emit_project_kernel` entry point. The cut is
deliberate: the contract is settled and tested; the production
pipeline auto-fire and the binary-vending step are scoped as
follow-ups so the contract can land without dragging in
release-engineering plumbing.

### Done

- **FR-001 / FR-002.** `emit_kernel()` writes spec 000,
  `standards/spec/**`, the pre-compiled registry, the
  `.kernel-version` marker, tenant gate wiring (workflow +
  Makefile), and the adapter-seeded scaffold-claim spec in
  one atomic call. Test:
  `kernel_emission::emit::tests::emits_full_kernel_layout`.
- **FR-003.** Default platform target is GitHub Actions
  (`crates/factory-engine/templates/kernel/tenant-ci.yml.tmpl`).
  Per-adapter overrides for GitLab CI / Azure Pipelines are
  parametric via `TenantGateContext` but ship without
  per-platform branching; see Deferred below.
- **FR-004.** `build_scaffold_claim_spec()` emits one draft
  conforming to spec 147 (`kind: capability`), spec 154
  (`establishes:` units from the adapter scaffold paths or a
  repo-root fallback), and — when an adapter manifest URI is
  provided — spec 161 (`references: knowledge-source`).
  Test:
  `kernel_emission::adapter_specs::tests::body_includes_required_frontmatter_keys`.
- **FR-005 (mode field).** `ToolchainMode::VendorBinaries` /
  `PinnedToolchain` is recorded in `.kernel-version`. The
  actual vending of the binaries themselves is Deferred.
- **FR-006.** `.kernel-version` round-trips through
  `KernelVersion::{to_yaml, from_yaml}` with the four
  `kernel.*` fields and three `adapter.*` fields plus
  `toolchain_mode`.
- **FR-008.** The emitted workflow invokes the tenant-resident
  `spec-code-coupling-check` against the tenant's own base/head
  refs.
- **FR-009.** `compute_kernel_hash()` over sorted entries is
  deterministic. Test:
  `kernel_emission::emit::tests::deterministic_emission_yields_hash_equal_kernels`.

### Deferred (follow-up specs, not regressions)

- **Production pipeline auto-fire.** `emit_project_kernel` is
  reachable on `FactoryEngine` and unit-tested, but no
  `transition_to_*` hook fires it automatically yet. The
  intended insertion point is "before the first adapter write
  in Phase 2"; wiring that requires resolving the tenant
  project root from `FactoryEngineConfig`, which is its own
  small spec.
- **Tenant binary vending (FR-005 binaries side).** Cross-target
  builds for `spec-compiler`, `spec-code-coupling-check`,
  `codebase-indexer`, `spec-lint` go through the OAP release
  pipeline (spec 117). The kernel-emitter records the mode
  but does not currently copy binaries into
  `<project>/tools/spec-spine/`. Vending mechanism is a
  follow-up.
- **Per-CI-platform overrides (FR-003 alternates).** GitLab
  CI / Azure Pipelines templates are not bundled yet. The
  `TenantGateContext` substitution machinery accepts the
  context the alternate templates would need; adding the
  templates themselves is straightforward but out of MVP
  scope.
- **SC-001 / SC-005 full E2E.** Driving a real `aim-vue-node`
  factory run end-to-end and verifying the produced project's
  CI passes and its specs surface in stagecraft's
  Requirements view (spec 163) requires the production
  pipeline auto-fire above. The contract tests demonstrate
  the kernel content is well-formed; the live integration
  test follows the auto-fire spec.
