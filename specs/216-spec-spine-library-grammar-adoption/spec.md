---
id: "216-spec-spine-library-grammar-adoption"
title: "Spec-Spine Library Grammar Adoption (amends bare-id; supersedes-partial reconciliation)"
feature_branch: "feat/216-spec-spine-library-grammar-adoption"
status: draft
implementation: in-progress  # Phase 1 landed (V-033 bare-id `amends`, all six Phase-1 ACs). Phase 2a in progress (§2.3: typed `supersedes` parse + V-034 + structured emission + additive schema widen). Phase 2b (coupling-gate supersession filtering of `authorities(P)`) stays sequenced per §2.4 until its own review pass. Stays `draft` pending Phase 2b + approval.
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
  - "153-invariant-freeze-additive-evolution"
  - "154-logical-unit-ownership-grammar"
code_aliases: ["SPEC_SPINE_LIBRARY_GRAMMAR_ADOPTION"]
extends:
  # New spec adds a row to the featuregraph golden (same precedent as
  # specs 212, 202, 196, 194, 193, 187, 183).
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
  # Phase 2a additively widens the registry schema's `supersedes.items` to a
  # string|object oneOf (FR-008). Strictly additive evolution of the
  # spec-132-frozen schema, authorized by spec 153; mirrors spec 154's own
  # `extends:` edge on this same file for the establishes/references grammar.
  - spec: "130-spec-coupling-primary-owner"
    nature: additive
    unit: { kind: file, path: standards/schemas/spec-spine/registry.schema.json }
  # This spec adds fixture tests (v033_amends_bare_id, v034_supersedes_typed)
  # to the spec-compiler test suite; additively extend 001's tests directory,
  # mirroring spec 154's own edge on this path. (Phase 1's v033 test was
  # covered only incidentally by Phase 1's spec 154 edit; declaring the edge
  # makes 216's authority over its own test files explicit.)
  - spec: "001-spec-compiler-mvp"
    nature: additive
    unit: { kind: directory, path: tools/spec-spine/spec-compiler/tests }
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
amends:
  # Phase 1 landed (FR-004): this spec formally amends the 2026-06-14 bare-id
  # `amends` clarification callouts in spec 130 §2.5 and spec 154 §5, folding
  # the PR-#358 clarification notes into a spec-recorded amendment. The amends
  # edge confers co-authority over each predecessor's spec.md (named by id; no
  # unit:/paths: is honoured) and no code authority; the compiler-convergence
  # code authority lives on `refines:` above. (Replaces the draft-time
  # `references: role: context` placeholder these ids previously carried.)
  - "130-spec-coupling-primary-owner"
  - "154-logical-unit-ownership-grammar"
---

# Feature Specification: Spec-Spine Library Grammar Adoption

**Feature Branch**: `feat/216-spec-spine-library-grammar-adoption`
**Created**: 2026-06-14
**Status**: Draft (Phase 1 landed; Phase 2a implementable; Phase 2b sequenced)
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

Phase 2 splits into **Phase 2a** (the producer side: parse, validate, emit
the structured form, plus the additive schema widen) and **Phase 2b** (the
consumer side: carry the now-emitted partial scope through to
`authorities(P)` and the coupling gate). 2a is implementable now and
behaviour-preserving for *consumers* (the gate ignores `supersedes` today, so
emitting the structured form changes no gating decision); 2b is the
authority-semantics change that carries the review weight and is sequenced
separately.

#### Grammar reconciliation (settled here)

