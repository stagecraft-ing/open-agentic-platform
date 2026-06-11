---
id: "207-tamper-evident-audit-chain"
title: "Tamper-Evident Audit Log Chain (ASI observability principle)"
feature_branch: "feat/207-tamper-evident-audit-chain"
status: draft
implementation: pending
kind: governance
domain: tooling
created: "2026-06-11"
authors: ["open-agentic-platform"]
language: en
summary: >
  Extend the proof-chain discipline from policy decisions to the audit
  surfaces themselves. Spec 047 hash-chains policy decision proofs, but
  the JSONL audit logs the platform actually accumulates (permission
  decisions per spec 068 NF-003, session activity, factory run logs) are
  plain append-only files: deletable, editable, and unanchored — failing
  the cross-cutting ASI 2026 observability principle ("immutable, signed,
  tamper-evident logs") and the ASI08/ASI10 logging mitigations that
  every incident reconstruction depends on. This spec chains audit
  records (each record carries the previous record's hash), anchors
  segment heads at rotation — run-scoped logs into the governance
  certificate's artifact list, session-scoped logs via the spec 198
  FR-014 platform countersign when connected, locally chained when
  offline — and ships an independent verifier that does not trust the
  producer. Honest scope: a chain makes tampering EVIDENT, not
  impossible; anchoring bounds (not eliminates) silent-truncation.
code_aliases: ["TAMPER_EVIDENT_AUDIT_CHAIN"]
compliance:
  - framework: "owasp-asi-2026"
    controls: ["ASI08", "ASI10"]
depends_on:
  - "047-governance-control-plane"
  - "068-permission-runtime"
  - "102-governed-excellence"
  - "198-factory-governance-envelope"
extends:
  # Same precedent as specs 196, 194, 193, 187, 183: a new spec adds a row
  # to the featuregraph golden.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
refines:
  - aspect: "hash-chained-audit-records"
    unit: { kind: file, path: crates/policy-kernel/src/audit.rs }
  - aspect: "chain-reuse-for-audit"
    unit: { kind: file, path: crates/policy-kernel/src/proof_chain.rs }
references:
  - role: machinery
    unit: { kind: file, path: platform/services/stagecraft/api/factory/signing.ts }
  - role: context
    unit: { kind: file, path: platform/services/stagecraft/api/factory/auditActions.ts }
  - role: context
    unit: { kind: file, path: docs/owasp-agentic-top-10-2026.md }
---

# Feature Specification: Tamper-Evident Audit Log Chain

**Feature Branch**: `207-tamper-evident-audit-chain`
**Created**: 2026-06-11
**Status**: Draft (follow-on filed by the ASI gap-closure pass)
**Input**: The ASI 2026 gap analysis (2026-06-10) found: "Policy
decisions get hash-chained proof records, but session/permission JSONL
logs are plain files." Every ASI entry's forensic mitigation (ASI02 m8,
ASI08 m10, ASI09 m2, ASI10 m1) presumes logs an attacker — or a rogue
agent with filesystem access — cannot quietly rewrite.

## Purpose

OAP's flagship audit artifact, the governance certificate, already
embodies the right trust posture: self-authenticating hashes plus an
independent verifier that does not trust the producer (spec 102 FR-007).
The day-to-day audit surfaces do not. A JSONL file under the user's home
directory is exactly as trustworthy as the last process with write access
to it — which, in an agentic environment, includes the agents being
audited. ASI10's "review logs for stealth infiltration" is circular if
the rogue agent can edit the review's input.

The fix is the discipline OAP already owns, applied uniformly: chain,
anchor, verify independently.

## Functional requirements (sketch — refine before implementation)

- **FR-001 — Hash-chained records.** Every audit record carries the hash
  of its predecessor (genesis-marked per segment), reusing the spec 047
  proof-chain linkage rather than inventing a second chain shape. Applies
  to the policy-kernel audit writer (permission decisions, spec 068
  NF-003 logs) and to factory run logs; stagecraft-side audit rows
  (database-resident) declare their anchoring story in plan.md rather
  than pretending the same mechanism fits.
- **FR-002 — Segment anchoring at rotation.** Log rotation closes a
  segment with a head record (segment hash, record count, time range).
  Run-scoped segments anchor by entering the run's governance certificate
  artifact list — tampering then fails `verify-certificate` with the
  existing artifact-hash diagnostic. Session-scoped segments anchor via
  the platform countersign (spec 198 FR-014: stagecraft seals, the local
  side holds no signing keys) when a platform connection exists; offline
  sessions chain locally and anchor retroactively at next connection,
  with the unanchored window visible — offline-first, honestly degraded,
  never silently unverifiable.
- **FR-003 — Independent verifier.** A `verify-audit-chain` verb (home
  decided in plan.md; the `verify-certificate` sister-binary pattern)
  walks a segment chain and exits non-zero naming the first broken
  record. It shares no state with the producer and runs offline for
  locally-chained segments.
- **FR-004 — Stated residual.** The chain detects modification and
  mid-chain deletion; it cannot detect deletion of an entire tail after
  the last anchor. The anchoring cadence is therefore the integrity
  budget: the residual window is bounded by rotation size/interval, and
  the spec records this as the accepted residual rather than implying
  tamper-proofness (the ASI08 residual-statement discipline).

## Acceptance criteria (sketch)

- **AC-1.** Flipping one byte in record N of a segment: the verifier
  exits non-zero naming record N.
- **AC-2.** Deleting a record mid-segment breaks the chain at the splice
  and is detected; truncating the tail past the last anchor is detected
  via the anchor's record count.
- **AC-3.** A run-scoped segment's head hash appears in the run
  certificate; `verify-certificate` fails after segment tampering.
- **AC-4.** An offline session produces locally-chained segments that
  verify offline; reconnecting anchors them and the unanchored window is
  queryable.
- **AC-5.** Rotation preserves continuity: segment N+1's genesis binds
  segment N's head.

## Out of scope

- WORM storage, HSM anchoring, external transparency logs — deployment
  hardening above the platform countersign is org infrastructure.
- The content of audit records (specs 047/068/172 own their schemas;
  this spec adds linkage and anchoring, not fields beyond them).
- Stagecraft database audit-row immutability enforcement (DB-side
  append-only is an operational posture; the plan.md decides what the
  platform can honestly attest about its own tables).
- Tamper *prevention* — explicitly: this spec delivers evidence, not
  immunity (FR-004).

## Sequencing

FR-001 and FR-003 are implementable now (policy-kernel is local and the
chain shape exists). FR-002's countersign anchoring follows spec 198
phase 4 machinery (landed) and should ride the same key infrastructure,
not duplicate it.
