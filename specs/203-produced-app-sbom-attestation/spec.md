---
id: "203-produced-app-sbom-attestation"
title: "Produced-App SBOM and Dependency-Audit Attestation (ASI04 forward)"
feature_branch: "feat/203-produced-app-sbom-attestation"
status: draft
implementation: pending
kind: capability
domain: platform
created: "2026-06-11"
authors: ["open-agentic-platform"]
language: en
summary: >
  Apply OAP's own supply-chain discipline forward to what the factory
  produces. OAP ships per-target CycloneDX SBOMs and an aggregate release
  BOM for itself (specs 116/117), but a scaffolded application receives
  only a pinned lockfile — no SBOM, no dependency-vulnerability scan
  artifact, no lockfile-parity gate. An auditor holding a produced app's
  governance certificate (spec 168) cannot answer "what is in this
  application and was it scanned?" — the exact ASI04 question. This spec
  emits a CycloneDX BOM and a dependency-audit artifact at scaffold
  completion, binds their content hashes into the per-project governance
  certificate, and adds a lockfile/BOM parity gate to the tenant CI the
  born-with kernel (spec 167) seeds. Absence of a scanner is recorded
  visibly, never silently (the spec 200 FR-004 posture).
code_aliases: ["PRODUCED_APP_SBOM_ATTESTATION"]
compliance:
  - framework: "owasp-asi-2026"
    controls: ["ASI04"]
depends_on:
  - "167-born-with-spec-spine-kernel"
  - "168-per-project-governance-certificate"
  - "112-factory-project-lifecycle"
  - "116-supply-chain-policy-gates"
extends:
  # Same precedent as specs 196, 194, 193, 187, 183: a new spec adds a row
  # to the featuregraph golden.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
refines:
  - aspect: "kernel-sbom-vending"
    unit: { kind: file, path: crates/factory-engine/src/kernel_emission/emit.rs }
  - aspect: "sbom-artifact-binding"
    unit: { kind: file, path: crates/factory-engine/src/governance_certificate.rs }
  - aspect: "lockfile-parity-gate"
    unit: { kind: file, path: crates/factory-engine/templates/kernel/tenant-ci.yml.tmpl }
references:
  - role: context
    unit: { kind: file, path: platform/services/stagecraft/api/factory/translator.ts }
  - role: context
    unit: { kind: file, path: docs/owasp-agentic-top-10-2026.md }
---

# Feature Specification: Produced-App SBOM and Dependency-Audit Attestation

**Feature Branch**: `203-produced-app-sbom-attestation`
**Created**: 2026-06-11
**Status**: Draft (follow-on filed by the ASI gap-closure pass)
**Input**: The ASI 2026 gap analysis (2026-06-10) found the most
asymmetric gap in the corpus: "OAP ships CycloneDX SBOMs for itself but
the scaffolded app gets only a pinned lockfile — no SBOM, no CVE scan
artifact, no lockfile-matches-manifest gate. The 'same discipline, applied
to ourselves' principle isn't yet applied *forward* to outputs."

## Purpose

The README's promise is that the provenance discipline applied to governed
agent execution is the discipline applied to the project's own releases.
For a factory, the claim that ultimately matters is the third leg: the
discipline applied to what it *produces*. Today the produced app's
supply-chain posture is: lockfile pinned (good), `.git` stripped (spec 112
§5.3, good), and nothing else. ASI04's mitigation 1 — provenance + SBOM,
operationalized with periodic attestation — has no produced-side artifact.

This spec makes the produced application carry its own bill of materials
and scan evidence, certificate-bound so the evidence is attested rather
than asserted.

## Functional requirements (sketch — refine before implementation)

- **FR-001 — BOM emission at scaffold completion.** The factory pipeline
  emits a CycloneDX BOM for the produced application, derived
  deterministically from the committed lockfile(s) of the scaffold output.
  Same inputs → byte-identical BOM (constitution Principle IV). The BOM is
  written into the produced tree as a tracked sidecar (location decided in
  plan.md; precedent: OAP's own `sbom-*.cdx.json` naming).
- **FR-002 — Dependency-audit artifact.** A dependency vulnerability scan
  (npm-audit/osv class; tool choice in plan.md) runs against the produced
  lockfile at scaffold time and writes a typed result artifact: tool,
  database timestamp, findings, severity counts. When no scanner or
  advisory database is available, the artifact records that absence
  explicitly with reason — a missing scan is visible evidence of a gap,
  never a silent skip.
- **FR-003 — Certificate binding.** The content hashes of the BOM and the
  audit artifact enter the per-project governance certificate's artifact
  list (spec 168). `verify-certificate` thereby detects post-hoc tampering
  with either; an auditor verifies the app's dependency evidence offline,
  without trusting the system that produced it (spec 102 FR-007 posture).
- **FR-004 — Tenant CI lockfile/BOM parity gate.** The kernel-seeded
  tenant CI gains a gate: the lockfile must satisfy the manifest, and the
  BOM must be regenerable from the lockfile to a matching hash. Drift in
  either direction fails the tenant PR with an attributable diagnostic.
- **FR-005 — Vended-tool pinning.** The BOM/audit tooling the tenant CI
  invokes is referenced through the `.kernel-version` pinned-toolchain
  mechanism (spec 167 FR-005), so the produced app's evidence chain does
  not float on whatever tool version the tenant happens to have.

## Acceptance criteria (sketch)

- **AC-1.** A fresh scaffold contains a CycloneDX BOM and an audit
  artifact; both hashes appear in the project's governance certificate and
  `verify-certificate` exits 0.
- **AC-2.** Tampering with the produced lockfile after emission fails the
  tenant CI parity gate; tampering with the BOM fails `verify-certificate`
  with a specific artifact-hash diagnostic.
- **AC-3.** Scaffolding with the scanner deliberately unavailable yields
  an audit artifact that records the absence and its reason; the
  certificate still binds it; nothing is silently skipped.
- **AC-4.** Two scaffolds from identical inputs produce byte-identical
  BOMs.

## Out of scope

- OAP's own release SBOMs (specs 116/117 own them; this spec is the
  produced-app leg only).
- Blocking policy on scan findings — what severity fails a tenant build is
  filed org policy; this spec guarantees the *evidence*, not the verdict.
- Agentic-dependency detection semantics (consumed by spec 210's
  falsifiability cross-check, which reads the BOM this spec emits).
- Container/image BOMs for deployed tenants (deployment is deployd-api
  territory; a future spec may extend attestation to images).

## Sequencing

Independent of spec 198's runtime closure. Requires spec 167's kernel
emission surface (present) and composes with spec 168's certificate; the
tenant CI gate (FR-004) lands with or after spec 209's enforcement
activation so the gate has a CI home that actually runs.
