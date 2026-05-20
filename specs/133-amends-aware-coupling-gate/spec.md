---
id: "133-amends-aware-coupling-gate"
slug: coupling-gate
title: "Coupling Gate — authority derivation, satisfaction semantics, constraint evaluation"
status: approved
implementation: complete
owner: bart
created: "2026-05-03"
approved: "2026-05-19"
kind: governance
risk: low
depends_on:
  - "101"
  - "130"
code_aliases: ["COUPLING_GATE"]
establishes:
  - tools/spec-code-coupling-check/src/lib.rs
  - tools/spec-code-coupling-check/src/main.rs
extends:
  - spec: "101-codebase-index-mvp"
    paths:
      - tools/codebase-indexer/src/lib.rs
    nature: additive
co_authority:
  - paths:
      - tools/spec-code-coupling-check/src/lib.rs
    section: authority-derivation
    with_specs:
      - "130-spec-coupling-primary-owner"
      - "152-path-co-authority"
summary: >
  The derivation algorithm, satisfaction semantics, constraint evaluation,
  and output format of the spec/code coupling gate. Given a diff and the
  compiled spec corpus (via spec 101's codebase-index), the gate decides
  whether each edited path's current authority is touched, evaluates
  separate constraint invariants, and reports failures in precise
  authority-set form rather than historical multi-claim noise. The
  authority function `authorities(P)` is exposed as a library API for use
  by registry-consumer and other consumers.
---

# 133 — Coupling Gate

## 1. Concern

This spec owns the **gate logic**: the algorithm that, given a diff and
the spec corpus, decides which edited paths are satisfied and which are
not. The CI job and contributor-facing affordances are owned by spec 127
(spec-code-coupling-workflow); the relationship-field semantics are
owned by spec 130 (spec-relationship-graph); the section-matching rules
are owned by spec 152 (path-co-authority).

The library API exposed here (`authorities(P)` and `authorities(P, S)`)
is the canonical query surface — any consumer that needs to ask "who
governs this path?" calls into this crate, not a re-implementation.

## 2. Inputs

The gate reads:

- **`build/codebase-index/index.json`** via typed deserialization
  (spec 101's `CodebaseIndex` type, governed-read per spec 103).
- The **diff** as a list of edited paths (`--paths-from` file or
  computed via `git diff --name-only <base>...<head>`).
- The **PR body** (optional; source of `Spec-Drift-Waiver:` lines).

The codebase-index carries the spec corpus's derived `implements:`
projection (per spec 130) along with each spec's relationship fields.
The gate does not re-parse spec.md files; it consumes the index
projection.

## 3. Authority derivation

For each path P appearing in the diff:

```
authorities(P) = {
  spec :
    (P ∈ spec.establishes)
    ∨ (∃ extends_entry ∈ spec.extends :
         P ∈ extends_entry.paths)
    ∨ (∃ refines_entry ∈ spec.refines :
         P ∈ refines_entry.paths)
    ∨ (∃ co_authority_entry ∈ spec.co_authority :
         P ∈ co_authority_entry.paths)
  ∧ ¬ ∃ later_spec :
       (∃ supersedes_entry ∈ later_spec.supersedes :
          supersedes_entry.spec = spec
          ∧ supersedes_entry.scope = full)
     ∨ (∃ supersedes_entry ∈ later_spec.supersedes :
          supersedes_entry.spec = spec
          ∧ supersedes_entry.scope = partial
          ∧ P ∈ supersedes_entry.paths)
}
```

Supersession resolution operates over the corpus once at gate startup
(O(N) for N specs), producing a transitively-resolved authority map.
When a chain `A ← B ← C` exists (B supersedes A, C supersedes B), the
gate resolves C's authority over A's surface transitively.

### 3.1 Legacy spec handling

For specs that author `implements:` directly (without relationship
fields and without `origin: retroactive: true`), the gate synthesizes a
relationship-graph view: each path under `implements:` is treated as
the spec's `establishes:` claim. This synthesis is performed by the
codebase-indexer during projection, so the gate sees a uniform
relationship-graph view regardless of authoring mode.

This is the bridge that lets the gate produce coherent output on a
mixed corpus during migration. The end-state (post-curated annotation
pass) is a corpus where every spec uses typed relationship fields and
the synthesis is a no-op.

## 4. Satisfaction semantics

For each edited path P in the diff, the gate computes:

```
hunks(P) = diff hunks touching path P
for each H ∈ hunks(P):
  if P has co_authority claims:
    S = section_containing(H)      # per spec 152 §2.2
    A = authorities(P, S)
  else:
    A = authorities(P)

  if A = ∅:
    if P (or P#S) matches an empty-authority-by-rule pattern:
      satisfied
    else:
      fail (path lacks current authority — spec required)
  else:
    if ∃ spec ∈ A : spec.spec_md is edited in diff
       ∨ ∃ spec ∈ A : ∃ amender X : X.amends entry references spec
                       ∧ X.spec_md is edited in diff:
      satisfied
    else:
      fail (current authority not touched; report A)
```

The amender clause preserves the spec-119 / spec-133 v1 amendment-as-
satisfaction property: editing an amending spec satisfies coupling for
the amended spec. This is by design — amendments are deliberate spec-
spine touches; treating them as authority-satisfying preserves the
"every spec change deserves visibility" property without forcing
synthetic empty edits to the amended spec.

## 5. Constraint evaluation

Distinct from authority satisfaction, the gate evaluates each
`constrains:` spec C in the corpus against the diff:

```
for each constraining spec C in corpus:
  for each constrains_entry ∈ C.constrains:
    if constrains_entry.kind = "invariant-freeze":
      check_invariant_freeze(C, constrains_entry, diff)
    # other kinds: check_<kind>(...)
```

`check_invariant_freeze` examines the diff for edits to the constrained
paths that *remove or narrow* a frozen invariant. The specific
mechanics depend on the path:

- **Schema files** (`*.schema.json`): a frozen field removed from
  `required:`, a property's type narrowed in a non-additive way, or
  a closed enum widened against a freeze flag.
- **Spec markdown files**: a frozen `## §` heading removed when its
  anchor is listed in the constraining spec's `unamendable:` set
  (preserves spec 132 V-011 semantics).
- **Source files**: pattern-based; the constraining spec declares the
  pattern. (V1 implementation: no pattern engine; v2 amendment will
  add one if a use case appears.)

Constraint failures exit with code 3 (distinct from authority failures
at code 1), so CI log inspection can distinguish the two cases.

## 6. Library API

The gate's authority function is exposed as a library entry point:

```rust
// tools/spec-code-coupling-check/src/lib.rs
pub fn authorities(index: &CodebaseIndex, path: &str) -> Vec<SpecId>
pub fn authorities_in_section(
    index: &CodebaseIndex,
    path: &str,
    section: &str,
) -> Vec<SpecId>
pub fn evaluate_constraints(
    index: &CodebaseIndex,
    diff: &[DiffEntry],
) -> Vec<ConstraintViolation>
```

`registry-consumer` consumes this API for the `--by-authority <path>`
and `--show-relationships <spec-id>` query verbs (spec 002 series'
follow-up). Future audit tools (an authority-map dashboard, a
relationship-graph visualization) consume the same API.

## 7. Output format

The gate produces one of three outputs:

### 7.1 Silent success

```
spec-code-coupling-check: 142 paths satisfied (no violations).
```

One line, exit 0. The path count is included so contributors confirm
the gate actually ran (vs. silently skipping inputs).

### 7.2 Authority failure

```
spec-code-coupling-check: 1 path requires authority touch.

  tools/factory-engine/src/lib.rs
    current authority: 075-factory-engine-mvp (establishes)
    section: (whole-file)
    fix: edit specs/075-factory-engine-mvp/spec.md
```

Per failing path: the authority set (typically 1–3 specs), the rule
that fired, the section if applicable, and the prescribed fix. No
mention of historical claimants.

### 7.3 Constraint failure

```
spec-code-coupling-check: 1 constraint violation.

  standards/schemas/spec-spine/registry.schema.json:152
    constraint: 132-constitutional-invariant-freeze (invariant-freeze)
    violation: the field `supersededBy` is frozen; removal requires
               amending spec 132 to retire the freeze.
```

Exit code 3, distinct from authority failures (code 1) and invocation
errors (code 2).

## 8. Migration and recursive verification

This spec's own commit must satisfy its own gate. The recursive
verification works because:

- This spec's `establishes:` claims `tools/spec-code-coupling-check/*`,
  so edits to that crate require touching this spec's spec.md (which
  the commit does).
- The relationship-field parsing lives in spec 001 (spec-compiler-mvp),
  which this spec extends; the spec-compiler edits in this commit are
  satisfied because spec 130 establishes the parser-relevant types.
- The schema edits in this commit are satisfied because spec 000
  (bootstrap-spec-system) establishes the registry schema, and spec 130
  constrains it via `invariant-freeze`.
- For paths whose current authority is a spec not edited in this
  commit, the migration relies on `origin: retroactive: true` markers
  carrying forward existing `implements:` claims (per spec 130 §2.8).

## 9. Performance

The gate runs in O(N × M) for N edited paths and M corpus specs.
Typical PR: N ≤ 50, M = 152, so ~7,600 lookups. Index load is the
dominant cost (~50 ms warm). Authority derivation is sub-millisecond
per path; constraint evaluation depends on constraint kind (invariant-
freeze: O(constrained-paths × diff-hunks)).

## 10. Cross-references

- Spec 127 — workflow contract (CI job, make target, contributor flow)
- Spec 130 — relationship-graph field semantics
- Spec 152 — section matching, empty-authority-by-rule patterns
- Spec 101 — codebase-index-mvp (input source)
- Spec 103 — governed-artifact reads
- Spec 132 — constitutional-invariant-freeze (canonical `constrains:` example)
