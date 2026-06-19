# Implementation Plan: Tamper-Evident Audit Log Chain (spec 207)

> Refines the spec sketch into firm decisions, per the spec's own
> "refine before implementation" instruction. The spec body's
> FR-001..FR-004 and AC-1..AC-5 are sharpened in lockstep with this plan.

## Decision summary (the deferrals the spec left to plan.md)

1. **Chain shape reuse (FR-001).** The audit chain reuses the spec 047
   linkage discipline, not a second chain shape. `proof_chain.rs` already
   computes `record_hash = sha256(canonical_json(record without
   record_hash))`. We extract that into a content-agnostic primitive
   `link_record_hash(value, hash_field)` and make the existing
   `compute_record_hash` a thin wrapper. The audit writer consumes the
   same primitive. This is the `chain-reuse-for-audit` refinement on
   `proof_chain.rs`.

2. **Verifier home (FR-003).** A standalone `verify_audit_chain` binary
   under `crates/policy-kernel/src/bin/`, mirroring the existing
   `verify_proof_chain` sister binary (which itself mirrors
   factory-engine's `verify_certificate`). It shares no state with the
   producer and runs offline. The walk logic lives in `audit.rs`
   (`verify_audit_chain`) so it is unit-testable without spawning a
   process; the binary is a thin CLI shell.

3. **Stagecraft DB audit rows (FR-001 carve-out).** Database-resident
   audit rows (`auditActions.ts`) are NOT file-chained by this spec.
   Honest posture: their anchoring story is the platform countersign
   (spec 198 FR-014) plus an append-only DB operational posture, not the
   per-record file chain. This is recorded here rather than pretending
   the file mechanism fits a relational table. No stagecraft code changes
   in this phase.

4. **Anchoring split (FR-002).** The LOCAL half (segment head at
   rotation + cross-segment continuity) lands now because AC-2 and AC-5
   require it and it is fully offline. The CROSS-REPO half (run-scoped
   segments entering the governance certificate artifact list;
   session-scoped segments countersigned by stagecraft per spec 198
   FR-014) is Phase 2 and rides the existing key infrastructure rather
   than duplicating it.

## Phase 1 (this PR): local chain + verifier

Scope = FR-001 + FR-003 + the local half of FR-002. Satisfies AC-1, AC-2,
AC-5 with local-only, deterministic tests.

### P1-a `proof_chain.rs` (refines: chain-reuse-for-audit)
- Add `pub fn link_record_hash(value: serde_json::Value, hash_field: &str)
  -> String`: strips `hash_field` from the object, canonicalises, returns
  `sha256:<hex>` via the existing `sha256_hex`.
- Reimplement `compute_record_hash` as `link_record_hash(value,
  "record_hash")`. Behaviour is byte-identical (regression-guarded by the
  existing spec 047 chain tests, which must stay green).

### P1-b `audit.rs` (refines: hash-chained-audit-records)
- Extend the written line with `previous_record_hash` and `record_hash`.
- `AuditLogger` carries `last_record_hash` (the chain head) and a
  `segment_id`.
- On open: if the file is non-empty, parse the last line and continue from
  its `record_hash`; if new/empty, seed a per-segment genesis marker
  `genesis:<segment_id>` (spec: "genesis-marked per segment").
- On `log`: build the line body (`timestamp` + entry fields +
  `previous_record_hash`), compute `record_hash` via `link_record_hash`,
  write, advance `last_record_hash`.
- On rotation (`maybe_rotate`): before moving the file, append a
  **segment head** line `{segment_head:true, segment_id, record_count,
  first_timestamp, last_timestamp, previous_record_hash, record_hash}`.
  The head's `record_hash` becomes the new chain head; the next segment's
  genesis `previous_record_hash` binds it (AC-5 continuity). The head's
  `record_count` is the truncation tripwire (AC-2).
- `pub fn verify_audit_chain(lines: &[serde_json::Value], expected_genesis:
  Option<&str>) -> Result<(), AuditChainError>`: walks records; for each
  recomputes `record_hash` (mismatch => name the index, AC-1); checks
  `previous_record_hash` links to the prior `record_hash` (broken link =>
  name the index, AC-2 mid-segment deletion); the first record's
  `previous_record_hash` must equal the genesis marker (or
  `expected_genesis` when chaining continuity); if a segment head is
  present, its `record_count` must equal the count of preceding data
  records (AC-2 closed-segment tail truncation).
- `AuditChainError` enum mirrors `ProofChainError`'s indexed-diagnostic
  shape.

### P1-c `verify_audit_chain` binary (NEW; establishes)
- `crates/policy-kernel/src/bin/verify_audit_chain.rs`: reads a JSONL
  segment file, parses lines to `Vec<Value>`, calls
  `audit::verify_audit_chain`, exits 0 / 1 (broken: prints the first bad
  record index and reason) / 2 (usage).
- `Cargo.toml`: new `[[bin]]` target. Re-export `verify_audit_chain` +
  `AuditChainError` from `lib.rs`.

### P1-d tests
- spec 047 regression: existing `proof_chain` tests stay green
  (proves the primitive extraction is behaviour-preserving).
- AC-1: flip one byte in record N => verifier errors naming N.
- AC-2: delete a mid-segment record => broken link at the splice;
  truncate a closed segment's tail => `record_count` mismatch.
- AC-5: rotation writes a head; next segment genesis binds the head.
- continuity-across-restart: reopen a logger on a non-empty file, log
  again, full chain verifies.

## Phase 2 (follow-on, NOT this PR): cross-repo anchoring

- **AC-3:** run-scoped segment head hash enters the run governance
  certificate artifact list (factory-engine); `verify-certificate` fails
  after tampering. Rides spec 102 / 198 phase-4 cert machinery.
- **AC-4:** session-scoped segments countersigned by stagecraft (spec 198
  FR-014); offline sessions chain locally and anchor retroactively at next
  connection, with the unanchored window queryable. Offline-first,
  honestly degraded.

## FR-004 residual (stated, not eliminated)

The chain detects modification and mid-chain deletion. It cannot detect
deletion of an entire tail after the last anchor inside the open segment.
The anchoring cadence (rotation size/interval) IS the integrity budget:
the unanchored residual window is bounded by `MAX_SIZE_BYTES` (10 MB) per
segment. Recorded as accepted residual per the ASI08 residual-statement
discipline, not implied tamper-proofness.

## Coupling edges (land in the implementation PR, per the 196 precedent)

- `establishes:` `crates/policy-kernel/src/bin/verify_audit_chain.rs` (new
  verifier binary; FR-003 home was open at filing, decided here).
- `extends:` (additive) `crates/policy-kernel/Cargo.toml` (new `[[bin]]`
  target on the spec-047-owned manifest).
- `extends:` `crates/featuregraph/tests/golden/features_graph.json`
  (already declared; regenerate the golden after implementation).
- `refines:` `audit.rs` + `proof_chain.rs` (already declared).

## Verification commands

```bash
cargo test --manifest-path crates/policy-kernel/Cargo.toml
cargo build --release --manifest-path crates/policy-kernel/Cargo.toml --bin verify_audit_chain
make pr-prep   # regenerate codebase index + coupling gate vs origin/main
```
