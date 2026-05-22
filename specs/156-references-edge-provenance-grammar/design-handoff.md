# Spec 156 — Design Handoff (References-edge Provenance Grammar)

**Status:** Captured-intent artifact. Lives until `spec.md` is authored
under this directory. Delete in the spec 156 closure PR — this doc is
ephemeral working state, not governance.

You are picking up the authoring task for spec
`156-references-edge-provenance-grammar`. The design pass is complete:
Q1 (relationship), Q2 (slug), and Q3 (grammar shape) are closed, plus
seven reconciliation items have been accepted that refine the initial
proposal. Authoring only — no further design work required unless the
spec author hits an unanticipated open question in the writing.

---

## Ground truth — read in this order before drafting

1. **Intent doc** — [`docs/owasp/factory/AIDE-VELOCITY-OAP-INTENT.md`](../../docs/owasp/factory/AIDE-VELOCITY-OAP-INTENT.md).
   §3.4 (Knowledge → Requirements lineage), §6.2 stage 6 (LLM
   synthesis emission contract), §9 candidate 5 (the spec slot this
   work occupies), and §7.2 ASI06 / ASI09 (compliance coupling — see
   reconciliation #4 below).
2. **Spec 154** — [`specs/154-logical-unit-ownership-grammar/spec.md`](../154-logical-unit-ownership-grammar/spec.md).
   §4 introduces the `references:` edge as the ninth, non-owning
   relationship; this spec extends it with a sibling `provenance:`
   field. **§3.4 of the intent doc reads loosely** — it suggests
   extending `unit.kind` with new values. That reading is wrong; see
   reconciliation #1 below for the correction and the §1 framing the
   spec must carry.
3. **Spec 155** — [`specs/155-logical-unit-resolution-semantics/spec.md`](../155-logical-unit-resolution-semantics/spec.md).
   Pattern precedent for typed kinds with resolution rules. Cited via
   `references:` with `role: precedent`; no functional dependency.
4. **Spec 102** — [`specs/102-governed-excellence/spec.md`](../102-governed-excellence/spec.md).
   FR-007 (the auditor's verifier does not trust the producer) is the
   load-bearing argument behind the URI shape decision in
   reconciliation #3a.
5. **Spec 154 Segment 3 (PR #187, commit `482ac312`)** — the resolver
   landed. Spec 156 needs an `## Indexer integration` section per
   reconciliation #6 below; the surface to extend is
   `tools/spec-spine/codebase-indexer/src/spec_scanner.rs::push_flat_units`
   and the `ResolvedUnit.kind` enum widening to include `knowledge`
   and `code-fingerprint`.

---

## Closed questions

### Q1 — Relationship typing

```
extends:  [154-logical-unit-ownership-grammar]   nature: additive
references: [155-logical-unit-resolution-semantics]   role: precedent
```

Rationale:

- Spec 154 introduces the `references:` edge. Spec 156 extends that
  edge's grammar by adding the typed-provenance dimension. Direct
  surface extension → `extends:` with `nature: additive`.
- Spec 155 refines resolution semantics for the six unit kinds. The
  new provenance kinds (`knowledge`, `code-fingerprint`) resolve
  against external systems (stagecraft DB, xray content-addressed
  store), not the workspace. They share *style* with 155 but no
  *machinery*. Naming 155 in `extends:` would falsely imply
  functional dependency. Citing 155 via `references:` with
  `role: precedent` makes the influence explicit without coupling.

### Q2 — Slug

`156-references-edge-provenance-grammar`

Mirrors the 154/155 artifact-aspect-form pattern:

- `154-logical-unit-ownership-grammar` — unit + ownership + grammar
- `155-logical-unit-resolution-semantics` — unit + resolution + semantics
- `156-references-edge-provenance-grammar` — references-edge + provenance + grammar

Bikeshed-alternatives considered: `156-knowledge-requirements-provenance`
(underweights the code-fingerprint side), `156-provenance-edge-kinds`
(misnames the artifact — the edge isn't new, the kinds are values
inside its new sub-field), `156-references-edge-provenance-typing`
(also fine but `-grammar` is the 154/155 parallelism cue).

### Q3 — Grammar shape (additive on 154; new sibling field)

`references:` entries gain a sibling `provenance:` field, mutually
exclusive with `unit:`. A single entry carries either a code logical
unit (the spec 154 form) or a typed external provenance pointer (the
spec 156 addition), never both.

```yaml
references:
  # Spec 154 form (unchanged) — code logical unit
  - role: evidence
    unit: { kind: symbol, id: canonical_json::canonicalize_value }

  # Spec 156 form (new) — typed external pointer
  - role: derivation
    provenance:
      kind: knowledge
      ref: "stagecraft://project/<project-uuid>/knowledge/<knowledge-uuid>"

  - role: derivation
    provenance:
      kind: code-fingerprint
      ref: "xray-fingerprint://<sha256>"
```

Field semantics:

- **`provenance.kind`** — enum `{knowledge, code-fingerprint}` for the
  initial set. Future kinds (sbom, cve, decision-record, adr) land via
  amendment specs, not by widening this spec.
- **`provenance.ref`** — typed URI string. Scheme is kind-aligned
  (see V-NN3). Knowledge URIs are project-scoped and self-locating
  (reconciliation #3a); xray-fingerprint URIs are flat for now and may
  gain a scope segment later (reconciliation #3b deferred).
- **`role:`** stays open-vocabulary per spec 154. `role: derivation` is
  *recommended* for provenance entries but not enforced (V-NN5 advisory).

V-rules (4 errors + 1 advisory):

| Code   | Severity | Rule |
|--------|----------|------|
| V-NN1  | error    | A `references:` entry MUST carry exactly one of `{unit:, provenance:}`. Both present → error. |
| V-NN2  | error    | `provenance.kind` MUST be one of `{knowledge, code-fingerprint}`. |
| V-NN3  | error    | `provenance.kind` ↔ `provenance.ref` scheme alignment: `knowledge` requires `stagecraft://project/<uuid>/knowledge/<uuid>`; `code-fingerprint` requires `xray-fingerprint://<sha256>`. |
| V-NN4  | error    | `provenance.ref` MUST be a syntactically well-formed URI matching its kind's scheme; opaque body MUST be non-empty. |
| V-NN5  | advisory | `provenance:` entries with `role:` unset get an info-level lint recommending `role: derivation` for searchability. |

The codes (V-NN1..V-NN5) get concrete numbers from the next free
slot in the V-NNN registry (`tools/shared/spec-types/src/lib.rs`) at
authoring time. The proposed wording is precise enough to assign and
implement without further design work.

---

## Reconciliations (accepted)

These refinements landed during a cross-session review. Each is a
substantive improvement, not stylistic. The spec author should treat
them as **settled** and reflect them in the spec body — they do not
re-open Q1/Q2/Q3.

### #1 — INTENT doc §3.4 wording divergence

INTENT §3.4 reads as "extend `unit.kind` with `knowledge` and
`code-fingerprint`." That reading would contaminate spec 154's
six-kind grammar with kinds that don't share its refactor-invariance
machinery. The correct shape is a **sibling** field, not an enum
extension.

**Spec 156 §1 MUST** carry a one-sentence orientation pointer along
the lines of: *"The INTENT doc's §3.4 wording suggests extending
`unit.kind`; this spec's grammar instead adds a sibling
`provenance:` field — see §3 for the rationale (the six unit kinds
share refactor-invariance machinery that the new kinds do not)."*
Per CONST-005 the spec wins; the orientation pointer prevents future
readers from being tripped by the INTENT doc's loose wording.

### #2 — `extends:` not `refines:`

INTENT §9 candidate 5 names the relationship as "refines spec 154's
unit grammar." That language is wrong: `refines` is behavior
tightening on existing surface; adding a sibling field is additive
surface extension. The correct relationship is `extends: 154`,
`nature: additive`. (Already in the Q1 closure above; mentioned here
for completeness.)

### #3a — Knowledge URI is project-scoped, not flat

Original proposal: `stagecraft://knowledge/<uuid>` (flat).

Accepted shape: `stagecraft://project/<project-uuid>/knowledge/<knowledge-uuid>`.

Rationale: spec 102 FR-007 — the auditor's verifier does not trust
the producer. A self-locating URI saves the verifier a
project-context lookup it may not have when resolving a provenance
ref against a tenant DB it doesn't already know the project shape of.
The `knowledge_objects.id` UUID alone is globally unique by virtue of
being a UUID, but the verifier's locator path benefits from carrying
the project context inline.

V-NN3's scheme-alignment rule widens slightly to also check the
`/project/<uuid>/` segment shape; minor surface, no design churn.

Confirmed against `platform/services/stagecraft/api/db/schema.ts:603`:
`knowledge_objects.id` is `uuid("id").defaultRandom().primaryKey()`.
Project scoping confirmed via `knowledge_objects.project_id`
(`uuid("project_id").notNull()`).

### #3b — Xray fingerprint URI stays flat; `?path=` deferred

`crates/xray/src/tools.rs:141::xray_fingerprint` takes a `repo_root`
argument — the fingerprint is over a whole tree at a specific commit.
The decomposition pipeline (INTENT §6.2 stage 2) targets the whole
imported tree, so flat `xray-fingerprint://<sha256>` covers every
known use case today.

A future amendment may add a scoped flavor (`?path=<subtree>` or
similar) if a "derived from this subtree's structure" case surfaces.
YAGNI applies; spec 156 ships the flat shape only.

### #4 — OWASP ASI06 + ASI09 coupling deserves a dedicated section

Per INTENT §7.2:

- **ASI06 (Memory & Context Poisoning)** posture: "Strong, with
  forward gap on pre-write / pre-read sanitization hooks." Provenance
  edges are the **structural reverse-lookup substrate** for this gap.
  If a knowledge item is later flagged as poisoned,
  `git grep "stagecraft://project/<uuid>/knowledge/<uuid>"` across
  `specs/` answers *"which specs were derived from this poisoned
  source?"* deterministically — without an LLM, without a narrative
  search.
- **ASI09 (Human-Agent Trust)** posture: gap on "deterministic
  structural-diff plan UI." Typed provenance edges let the
  stagecraft Requirements view (INTENT §3.4) render *"this spec came
  from X"* as a typed, verifiable link rather than an LLM-narrative
  summary. Same anti-anthropomorphic-trust posture spec 102 takes,
  applied one layer up (at spec authoring rather than at run-time
  artifact certification).

Spec 156 MUST carry a `## Compliance` section (or equivalent) naming
both controls explicitly — not as a passing mention, but as
load-bearing design rationale. The structural reverse-lookup property
is one of the core arguments for typed-URI over opaque-blob
provenance refs.

### #5 — Dangling provenance: lax-by-design

If a `provenance:` entry references a knowledge item that's later
deleted (or an xray fingerprint over a tree state no longer
reachable), the indexer / spec-lint MUST NOT fire a diagnostic.

Rationale: provenance is a historical record of derivation *at the
time of derivation*. Deleting the source retroactively does not
invalidate the spec. The certificate hash at derivation time is the
load-bearing artifact (per spec 102), not the live availability of
the source.

Precedent: spec 154 §4 — `references:` is non-owning by design, the
coupling gate ignores it. Provenance edges inherit that non-owning
stance exactly.

Going strict here would cascade knowledge deletions into spec-lint
failures across the corpus — wrong seam.

### #6 — Codebase-indexer integration (this is the operationally critical piece)

Spec 154 Segment 3 (PR #187, commit `482ac312`) shipped the resolver
that walks `references:` entries and builds `resolved_units`. Spec
156's `provenance:` entries need to thread through the same machine:

- **`spec_scanner.rs::push_flat_units`** currently inspects
  `references:` entries for `unit:`. Add a second arm for
  `provenance:`. The `UnitEntry` type widens accordingly (or a new
  parallel `ProvenanceEntry` type is introduced — author's call;
  internal indexer concern).
- **`ResolvedUnit.kind`** widens from the spec 154 six-kind enum to
  include `knowledge` and `code-fingerprint`.
- **The resolver short-circuits** on provenance kinds: emit a
  `ResolvedUnit` with `kind: knowledge` / `kind: code-fingerprint`,
  `ownership: false`, and `locations: []`. **Empty locations by
  design, not by failure** — the invariant matches the non-owning
  semantic from reconciliation #5. No `I-008` / `I-108` diagnostic
  fires on dangling provenance.
- **Schema bump:** the codebase-index schema's `resolvedUnit.kind`
  enum widens to admit the two new kinds. Strictly additive (existing
  consumers see the new kinds and either route them by name or ignore
  them).

Spec 156 MUST carry a `## Indexer integration` section covering this.
Without it, the spec is grammar-without-machinery — the grammar
compiles, but the indexer's `resolved_units` field doesn't carry
provenance entries through to consumers, breaking the stagecraft
Requirements view's "render the provenance link" path
(INTENT §3.4 + §6.2 stage 6).

This is the most operationally important reconciliation. The
grammar-only framing in the initial proposal missed it.

### #7 — Factory-engine relocation is orthogonal

INTENT §8.1 names an in-flight relocation of the factory / adapter
machinery into stagecraft. Spec 156 is grammar-only — it does not
depend on factory location. The emission contract (decomposition
pipeline stage 6, INTENT §6.2 producing draft specs that carry these
edges) is **INTENT candidate 3**, a separate spec. The relocation
affects candidate 3 but not spec 156. Confirmed clean separation; no
bleed.

---

## Authoring checklist (8 green-light items + 4 reconciliation absorptions)

The original 8 items (now confirmed):

- [x] `extends: ["154"]` + `references: ["155"]` posture (Q1)
- [x] Slug `156-references-edge-provenance-grammar` (Q2)
- [x] `provenance:` as the new field name, mutually exclusive with `unit:` (Q3)
- [x] Two initial kinds: `knowledge`, `code-fingerprint` (Q3)
- [x] URI-shape `ref:` with kind ↔ scheme alignment enforced (Q3)
- [x] Five V-rules (one advisory, four error) (Q3)
- [x] `role:` stays open-vocabulary; `derivation` recommended for provenance entries (Q3)
- [x] References-edge non-owning semantic preserved — coupling gate unaffected (Q3 + #5)

Reconciliation absorptions to land in the spec body:

- [ ] Spec 156 §1 carries the one-sentence INTENT §3.4 orientation pointer (#1)
- [ ] Knowledge URI is project-scoped: `stagecraft://project/<project-uuid>/knowledge/<knowledge-uuid>` (#3a)
- [ ] Spec 156 has a dedicated `## Compliance` section covering ASI06 reverse-lookup substrate + ASI09 typed-link rendering (#4)
- [ ] Spec 156 has a dedicated `## Indexer integration` section covering the `spec_scanner` second arm, `ResolvedUnit.kind` widening, lax-on-danglers, schema bump (#6)

---

## Stop conditions

Surface rather than improvise if any of the following:

1. **A URI shape unforeseen here surfaces during authoring.** E.g. a
   knowledge item that lives in a non-stagecraft system (DAM, external
   wiki), or an xray fingerprint that needs to encode multi-commit
   provenance. The two-kind initial set is deliberately small; new
   kinds land as future amendment specs, not by widening this spec
   mid-authoring.
2. **The V-rule numbering conflicts with concurrent work.** Check the
   next free slot in `tools/shared/spec-types/src/lib.rs` against
   the in-flight spec corpus at authoring time. The V-NN1..V-NN5
   placeholders get concrete numbers at that point.
3. **The codebase-indexer schema bump introduces a non-additive
   change.** Reconciliation #6 is described as additive; if the
   `resolved_units` field shape needs to change rather than just
   admit two new kind enum values, halt and surface — that's a
   spec-level concern, not an implementation detail.
4. **Spec 102 FR-007 verifier integration surfaces a constraint not
   covered by reconciliation #3a.** The project-scoped URI shape was
   designed for the verifier's locator path; if a real verifier
   implementation needs a different shape, that's a halt-and-discuss
   point.

---

## Spec 157 — independent, no carryover from this analysis

INTENT §9 candidate 14 names spec 157: **OPC multi-session-by-project-path
session model (formalised)**. The reconciliation analysis confirmed it
is independent of spec 156 — no shared reconciliation items, no
machinery overlap. Spec 157 refines spec 052 (state-persistence); its
authoring inputs are INTENT §4.2 + the existing OPC session-binding
code at `product/apps/desktop/`. No design-handoff doc needed for spec
157 from this work; it picks up directly from the INTENT doc.

---

## After spec 156 lands

- This doc gets deleted (or archived) in the spec 156 closure PR.
- The auto-memory entry `project_spec_156_design_alignment` gets
  deleted at the same time (the spec body absorbs every load-bearing
  fact).
- INTENT doc §3.4 should be lightly amended (one sentence)
  cross-referencing spec 156 to close the divergence flagged in
  reconciliation #1.
