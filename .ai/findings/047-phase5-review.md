# 047 Phase 5 Review — Proof Chain + P3-003 Fix

> Reviewer: **claude** | Date: 2026-03-30 | Verdict: **APPROVED**

## Scope

Phase 5 deliverables per plan: append-only proof records (FR-009), independent chain verification (FR-010), linear storage budget (NF-004), 100-record verification test (SC-009), standalone verifier binary. Also: P3-003 fix (allowlist deny returns originating rule IDs).

## Requirement-by-requirement assessment

### FR-009 — Proof record schema ✅

Spec: "Every policy decision produces a proof record containing: decision ID (UUID), timestamp, policy bundle content hash, rule ID(s) consulted, input context hash, decision outcome, and a chained hash linking to the previous proof record."

Implementation (`proof_chain.rs:29–40`): `ProofRecord` struct contains all seven fields:
| Spec field | Code field | Present |
|---|---|---|
| decision ID (UUID) | `id: String` | ✅ |
| timestamp | `timestamp: String` | ✅ |
| policy bundle content hash | `policy_bundle_hash: String` | ✅ |
| rule ID(s) consulted | `rule_ids: Vec<String>` | ✅ |
| input context hash | `input_context_hash: String` | ✅ |
| decision outcome | `decision: ProofRecordDecision` | ✅ |
| chained hash | `previous_record_hash: String` | ✅ |

`record_hash` computed as `SHA-256(canonical_json(record without record_hash field))` — matches the spec's proof chain structure section exactly.

`ProofRecordDecision` enum: `Allow`, `Deny`, `Degrade` — matches the three spec decision outcomes.

`ProofPrivilege` enum: `Full`, `Restricted`, `ReadOnly` (serialized as `read-only`), `Suspended` — matches FR-008 privilege levels, correctly included in proof records to capture the coherence state at decision time.

`ProofChainWriter` (`proof_chain.rs:101–142`): append-only construction. Genesis record's `previous_record_hash` set to `bundle_hash` — matches spec: "The chain is rooted at a genesis record whose `previous_record_hash` is the policy bundle's content hash." Each subsequent record links to the prior record's `record_hash`.

### FR-010 — Independent verification ✅

Spec: "The proof chain is append-only and can be verified independently: given the chain and the policy bundle, any third party can replay decisions and confirm the chain integrity."

Implementation (`proof_chain.rs:145–170`): `verify_proof_chain(records, expected_bundle_hash)` performs five checks per record:
1. `policy_bundle_hash == expected_bundle_hash` — binds every record to the expected bundle
2. `compute_record_hash(rec) == rec.record_hash` — verifies self-hash integrity
3. `nf004_payload_bytes(rec) <= 1024` — NF-004 budget check (see below)
4. Genesis: `previous_record_hash == expected_bundle_hash` — roots chain to bundle
5. Non-genesis: `previous_record_hash == records[i-1].record_hash` — chain link integrity

Standalone verifier binary (`bin/verify_proof_chain.rs`): reads `<policy_bundle_hash> <chain.json>` from CLI args, deserializes `Vec<ProofRecord>`, calls `verify_proof_chain`, exits 0 with "ok" on success or exits 1 with error message. Correct usage pattern for third-party verification.

### NF-004 — Linear storage budget ✅

Spec: "Proof chain storage grows at most linearly with the number of decisions; each record is fixed-size (< 1KB excluding the input context hash)."

Implementation: `NF004_MAX_BYTES_EXCLUDING_CONTEXT = 1024` (1KB). `nf004_payload_bytes()` clones the record, empties `input_context_hash`, serializes to JSON, and returns byte length. This correctly excludes the variable-sized context hash from the budget while measuring all other fields.

The verifier enforces NF-004 on every record in the chain — a record exceeding 1KB (sans context hash) fails verification.

### SC-009 — 100-record chain verification ✅

Spec: "A proof chain of 100 decisions can be independently verified: recomputing each record hash and chain link confirms integrity."

