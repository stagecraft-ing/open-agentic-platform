# 047 Governance Control Plane — phased implementation plan

> **Non-authoritative.** Planning scratch for agent coordination only. Canonical contract remains `specs/047-governance-control-plane/spec.md`.

## Goal

Implement Feature 047 as a deterministic policy compilation + enforcement system: compile policy markdown (`CLAUDE.md` and `.claude/policies/*.md`) into policy bundles, enforce them via a Rust WASM kernel, degrade privilege on coherence drift, and produce verifiable proof-chain audit records.

## Pre-implementation decisions (G-001 to G-005)

- **G-001 (repo topology):** add a dedicated compiler binary at `tools/policy-compiler/` and keep bundle outputs under `build/policy-bundles/` to mirror spec-compiler conventions without coupling runtime concerns into Feature 001 binaries.
- **G-002 (rule extraction format):** support explicit machine-readable rule blocks first (e.g., fenced `policy` blocks or HTML policy directives). Freeform prose compiles as advisory `mode=log` unless explicitly annotated, aligning with risk mitigation R-001.
- **G-003 (deterministic emission):** canonicalize rule ordering by source precedence, then by rule ID, and serialize with stable map key order to satisfy FR-005 / SC-002.
- **G-004 (WASM target):** start with `wasm32-unknown-unknown` and host all required inputs via function args to preserve NF-003 (no FS/network/syscall access).
- **G-005 (proof-chain storage):** keep append-only proof records in host-side storage with deterministic record hashing in shared Rust logic so chain verification is identical across runtime and offline verifier.

## Implementation slices

### Phase 1 — Compiler skeleton, discovery, parsing, validation (FR-001, FR-002, FR-011)

Deliverables:
- Scaffold `tools/policy-compiler/` crate and CLI command surface (`compile`, optional `validate`).
- Implement policy file discovery with explicit precedence:
  - repo root `CLAUDE.md`
  - `.claude/policies/*.md`
  - subdirectory `CLAUDE.md`
- Implement rule extraction into structured policy-rule records:
  - `id`, `description`, `mode`, `scope`, optional gate metadata
- Add V-series violations for malformed rules, duplicate IDs, invalid scopes/modes, and missing required fields.

Validation:
- Unit tests for path discovery precedence and duplicate resolution.
- Fixture tests for valid/invalid policy-rule parsing and V-series error codes.

### Phase 2 — Constitution/shard classification + deterministic bundle emission (FR-003, FR-004, FR-005, SC-001, SC-002)

Deliverables:
- Implement classification:
  - constitution = `scope=global` + `mode=enforce`
  - shards = all other rules grouped by scope tags
- Emit bundle artifact with:
  - constitution rules
  - shard index/map
  - metadata (version, compilation timestamp, source manifest, content hash)
- Compute content hash over canonical payload excluding timestamp semantics.

Validation:
- Golden tests for byte-identical output (except timestamp field treatment).
- SC-001 fixture proving constitution + shard sections exist and are structurally valid.

### Phase 3 — WASM policy kernel + gate enforcement (FR-006, FR-007, SC-003, SC-004, SC-005, SC-006)

Deliverables:
- Add Rust policy-kernel crate with exported evaluate entrypoint:
  - `evaluate(context, policy_bundle) -> policy_decision`
- Implement four gates:
  - destructive operation guard
  - secrets scanner
  - tool allowlist
  - diff size limiter
- Return decision payloads with consulted rule IDs and machine-readable reasons.

Validation:
- SC-003/004/005/006 scenario tests for each gate deny path.
- Determinism tests for identical input -> identical decision bytes.

### Phase 4 — Coherence scheduler + privilege degradation (FR-008, SC-007, SC-008)

Deliverables:
- Implement rolling-window coherence score calculation with decay factor defaults.
- Add privilege assignment and monotonic degradation model:
  - full -> restricted -> read-only -> suspended
- Add explicit human-restore guard path (no self-promotion in-session).

Validation:
- Windowed behavior tests for threshold crossings.
- Monotonicity tests proving no automatic restricted/read-only -> full transition.

### Phase 5 — Proof-chain records + independent verification (FR-009, FR-010, NF-004, SC-009)

Deliverables:
- Define proof-record schema and hash-chain construction:
  - decision ID, bundle hash, context hash, rule IDs, outcome, previous hash, record hash
- Implement append-only writer and standalone verifier utility.
- Bind genesis record to bundle hash as chain root.

Validation:
- SC-009 verification on 100-record synthetic chain.
- Fixed-size budgeting checks to keep per-record payload under NF-004 target.

### Phase 6 — Runtime integration + performance + verification artifacts (NF-001, NF-002, NF-003, SC-010, SC-011)

Deliverables:
- Integrate policy evaluation into axiomregent dispatch path:
  - `check_tier -> check_permissions -> evaluate_policy -> dispatch`
- Surface distinct `PolicyDenied` error channel from permission denial.
- Add compiler/runtime benchmarks for compilation and evaluation latency.
- Produce canonical execution evidence under `specs/047-governance-control-plane/execution/verification.md`.

Validation:
- SC-010 benchmark proving <5ms p99 kernel evaluation (excluding I/O).
- NF-002 benchmark for 50-source-file compile under 2s.
- SC-011 command/result evidence captured in verification artifact.

## Proposed file touchpoints (initial)

- `tools/policy-compiler/` (new CLI binary + compilation pipeline)
- `build/policy-bundles/` (generated artifacts)
- `crates/axiomregent/` (router integration + `PolicyDenied`)
- `crates/` new policy-kernel/proof modules as needed for shared deterministic logic
- `specs/047-governance-control-plane/execution/verification.md` (evidence after implementation)

## Risks and guardrails

- Prefer explicit rule annotations over heuristic prose extraction to avoid false enforcement.
- Keep kernel pure/deterministic and fully input-driven to satisfy NF-003.
- Preserve hard invariant that secrets scanner cannot be weakened by shard overrides.
- Treat `.ai/` artifacts as temporary coordination; promote durable outcomes into canonical specs/execution records/code.
