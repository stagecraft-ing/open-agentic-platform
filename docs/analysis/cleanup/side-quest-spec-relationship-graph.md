# Side Quest — Spec Relationship Graph & Coupling Architecture

## What this is

A single-commit, branch-local architectural correction. The OAP spec
format conflates eight distinct relationships between specs and code
into one `implements:` field. Four specs — 127, 130, 133, and the spec
owning the bypass mechanism — sit in this gap as heuristic compensation.
This commit replaces the heuristics with the data: explicit
relationship fields in spec frontmatter, a coupling gate that derives
authority from the graph, named-anchor sectioning for co-governed
resources, and four spec slots rewritten in place to host the new
governance model as if it were always the model.

No retirements. Every spec involved is rewritten — new title, new
body, new frontmatter — under the same id. The corpus, post-commit,
reads as if the new paradigm was native from the start. There is no
"v2" framing, no retirement rationale, no archaeological footnotes.

The commit lands on `cut-d/autonomous-run-20260519-025506`, ahead of
Epic 2. Epic 2 then benefits from the new gate throughout its 13
phases.

## What this is NOT

- Not a feature pass. No consumer gains new behavior beyond what the
  new gate logic requires.
- Not a corpus completionist pass. The commit annotates the specs
  whose relationships are *active load-bearing dependencies* (the
  time-series clusters, the known cross-cutters, the Makefile co-
  authority case). Specs outside the load-bearing core receive
  minimum `origin: retroactive: true` annotation to keep spec-lint
  clean. Full curated annotation of all 152 specs is a follow-up.
- Not negotiable on the single-commit landing. WIP commits are
  allowed during the session; the final landing is one squashed
  commit on the branch.
- Not a chance to also fix V-010 dormancy, the standard.schema.json
  duplicate, the render-path question, or any other item Epic 1
  surfaced. Those wait for their phases.
- Not a `refactor(cleanup):` commit shape. This is `feat(spec-
  governance):` — the architectural addition is the point.

## Pre-conditions

- Branch: `cut-d/autonomous-run-20260519-025506`.
- `git status` clean.
- Epic 1 commits (the 10–11 `docs(cleanup):` commits) present at
  HEAD.
- Epic 2 has not yet started. (If Epic 2 commits are present, halt —
  this surgery must precede Epic 2.)
- `cargo test --workspace` clean.
- `make pr-prep` red on the baseline. (This is the failure mode being
  fixed. If somehow green, halt — the conditions have changed and
  this commit may not be needed in its current form.)

## The four spec slots

The surgery rewrites four specs in place. New titles and new bodies
are authoritative; the ids carry forward, nothing else. The previous
content is irrelevant — do not reference it, do not link to it, do
not summarize it in the new spec body. The corpus moves forward.

### Spec 127 → `spec-code-coupling-workflow`

**Concern.** The CI job, make target, failure semantics, and
contributor-facing affordances of the coupling check. The *workflow
contract*: what `make pr-prep` does, what the gate's exit codes mean,
how PRs interact with the gate, what the operator sees when the gate
fails or passes.

**Out of scope for 127.** The derivation logic (lives in 133), the
relationship field semantics (lives in 130), the sectioning mechanism
(lives in the fourth slot).

**Key sections.**
- The make target and its phases.
- The CI workflow that invokes it.
- Exit code contract and human-readable output format.
- Contributor flow: how a failure is communicated, what action it
  prescribes, where the documentation lives.
- Relationship to spec 103 (governed-artifact-reads): the gate uses
  consumer binaries, never raw artifact reads.

**Frontmatter.** Uses `extends: 130` (the workflow extends the
relationship-graph model) and `extends: 133` (the workflow invokes
the gate). May use `co_authority:` on `Makefile#spec-code-coupling`
and the workflow file.

### Spec 130 → `spec-relationship-graph`

**Concern.** The constitutional spec for the eight relationship
fields. Defines the format, the semantics, the well-formedness rules,
and the derived `implements:` view. This is the foundational spec the
constitution references.

