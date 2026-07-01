---
id: "220-tenant-emit-governance-certificate"
title: "Tenant-Emit Governance Certificate (the emitter, its firing, and the tenant signer identity)"
feature_branch: "feat/220-tenant-emit-governance-certificate"
status: approved
implementation: in-progress
kind: capability
domain: platform
created: "2026-06-20"
authors: ["open-agentic-platform"]
language: en
summary: >
  The residual R-2 emit leg. Spec 168 made the certificate emitter capable of
  tenant-mode emission (a required signer, the run-dir post-hoc path) and spec
  219 vended the verify-only half (tenant-tail). But the emitter never reached a
  produced app: tenant-tail is verify-only by construction, and spec 209 FR-003
  explicitly deferred the emitter, its firing, and the tenant signer identity to
  this spec. A born-with app can therefore re-check a certificate it is handed
  but cannot produce one of its own, so spec 209's verify-certificate CI step is
  wired but dormant and the audit chain still terminates at the tenant boundary.
  Spec 220 closes that: it delivers a tenant-side emitter (the post-hoc
  build-certificate path, which needs only a laid-out run directory and a signing
  key, not OAP's pipeline orchestration), fires it as the terminal step of a
  tenant run (the spec 168 FR-002 "automatic at completion" guarantee), and
  defines the tenant signer identity and key custody (an operator-supplied
  Ed25519 secret outside the agent's write scope, an attributable principal,
  anonymous signing forbidden). The emitted certificate is Ed25519-signed and
  self-authenticating but carries no platform countersign (a tenant run is
  outside OAP's admission/grant flow), so it verifies offline as
  "verifiable-but-unsealed" under tenant-tail verify-certificate. This activates
  spec 209's dormant verify step and extends the independently-verifiable audit
  chain one commit past handoff.
code_aliases: ["TENANT_EMIT_GOVERNANCE_CERTIFICATE", "TENANT_CERT_EMIT"]
compliance:
  # Same controls spec 168 carries: a tenant's pipeline producing verifiable
  # provenance (ASI04) and an auditor who does not depend on the tenant's
  # narrative (ASI09). 220 is the leg that makes 168's claims true tenant-side.
  - framework: "owasp-asi-2026"
    controls: ["ASI04", "ASI09"]
depends_on:
  - "102-governed-excellence"
  - "168-per-project-governance-certificate"
  - "167-born-with-spec-spine-kernel"
  - "198-factory-governance-envelope"
  - "209-tenant-kernel-ci-enforcement"
  - "218-run-cert-corpus-binding"
  - "219-tenant-tail-verifier-toolkit"
establishes:
  # Spec 220 FR-003 (OQ-3): the platform mints the tenant's Ed25519 signing key
  # and sets it as the produced repo's OAP_SIGNING_KEY Actions secret at project
  # creation (tenant = project = repo). New stagecraft module: mint a 32-byte
  # Ed25519 seed (standard base64, the emitter's `decode_seed` shape) and PUT it
  # as a libsodium sealed-box Actions secret. Plus its unit test.
  - unit: { kind: file, path: platform/services/stagecraft/api/projects/scaffold/tenantSigningKey.ts }
  - unit: { kind: file, path: platform/services/stagecraft/api/projects/scaffold/tenantSigningKey.test.ts }
extends:
  # Same featuregraph-golden precedent specs 196/194/193/187/183/209/219 follow.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
  # 220 additively extends the emitter 168 established: the post-hoc
  # build_certificate.rs gains a --require-operator-key flag (FR-003) that refuses
  # the ephemeral fallback in production, plus the tenant firing posture. Edit
  # confined to build_certificate.rs; governance_certificate.rs is untouched
  # (203/Dev1 owns the next cert-version bump there, see "Decisions" below).
  - spec: "168-per-project-governance-certificate"
    nature: additive
    unit: { kind: file, path: crates/factory-engine/src/bin/build_certificate.rs }
  # 220 extends 218's corpus binding (read, never recompute) from the in-process
  # factory_run.rs read-path to the post-hoc build_certificate.rs path, mirroring
  # the same pattern: read OAP_CORPUS_ATTESTATION_PATH, hash via the public
  # spec_spine_core::attest::attestation_hash reader seam, and call the public
  # CertificateBuilder::corpus_binding(). No cert schema change (reuses the 1.6.0
  # CorpusBinding); no governance_certificate.rs edit.
  - spec: "218-run-cert-corpus-binding"
    nature: additive
    unit: { kind: file, path: crates/factory-engine/src/bin/build_certificate.rs }
  # 220 adds its operator-key (FR-003) and corpus-binding (FR-007) tests to
  # 168's tenant-emission integration suite, the shared file 168 established
  # for these tests. Additive: new test fns, no change to 168's own cases.
  - spec: "168-per-project-governance-certificate"
    nature: additive
    unit: { kind: file, path: crates/factory-engine/tests/tenant_emission_integration.rs }
  # FR-003 provisioning is invoked from spec 112's Create flow: create.ts gains a
  # `provision-signing-key` step (mint + set the secret before the first push, so
  # commit #1's born-with CI fires build-certificate --require-operator-key against
  # a resolvable operator key) and records the signer public key in the
  # project.created audit. Additive edit to the spec-112-established file.
  - spec: "112-factory-project-lifecycle"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/api/projects/create.ts }
  # The matching `provision-signing-key` ScaffoldStep union member is an additive
  # edit to the spec-140-established scaffold/types.ts.
  - spec: "140-acme-vue-node-scaffold-source-id-cutover"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/api/projects/scaffold/types.ts }
  # The signer minting needs libsodium-wrappers (Ed25519 seed + GitHub sealed-box
  # secret). Additive dependency on the spec-116-owned stagecraft package.json;
  # spec 116's supply-chain gate independently audits the new dependency.
  - spec: "116-supply-chain-policy-gates"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/package.json }
