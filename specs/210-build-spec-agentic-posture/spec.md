---
id: "210-build-spec-agentic-posture"
title: "Build Spec Agentic-Posture Declaration (Least Agency for produced apps)"
feature_branch: "feat/210-build-spec-agentic-posture"
status: draft
implementation: pending
kind: governance
domain: platform
created: "2026-06-11"
authors: ["open-agentic-platform"]
language: en
summary: >
  Make the produced application's agentic surface a declared,
  falsifiable contract fact. Produced apps are conventional web
  applications today, but nothing STATES that: the Build Spec is silent
  on whether the application embeds model calls, tools, or agent loops,
  so a tenant that later adds an LLM SDK acquires an ungoverned agentic
  surface while still carrying OAP's governance certificate — the
  certificate then implies a coverage it does not have. This spec adds
  an open-standard Build Spec field, agentic_posture: none | declared |
  governed, binds it into the governance certificate, and makes it
  falsifiable by cross-checking the declaration against the produced
  app's SBOM (spec 203): posture "none" with a known agent/LLM SDK in
  the dependency tree fails verification with an attributable
  diagnostic. "Declared" requires the agentic surfaces to be enumerated;
  "governed" requires referencing a governance envelope for that
  surface, bridging the spec 198 contract from the factory run to the
  thing the factory built. This is cross-cutting principle 1 — Least
  Agency — applied to outputs: autonomy is a deliberate, stated choice,
  never a silent acquisition.
code_aliases: ["BUILD_SPEC_AGENTIC_POSTURE"]
compliance:
  - framework: "owasp-asi-2026"
    controls: ["ASI02", "ASI10"]
depends_on:
  - "197-factory-contract-open-standard-extensions"
  - "198-factory-governance-envelope"
  - "203-produced-app-sbom-attestation"
  - "168-per-project-governance-certificate"
extends:
  - spec: "197-factory-contract-open-standard-extensions"
    nature: additive
    unit: { kind: file, path: standards/schemas/factory/build-spec.schema.yaml }
  # Same precedent as specs 196, 194, 193, 187, 183: a new spec adds a row
  # to the featuregraph golden.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
refines:
  - aspect: "agentic-posture-certificate-binding"
    unit: { kind: file, path: crates/factory-engine/src/governance_certificate.rs }
references:
  - role: gate-declaration
    unit: { kind: file, path: standards/schemas/factory/governance-envelope.schema.yaml }
  - role: context
    unit: { kind: file, path: docs/owasp-agentic-top-10-2026.md }
---

# Feature Specification: Build Spec Agentic-Posture Declaration

**Feature Branch**: `210-build-spec-agentic-posture`
**Created**: 2026-06-11
**Status**: Draft (follow-on filed by the ASI gap-closure pass)
**Input**: Spec 198's out-of-scope section assigns the produced app's
own ASI posture to "its Build Spec + governance certificate, not the
factory-run envelope" and defers Build Spec field changes to "spec 197 /
future contract specs". This is that future contract spec. The ASI 2026
gap analysis (2026-06-10) framed the hole: "if a tenant embeds agents
later, nothing is inherited and the certificate is silent."

## Purpose

The ASI Top 10's first cross-cutting principle is Least Agency: do not
deploy autonomy where it is not needed — and, by extension, never
acquire it without deciding to. OAP enforces a great deal about how the
factory *run* behaves and currently nothing about whether the *product*
is itself an agentic application. The silence has a concrete failure
mode: OAP's governance certificate travels with the produced app as its
trust artifact, and an auditor reading it today cannot distinguish "this
application has no agentic surface" from "nobody asked."

A declaration field fixes the semantics only if it is falsifiable —
otherwise it is paperwork. The falsifiability comes from evidence OAP
already plans to hold: the produced app's SBOM (spec 203). A dependency
on a known model-SDK family is not proof of an agent loop, but it is
exactly the tripwire that forces the declaration to be revisited — the
cross-check turns "posture: none" from an assertion into a verified
claim with a named contradiction when it breaks.

## Functional requirements (sketch — refine before implementation)

- **FR-001 — Schema field.** `build-spec.schema.yaml` gains
  `agentic_posture: none | declared | governed` as an additive,
  open-standard extension following spec 197's versioning discipline
  (minor bump; absent field defaults to `none` for pre-existing Build
  Specs, with the default recorded as defaulted, not authored).
  - `none` — the application embeds no model calls, agent loops, tool
    surfaces, or persistent agent memory.
  - `declared` — agentic surfaces exist and are enumerated: model APIs
    consumed, tool surfaces exposed, memory persistence, human approval
    points. Enumeration non-empty by schema.
  - `governed` — declared, plus each surface references a governance
    envelope (the spec 198 schema, reused at application level) that a
    verifier can resolve.
- **FR-002 — Certificate binding.** The posture (and enumeration, when
  present) is bound into the governance certificate; the produced app's
  trust artifact states its agentic surface explicitly.
- **FR-003 — Falsifiability cross-check.** `verify-certificate` (or the
  tenant CI gate home from spec 209) cross-checks posture against the
  spec 203 SBOM: `none` + a dependency matching the agent/LLM SDK
  watchlist fails with a diagnostic naming the package and the
  contradicted declaration; `declared`/`governed` with an empty
  enumeration fails schema. The watchlist is data, versioned with the
  standard (plan.md decides its home), and a watchlist miss is stated
  residual — absence of a match is not proof of absence of agency.
- **FR-004 — Governed-posture bridge.** For `governed`, the referenced
  envelope is validated against the spec 198 schema (shape validation
  at this layer; runtime admission of the app's own agents is the
  tenant's deployment concern and explicitly out of scope here). The
  bridge gives a tenant that chooses agency the same contract grammar
  OAP itself is governed by — inheritance by standard, not by fork.

## Acceptance criteria (sketch)

- **AC-1.** Build Spec schema bump parses both pre-existing specs
  (defaulted `none`) and authored postures; schema-parity (125/191)
  green across the Rust/TS twins.
- **AC-2.** The certificate of a fresh scaffold records the posture;
  tampering the posture fails `verify-certificate`.
- **AC-3.** Fixture: produced app declaring `none` with an
  `@anthropic-ai/sdk`-class dependency in its SBOM fails the
  cross-check naming package and declaration; removing the dependency
  or moving to `declared` passes.
- **AC-4.** Fixture: `governed` posture with an unresolvable or
  non-conformant envelope reference fails; a conformant reference
  passes shape validation.

## Out of scope

- Runtime governance of the produced app's agents (the tenant's
  deployment owns admission/enforcement; OAP provides the contract
  grammar, spec 198 the schema).
- The SBOM emission itself (spec 203).
- An agentic starter kit / scaffolding for governed agent surfaces in
  produced apps — a worthwhile future adapter concern, deliberately not
  smuggled into a contract spec.
- Build Spec fields beyond `agentic_posture` (spec 197 and future
  contract specs own their own extensions).

## Sequencing

After spec 203 (the cross-check needs the SBOM to exist) and following
spec 197's open-standard change discipline. The schema field and
certificate binding (FR-001/002) can land ahead of the cross-check
(FR-003) if sequencing demands, with the cross-check's absence recorded
as a stated residual until it lands.
