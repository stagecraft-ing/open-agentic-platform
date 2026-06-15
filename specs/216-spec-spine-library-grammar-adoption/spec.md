---
id: "216-spec-spine-library-grammar-adoption"
title: "Spec-Spine Library Grammar Adoption (amends bare-id; supersedes-partial reconciliation)"
feature_branch: "feat/216-spec-spine-library-grammar-adoption"
status: draft
implementation: pending
kind: governance
domain: tooling
created: "2026-06-14"
authors: ["open-agentic-platform"]
language: en
summary: >
  Converge OAP's spec-compiler frontmatter grammar onto the standalone
  spec-spine library, which models `amends` and `supersedes` as typed
  fields and rejects malformed entries at deserialization. OAP's compiler
  parses both through `optional_string_list`, which silently drops any
  non-string entry: an object-form `amends`/`supersedes` records nothing
  and confers no authority. That silent drop was the spec 125 defect PR
  #358 corrected. Phase 1 (this spec, implementable now) narrows `amends`
  to bare-id and replaces the silent drop with a loud V-033 compile error,
  a behaviour-preserving convergence for the real corpus (zero live
  object-form `amends` remain after PR #358). Phase 2 (sequenced)
  reconciles `supersedes`, whose silent drop discards the structured
  partial-supersession form the library honours and the relationship graph
  (specs 130/154) documents: that is an authority-transfer change and
  carries the real review weight.
depends_on:
  - "001-spec-compiler-mvp"
  - "130-spec-coupling-primary-owner"
  - "132-constitutional-invariant-freeze"
  - "133-amends-aware-coupling-gate"
  - "154-logical-unit-ownership-grammar"
code_aliases: ["SPEC_SPINE_LIBRARY_GRAMMAR_ADOPTION"]
extends:
  # New spec adds a row to the featuregraph golden (same precedent as
  # specs 212, 202, 196, 194, 193, 187, 183).
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
refines:
  # The narrowing itself. `amends` (Phase 1) and `supersedes` (Phase 2)
  # parse through the same `optional_string_list` site; this spec tightens
  # that aspect from silent-drop to typed-or-reject. Two files share the
  # aspect: the compiler parse site + V-033, and the shared frontmatter
  # known-key comment that documents the field grammar.
  - aspect: "amends-supersedes-frontmatter-grammar"
    unit: { kind: file, path: tools/spec-spine/spec-compiler/src/lib.rs }
  - aspect: "amends-supersedes-frontmatter-grammar"
    unit: { kind: file, path: tools/shared/spec-types/src/lib.rs }
references:
  # The clarification callouts this spec formalises as a frontmatter
  # amendment when Phase 1 lands (FR-004). Non-owning: this spec does not
  # reshape 130/154's grammar; it records the decision they already
  # describe and converges the compiler onto it.
  - role: context
    unit: { kind: file, path: specs/130-spec-coupling-primary-owner/spec.md }
  - role: context
    unit: { kind: file, path: specs/154-logical-unit-ownership-grammar/spec.md }
---

# Feature Specification: Spec-Spine Library Grammar Adoption

**Feature Branch**: `feat/216-spec-spine-library-grammar-adoption`
**Created**: 2026-06-14
**Status**: Draft (Phase 1 implementable; Phase 2 sequenced)
**Input**: Follow-up to PR #358 (`fix(125): migrate structured amends to
bare-id + refines`) and the 2026-06-14 clarification callouts added to
spec 130 §2.5 and spec 154 §5.

## Overview

The standalone spec-spine library (the published grammar OAP is converging
onto under the collapse mission) models the relationship edges as typed
Rust fields: `amends: Vec<String>` and structured `supersedes`, rejecting
malformed entries at deserialization (`spec-spine-types/src/frontmatter.rs`).
OAP's in-tree spec-compiler reaches the same observable contract for the
common case but by a weaker mechanism, and the two diverge precisely where
input is malformed.

OAP parses both `amends` and `supersedes` through one helper,
`optional_string_list` (`tools/spec-spine/spec-compiler/src/lib.rs`, helper
defined at `:1370`):

```rust
fn optional_string_list(m: &serde_yaml::Mapping, key: &str) -> Option<Vec<String>> {
    let v = m.get(key)?;
    let arr = v.as_sequence()?;
    let mut out = Vec::new();
    for x in arr {
        out.push(x.as_str()?.to_string());   // non-string entry -> None for the whole field
    }
    Some(out)
}
```

Because `x.as_str()?` returns `None` for the whole call on any non-string
entry, and the call sites apply `.unwrap_or_default()`, an object-form
entry is **silently discarded**: the field collapses to an empty list, no
relationship is recorded, and no authority is conferred. This is the latent
defect PR #358 corrected in spec 125 (an `amends: [{spec, unit}]` that
cleared no ownership). The frontmatter known-key comment at
`tools/shared/spec-types/src/lib.rs:121` ("rebound to support object form
… when relationship-graph semantics are desired") describes a capability
that was never wired through the compiler and is, today, misleading.

This spec converges OAP's grammar onto the library and makes the silent
drop legible, in two phases. **Phase 1** narrows `amends` (cosmetic: both
systems already mean bare-id; only OAP's failure mode is wrong) and is
implementable now. **Phase 2** reconciles `supersedes`, where the silent
drop discards a structured form the library *honours* (partial
supersession) and the relationship graph documents: that is a change to
authority-transfer semantics, the coupling gate's most load-bearing path,
and is sequenced separately so it gets the review attention it warrants.

---

## Phase 1: `amends` is bare-id only (implementable now)

### 1.1 Current state (the real mechanism, not an object-form arm)

There is **no object-form `amends` parser arm to remove**. OAP already
captures `amends` as a list of ids
(`optional_string_list(fm, "amends").unwrap_or_default()`,
`lib.rs:275`). The library models `amends` as `Vec<String>` and rejects an
object entry with a clean deserialization error
(`spec-spine-types/src/frontmatter.rs:164`). OAP's observable contract is
already the same (bare-id); only the *failure mode* differs (OAP drops
silently, the library rejects loudly).

### 1.2 Decision

Narrow OAP's accepted `amends` grammar to bare-id only, and make the silent
drop a hard, legible compile error. An `amends` edge grants co-authority
over the predecessor's `spec.md` (named by the id; no `unit:`/`paths:` is
needed or honoured) and confers **no** code authority. Section-scoped
amendment intent continues to use the existing `amends_sections:` field
(spec 132), not an object form. Code authority belongs on
`refines:`/`extends:`, which the coupling gate honours.

### 1.3 Functional Requirements (Phase 1)

- **FR-001 (grammar).** `amends` is a list of spec-id strings
  (`Vec<String>`), matching `spec-spine-types::frontmatter.amends`. No
  object/mapping entry is accepted.
- **FR-002 (loud failure replaces silent drop).** When any `amends` entry
  is a non-string, the spec-compiler emits a new error-severity validation
  code **V-033** on the owning spec, instead of the current empty-list
  collapse. (V-033 was confirmed free across the spec-compiler and
  spec-lint crates as of 2026-06-14; re-confirm against the live code table
  at implementation time, since lower gaps are emission-vs-reservation
  ambiguous.) The message names the migration path verbatim:
  > `amends` takes spec ids only (`amends: ["NNN-slug", ...]`); an object
  > entry confers no authority and is dropped. To claim code, use
  > `refines:`/`extends:` units; to scope to sections of the amended spec,
  > use `amends_sections:` (spec 132). See spec 130 §2.5.

  Implementation: replace the `optional_string_list(fm, "amends").unwrap_or_default()`
  site at `lib.rs:275` with a parse that distinguishes "absent" (Ok, empty)
  from "present but contains a non-string" (V-033 error), mirroring the
  library's reject-on-object behaviour.
- **FR-003 (comment truth).** Correct the known-key comment at
  `tools/shared/spec-types/src/lib.rs:121` to state that `amends` is bare-id
  only and that object-form entries are rejected (V-033). Remove the
  "rebound to support object form" claim.
- **FR-004 (docs amendment record).** When Phase 1 lands, this spec becomes
  the formal amender of the 2026-06-14 clarification callouts in spec 130
  §2.5 and spec 154 §5: add `amends: ["130-spec-coupling-primary-owner",
  "154-logical-unit-ownership-grammar"]` to this spec's frontmatter, and
  bump `amended:` / set `amendment_record:` to this spec's id in both
  files, folding the PR-#358 clarification notes into a spec-recorded
  amendment. (Deferred to implementation deliberately: at draft-file time
  the callouts stand on their own as notes recording a landed PR decision,
  per the bookkeeping in the 2026-06-14 clarification commit.)

### 1.4 Non-goals (Phase 1)

- **L-005 is untouched.** L-005 (spec-lint `lib.rs:516`) governs
  bare-string paths inside workspace members (file → crate migration, spec
  154 §3.1), not `amends`. Phase 1 does not modify it. Citing L-005 for
  `amends` would be a false mechanism.
- **`supersedes` is Phase 2, not Phase 1.** It routes through the same
  `optional_string_list` site (`lib.rs:282`) and so silently drops the
  structured partial form. Reconciling it changes authority-transfer
  semantics, not just a failure mode; see Phase 2.
- **No registry schema bump.** `amends` already emits as a string list; the
  registry schema (frozen by spec 130/132 invariant-freeze) is unchanged.
  This is a validation narrowing on the input side only, additive-compatible
  per spec 153: no previously-*valid* document changes meaning, because the
  only documents affected are object-form `amends`, which were silently void
  already.

### 1.5 Acceptance Criteria (Phase 1)

- **AC-001.** A fixture spec with `amends: [{spec: "001", unit: {...}}]`
  fails `spec-compiler compile` with V-033 (error), naming the bare-id +
  `refines`/`amends_sections` remediation. (Previously: compiled, `amends`
  silently empty.)
- **AC-002.** A fixture spec with `amends: ["001-foo", "002-bar"]` compiles
  unchanged and records both ids.
- **AC-003.** Registry output for the *current* corpus is byte-identical
  before and after the change (zero live object-form `amends` remain after
  PR #358), proving the narrowing is behaviour-preserving for the real
  corpus.
- **AC-004.** `registry-consumer validate-graph` returns no problems;
  spec-lint `--fail-on-warn` exits 0.
- **AC-005.** The OAP corpus compiles under the standalone spec-spine
  library with no `amends`-related rejection (the convergence goal PR #358
  began).
- **AC-006.** Spec 130 and spec 154 frontmatter carry this spec's id in
  `amendment_record:` and a bumped `amended:` date (FR-004).

---

## Phase 2: `supersedes` partial-supersession reconciliation (sequenced)

### 2.1 The asymmetry

OAP's compiler also routes `supersedes` through `optional_string_list`
(`lib.rs:282`), silently dropping the structured partial form
(`{spec, scope: partial, paths|units, rationale}`) that spec 130 §2.4 and
spec 154 §5 document and that the standalone library honours (the library's
partial-supersession design). So a spec authored with structured partial
supersession compiles in OAP as `scope: full` for each id (or as nothing,
if the entry is a non-string mapping), discarding the per-unit scope that
makes partial supersession meaningful.

### 2.2 Why this is sequenced, not folded into Phase 1

Unlike `amends` (where both systems already mean bare-id and only the
failure mode differs), `supersedes` is a genuine semantic divergence: the
library computes a different authority set for a partially-superseded path
than OAP does today. Reconciling it changes `authorities(P)` (spec 130 §
"Authority as a derived property", spec 154 §6), which the coupling gate
(spec 133) consumes on its most load-bearing path. That work needs its own
fixture matrix and review pass; landing it under the same FR set as the
cosmetic `amends` change would bury the risk.

### 2.3 Scope (to be refined to implementable FRs before Phase 2 starts)

Phase 2 will, at minimum:

1. Parse structured `supersedes` (`scope: full | partial`, per-unit/path
   targets) into the typed form the library and the relationship graph
   already define, replacing the `optional_string_list` site at `lib.rs:282`
   with a typed parse that distinguishes full from partial and rejects
   malformed entries (a sibling V-code).
2. Carry per-unit partial scope through to `authorities(P)` so the coupling
   gate's supersession resolution matches the library and the documented
   semantics.
3. Add a fixture matrix proving full vs partial supersession resolve to the
   correct authority sets, and prove the OAP corpus compiles under the
   library with no `supersedes`-related rejection.

Phase 2 FRs/ACs are intentionally left as scope here; this section is
promoted from sketch to implementable FRs as a follow-up, the same way the
ASI-gap batch (specs 200, 202) was refined before implementation.

---

## Success Criteria

- **SC-001.** OAP's spec-compiler and the standalone spec-spine library
  accept and reject the same `amends` documents (Phase 1) and the same
  `supersedes` documents (Phase 2): the convergence is verifiable by
  compiling the OAP corpus under both and getting no grammar-class
  divergence.
- **SC-002.** Malformed relationship-edge input fails loudly at compile
  with a named V-code and a remediation message, instead of being silently
  dropped. No future spec 125-class defect (authority that was never
  recorded) can pass compilation undetected.
- **SC-003.** No previously-valid spec changes meaning: the registry is
  byte-identical across Phase 1 for the real corpus.

## Coupling and migration

Phase 1 touches `tools/spec-spine/spec-compiler/src/lib.rs` (V-033 + the
parse site) and `tools/shared/spec-types/src/lib.rs` (the known-key
comment), both declared under this spec's `refines:`
`amends-supersedes-frontmatter-grammar` aspect; the spec 130/154 frontmatter
bump (FR-004) is coupling-clean as each spec owns its own `spec.md`. The
featuregraph golden gains this spec's row under the `extends: 034` edge.
Regenerate the codebase index and the featuregraph golden in the
implementing commit. Phase 2 extends the same `refines:` aspect to the
`supersedes` parse site and is coupling-gated separately.