refines:
  # Leg 3 (kernel wiring): 220 fills the emitter slot spec 219 left deferred
  # (`status: pending-spec-220`). The born-with kernel toolchain manifest now
  # pins the released `tenant-emit` npm package and its `build-certificate`
  # invoke, mirroring the tenant-tail verifier block 219 landed in this same
  # file (FR-001 "kernel-pinned next to tenant-tail and spec-spine"). Same
  # `refines:`-by-aspect authority pattern 219 used for the toolchain template.
  - aspect: "emitter-pin-tenant-emit"
    unit: { kind: file, path: crates/factory-engine/templates/kernel/toolchain.yaml.tmpl }
  # The render test in kernel_emission/templates.rs asserts the deferred
  # `pending-spec-220` marker; filling the slot flips that assertion to the
  # pinned tenant-emit emitter block in lockstep with the template. This edge
  # makes the test change coupling-gate clean (mirroring 219's
  # `toolchain-render-tests-npm-pin` edge for the same file).
  - aspect: "toolchain-render-tests-emitter-pin"
    unit: { kind: file, path: crates/factory-engine/src/kernel_emission/templates.rs }
amends:
  # Clarification (not supersession): 168 FR-001/FR-002 (a tenant ships and fires
  # the emitter, automatic at completion) were capability-complete in-engine under
  # 168; the tenant-facing DELIVERY is this spec. See "Relationship to spec 168"
  # below and 168's "Amendments received" callout, landed in the same change.
  - "168-per-project-governance-certificate"
references:
  # Extraction SOURCES and context (not authority). The emit core stays in OAP
  # and keeps working; whatever tenant distributable carries it is tracked in
  # prose (the same cross-repo posture spec 219 records for tenant-tail and spec
  # 209 records for the template-encore CI leg). These are NOT extends edges
  # because OAP's behavior is preserved, not extended.
  - role: context
    unit: { kind: file, path: crates/factory-engine/src/governance_certificate.rs }
  # build_certificate.rs was promoted from a context reference to an owning
  # extends edge (above): 220 now modifies it. governance_certificate.rs stays a
  # context reference: 220 calls its public API (CertificateBuilder::corpus_binding,
  # the Signer types) but does not edit it, leaving that file to 203/Dev1's bump.
  - role: context
    unit: { kind: file, path: crates/factory-engine/src/platform_jws.rs }
  - role: context
    unit: { kind: file, path: specs/168-per-project-governance-certificate/spec.md }
  - role: context
    unit: { kind: file, path: specs/209-tenant-kernel-ci-enforcement/spec.md }
  - role: context
    unit: { kind: file, path: specs/219-tenant-tail-verifier-toolkit/spec.md }
