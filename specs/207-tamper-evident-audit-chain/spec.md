---
id: "207-tamper-evident-audit-chain"
title: "Tamper-Evident Audit Log Chain (ASI observability principle)"
feature_branch: "feat/207-tamper-evident-audit-chain"
status: draft
implementation: in-progress
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
establishes:
  # FR-003 (Phase 1): the independent audit-chain verifier. Its home was left
  # open at filing ("home decided in plan.md") and is decided in plan.md: a
  # sister binary to verify_proof_chain, in policy-kernel.
  - unit: { kind: file, path: crates/policy-kernel/src/bin/verify_audit_chain.rs }
  # FR-003 end-to-end coverage: the verifier CLI exit-code contract (clean
  # exits 0, tampered exits non-zero naming the record, no-arg exits usage).
  - unit: { kind: file, path: crates/policy-kernel/tests/verify_audit_chain_cli.rs }
  # FR-002 (Phase 2a, AC-3): the run-audit chain writer (factory run audit
  # serialized as a hash-chained segment, reusing the policy-kernel primitive).
  - unit: { kind: file, path: crates/factory-engine/src/run_audit_chain.rs }
  # AC-3 end-to-end: anchored segment is tamper-evident under verify_certificate.
  - unit: { kind: file, path: crates/factory-engine/tests/run_audit_anchoring.rs }
extends:
  # Same precedent as specs 196, 194, 193, 187, 183: a new spec adds a row
  # to the featuregraph golden.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
  # Phase 1: the new [[bin]] target for verify_audit_chain is an additive
  # extension of the spec-047-owned policy-kernel manifest.
  - spec: "047-governance-control-plane"
    nature: additive
    unit: { kind: file, path: crates/policy-kernel/Cargo.toml }
refines:
  - aspect: "hash-chained-audit-records"
    unit: { kind: file, path: crates/policy-kernel/src/audit.rs }
  - aspect: "chain-reuse-for-audit"
    unit: { kind: file, path: crates/policy-kernel/src/proof_chain.rs }
  # Phase 2a (AC-3): emit_certificate writes + anchors the run-audit segment,
  # and the run accrues phase confirmations into a local audit trail.
  - aspect: "run-audit-emission-and-anchoring"
    unit: { kind: file, path: crates/factory-engine/src/bin/factory_run.rs }
  # Module registration for the run-audit chain writer.
  - aspect: "run-audit-module-registration"
    unit: { kind: file, path: crates/factory-engine/src/lib.rs }
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

## Functional requirements

> Refined from the filing sketch per the spec's own instruction; the
> plan.md decisions are folded in. Phase 1 (this implementation) lands
> FR-001, FR-003, and the LOCAL half of FR-002. Phase 2 lands the
> cross-repo anchoring (run certificate + platform countersign). See
> Sequencing and the Implementation log.

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

> **Refinement (Phase 1 honesty).** "Detects modification" holds only
> against the external anchor. A self-referential chain catches edits that
> do NOT recompute downstream hashes (in-place edits, mid-segment deletion,
> reordering). It does NOT, on its own, catch an edit that re-establishes
> internal consistency: a writer of an OPEN, not-yet-anchored segment can
> re-genesis and recompute every `record_hash` (whole-segment rewrite,
> head-prefix deletion, tail deletion) and the walker passes. That entire
> class is the Phase 1 residual until FR-002 anchoring (run certificate /
> platform countersign, Phase 2) supplies the external trust root. The
> residual is therefore broader than tail deletion alone; it is bounded by
> the anchoring cadence, not eliminated by the chain.

## Acceptance criteria

> Phase 1 satisfies AC-1, AC-2, AC-5 (local, offline, deterministic,
> covered by `crates/policy-kernel` unit + CLI tests). AC-3 and AC-4
> (cross-repo anchoring) are Phase 2.

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

## Implementation log

**Phase 1 (2026-06-19).** Local chain + independent verifier, in
`crates/policy-kernel`. Plan and decisions: `plan.md`.