Test `sc009_hundred_record_chain_verifies` (`proof_chain.rs:181–198`): builds a 100-record chain via `ProofChainWriter`, then calls `verify_proof_chain` which recomputes every hash and every link. Passes.

### P3-003 — Allowlist originating rule IDs ✅ (RESOLVED)

Previous finding: allowlist gate returned hardcoded `KERNEL:BUILTIN-ALLOWLIST` instead of originating rule IDs.

Fix (`lib.rs:156–192`): `gate_tool_allowlist` now collects `originating_rule_ids: BTreeSet<String>` from both constitution and shard `tool_allowlist` rules, returning the sorted set in the deny decision. Test `p3_003_allowlist_merges_originating_rule_ids_from_shards` verifies constitution + shard rule IDs (`T-CON`, `T-SHARD`) are both present in the deny.

## Verification

- **14/14 tests pass** (`cargo test --manifest-path crates/policy-kernel/Cargo.toml`)
- **WASM clean** (`cargo check --target wasm32-unknown-unknown --lib`)
- **Binary compiles** (`verify_proof_chain` binary registered in `Cargo.toml`)

## Exports

`lib.rs:10–13` re-exports all proof chain public API: `compute_record_hash`, `nf004_payload_bytes`, `verify_proof_chain`, `ProofChainError`, `ProofChainWriter`, `ProofPrivilege`, `ProofRecord`, `ProofRecordDecision`, `NF004_MAX_BYTES_EXCLUDING_CONTEXT`. Complete and correct.

## Findings

### P5-001 — No tamper-detection negative test for `record_hash` (LOW)

`broken_link_fails` tests chain-link tampering but there is no test that modifies a field within a record (e.g., flipping `decision` from Allow to Deny) and asserts `RecordHashMismatch`. The SC-009 100-chain test validates the happy path but doesn't exercise hash-tamper detection directly.

**Recommendation**: Add a test that mutates a record's `decision` field after construction and asserts `verify_proof_chain` returns `RecordHashMismatch`.

### P5-002 — `id` field not enforced as UUID format (LOW)

FR-009 specifies "decision ID (UUID)" but `ProofRecord.id` is `String` with no validation. The `ProofChainWriter::append` accepts arbitrary strings. Tests use UUID-like strings but nothing enforces the format.

**Impact**: Interoperability — external consumers expecting UUID format may break on non-UUID IDs. Low impact since the kernel is the producer and can enforce format at the call site.

### P5-003 — `timestamp` field not validated (INFO)

`timestamp` is `String` with no ISO-8601 validation. Same rationale as P5-002 — the kernel trusts its caller to provide well-formed timestamps. Acceptable for a no-I/O kernel.

### P5-004 — NF-004 budget measured via JSON serialization (INFO)

`nf004_payload_bytes` measures the *JSON serialization* byte count, not a wire-format or storage-format count. The spec says "< 1KB" without specifying the measurement method. JSON serialization is a reasonable proxy and produces a conservative (larger) estimate.

### P5-005 — `sha256_hex` is public but undocumented (INFO)

`sha256_hex` is `pub` at module level and re-exported implicitly via the `proof_chain` module, but not listed in the explicit `pub use` in `lib.rs`. It could be used by external consumers but isn't part of the declared API surface. Minor — it's a utility.

### P5-006 — Empty chain returns `EmptyChain` error (INFO)

`verify_proof_chain` rejects empty chains. This is a sound design choice (a proof chain with zero records is meaningless), though the spec doesn't explicitly address this edge case.

## Summary

| Requirement | Status | Notes |
|---|---|---|
| FR-009 | ✅ | All 7 proof record fields present, hash computation spec-faithful |
| FR-010 | ✅ | 5-check verification + standalone binary |
| NF-004 | ✅ | 1024-byte budget excluding context hash, enforced in verifier |
| SC-009 | ✅ | 100-record chain build + verify test |
| P3-003 | ✅ RESOLVED | Originating rule IDs from constitution + shards |

**Findings**: 6 total — 0 HIGH, 0 MEDIUM, 2 LOW (P5-001, P5-002), 4 INFO (P5-003..P5-006). No blockers. **047 Phase 5 approved.**
