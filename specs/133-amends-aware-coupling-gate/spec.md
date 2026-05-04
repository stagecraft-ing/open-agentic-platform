---
id: "133-amends-aware-coupling-gate"
slug: amends-aware-coupling-gate
title: "Amends-aware spec/code coupling gate — recognise `amends:` as valid coupling"
status: approved
implementation: complete
owner: bart
created: "2026-05-03"
approved: "2026-05-04"
closed: "2026-05-04"
kind: governance
risk: low
depends_on:
  - "119"  # project-as-unit-of-governance (introduced the amends:/amendment_record: protocol)
  - "127"  # spec-code-coupling-gate (the gate this spec extends)
  - "130"  # spec-coupling-primary-owner (the heuristic this spec composes with)
code_aliases: ["AMENDS_AWARE_COUPLING"]
implements:
  - path: tools/spec-code-coupling-check
  - path: tools/codebase-indexer
  - path: schemas/codebase-index.schema.json
summary: >
  Spec 127's gate currently only recognises `implements:` as a coupling
  relationship. When a spec is edited via the `amends:` mechanism (spec 119
  protocol), the amended spec's path is not seen as covered by the amender,
  even though the amender carries `amends: ["NNN"]` and the amended spec
  carries the symmetric `amendment_record:` back-link. This spec extends
  the gate to honour `amends:` as a valid coupling alongside `implements:`,
  closing a class of false positives that surfaced on 2026-05-02 when spec
  132's amendment to spec 000 forced waivers on every subsequent commit.
---

# 133 — Amends-Aware Spec/Code Coupling Gate

## 1. Problem Statement

Spec 127 introduced a PR-time gate that fails when a diff path is claimed
by a spec's `implements:` list but the claiming spec's `spec.md` is absent
from the same diff. Spec 130 amended that to a primary-owner heuristic
(any one claimant's edit clears the path).

Both versions of the rule operate on `implements:` only. They are blind
to the `amends:` relationship introduced by spec 119. This produces two
predictable false-positive patterns:

### 1.1 The amended-spec edit case

When a spec landing applies an amendment to an older spec — e.g. spec 132
amends spec 000 by adding `unamendable:` anchors — the historical sequence is:

1. The amender (spec 132) lands first as a tooling-only change. Its
   frontmatter declares `amends: ["000"]`.
2. The amended spec (000) is later edited to apply the amendment. A
   `amendment_record: "132-..."` line is added to spec 000's frontmatter
   (and `amended:` date), per spec 119 §"Amendment frontmatter convention".

At step 2, the gate sees `specs/000-bootstrap-spec-system/spec.md` in the
diff. It looks up `implements:` claimants and finds spec 119
(`project-as-unit-of-governance`, the spec that introduced the amendment
protocol — which legitimately claims spec 000 as part of the protocol's
implementation surface). The gate then demands an edit to spec 119's
spec.md or a waiver. But spec 119 has nothing to do with this particular
amendment — spec 132 does. The gate is asking the wrong owner.

### 1.2 The cascade-amender case

If spec 132 amends both spec 000 and spec 087 in a single landing (a real
scenario: spec 119 itself amended five specs at once), each amended spec's
edit fires the gate against `implements:` claimants that have no relation
to the amendment. Each requires a waiver. The waivers are accurate but
high-noise — they document the gate's own blindness, not a real coupling
gap.

### 1.3 Why this matters now

This problem was theoretical until 2026-05-02, when spec 132 landed and
the `/tmp/spec_000_proposed_amendment.diff` was applied to spec 000.
Every subsequent commit on the same branch then needed a `Spec-Drift-
Waiver:` for spec 000 even when the commit had nothing to do with the
amendment. The friction is real: commits `ecdba50` and `de72f8b` both
carry the same waiver text, and the pattern would repeat on every commit
until the branch is pushed (resetting the diff baseline).

## 2. Goals

- **Recognise `amends:` as a valid coupling.** When a diff edits an
  amended spec's `spec.md`, an edit to any amender (a spec with
  `amends: ["<that-spec>"]` in its frontmatter) clears the path — exactly
  the same primary-owner semantics spec 130 introduced for `implements:`.
