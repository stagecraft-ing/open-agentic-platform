---
id: "220-tenant-emit-governance-certificate"
title: "Tenant-Emit Governance Certificate (the emitter, its firing, and the tenant signer identity)"
feature_branch: "feat/220-tenant-emit-governance-certificate"
status: draft
implementation: pending
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
  - "219-tenant-tail-verifier-toolkit"
extends:
  # Same featuregraph-golden precedent specs 196/194/193/187/183/209/219 follow.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
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
  - role: context
    unit: { kind: file, path: crates/factory-engine/src/bin/build_certificate.rs }
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
**Status**: Draft (the residual R-2 emit spec deferred by specs 209 FR-003 and 219)
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
`amends: 168` edge and lands a clarification callout in spec 168's "Amendments
received" section in the same change: 168's FR-001/FR-002 were capability-complete
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
  optional platform seal). The verifier round-trip is the contract spec 220 must
  satisfy.

The three concrete tenant-side gaps (the crux): **no emitter binary** reaches a
produced app, **no signer-key custody** model is defined for a tenant, and **no
firing point** invokes the emitter at a tenant run's completion.

## Functional requirements

- **FR-001: Tenant emitter delivered.** A produced app gains a vended emitter
  that runs the post-hoc certificate build against a tenant `.factory/runs/<run-id>/`
  directory and writes a signed `governance-certificate.json` there. Because spec
  219 makes tenant-tail verify-only **by construction** (no emitter verb, no
  emitter dependency, structurally testable), the emitter is a **separate**
  distributable, never a tenant-tail verb. Its exact packaging (a dedicated emit
  repo mirroring tenant-tail's shape, vs a binary shipped through the born-with
  kernel) is OQ-1. The emit core stays in OAP (`build_certificate.rs` +
  `governance_certificate.rs`); the tenant distributable carries an extracted copy
  kept in behavior parity, exactly as tenant-tail does for the verifier.
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
  - **Anonymous signing is forbidden.** A run that cannot resolve a signer halts
    before emitting rather than writing a null-signer certificate (spec 168 FR-007,
    the spec 200 FR-004 fail-closed posture). The ephemeral key fallback is dev-only
    and marks the certificate untrusted (`signing_attestation.kind: ephemeral`); a
    production tenant emission uses `operator`.
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
  CI step, turns that step from dormant to enforcing.
- **FR-006: Stage-grammar flexibility and determinism.** The emitter accepts the
  tenant's own stage shape (spec 168 §2.4: stages need only be representable as
  `{stage_id, input_hashes, output_hashes, runtime_metadata}`); it does not require
  the tenant pipeline to match OAP's s0..s6 grammar. Re-emitting from the same run
  directory produces byte-identical hashes (spec 168 FR-009), modulo the signer
  field which carries per-run identity.

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

## Open questions

- **OQ-1: emit distributable packaging.** The verify side chose a dedicated repo
  (tenant-tail). The emit side is identity-bearing and key-coupled, which may favor
  shipping the emitter binary through the born-with kernel (next to the pinned
  toolchain) rather than as a standalone npm tool, so the key-custody story and the
  binary travel together. Decide: dedicated emit repo (tenant-tail sibling) vs
  kernel-shipped binary vs npm tool. Whichever is chosen, FR-001's verify-only
  separation from tenant-tail holds.
- **OQ-2: licensing of the extracted emitter.** tenant-tail relicensed the
  verify-only core to Apache-2.0 (the sole copyright holder's prerogative). The
  emitter's relicense is the same prerogative and an explicit decision, not an
  oversight; record it where the emitter is vended.
- **OQ-3: tenant signing-key provisioning UX.** How a produced app's CI obtains its
  Ed25519 secret: a manually configured CI secret (the FR-003 default) vs a
  platform-issued per-tenant key. The platform-issued path ties to the deferred
  uplink (a platform that issues tenant keys can also countersign), so OQ-3 and the
  uplink are likely answered together.
- **OQ-4: in-engine changes, if any.** The post-hoc `build-certificate` already
  carries the run-dir path and `--signer-subject` flags, so FR-001 may need no
  OAP-side code change beyond extraction. Confirm during implementation whether a
  tenant-mode flag (e.g. forcing `operator` attestation and refusing the ephemeral
  fallback in production) is warranted on the OAP `build_certificate.rs` source of
  truth, or lives only in the vended copy.

## Sequencing

Implementable now. The verifier and the seeded `verify-certificate` CI step exist
(specs 219 / 209); the emit core and its post-hoc path exist in-engine; the
certificate format and the unsealed posture are settled (specs 102/168/198). The
net-new work is the emit distributable (OQ-1), the tenant key-custody model
(FR-003), and the firing step (FR-002), plus the owed spec-168 clarification
amendment. Closure is gated on AC-2: a real produced app emitting a certificate
that the spec 209 CI step verifies green, which is also the end-to-end validation
spec 209's own AC-1 silently required.