- **FR-001 (done).** The spec 047 record-hash linkage was extracted into a
  content-agnostic primitive `proof_chain::link_record_hash(value,
  hash_field)`; `compute_record_hash` is now a thin wrapper over it
  (behaviour byte-identical, guarded by the existing spec 047 chain
  tests). `audit::AuditLogger` writes each JSONL record with
  `previous_record_hash` + `record_hash` computed through that primitive;
  a fresh segment is genesis-marked, and a process restart on a non-empty
  file recovers the chain head and continues unbroken.
- **FR-002 (local half done; cross-repo deferred).** Rotation closes a
  segment with a `segment_head` record (`segment_id`, `record_count`,
  first/last timestamp); the head hash becomes the next segment's genesis
  binding (continuity), and the `record_count` is the closed-segment
  truncation tripwire. Run-certificate and platform-countersign anchoring
  (AC-3, AC-4) are Phase 2.
- **FR-003 (done).** `verify_audit_chain` binary (sister to
  `verify_proof_chain`): walks a segment, recomputes hashes, checks links,
  validates a trailing head's count, and exits non-zero naming the first
  broken record. `verify_audit_chain(records, expected_genesis)` is the
  unit-testable core; the binary is a thin CLI shell.
- **FR-004 (stated).** Until FR-002 anchoring lands (Phase 2), the open
  not-yet-anchored segment is the residual: any internally-consistent
  rewrite (whole-segment re-hash, head-prefix deletion, tail deletion)
  passes the self-referential walker. The chain catches only edits that
  fail to recompute downstream hashes. The 10 MB rotation size bounds the
  unanchored window; the external anchor closes the class. See the FR-004
  refinement note above (corrected after local review).

Carve-out (plan.md decision 3): stagecraft database-resident audit rows
(`auditActions.ts`) are NOT file-chained; their honest anchoring story is
the platform countersign plus append-only DB posture, not the per-record
file chain. No stagecraft code changes in Phase 1.

Coupling note: the implementation PR adds `establishes:` edges for the new
verifier binary and its CLI test, and an additive `extends:` edge on the
spec-047-owned `crates/policy-kernel/Cargo.toml` for the new `[[bin]]`
target, per the spec-196 "edge lands with the code" precedent.

**Phase 2a (2026-06-19, PR A): run-certificate anchoring (AC-3).** Factory
runs now emit a hash-chained run-audit segment, anchored into the run
governance certificate so `verify-certificate` catches tampering.

- **Chain (FR-001).** `crates/factory-engine/src/run_audit_chain.rs`
  serializes the run's audit trail to `<run_dir>/run-audit/run-audit.jsonl`,
  each record carrying `previous_record_hash` + `record_hash` via the SAME
  `policy_kernel::proof_chain::link_record_hash` primitive (genesis link
  `genesis:<run_id>`). `verify_audit_chain` (content-agnostic) validates it.
- **Anchor (FR-002, AC-3).** `factory-run`'s `emit_certificate` writes the
  segment then builds the certificate with stage list `OAP_STAGE_IDS +
  "run-audit"`, so the existing `stage_record_for` scan binds the segment
  file's SHA-256 into the cert and the existing `verify_certificate`
  artifact-hash loop catches any tamper. No bespoke certificate code: the
  segment is a stage artifact. Proven end-to-end by
  `tests/run_audit_anchoring.rs` (clean verifies; tampered segment fails,
  diagnostic names the segment).
- **Population.** The CLI's `FactoryPipelineState` is distinct from the
  harness `PipelineState` that carries the OPC-side audit vec (which is why
  `record_audit` was never wired), so the run audit is a binary-local
  `Vec<AuditEntry>` accruing a `StageConfirmed` per phase boundary
  (phase-1 / transition / phase-2). Per-gate recording inside the dispatch
  path is a deliberate follow-up; the mechanism is complete and anchored.
- **Coupling:** `establishes:` the new writer + anchoring test; `refines:`
  `factory_run.rs` + the factory-engine `lib.rs` module registration. No
  `governance_certificate.rs` change (the stage-scan is reused as-is).

**Phase 2b (PR B, deferred): platform countersign (AC-4).** Session-scoped
segments countersigned by stagecraft (spec 198 FR-014), offline-first with
retroactive anchoring. Cross-repo; sequenced after the active stagecraft
deploy settles. Design in `plan.md` (Phase 2b).