**Out of scope for 130.** Gate enforcement (lives in 133), workflow
mechanics (127), sectioning (fourth slot).

**Key sections.**
- The eight relationship fields with full semantic definitions:
  `establishes`, `extends`, `refines`, `supersedes`, `amends`,
  `co_authority`, `constrains`, `origin`.
- Well-formedness rules: which fields can co-exist on one spec, which
  are mutually exclusive, what shapes are valid.
- The derived `implements:` view: how it's computed, why it's
  preserved for compatibility, what consumers of `implements:` see.
- Constraint semantics: how `constrains:` relationships work, what
  kinds of constraints are expressible, how they're distinct from
  behavior-authority.
- The constitutional declaration: relationships are first-class; the
  graph is the canonical representation of spec-to-code and spec-to-
  spec governance.

**Frontmatter.** Heavy use of `establishes:` on its own concept
(metacircular — the spec that establishes the relationship graph is
the foundational instance of using the relationship graph). Uses
`constrains: registry.schema.json` (it constrains what the registry
schema must accept).

### Spec 133 → `coupling-gate`

**Concern.** The derivation algorithm, satisfaction semantics, and
output format of the coupling gate. The *gate logic*: given a diff
and the spec corpus, decide what the diff's coupling status is and
report it precisely.

**Out of scope for 133.** Workflow integration (127), field semantics
(130), section matching (fourth slot — though the gate consumes
section-level data from there).

**Key sections.**
- Authority derivation: `authorities(P)` as a function over the
  relationship graph, with supersession resolution (full and partial),
  amendment handling, refinement and extension treatment.
- Satisfaction condition: for each edited path, the rule that
  determines whether coupling is satisfied. Includes the empty-
  authority branch (when a path has no current authority, the gate's
  rule for accepting or rejecting based on path classification).
- Constraint evaluation: distinct from satisfaction. The gate
  separately checks that no `constrains:` spec is violated by the
  diff. Constraint failure is its own failure mode with its own exit
  code path.
- Output format: when the gate fails, it reports current authority
  per path, not historical claimants. Output is precise, small,
  actionable.
- Library API: the derivation function is exposed for use by
  registry-consumer and any other consumer that needs authority
  queries.

**Frontmatter.** `extends: 130` (the gate operates over the
relationship graph). Uses `co_authority:` on the gate's source files.

### Spec <BYPASS-SLOT> → `path-co-authority`

**Concern.** Named-anchor sectioning, diff-to-section matching, and
multi-section file governance. The mechanism by which a single file
can be governed by multiple specs with non-overlapping authority.

**Out of scope for fourth slot.** Anything about the graph itself
(130), the gate (133), or the workflow (127).

**Key sections.**
- Named anchors: the syntax `<path>#<anchor>`, what an anchor names
  (a section, a target, a heading, a region), per file type
  conventions (Makefile target groupings, markdown heading anchors,
  workflow `jobs.<name>`, source-file `// region: <name>` directives).
- Section boundaries: how the gate determines which section a diff
  hunk belongs to, ambiguity handling (when a hunk crosses sections
  or sits outside any named section).
- Co-authority resolution: when a path has multiple `co_authority:`
  claims, the gate matches diff sections to claimed sections and
  requires the matching spec(s) to be edited.
- The empty-authority-by-rule branch: paths and sections that
  legitimately have no governing spec (vendored code, generated
  artifacts, well-known boilerplate). How such paths are declared and
  recognized by the gate.

**Frontmatter.** `extends: 130` and `extends: 133`. `co_authority:`
heavy use as the canonical exemplar — the spec that establishes co-
authority is itself co-author on the gate source files and the
Makefile.

## Identifying the bypass slot

At session start, identify the spec that owns
`.github/spec-coupling-bypass.txt`. Search:

1. `git grep -nE "spec-coupling-bypass\\.txt|bypass.*coupling|coupling.*bypass" -- specs/`
2. The `implements:` rows of specs in the 120s neighborhood
   (coupling-gate cluster).