---

# Feature Specification: Tenant-Emit Governance Certificate

**Feature Branch**: `feat/220-tenant-emit-governance-certificate`
**Created**: 2026-06-20
**Status**: Draft, decisions resolved 2026-06-21 (the residual R-2 emit spec
deferred by specs 209 FR-003 and 219; OQ-1..OQ-4 are now decided, see "Decisions")
**Input**: Spec 219's R-1 read concluded that the certificate verify core extracts
cleanly (shipped as tenant-tail) while "the emitter (`build-certificate`) is
identity-bearing and harness-bound and is explicitly NOT here; it ships with its
firing in a future emit spec (residual R-2)." Spec 209 FR-003 reframed the same
boundary: the tenant-emit leg "is deferred to the future emit spec (residual
R-2): tenant-tail is verify-only by construction." This spec is R-2.

## Relationship to spec 168 (capability landed; delivery deferred to here)

Spec 168 (`per-project-governance-certificate`) is marked
`implementation: complete`, and at the **capability** level it is: the emitter
gained a tenant-mode path that requires an attributable signer
(`governance_certificate.rs` `Signer` constructor rejects an empty subject), a
post-hoc build path from a run directory (`build_certificate.rs`), and a tenant
emission integration test (`tenant_emission_integration.rs`). What 168 did **not**
deliver is the *tenant-facing* guarantee its FR-001/FR-002 assert: that a produced
app "ships with both an emitter and a verifier" and that "emission is automatic at
run completion." Spec 219 subsequently vended only the verifier (tenant-tail,
verify-only by construction), and spec 209 FR-003 deferred the emitter explicitly.
So today a produced app has the *capability's verifier half* but cannot emit.