§2.1 above referred to the partial form as `{spec, scope: partial,
paths|units, rationale}`. The canonical partial-scope key is **`unit:`**, not
`paths:`. Spec 154 §6 modernised the relationship-field grammar to logical
units; the standalone library (`spec-spine-types::edges::SupersedeScoped`)
and every live partial-supersession spec (114, 199, 214) use `unit:` (or a
prose `note:`). `paths:` survives only in spec 130 §2.4's pre-154 doc
template and is used by no live spec. Phase 2a converges on `unit:` and FR-009
amends spec 130 §2.4's template to match. The accepted envelope keys are
`{spec, scope, unit, note, rationale}` (mirroring the library's
`deny_unknown_fields`); a partial entry may scope by `unit:` (114, 214) **or**
by a prose `note:` with no unit (199 is the live precedent), so partial does
**not** require `unit:`.

### 2.3 Phase 2a: typed `supersedes` parse + structured emission (implementable now)

#### Functional Requirements (Phase 2a)

- **FR-005 (typed parse).** Replace the `optional_string_list(fm,
  "supersedes")` site at `lib.rs:282` with a typed parse accepting a bare
  spec-id string (full-scope shorthand) or a `{spec, scope: full|partial,
  unit?, note?, rationale?}` object, mirroring
  `spec-spine-types::edges::SupersedeItem` (untagged `Full(String) | Scoped`).
- **FR-006 (loud reject, V-034).** A malformed `supersedes` entry emits a new
  error-severity **V-034** on the owning spec instead of the silent
  empty-list collapse. Malformed = a non-string non-mapping entry; a mapping
  missing `spec`; a `scope` value outside `{full, partial}`; or an unknown
  envelope key (allowed: `spec, scope, unit, note, rationale`), mirroring the
  library's `deny_unknown_fields`. The `unit:` value's internal shape is
  already validated by V-021..V-024 (`validate_relationship_units`); V-034
  covers the envelope only. Partial scope does **not** require `unit:` (a
  prose `note:` suffices; spec 199 is the live precedent). (V-034 confirmed
  free across spec-compiler + spec-lint as of 2026-06-15.)
- **FR-007 (structured emission + full normalisation).** Emit the structured
  form into `registry.json`: a full-scope entry (bare string, `{spec}`, or
  `{spec, scope: full}`) normalises to a bare spec-id string; a partial-scope
  entry emits `{spec, scope: "partial", unit?, note?, rationale?}` with the
  unit re-canonicalised. Mirrors the library's `normalize_supersedes`. The
  `Feature.supersedes` field widens from `Option<Vec<String>>` to
  `Option<Value>` (the `establishes` precedent at `lib.rs:1045`).