- **Recognise `amendment_record:` as a valid coupling from the other
  direction.** When a diff edits both a spec body and adds/changes
  `amendment_record:` in another spec's frontmatter, the symmetric link
  alone establishes coupling.
- **Compose with spec 130's heuristic.** The gate continues to require
  *any one* legitimate owner's edit, not all. The amender is a
  legitimate owner of an amendment-driven edit. The original primary
  `implements:` owner is also a legitimate owner. Either clears the
  path.
- **No new authoring burden.** Amenders already carry `amends:` per
  spec 119; amended specs already carry `amendment_record:`. This spec
  extends the gate to *read* what the protocol already produces.

## 3. Scope

### In scope

- `tools/spec-code-coupling-check/src/lib.rs`: extend the coupling
  resolver to walk amender→amended relationships when scoring a path.
- `tools/spec-code-coupling-check/src/index.rs` (or wherever the index
  is loaded): surface `amends:` and `amendment_record:` from the
  codebase-index into the resolver's input set.
- Schema review of `build/codebase-index/index.json`: confirm whether
  amender/amendment_record links are already captured by the
  codebase-indexer, and surface them in the index if not.
- New integration test under `tools/spec-code-coupling-check/tests/cli.rs`:
  assert that a fixture pairing `specs/A/spec.md` (amended) +
  `specs/B/spec.md` (amender, `amends: ["A"]`) clears the gate when both
  appear in the diff.
- Update spec 127 and spec 130 cross-references to point to this spec.

### Out of scope