3. `.github/spec-coupling-bypass.txt` itself for inline references to
   an owning spec id.

The bypass owner is expected to sit in the 120s near 127, but may
be elsewhere. If the bypass mechanism turns out to have no single
owning spec (it's an undocumented convention), halt and surface — the
operator decides whether to create the fourth slot from scratch (at a
new id) or to absorb its concerns into one of 127/130/133.

## The relationship graph — canonical definition

The eight fields, formally. The 130 spec body elaborates with
examples and edge cases; this section is the normative contract that
all four new specs and the gate implementation must match.

```yaml
# 1. Establishment — one-time, this spec caused this code to exist.
establishes:
  - <path>

# 2. Extension — this spec adds to another spec's authority surface.
extends:
  - spec: <id>
    paths: [<path>, ...]      # subset of the extended spec's surface
    nature: additive | wrapping

# 3. Refinement — this spec tightens behavior across paths.
refines:
  - paths: [<path>, ...]
    aspect: <short-tag>
    refines_specs: [<id>, ...]   # optional

# 4. Supersession — this spec replaces another spec's authority.
supersedes:
  - spec: <id>
    scope: full | partial
    paths: [<path>, ...]      # required for partial; omitted for full
    rationale: <one-line>

# 5. Amendment — this spec patches another spec without replacement.
amends:
  - spec: <id>
    change_type: clarification | correction | restriction
    paths: [<path>, ...]

# 6. Co-authority — this spec governs a named section of a shared path.
co_authority:
  - paths: [<path>]
    section: <named-anchor>
    with_specs: [<id>, ...]   # other specs co-governing other sections

# 7. Constraint — meta-authority over how others may shape code.
constrains:
  - spec: <id>                # optional; constraints can be path-only
    kind: invariant-freeze | <other>
    paths: [<path>, ...]

# 8. Origin — bootstrap or retroactive marker.
origin:
  retroactive: true
  paths: [<path>, ...]
```

`implements:` is preserved as a *derived* view, computed from the
union of paths appearing in `establishes`, `extends.paths`,
`refines.paths`, and `co_authority.paths`. Authors no longer write
`implements:` directly. The spec-compiler computes it and emits it
into the registry for backward compatibility with consumers.

## Authority derivation — canonical definition

For each path P:

```
authorities(P) = {
  spec :
    (spec establishes P)
    OR (spec extends Y for which P in extends.paths)
    OR (spec refines P)
    OR (spec co_authority P)
    OR (spec amends Y for which P in amends.paths)
  AND NOT exists later_spec :
    (later_spec supersedes spec with scope=full)
    OR (later_spec supersedes spec with scope=partial AND P in
        later_spec.supersedes.paths)
}
```

`constrains:` specs are not authorities over P's behavior; they are
checked separately as constraint-satisfaction by the gate.

For co-authority paths, authority is section-scoped:

```
authorities(P, S) where S is a section of P = {
  spec :
    spec co_authority P with section=S
  AND NOT (superseded as above)
}
```

A diff hunk H is matched to section S if H's lines fall within S's
anchor boundaries (per per-file-type rules defined in the fourth
slot's spec).

## Coupling gate v2 — canonical semantics

```
for each edited path P in diff:
  hunks = diff hunks touching P
  for each hunk H in hunks:
    if P has co_authority claims:
      S = section containing H
      authority_set = authorities(P, S)
    else:
      authority_set = authorities(P)

    if authority_set is empty:
      if P (or P#S) matches an empty-authority-by-rule pattern:
        satisfied
      else:
        fail (path lacks current authority — spec required)
    else:
      if any spec in authority_set is edited in diff
         OR any spec in authority_set has an amendment-record edit in
            diff:
        satisfied
      else:
        fail (current authority not touched; report authority_set)

for each constrains-spec C in corpus:
  if diff violates any invariant declared by C:
    fail (constraint violation; report C and the violated invariant)
```

The "claimed by N specs" output of the current gate is replaced by:

- For satisfied cases: silent (one-line summary at exit).
- For failure cases: per-path, the current authority set (typically
  1–3 specs), the section if applicable, the rule that fired, and a
  one-line prescribed action.

## Schema, parser, and tool changes

All in scope for this commit:

- **`specs/000-bootstrap-spec-system/contracts/registry.schema.json`**
  (or post-I4 location; this commit precedes I4 so the path is
  pre-cleanup): add the eight relationship fields. Mark `implements:`
  as `readOnly` since it's derived.
- **`tools/spec-compiler/src/lib.rs`**: extend the frontmatter parser
  for the new fields. Add the derivation pass that populates
  `implements:` in the registry from the relationship graph.
  Preserve all existing V-codes; their semantics over the derived
  `implements:` are unchanged.
- **`tools/shared/spec-types/src/lib.rs`**: add Rust types for the
  eight fields. Add a new V-code (next free; expected V-020 unless
  V-010's dormancy is resolved differently) for "spec lacks
  relationship fields."
- **`tools/spec-lint/src/lib.rs`**: emit the new V-code when a spec
  has no relationship fields and no `origin: retroactive: true`.
- **`tools/spec-code-coupling-check/src/lib.rs` and `main.rs`**:
  rewrite per §Coupling gate v2 — canonical semantics. Expose
  `authorities(P)` and `authorities(P, S)` as library functions.
  Output format changes to match §canonical semantics.
- **`tools/registry-consumer/src/lib.rs`**: query surface gains
  `--by-authority <path>` and `--show-relationships <spec-id>`.
  Existing CLI contracts are preserved (no breaking changes to
  existing flags or output shapes).
- **`Makefile` `pr-prep` and related targets**: no change to target
  names; the underlying coupling-check binary's behavior change flows
  through.

## Constitution

`standards/spec/constitution.md` (post-I3 location) or
`.specify/memory/constitution.md` (pre-I3 — this commit lands before
I3, so the latter is the actual location; the I3 `git mv` carries
it forward) gains a new section: **§Spec Relationship Graph**.

The section is ≤ 200 lines. It normatively introduces:

- The thesis: spec governance operates over a relationship graph, not
  a flat claim list.
- The eight relationships, summarized (130 has the full definitions).
- Authority as a derived property of the graph.
- Co-authority and sectioning as the mechanism for shared resources.
- Constraints as meta-authority distinct from behavior-authority.
- The well-formedness expectation: every spec declares its
  relationships explicitly; the corpus is self-describing.

This section is referenced by all four rewritten specs.

## Corpus annotation scope

Annotate, in this commit, the load-bearing clusters whose absence of
annotation currently causes coupling-gate failures or whose
relationships are otherwise structurally required. The clusters,
with the relationships the agent should expect to use:

| Cluster | Specs | Expected relationships |
|---|---|---|
| registry-consumer time-series | 002, 003, 007–031 | 002 `establishes`; 003–031 mostly `extends` of 002 or nearest predecessor; some `refines` (e.g., 020 error-shape across siblings); 029 `supersedes` partial of contract-related earlier specs |
| spec-compiler | 001, 003, 039, 091, 102, 132, 151 | 001 `establishes`; 003, 039, 091 `extends`/`refines`; 102 `refines` aspect=governance; 132 `constrains` (invariant-freeze); 151 `extends` |
| codebase-indexer | 101, 118, 129, 133 | 101 `establishes`; 118 `extends` (workflow-traceability angle); 129 `extends` (granular metadata); 133 (this slot is being rewritten — its new identity as `coupling-gate` may `extends` codebase-indexer or may not depending on the new body's scope) |
| factory-engine | 075, 094, 102, 110 | 075 `establishes`; 094 `extends`; 102 `refines` aspect=governance; 110 `extends` |
| featuregraph | 034, 039, 091, 093, 096 | 034 `establishes`; 039 `amends` (id-reconciliation correction); 091 `extends`; 093, 096 `extends` |
| src-tauri (desktop) | 032, 041, 064, 065, 076, 083, 084, 119 | 032 `establishes`; rest `extends`/`refines`/`co_authority` per actual concern |
| Makefile co-authority | 102, 104, 105, 116, 127, 128, 134, 135 | Each `co_authority` on `Makefile#<section>` per their actual targets |
| AGENTS.md / CLAUDE.md / governed-artifact-reads.md | 103, 131 | 103 `establishes` on AGENTS.md and governed-artifact-reads.md; 131 `co_authority` on CLAUDE.md with whatever else |
| CI workflows | 116, 117, 118, 127 | `co_authority` on the workflow files per the actual sectioning |
| Spec-spine tooling | 001, 002, 006 (plus the four rewrite slots) | Per their concerns |

Specs outside the load-bearing core: minimum annotation —
`origin: retroactive: true` with their existing `implements:` paths
carried forward as the spec's surface. This keeps spec-lint clean
without forcing full curation now.

Expected scope: 50–80 specs touched out of 152.

## Operator decisions (resolve before firing)

| # | Decision | Resolution |
|---|---|------------|
| 1 | Four-slot rewrite confirmed: 127 → `spec-code-coupling-workflow`, 130 → `spec-relationship-graph`, 133 → `coupling-gate`, <bypass-slot> → `path-co-authority` | `correct`  |
| 2 | New V-code number for "spec lacks relationship fields" — next free (V-020 expected; V-010 dormancy unchanged) | `correct`  |
| 3 | Empty-authority-by-rule patterns — initial set defined in this commit. Recommended: vendored grammars, generated artifacts under `build/` (pre-I9) or `.derived/` (post-I9), `Cargo.lock` files when the corresponding `Cargo.toml` has authority, `package-lock.json`/`pnpm-lock.yaml` when the corresponding `package.json` has authority. | `correct`  |
| 4 | Constraint kinds enumerated in 130 — initial set. Recommended: `invariant-freeze` (the V-132 use case), with the `kind` field as `string` (open vocabulary) so future kinds don't require schema change. | `correct`  |
| 5 | Mid-session halt for cluster review: allow exactly one halt after the registry-consumer cluster draft, before applying the pattern to other clusters | `correct`  |
| 6 | Co-authority section naming convention: `#kebab-case-anchor` matching existing structure (Makefile target groupings, workflow `jobs.<name>`, markdown headings) | `correct`  |
| 7 | Spec-Drift-Waiver mechanism disposition: retire as a concept (the new "empty-authority-by-rule" branch + `co_authority:` removes the legitimate use cases) | `correct`  |
| 8 | Bypass file disposition: `.github/spec-coupling-bypass.txt` deleted; the empty-authority-by-rule patterns are codified in the fourth-slot spec body, not in a separate file | `correct`  |
| 9 | Section-matching algorithm for source files (Rust, TypeScript): use `// region: <name>` / `// endregion` markers, or fall back to "no section match"? Recommended: yes, support `// region:` / `// endregion` as opt-in markers; absence means whole-file authority | `correct`  |
| 10 | Whether `extends:` and `refines:` can both appear on the same spec for the same path: recommended yes (a spec can extend a predecessor *and* refine a cross-cutting concern); 130 must document this | `correct`  |

## Execution model

One session, autonomous, with one mid-session halt allowed. Agent
works in WIP commits during the session; final landing is `git reset
--soft <pre-surgery-HEAD>` + one commit.

**Session phases (no inter-phase commits to the branch):**

1. **Read.** Constitution, contract.md, specs 127, 130, 133, registry
   schema, spec-compiler parser, spec-code-coupling-check, current
   spec-lint V-codes, `.github/spec-coupling-bypass.txt`. Identify the
   bypass-slot spec id.
2. **Design fixation.** Confirm the field set + semantics against
   §The relationship graph and §Authority derivation. If anything is
   ambiguous from the prompt, halt.
3. **Schema + types + parser.** Implement the new fields end-to-end.
   Derivation pass for `implements:`. `cargo test --workspace` green.
4. **Gate v2.** Reimplement `spec-code-coupling-check` per §Coupling
   gate v2. Library functions exposed.
5. **Constitution.** Author the §Spec Relationship Graph section in
   the constitution.
6. **The four rewrites.** Author new bodies for 127, 130, 133, and
   the bypass slot. Each spec uses the new format in its own
   frontmatter as a canonical exemplar.
7. **Registry-consumer cluster annotation.** Draft and apply.
   **HALT** for operator review.
8. **Resume: remaining clusters.** Apply the reviewed pattern across
   the other clusters (spec-compiler, codebase-indexer,
   factory-engine, featuregraph, src-tauri, Makefile, workflows,
   tooling).
9. **Minimum annotation for non-core specs.** `origin: retroactive:
   true` pass for the remaining ~70–100 specs.
10. **Bypass file removal.** `.github/spec-coupling-bypass.txt`
    deleted. Its content is now codified in the fourth-slot spec.
11. **Verification.** `cargo test --workspace`, `make pr-prep` under
    gate v2 (the commit must pass its own gate), `/init`, full
    spec-lint pass with new V-code.
12. **Squash.** `git reset --soft <pre-surgery-HEAD>`; one commit per
    §Commit shape.

## Verification gate

The single landing commit must satisfy simultaneously:

- `cargo build --workspace --release` clean.
- `cargo test --workspace` clean.
- `make pr-prep` clean **under gate v2** — the commit passes its own
  gate. Recursive verification.
- `/init` clean end-to-end.
- `spec-lint` clean, including the new V-code: every spec has either
  relationship fields or `origin: retroactive: true`.
- The registry produced by the new spec-compiler validates against
  the updated registry schema.
- Manual operator review (at the squash step) of: the constitutional
  §Spec Relationship Graph section, the four rewritten spec bodies
  (127, 130, 133, fourth slot), the commit message.

## Trip-wires

- **The bypass mechanism has no single owning spec.** Halt; operator
  decides whether to create the fourth slot at a new id or to absorb
  its concerns into 127/130/133.
- **Authority derivation produces a cycle** (spec A's extends points
  at spec B which supersedes A): halt; the data is contradictory.
- **A cluster requires a ninth relationship kind** not in the eight
  defined: halt, surface, and the operator approves a ninth field on
  the spot if it's clearly needed.
- **A spec currently in `extends`-chain has `status: retired`**: halt;
  the operator decides how supersession of retired specs is recorded.
- **`make pr-prep` under gate v2 fails on the surgery commit
  itself**: halt. The commit cannot land if it doesn't pass its own
  gate. Re-examine the relationship annotations on the touched specs.
- **A consumer outside the four-tool set reads the old spec bodies
  textually** (e.g., a doc generator embeds 130's previous body): halt;
  surface for in-commit update or defer.
- **The single-commit squash produces a diff over 30,000 lines**:
  halt for operator scale-check before pushing. Not a failure, but a
  pause.
- **Mid-session: registry-consumer cluster draft surfaces a design
  problem** with the field set: halt; redesign before applying to
  other clusters.

## Commit shape

Single squashed commit, conventional-commits style:

```
feat(spec-governance): establish spec relationship graph and coupling
architecture

OAP spec governance now operates over an explicit relationship graph.
Eight frontmatter fields — establishes, extends, refines, supersedes,
amends, co_authority, constrains, origin — encode how specs relate to
code and to each other. Authority over each code path is derived from
the graph; the coupling gate consumes the derivation; co-authority and
named-anchor sectioning support shared resources.

Four specs are rewritten in place to host this architecture:

- 127 spec-code-coupling-workflow — the CI job, make target, failure
  semantics, and contributor flow of the coupling check.
- 130 spec-relationship-graph — the foundational spec defining the
  eight relationship fields, their semantics, well-formedness, and
  the derived implements: view. Referenced by the constitution.
- 133 coupling-gate — the derivation algorithm, satisfaction
  semantics, constraint evaluation, and output format of the gate.
- <NNN> path-co-authority — named-anchor sectioning, diff-to-section
  matching, multi-section file governance, and the empty-authority-
  by-rule mechanism.

Tooling: spec-compiler parses the new fields and emits a derived
implements: view to the registry. spec-types adds Rust types and a
new V-code (V-NNN) for specs lacking relationship fields. spec-lint
enforces it. spec-code-coupling-check is rewritten against the new
derivation; output reports current authority sets (typically 1–3
specs), not historical claimants. registry-consumer gains
--by-authority and --show-relationships queries.

Constitution: new §Spec Relationship Graph section normatively
declares the model.

Corpus: <count> specs annotated with relationship fields across the
load-bearing clusters (registry-consumer, spec-compiler,
codebase-indexer, factory-engine, featuregraph, src-tauri, Makefile,
CI workflows, spec-spine tooling). Remaining specs receive
origin: retroactive: true with their existing implements: paths
preserved as their surface. Full curated annotation of the remaining
corpus is a follow-up.

Bypass mechanism: .github/spec-coupling-bypass.txt is removed; its
patterns are codified in <NNN>'s empty-authority-by-rule section.

This commit passes its own gate: under the new derivation, every
edited path's current authority spec is also edited in this commit.

Refs: docs/analysis/cleanup/cleanup-master-plan.md
```

## Hard rules

- **One landing commit.** WIP commits in-session, squash before push.
- **The commit passes its own gate.** Recursive verification is the
  single hardest constraint and the proof of design correctness.
- **No retirements.** All four slots are rewrites in place. New
  titles, new bodies, new frontmatter. No `status: retired` on these
  ids. No retirement_rationale. No references to the previous content
  anywhere in the new bodies or the commit message.
- **No other concerns.** V-010 dormancy, schema duplicates, render-
  path resolution, anything else from Epic 1's open questions — all
  defer.
- **Single mid-session halt allowed**, after the registry-consumer
  cluster draft (decision #5). Further halts indicate a real blocker
  for operator input, not design checkpoints.
- **Existing `implements:` lists are preserved as the derivation
  source** for retroactively-annotated specs (`origin: retroactive:
  true` carries the existing paths forward as the spec's surface).
- **No new consumer behavior** beyond what the gate's new derivation
  and the registry-consumer's new query verbs require. Factory,
  axiomregent, registry, etc. are untouched.
- **Each of the four rewritten specs uses the new format in its own
  frontmatter.** They are canonical exemplars; if the format isn't
  expressive enough to describe these four specs themselves, the
  format is wrong.

## What success looks like

After the commit lands:

- `make pr-prep` is green on `cut-d/autonomous-run-20260519-025506`.
- The corpus reads as if the relationship graph was always the model.
  Specs 127, 130, 133, and the fourth slot are first-class
  inhabitants of the new paradigm, not patched survivors of the old
  one.
- The constitution declares relationship-graph governance.
- Coupling-gate output, when it fails on future branches, names
  current authority sets, not historical claimants. Output is small,
  precise, actionable.
- Co-governed resources (Makefile, workflows, shared source files)
  have section-scoped authority. Editing the supply-chain section of
  the Makefile requires touching the supply-chain spec, not all eight
  Makefile-touching specs.
- Constraints are first-class. Invariant-freeze and similar meta-
  governance concerns have a home that isn't conflated with behavior
  authority.
- Epic 2's 13 phases proceed against a gate that won't drown them in
  multi-claim noise.
- Three classes of workaround mechanism — primary-owner heuristics,
  amendment-as-bypass, file-level coupling bypass — no longer exist
  as separate concepts. They're absorbed into a single coherent
  graph.

Begin by reading the canonical inputs and identifying the bypass-slot
spec. Confirm pre-conditions. Then proceed through the session
phases. Halt once, mid-session, for cluster-draft review.

The corpus deserves coherent governance. This is that commit.