- **FR-008 (additive schema extension).** Widen `registry.schema.json`'s
  `supersedes.items` from `{type: string}` to `oneOf: [{type: string},
  {$ref: supersedeScopedItem}]`, adding the `supersedeScopedItem` `$def`
  (`additionalProperties: false`, `required: [spec]`, the `unit` field
  `$ref`-ing `logicalUnit`). Strictly additive per spec 153 (every prior
  array-of-strings stays valid), authorised by this spec's `extends:
  {nature: additive, unit: registry.schema.json}` edge (the spec 154
  precedent on this same file). No `specVersion` bump (additive item-shape
  widening, as 154's establishes extension was).
- **FR-009 (comment + doc truth).** Correct the `supersedes` known-key comment
  at `tools/shared/spec-types/src/lib.rs` to describe the typed grammar
  (bare string = full; object = `{spec, scope, unit?, note?, rationale?}`;
  malformed rejected with V-034). Amend spec 130 §2.4's doc-template `paths:`
  to the canonical `unit:` form per the reconciliation above.

#### Acceptance Criteria (Phase 2a)

- **AC-007.** A fixture with `supersedes: [{spec, scope: partial, unit: {kind:
  file, path: ...}}]` compiles and the registry records the structured
  partial entry verbatim (previously: silently dropped to `null`).
- **AC-008.** Fixtures `supersedes: ["001-x"]`, `[{spec: "001-x"}]`, and
  `[{spec: "001-x", scope: full}]` all compile and emit the byte-identical
  bare-string form `["001-x"]` (full normalisation).
- **AC-009.** A fixture with a malformed entry (missing `spec`, bad `scope`,
  unknown key, or non-string-non-map) fails compile with V-034 naming the
  remediation. A partial entry with only a `note:` (no `unit:`) does **not**
  trip V-034.
- **AC-010.** The real OAP corpus compiles: spec 214's four partial entries
  and spec 199's note-scoped partials appear structured in the registry (no
  longer `null`); specs 073/108/154's full entries appear as bare strings.
  `registry-consumer validate-graph` clean; `spec-lint --fail-on-warn` exit 0.
- **AC-011.** The compiler's embedded-schema self-validation passes on the
  structured registry. The widen is additive: a registry with only
  bare-string `supersedes` still validates.
- **AC-012.** The emitted grammar is structurally equivalent to the library's
  `SupersedeItem` model (`Full(String) | Scoped{spec, scope, unit, note,
  rationale}`), so OAP and the library accept/reject the same `supersedes`
  documents (the SC-001 convergence goal for the parse/emit half).

### 2.4 Phase 2b: coupling-gate supersession filtering (sequenced)

The consumer half. With 2a emitting partial scope into the registry, 2b
carries it through to `authorities(P)`:

1. Align `registry-consumer`'s structured-`supersedes` reader (`lib.rs:1016`)
   from `paths:` to `unit:` so `show-relationships` / `by-authority` see the
   per-unit partial scope (today the reader extracts `paths`, absent on the
   `unit:` form; the full item survives in `meta`, so 2a loses no data).
2. Teach the coupling gate's `legitimate_owners()`
   (`spec-code-coupling-check/src/lib.rs:794`) to exclude full- and
   partially-superseded predecessors from the authority set. The gate does
   **zero** supersession filtering today, so this is the load-bearing
   `authorities(P)` change spec §2.2 flags.
3. A fixture matrix proving full vs partial supersession resolve to the
   correct authority sets, plus a corpus pass proving no spec loses
   legitimate authority unexpectedly.

Phase 2b FRs/ACs are promoted from this sketch in its own PR, the same way 2a
was promoted here.

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
- **SC-003.** No previously-valid spec changes meaning. Phase 1 is
  registry-byte-identical for the real corpus (zero live object-form
  `amends`). Phase 2a is intentionally **not** byte-identical: it is an
  authority-representation change, so specs that already author structured
  partial `supersedes` (214, 199) move from `null` to the structured form
  they always meant. No previously-*valid* `supersedes` document changes
  meaning (the widen is additive; bare strings and full objects still
  normalise to the same bare strings).

## Coupling and migration

Phase 1 touches `tools/spec-spine/spec-compiler/src/lib.rs` (V-033 + the
parse site) and `tools/shared/spec-types/src/lib.rs` (the known-key
comment), both declared under this spec's `refines:`
`amends-supersedes-frontmatter-grammar` aspect; the spec 130/154 frontmatter
bump (FR-004) is coupling-clean as each spec owns its own `spec.md`. The
featuregraph golden gains this spec's row under the `extends: 034` edge.
Regenerate the codebase index and the featuregraph golden in the
implementing commit.

Phase 2a extends the same `refines:` aspect to the `supersedes` parse site
(V-034) in `lib.rs` and the known-key comment in `spec-types`, both already
declared. It additionally edits `standards/schemas/spec-spine/registry.schema.json`
(FR-008), authorised by the new `extends: {nature: additive, unit:
registry.schema.json}` edge (the spec 154 precedent on this file), and amends
spec 130 §2.4's doc-template (FR-009), coupling-clean via this spec's existing
`amends: [130]` co-authority over 130's `spec.md`. Regenerate the codebase
index and the featuregraph golden in the implementing commit (spec 214's and
spec 199's `supersedes` now emit structured). Phase 2b (the coupling-gate
`legitimate_owners()` filtering and the `registry-consumer` reader alignment)
is coupling-gated separately in its own PR.