- Rewriting spec 127 or spec 130. Both stand; this spec adds a third
  resolver path alongside `implements:` (spec 127's original) and
  primary-owner (spec 130's amendment).
- Generalising to arbitrary spec-to-spec links (e.g. `superseded_by:`).
  Supersession is a different relationship — the old spec is no longer
  operative — and should not silently clear gate findings on the
  superseded spec. If a use case emerges, it is a separate spec.
- Backfilling `amends:` on historical specs. The protocol started with
  spec 119; pre-119 specs that *should* have used it (e.g. major
  pre-protocol rewrites) are not in scope.

## 4. Functional Requirements

- **FR-001:** When a path `specs/X/spec.md` is in the diff and any spec
  Y in the index declares `amends:` containing `X` (or a slug-prefix
  match), an edit to `Y/spec.md` in the same diff clears the path.
- **FR-002:** When the amended spec's frontmatter carries
  `amendment_record: "<Z>"` and Z is also in the diff, the path is
  cleared regardless of `amends:` direction. (This handles the case
  where an old spec is touched up and the amender link is the only
  back-reference.)
- **FR-003:** The amends-aware resolver composes with spec 130's
  primary-owner heuristic. The set of legitimate owners for a path is
  `implements_claimants ∪ amenders ∪ amendment_record_targets`. Any
  one in the diff clears the path.
- **FR-004:** The renderer for unresolved paths lists all three owner
  classes when reporting violations, labelled by source so reviewers
  can sanity-check coverage. Example output:
  ```
  specs/000-bootstrap-spec-system/spec.md
    implements:        119-project-as-unit-of-governance
    amends:            132-constitutional-invariant-freeze
    amendment_record:  132-constitutional-invariant-freeze
  ```
- **FR-005:** If neither the codebase-index nor the spec frontmatter
  carries the amend links (e.g. on a stale index), the gate falls back
  to the spec 130 behaviour with no error. The new rule strictly
  expands the set of accepted couplings; it never removes existing ones.
- **FR-006:** The codebase-indexer (spec 101) surfaces `amends:` and
  `amendment_record:` in the traceability layer if it does not already.
  Schema bump if needed; consumer-side changes only if the data is
  absent today.

## 5. Implementation Shape

### 5.1 Inputs

`build/codebase-index/index.json` already carries spec frontmatter
`status` and `dependsOn` per spec 102. The index does not currently
surface `amends:` or `amendment_record:`. The first task is to confirm
this and, if true, extend the indexer:

- `tools/codebase-indexer/src/spec_scanner.rs`: read `amends:` (list)
  and `amendment_record:` (string or list) from spec frontmatter.
- `tools/codebase-indexer/src/types.rs`: add `amends: Vec<String>`
  and `amendmentRecord: Option<String>` (or `Vec<String>`) to the
  trace mapping struct.
- `schemas/codebase-index.schema.json`: declare the new fields.
- Bump the codebase-index `schemaVersion` (compile-time const in
  `tools/codebase-indexer/src/types.rs`).

### 5.2 Resolver change

Today's resolver (sketched):

```rust
fn legitimate_owners(path: &str, index: &Index) -> BTreeSet<String> {
    index.mappings_claiming(path).map(|m| m.spec_id.clone()).collect()
}
```

Extended:

```rust
fn legitimate_owners(path: &str, index: &Index) -> BTreeSet<String> {
    let mut owners = BTreeSet::new();

    // Path 1 — implements: (spec 127, refined by spec 130)
    for m in index.mappings_claiming(path) {
        owners.insert(m.spec_id.clone());
    }

    // Path 2 — amends: (this spec, FR-001)
    if let Some(amended_id) = index.spec_id_for_spec_path(path) {
        for m in index.mappings_amending(&amended_id) {
            owners.insert(m.spec_id.clone());
        }

        // Path 3 — amendment_record: (this spec, FR-002)
        if let Some(amended) = index.mapping_for_spec_id(&amended_id) {
            if let Some(record) = &amended.amendment_record {
                owners.insert(record.clone());
            }
        }
    }

    owners
}
```

The renderer changes shape to label each owner by source, per FR-004.

**Strict-expansion gate.** Per FR-005, the amend pathways (Paths 2 and
3) MUST NOT enrol a path that today has zero `implements:` claimants.
If they did, editing one's own `spec.md` would create a new firing
condition whenever some unrelated amender exists but is not in the
diff — converting today's silent paths into tomorrow's failures. The
implementation gates Paths 2 and 3 on `owners.implements.is_empty()
== false`: amend resolution adds owners to a path that is already
firing today; it never elevates a silent path to a firing one. This
is the runtime expression of FR-005's "strictly expands the set of
accepted couplings; it never removes existing ones." Worked example
(§8) preserves correctness because spec 119 already claims
`specs/000-bootstrap-spec-system` via `implements:`, so spec 000's
`spec.md` enters the gate via Path 1; Paths 2 and 3 then add
spec 132 as an additional cleared-via candidate.

### 5.3 No production behaviour change for non-amend cases

A diff that doesn't touch any `specs/*/spec.md` path is unaffected. A
diff that touches `specs/X/spec.md` but no spec amends X resolves
identically to today. The new paths only fire when amend links exist.

## 6. Acceptance Criteria

- **AC-1:** A diff containing `specs/A/spec.md` (existing approved spec)
  + `specs/B/spec.md` (with `amends: ["A"]`) exits 0 even if no
  `implements:` claimant of A is in the diff.
- **AC-2:** A diff containing `specs/A/spec.md` only, where some other
  spec C declares `amends: ["A"]` but C is not in the diff, fails with
  C listed under the `amends:` source class in the renderer output.
  The reviewer can amend C or use a waiver, same as today.
- **AC-3:** A diff containing `specs/A/spec.md` + `specs/D/spec.md`,
  where A's frontmatter has `amendment_record: "D"`, exits 0 by FR-002
  even if D's frontmatter doesn't (yet) have `amends: ["A"]` — the
  symmetry can be one-sided during a partial protocol application.
- **AC-4:** A diff with no `specs/*/spec.md` paths is unaffected — the
  new resolver paths return empty for non-spec paths.
- **AC-5:** The codebase-index renderer (`CODEBASE-INDEX.md`) shows
  `amends:` and `amendment_record:` columns in the traceability layer
  if they are non-empty.
- **AC-6:** Reproducing the 2026-05-02 false-positive case with the
  extended gate clears it without a waiver: a fixture pairing spec 132
  (with `amends: ["000"]`) + spec 000 (with `amendment_record: "132"`)
  clears `specs/000-bootstrap-spec-system/spec.md` when spec 132 is
  also in the diff.

## 7. Risks and Mitigations

- **Risk:** Permissive expansion of accepted couplings could mask real
  coverage gaps — e.g. spec X amends spec Y in 2026-04, then in 2026-08
  someone edits spec Y's body unrelated to the amendment, and X's
  inclusion silently clears the gate.
  **Mitigation:** the renderer labels every cleared owner by source.
  Reviewers see "amends: 132" on the cleared line and can decide
  whether the amender's edit is actually relevant to the spec Y change.
  This is the same review-time discretion spec 130 introduced for the
  primary-owner heuristic — friction in the right place, not coercive
  scale.

- **Risk:** Stale codebase-index missing the new fields breaks the
  resolver.
  **Mitigation:** FR-005 — fall back to spec 130 behaviour. The new
  rule strictly expands acceptance; absence of data never tightens it.

- **Risk:** A future spec resurrects `superseded_by:` or
  `replaced_by:` as a coupling source (the out-of-scope item in §3).
  **Mitigation:** explicitly out of scope; if pursued, write a new
  spec that extends this resolver further with documented rationale.

## 8. Worked Example

Reproducing the 2026-05-02 false-positive (commit `ecdba50`):

```
$ git diff --name-only origin/main HEAD
.github/workflows/ci-supply-chain.yml
Makefile
specs/000-bootstrap-spec-system/spec.md       ← amended via spec 132
specs/116-supply-chain-policy-gates/spec.md
build/codebase-index/index.json
```

**Today's gate (spec 130 only):** fires on
`specs/000-bootstrap-spec-system/spec.md`, demanding an edit to spec
119 (the only `implements:` claimant). Reviewer must add spec 119 to
the diff or supply a waiver.

**Extended gate (this spec):** sees that spec 132 declares
`amends: ["000"]` and spec 000 declares
`amendment_record: "132-constitutional-invariant-freeze"`. Both spec
132 and spec 000 are in the broader diff range (spec 132 landed in an
earlier commit on the same branch, before push). Path is cleared by
the amends: source. No waiver needed.

When the same branch is pushed and the baseline resets, future commits
that edit only `specs/000-bootstrap-spec-system/spec.md` — without
spec 132 also in the diff — will still fire the gate against spec 119
(or against spec 132 if the amend chain is fresh). The amends-aware
extension does not silently clear arbitrary spec 000 edits; it only
clears edits whose amendment provenance is in the same diff.

## 9. Cross-references

- **Spec 119** introduced the `amends:` / `amendment_record:` protocol
  this spec consumes.
- **Spec 127** is the gate this spec extends.
- **Spec 130** is the primary-owner heuristic this spec composes with.
- **Spec 132** is the worked example: it amends spec 000 and surfaces
  the false-positive class.
- **Spec 101 / 102** govern the codebase-index that this spec relies on
  for amend-link surfacing.

## 10. Implementation order (non-normative)

1. Verify whether `build/codebase-index/index.json` already carries
   `amends:` / `amendment_record:` per the spec scanner's existing
   reads (the indexer already reads `dependsOn` and `status`; the
   amend fields may have been silently captured).
2. If absent: extend the indexer (spec 101 amendment, schema bump).
3. If present: extend the gate's resolver only.
4. Add the new integration tests (AC-1 through AC-6 fixtures).
5. Update spec 127 §"Defect log" and spec 130 §"Amendment record"
   with cross-references to this spec.
6. Land in one PR: gate extension + indexer change (if needed) +
   tests + cross-reference updates.
