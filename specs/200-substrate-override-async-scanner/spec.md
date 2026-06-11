---
id: "200-substrate-override-async-scanner"
title: "Substrate Override Async Scanner (ASI06 model-assisted detection)"
feature_branch: "feat/200-substrate-override-async-scanner"
status: draft
implementation: pending
kind: platform
domain: platform
created: "2026-06-10"
authors: ["open-agentic-platform"]
language: en
summary: >
  The model-assisted leg of the substrate override-write contract (spec 198
  FR-013 d, ASI06 m2): an asynchronous scanner that inspects user_body
  revisions for poisoning patterns the deterministic gate cannot express as
  rules, and quarantines suspect artifacts via the spec 198 FR-010 revocation
  machinery pending human review. The cross-cutting principle is preserved
  verbatim: a model may DETECT, only rules may BLOCK — the scanner never
  rejects a write synchronously and never lifts its own quarantines.
code_aliases: ["SUBSTRATE_OVERRIDE_ASYNC_SCANNER"]
depends_on:
  - "198-factory-governance-envelope"
  - "139-factory-artifact-substrate"
refines:
  - aspect: "override-write-async-scanning"
    unit: { kind: file, path: platform/services/stagecraft/api/factory/artifacts.ts }
references:
  - role: machinery
    unit: { kind: file, path: platform/services/stagecraft/api/factory/revocations.ts }
  - role: context
    unit: { kind: file, path: platform/services/stagecraft/api/factory/overrideGate.ts }
  - role: context
    unit: { kind: file, path: docs/owasp-agentic-top-10-2026.md }
  - role: analog
    unit: { kind: file, path: platform/services/stagecraft/api/knowledge/extractionCore.ts }
---

# Feature Specification: Substrate Override Async Scanner

**Feature Branch**: `200-substrate-override-async-scanner`
**Created**: 2026-06-10
**Status**: Draft (follow-on stub filed by spec 198 phase 5, AC-7)
**Input**: Spec 198 FR-013(d) names this spec as the asynchronous,
model-assisted leg of the `user_body` write contract. FR-013(a–c) — the
deterministic gate, provenance stamping, and the verified-flag trust class —
landed with spec 198; this spec owns what rules cannot express.

## Purpose

The deterministic gate (spec 198 FR-013 a, `overrideGate.ts`) refuses
carrier classes a regex can name: zero-width/bidi characters, hidden
comments, data URIs, encoded blobs, ANSI escapes, secret shapes. It cannot
name *semantic* poisoning: an override that subtly redirects an agent's
goal, weakens a verification instruction, or plants a plausible-looking
falsehood an LLM downstream will trust (ASI06 — memory and context
poisoning). Detection of that class is a judgment task, and judgment is
exactly what must never block synchronously (untrusted-model-output
composing with untrusted input). Hence the architecture spec 198 fixed:

> A model may **detect**; only rules may **block**.

## Functional requirements (sketch — refine before implementation)

- **FR-001 — Async dispatch.** Every accepted `user_body` revision enqueues
  a scan job (PubSub topic + worker, mirroring the spec 115 extraction
  pipeline shape: topic, run row, CAS, staleness sweeper). The write path's
  latency is untouched; the scanner NEVER gates the write.
- **FR-002 — Quarantine-only outcome.** A positive detection quarantines
  the artifact through spec 198 FR-010 machinery (`factory_revocations`,
  `scope_kind='content-hash'`, `mode='quarantined'`) with the model's
  rationale recorded as evidence. Quarantine takes effect at serve, bind,
  and grant renewal exactly like any other revocation — the enforcement
  surface is the existing rule, not the model.
- **FR-003 — Human reintegration.** Lifting a scanner quarantine follows
  the FR-010 contract: fresh two-sided validation plus explicit human
  approval (`lifted_by`). The scanner cannot lift its own quarantines, and
  a re-scan returning clean is advisory evidence only.
- **FR-004 — Cost and policy gates.** Model invocation honours the org's
  extraction policy slice (cost ceilings, model allowlist) per the spec 115
  agent-extractor precedent; scanning degrades to no-op (with an audit
  trail) when no model budget is granted — absence of scanning is visible,
  never silent.
- **FR-005 — Verified-flag interplay.** Scanning is orthogonal to the
  FR-013(c) verified flag: a quarantine on a verified override stands (the
  revocation wins, fail-closed); verification of a quarantined override is
  refused until the quarantine lifts.

## Acceptance criteria (sketch)

- **AC-1.** A `user_body` write returns before any model work begins; scan
  jobs survive worker restarts (run-row + sweeper evidence).
- **AC-2.** A seeded poisoning fixture produces a `factory_revocations`
  quarantine row naming the content hash, with rationale; serve/bind/grant
  paths refuse it with the existing FR-010 errors.
- **AC-3.** No code path exists in which scanner output synchronously
  rejects a write (negative grep + test posture).
- **AC-4.** Quarantine lift requires a human actor id; the scanner's
  service identity is rejected.

## Out of scope

- The deterministic gate rules (spec 198 FR-013 a owns them; new
  rule-expressible classes graduate INTO `overrideGate.ts`, not here).
- Scanning upstream factory content (admitted via the spec 198 envelope;
  upstream trust is the admission gate's job).
- Knowledge-object extraction scanning (spec 115's pipeline owns its own
  content classes).

## Sequencing

Implementation is gated on spec 198 reaching `implementation: complete`
(first sealed admission + grant chain verified end-to-end), so the
quarantine machinery this spec leans on is runtime-proven first.