Spec 220 delivers the missing half. It does not supersede spec 168; it is the
delivery vehicle for 168 FR-001/FR-002 at the tenant boundary. **Reconciliation
(surfaced, not silently absorbed):** spec 168's `implementation: complete` reads
at the tenant-facing level as more than was delivered. This spec carries an
`amends: 168` edge, and the clarification callout already landed in spec 168's
"Amendments received" section when this spec was filed (#397, 168 `amended:
2026-06-20`): 168's FR-001/FR-002 were capability-complete
in-engine (the tenant-mode emitter, the required-signer enforcement, the post-hoc
build path, and the integration test), and the tenant-facing delivery (vending the
emitter, firing it, and the tenant key custody) lands here. The clarification does
not flip 168's status; it records that "complete" was capability-level and that
the tenant boundary is crossed by spec 220.

## Purpose

The certificate is the load-bearing artifact: a self-authenticating
`governance-certificate.json` an auditor verifies without trusting the system that
produced it (spec 102 FR-007). Spec 168's thesis is that the same discipline must
extend to the tenant boundary, or "the single audit chain you hand a regulator" is
true up to handoff and unverifiable one commit later. Spec 219 delivered the
tenant's ability to *check* a certificate; spec 209 wired a `verify-certificate`
step into the seeded tenant CI. But with no tenant emitter, that step is dormant:
it verifies a certificate only "when one is present," and none ever is. The chain
still terminates at the tenant boundary.

This spec lets a produced app **produce** its own certificate, so the verify step
has something real to check and the audit chain crosses the boundary.

## What exists today (grounding the scope)

The emit core is **post-hoc**, which is what makes tenant emission tractable
without shipping OAP's pipeline engine:

- **`build-certificate` reconstructs a certificate from a finished run directory.**
  It scans `<run-dir>/<stage-id>/<artifact-files>`, computes SHA-256 over each
  file, reads the frozen Build Spec, and writes
  `<run-dir>/governance-certificate.json`. It does **not** re-run the pipeline or
  require `FactoryPipelineState`. It accepts `--signer-subject` /
  `--signer-identity-provider` and resolves the signing key from
  `OAP_SIGNING_KEY` / `OAP_SIGNING_KEY_PATH` (operator-supplied) or an ephemeral
  fallback (marked untrusted).
- **The signer is identity-bearing.** The `Signer` field (subject,
  identityProvider, sessionId) names the principal; its constructor rejects an
  empty/whitespace subject (168 FR-007: anonymous signing forbidden). The Ed25519
  signing key is operator-supplied and lives outside the agent's write scope (102
  FR-008.1).
- **The inter-stage manifest chain (spec 170) is optional.** The emitter
  reconstructs it from `<run-dir>/manifests/` + `keychain.json` if present, or
  establishes a fresh root key; a tenant that does not sign hand-offs still emits
  a valid certificate.
- **The platform countersign (spec 198 FR-014) is applied post-emission by
  stagecraft on sync-back.** It is not part of emission. A certificate with no
  countersign is "verifiable-but-unsealed" and verifies offline (198 FR-014 AC-4).
- **tenant-tail `verify-certificate` already accepts this shape offline**
  (artifact-hash chain, Ed25519 signature, self-hash, optional inter-stage chain,
  optional platform seal, optional `--corpus-attestation`). The verifier round-trip
  is the contract spec 220 must satisfy.
- **The corpus-binding read-path exists in-process (spec 218).** `factory_run.rs`
  reads `OAP_CORPUS_ATTESTATION_PATH`, hashes the attestation via the public
  `spec_spine_core::attest::attestation_hash` reader seam, and binds it through
  `CertificateBuilder::corpus_binding()`. FR-007 mirrors that read-path onto the
  post-hoc `build_certificate.rs`; it is an extension of the emitter work, not a
  fourth independent gap.

The three concrete tenant-side gaps (the crux): **no emitter binary** reaches a
produced app, **no signer-key custody** model is defined for a tenant, and **no
firing point** invokes the emitter at a tenant run's completion.

## Functional requirements

- **FR-001: Tenant emitter delivered.** A produced app gains a vended emitter
  that runs the post-hoc certificate build against a tenant `.factory/runs/<run-id>/`
  directory and writes a signed `governance-certificate.json` there. Because spec
  219 makes tenant-tail verify-only **by construction** (no emitter verb, no
  emitter dependency, structurally testable), the emitter is a **separate**
  distributable, never a tenant-tail verb (preserving spec 219 FR-002 / AC-6).
  **Packaging (OQ-1, decided):** a dedicated `tenant-emit` repository, a sibling
  to tenant-tail. A Rust core with npm + py distributions produced at release,
  consuming the pinned `spec-spine`, carrying the same GitHub release/CI workflows
  tenant-tail does, and kernel-pinned so the born-with toolchain installs it next
  to tenant-tail and spec-spine. **License (OQ-2, decided):** Apache-2.0, matching
  tenant-tail and the rest of the vended toolchain (the sole copyright holder's
  prerogative, recorded here as an explicit decision). The emit core stays in OAP
  (`build_certificate.rs` + `governance_certificate.rs`); the `tenant-emit`
  distributable carries an extracted copy of both, kept in behavior parity exactly
  as tenant-tail does for the verifier. OAP-side, this spec modifies **only**
  `build_certificate.rs` (the FR-003 flag and the FR-007 corpus read-path);
  `governance_certificate.rs` is consumed through its public API and left
  unchanged for spec 203 (Dev1) to carry its next version bump.
- **FR-002: Firing at completion (automatic, not opt-in).** The tenant pipeline
  invokes the emitter as its terminal step on every run (success or halt),
  realizing spec 168 FR-002. In the born-with shape the firing is a step in the
  seeded CI / pipeline runner, the emit-side counterpart of the
  `verify-certificate` step spec 209 FR-001 already seeded. Once this step exists,
  spec 209's dormant verify step verifies a real certificate (closing that loop).
- **FR-003: Tenant signer identity and key custody.** Emission requires an
  attributable signer and a signing key the agent cannot mint:
  - The Ed25519 signing key is an **operator-supplied tenant secret** (a CI or
    deployment secret resolved via `OAP_SIGNING_KEY` / `OAP_SIGNING_KEY_PATH` or a
    tenant-scoped equivalent), held outside the repository and outside any agent's
    write scope (spec 102 FR-008.1 posture, spec 198 FR-014 "every agent is
    keyless" posture turned tenant-ward).
  - The `signer` field names the tenant principal: a Rauthy-issued JWT subject for
    human-driven runs (spec 168 FR-007, specs 106/137) or an attributable service
    identity for unattended runs.
  - **Anonymous signing is forbidden, and the ephemeral fallback is refused in
    production.** A run that cannot resolve a signer halts before emitting rather
    than writing a null-signer certificate (spec 168 FR-007, the spec 200 FR-004
    fail-closed posture). The ephemeral key fallback is dev-only and marks the
    certificate untrusted (`signing_attestation.kind: ephemeral`). A production
    tenant emission MUST pass the new `--require-operator-key` flag, under which the
    emitter exits non-zero with a named diagnostic if signing material resolves to
    `ephemeral` rather than `operator`. This is the **single OAP in-engine change**
    this spec makes: a CLI-level flag on `build_certificate.rs`. It is deliberately
    not pushed into `resolve_signing_material`, so OAP's own dev / CI runs keep
    their legitimate ephemeral path (confirmed by the OQ-4 read of the emit core).
  - **Key provisioning (OQ-3, decided).** Because a tenant is a project and a
    project is a repo, the platform that creates the project mints the tenant's
    Ed25519 key and sets it as the repo's CI secret (resolved by the emitter via
    `OAP_SIGNING_KEY` / `OAP_SIGNING_KEY_PATH`) at project-creation time. This
    collapses the apparent fork (a manually-configured CI secret vs a
    platform-issued per-tenant key) into one flow, and it is the same provisioning
    hook the deferred platform countersign / uplink (Out of scope) reuses to issue
    a sealing identity. A tenant that self-hosts may configure the CI secret by
    hand instead; the emitter contract is identical either way.
- **FR-004: Unsealed-but-verifiable posture.** A tenant run is outside OAP's
  admission/grant flow, so the emitted certificate carries **no platform
  countersign** (spec 198 FR-014). It is Ed25519-signed by the tenant signer and
  self-authenticating. tenant-tail `verify-certificate` accepts it offline and
  reports it "verifiable-but-unsealed" (198 FR-014 AC-4); `--require-sealed` fails
  it, which a tenant that has opted into a platform countersign may set. The
  platform countersign and the tenant-to-OAP certificate uplink are deferred (spec
  168 already defers the uplink; see Out of scope).
- **FR-005: Verifier round-trip closes the loop.** A certificate emitted under
  FR-001 verifies clean under tenant-tail `verify-certificate` (artifact-hash
  chain re-derived from the run dir, Ed25519 signature, self-hash). Tampering any
  referenced artifact, or any certificate field, fails verification with the spec
  102 / 168 diagnostic contract. The same certificate, fed to the spec 209 FR-001
  CI step, turns that step from dormant to enforcing. When the certificate carries
  a corpus binding (FR-007), the round-trip also covers it: tenant-tail
  `verify-certificate --corpus-attestation <file>` re-derives and matches the bound
  hash (spec 218 FR-003), and a mismatched or tampered attestation fails with the
  spec 218 diagnostic.
- **FR-006: Stage-grammar flexibility and determinism.** The emitter accepts the
  tenant's own stage shape (spec 168 §2.4: stages need only be representable as
  `{stage_id, input_hashes, output_hashes, runtime_metadata}`); it does not require
  the tenant pipeline to match OAP's s0..s6 grammar. Re-emitting from the same run
  directory produces byte-identical hashes (spec 168 FR-009), modulo the signer
  field which carries per-run identity.
- **FR-007: Tenant corpus binding (read, never recompute).** The tenant emitter
  binds the certificate to a corpus attestation over the tenant's **own** `specs/`
  corpus, the same chain edge spec 218 added to OAP's run cert, now extended to the
  post-hoc tenant path. The seeded tenant CI runs `spec-spine attest` over the
  produced app's corpus (every born-with app carries a `specs/` corpus and a pinned
  spec-spine, specs 167/209) to emit `attestation.json`; the emitter reads it via
  `OAP_CORPUS_ATTESTATION_PATH`, hashes it through the public
  `spec_spine_core::attest::attestation_hash` reader seam, and populates
  `corpus_binding` via the public `CertificateBuilder::corpus_binding()`. Read,
  never recompute (spec 218's load-bearing invariant): the emitter never compiles
  or re-attests the corpus, and the FR-002 deny/clippy guards (now in main) keep
  the extracted copy honest too. The binding is additive and optional exactly as in
  spec 218: an unbound tenant certificate is the named "unbound" state, not a
  failure. The bound attestation is the tenant's own governance ledger ("I ran a
  pipeline, and at this commit my spec corpus was in this consistent state"),
  distinct from OAP's platform corpus; the two certificates carry parallel,
  independently reproducible corpus links. **Scope confinement:** this wiring lands
  entirely in `build_certificate.rs` (mirroring the `factory_run.rs` read-path),
  reuses the 1.6.0 `CorpusBinding` with no schema change and no version bump, and
  does not touch `governance_certificate.rs` (left to spec 203's 1.7.0 SBOM bump).

## Acceptance criteria

- **AC-1.** A born-with app lays out a run under `.factory/runs/<run-id>/<stage-id>/`
  and, at run completion, the vended emitter writes a signed
  `governance-certificate.json` under that run directory with an attributable
  signer and `signing_attestation.kind: operator`.
- **AC-2.** tenant-tail `verify-certificate <cert> --artifact-dir <run-dir>` exits
  0 on that certificate; and the spec 209 FR-001 seeded CI `verify-certificate`
  step, run against a produced app that now emits, verifies a real certificate
  green end-to-end (the dormant step activates).
- **AC-3.** Tampering any artifact the certificate references, or any certificate
  field, makes `verify-certificate` exit 1 with a specific mismatch diagnostic.
- **AC-4.** A run with no resolvable signer (no `signer-subject`, no identity
  context) halts before emission with an attributable error; no null-signer
  certificate is written.
- **AC-5.** The emitted certificate verifies fully offline (no network), is
  reported "verifiable-but-unsealed," and `--require-sealed` fails it (there is no
  platform countersign on a tenant run).
- **AC-6.** The emitter is not reachable as a tenant-tail verb and tenant-tail's
  verify-only boundary (spec 219 FR-002, AC-6) is unbroken: the two tools remain
  distinct distributables.
- **AC-7.** Re-emitting from the same run directory yields identical artifact and
  certificate hashes (determinism), modulo the signer field.
- **AC-8.** With a tenant corpus attestation present, the emitted certificate
  carries `corpus_binding.corpus_attestation_hash` equal to that attestation's hash
  (FR-007), and tenant-tail `verify-certificate --corpus-attestation <file>`
  reports it verified; a mismatched or tampered attestation fails with the spec 218
  diagnostic; with no attestation supplied the certificate is emitted "unbound"
  (named, not a failure). The emitter performs no corpus recompute on any path.

## Out of scope

- **The platform countersign and the tenant-to-OAP uplink.** A tenant run does not
  pass through OAP's admission seal / run-grant / countersign flow (spec 198
  FR-014); the tenant certificate is unsealed by design. Sealing a tenant run, and
  aggregating tenant certificates into a portfolio audit view at the substrate, is
  the deferred uplink spec 168 already names. FR-004 leaves the `--require-sealed`
  hook in place for when it lands.
- **The certificate format and verdict logic.** Owned by specs 102/168 (cert),
  170 (inter-stage chain), 198 (platform seal), 121 (provenance). Spec 220 changes
  who emits and where the key lives, not what a valid certificate is.
- **The tenant pipeline / run-harness itself.** How a produced app lays out its own
  `.factory/runs/<run-id>/` (its adapter's stage grammar) is the tenant's pipeline
  concern; spec 220 requires only the post-hoc directory convention the emitter
  scans. Authoring a generic tenant pipeline runner is separate work.
- **Schema evolution across kernel generations.** When the spec 102 format evolves,
  born-earlier tenants carry an older emitter; compatibility is the deferred
  kernel-update-propagation concern (spec 167), not this spec.
- **OAP-side emission.** Specs 102/168 own OAP's own factory-run emission; 220 is
  strictly the tenant boundary.

## Decisions (the OQs, resolved)

The four open questions the draft carried are now decided; this section records the
resolution and the evidence behind it so implementation does not reopen them.

- **OQ-1 (packaging): a dedicated `tenant-emit` repository, kernel-pinned.** A
  sibling to tenant-tail: Rust core, npm + py at release, consuming the pinned
  spec-spine, with tenant-tail's GitHub workflows, pinned by the born-with kernel
  next to tenant-tail and spec-spine. Not folded into spec-spine (its ledger seal
  signs the corpus attestation, a different object than the run certificate, so the
  emitter cannot reuse it) and not a tenant-tail verb (spec 219's verify-only
  boundary is by construction). The dedicated-repo shape keeps the verify/emit
  separation legible while kernel-pinning keeps the binary, the operator key
  (OQ-3), and the firing step co-located.
- **OQ-2 (license): Apache-2.0**, matching tenant-tail and the rest of the vended
  toolchain.
- **OQ-3 (key provisioning): platform-mints-at-creation.** Tenant = project = repo,
  so the platform that creates the project mints the Ed25519 key and sets it as the
  repo CI secret at creation (see FR-003). The same hook the deferred uplink reuses
  to countersign.
- **OQ-4 (in-engine footprint): one CLI flag plus the corpus read-path, one file.**
  The OQ-4 read of the emit core confirmed the post-hoc path already carries the
  run-dir arg, `--tenant-mode`, the signer flags, `--stage-ids auto`, the `Signer`
  empty-subject rejection, and the unsealed-but-verifiable posture. The single
  genuine gap is FR-003's "production uses operator": the `--require-operator-key`
  flag on `build_certificate.rs` (CLI-level only). FR-007's corpus read-path is a
  second small addition to the same file, calling public API. Net OAP in-engine
  footprint: ~35 LOC in `build_certificate.rs`; `governance_certificate.rs`
  unchanged.

### Coordination with spec 203 (Dev1, produced-app SBOM attestation)

Spec 203's plan (merged via #403) is concurrently editing
`governance_certificate.rs` (new SBOM artifact structs after `CorpusBinding`, cert
version 1.6.0 -> 1.7.0) and references this spec's firing step. To avoid a
shared-file authority tangle and a version-bump race, spec 220 deliberately stays
out of `governance_certificate.rs`: it edits only `build_certificate.rs` and reaches
the cert types through their public API. Spec 203 owns the next cert-version bump;
spec 220 adds no field and no version. The two compose at the tenant firing point
(a tenant run generates its SBOM (203) and its corpus attestation (FR-007), then
emits one certificate (220) that can bind both), and that sequencing is the only
contract between them. This coordination is prose, not a registry edge (203 and 220
carry no formal relationship), tracked the way spec 218 tracks its cross-corpus
dependency.

## Sequencing

Implementable now, and the OQs are decided (see "Decisions"). The verifier and the
seeded `verify-certificate` CI step exist (specs 219 / 209); the emit core and its
post-hoc path exist in-engine; the certificate format, the corpus binding, and the
unsealed posture are settled (specs 102/168/198/218). The work splits into three
legs, mirroring the spec 219 shape (build the tenant distributable, then wire the
OAP-side leg that pins it):

1. **OAP engine (this repo, source of truth):** the `--require-operator-key` flag
   (FR-003) and the corpus read-path (FR-007) on `build_certificate.rs`, ~35 LOC in
   one file, plus a test. Lands first so the extracted copy is in behavior parity.
2. **`tenant-emit` repo (parallel session):** extract `build_certificate.rs` +
   `governance_certificate.rs`, package Rust + npm + py, port the
   `tenant_emission_integration` tests, consume the pinned spec-spine, carry
   tenant-tail's GitHub workflows, Apache-2.0, release (FR-001).
3. **OAP kernel wiring + closure:** the born-with kernel template pins the released
   `tenant-emit` and seeds the firing step (FR-002, the emit-side counterpart of
   the verify step spec 209 seeded) plus the `spec-spine attest` step (FR-007).

Closure is gated on AC-2: a real produced app emitting a certificate that the spec
209 CI step verifies green, which is also the end-to-end validation spec 209's own
AC-1 silently required, so landing this leg unblocks spec 209's closure.

## Implementation status (2026-07-01)

**OAP-side factory-engine cert work: landed (Legs 1 and 3-pin).** The engine
surface this spec owns is merged and tested on main:

- **Leg 1 (engine, #407).** `build_certificate.rs` carries the
  `--require-operator-key` flag (FR-003): a production tenant emission exits
  non-zero with a named diagnostic when signing material resolves to
  `ephemeral` rather than `operator`, so no untrusted certificate is written.
  It also carries the FR-007 corpus read-path: `resolve_corpus_binding()` reads
  `OAP_CORPUS_ATTESTATION_PATH` (or `--corpus-attestation`), hashes the supplied
  attestation via the public `spec_spine_core::attest::attestation_hash` seam,
  and binds it through `CertificateBuilder::corpus_binding()`. Read, never
  recompute: the emitter never compiles or re-attests the corpus. Three
  integration tests cover the operator-key halt, the operator-key pass, and the
  corpus round-trip (`tenant_emission_integration.rs`).
- **Leg 3 pin (#410).** The born-with kernel toolchain manifest
  (`toolchain.yaml.tmpl`) pins the vended `tenant-emit` emitter next to
  tenant-tail and spec-spine (FR-001 kernel-pinning); the `pending-spec-220`
  deferred marker is gone and the `templates.rs` render test asserts the pinned
  block.
- **Scope confinement held.** 220 touched only `build_certificate.rs` (engine)
  and the kernel template files; `governance_certificate.rs` was left untouched
  for spec 203's 1.7.0 SBOM bump, exactly as the "Coordination with spec 203"
  section required. No shared-file authority tangle occurred.

**2026-07-01: Legs 1 + 2 + FR-003 provisioning landed; only the live AC-2 run
remains, so implementation stays `in-progress` (not `complete`).** The three
external legs the prior status listed are now built:

1. **Leg 2 (`tenant-emit` published).** `tenant-emit@0.2.0` (unscoped) plus its
   five `@tenant-emit/cli-<triple>` platform packages are live on npm, so the
   kernel's `npx --no-install tenant-emit build-certificate` pin resolves.
   `build-certificate` carries the FR-003 `--require-operator-key`, FR-007
   `--corpus-attestation`, and spec 203 `--sbom-dir` flags.
2. **FR-002 firing step.** Seeded into the prebuilt template's external
   `spec-spine.yml` (`stagecraft-ing/template-encore`), gated on `.kernel-version`
   so it is dormant in the template and active in a produced app: a terminal
   `tenant-emit build-certificate <run-dir> --tenant-mode --require-operator-key
   --sbom-dir . --corpus-attestation attestation.json` after SBOM/audit
   generation (spec 203 FR-001/FR-002) and the FR-004 parity gate, with the
   existing spec 209 `verify-certificate` step extended to re-check the SBOM and
   corpus bindings (`--sbom-dir` / `--corpus-attestation`).
3. **FR-003 key custody (OQ-3): provisioning now built.** Stagecraft's Create
   flow mints a per-tenant Ed25519 seed and sets it as the produced repo's
   `OAP_SIGNING_KEY` Actions secret before the first push
   (`api/projects/scaffold/tenantSigningKey.ts`, wired from `create.ts`), so the
   born-with CI's `--require-operator-key` firing resolves an operator key rather
   than halting on the ephemeral fallback. The signer public key is recorded in
   the `project.created` audit for attribution.

**Remaining: the live AC-2 end-to-end.** With Legs 1-3 in place a real
produced-app run should emit a certificate the dormant spec 209 verify step then
accepts green. That single end-to-end demonstration (scaffold a tenant, let its
CI run) is the only closure gate left; the code path is complete. The OAP-side
cert engine and the platform-side key custody are done; the tenant boundary is
now crossed in code, pending the live-run confirmation.
