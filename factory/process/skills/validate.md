<!--
SPDX-License-Identifier: AGPL-3.0-or-later
Copyright (C) 2026 Bartek Kus
Spec: specs/121-claim-provenance-enforcement/spec.md — FR-023
-->

# Factory Process Validation Rules

Process-level validation rules. These apply to every adapter and bind to
the Stage 1 quality-gate machinery. The runtime enforcement lives in
`crates/factory-engine/src/stages/quality_gates.rs::evaluate_qg13`; this
file is the human-readable record.

## FAC-S1-011 — External Entity Provenance

**Rule:** Every claim emitted by Stage 1 (or any later stage that mints
IDs) that names an external entity — any organization, system, product,
or proper noun NOT in the project allowlist — MUST satisfy one of:

1. **`DERIVED`** — the claim carries at least one `Citation` whose
   `quoteHash` verifies verbatim against the typed extraction corpus
   (`ExtractionOutput` from spec 120). Verification is exact, with NFC
   + collapsed-whitespace normalisation; curly-vs-straight quote
   tolerance is fail-closed (spec 121 §7).

2. **`ASSUMPTION`** — the claim carries an `AssumptionTag` with a
   non-empty `owner`, a `rationale`, and an `expiresAt` no more than 90
   days from `taggedAt`. The claim consumes one slot in the project's
   assumption budget (FR-029).

A claim with external-entity references and no citation AND no
assumption tag is recorded as `REJECTED`.

The detection of external entities is allowlist-driven, not NER. The
allowlist is auto-derived from (a) the built-in core, (b) project name
+ slug + workspace name, (c) capitalized-token frequency scan over the
typed extraction corpus, (d) `entity-model.yaml` from prior Stage 2
runs (when present), (e) charter vocabulary from spec 122 (when shipped).

## QG-13 ExternalProvenance gate

**Mode: STRICT (default for all projects from the first run).**
Any `REJECTED` claim FAILs `QG-13_ExternalProvenance`. The Stage 1
gate machinery (`evaluate_qg13`) blocks pipeline advancement; the
factory pipeline halts at Stage 1 (orchestrator rule 4) and surfaces
the rejection in the desktop UI with the three-action remediation
panel (supply citation / downgrade to ASSUMPTION / promote ASSUMPTION).

**Mode: PERMISSIVE (explicit, audit-logged opt-in).**
The gate WARNs instead of blocking. PERMISSIVE requires an explicit
`reason` field in `provenance:` of `factory-config.yaml`; absence of
the reason is a config-parse error (FR-027). PERMISSIVE is intended
only for projects whose BRDs predate spec 121 and need a retroactive
audit before STRICT can pass. There is NO permissive ramp; PERMISSIVE
is per-project and audit-logged via `factory.provenance_mode_changed`.

**Workspace policy override (FR-026).**
A workspace administrator may pin STRICT globally via the
`WorkspaceProvenancePolicy` policy slice. Project config that
specifies PERMISSIVE under a STRICT-pinned workspace is silently
overridden; the audit row records `workspacePinApplied: true`.

**Validator independence.**
The validator that enforces FAC-S1-011 lives in `crates/provenance-
validator` and structurally cannot pull an LLM client crate (FR-001;
enforced by `tests/no_llm_deps.rs`). The model that mints the claims
cannot fool the validator that grades them.

**Fail-closed on panic.**
A panic in the validator surfaces as `qg13_validator_panic` and the
gate FAILs (FR-005). There is no "validator unavailable → admit"
path.

**Audit log.**
Every gate run emits `factory.provenance_validated` with the per-mode
counts. STRICT⇄PERMISSIVE transitions emit
`factory.provenance_mode_changed` with the operator-of-record actor
and the `reason` from `factory-config.yaml`.
