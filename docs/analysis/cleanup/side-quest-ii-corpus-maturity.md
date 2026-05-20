# Side Quest II — Spec Relationship Graph Corpus Maturity

## What this is

A single-commit, branch-local completion pass. The first side quest
installed the spec relationship graph as the model. This commit
*populates* the model across the full corpus and *activates* its full
semantics: every spec declares its relationships honestly,
section-scoped authority is live in the gate, the graph is queryable
through registry-consumer, and the legacy `implements:` field is
excised once the audit confirms no consumer depends on it.

The corpus, post-commit, is the spec spine at representational
maturity. There is one canonical way to express how specs relate to
code and to each other; there is no parallel legacy field carrying
ambiguous duplicate authority; the gate enforces section-scoped
governance; humans and tools can query the graph directly.

The commit lands on `cut-d/autonomous-run-20260519-025506`, ahead of
Epic 2.

## What this is NOT

- Not a continuation of the first surgery's framing. The first surgery
  installed the model. This commit makes the corpus *be* the model
  rather than approximating it.
- Not negotiable on the single-commit landing. WIP commits in-session;
  final landing is one squashed commit.
- Not a feature pass beyond the explicit query verbs. No new consumer
  behavior outside registry-consumer's new query surface and the
  gate's section-matching activation.
- Not an Epic 2 phase. Epic 2's structural cleanup follows this. This
  commit is the last governance-model work before structural work
  resumes.
- Not a chance to revisit the eight relationship fields. The field set
  is locked. If a cluster genuinely requires a ninth relationship,
  trip-wire fires and the operator decides — but this is the corpus
  using the model, not redesigning the model.

## Pre-conditions

- Branch: `cut-d/autonomous-run-20260519-025506`.
- `git status` clean.
- The first surgery commit (`8fc400d1` —
  `feat(spec-governance): establish spec relationship graph and
  coupling architecture`) is present at HEAD or in the recent history.
- Epic 2 has not yet started. (If Epic 2 commits are present, halt —
  this completion pass must precede Epic 2.)
- `cargo test --workspace` clean.
- `make pr-prep` green on the baseline. (The first surgery made this
  green; if it's red, halt and surface — something regressed.)

## Scope — six concerns

This commit addresses six concerns. They are tightly coupled: the
audit informs the excision; the excision informs the annotation
shape; the annotation drives section-matching activation; section
activation surfaces second-wave annotation corrections; the query
surface is built against the now-active semantics; full-corpus
verification confirms the whole stands together.

### Concern 1 — `implements:` consumer audit and excision decision

Enumerate every consumer of `implements:` across the entire repository.
For each, classify:

- **Internal derivation/emission** (the spec-compiler emits it; tools
  derive it from the graph): removable.
- **Internal read for governance** (the gate, the lint, the indexer
  read it for their internal logic): removable if the same logic can
  read relationship fields directly.
- **External read by non-governance consumer** (featuregraph,
  factory-engine, desktop app, platform services, anything in
  `crates/` outside the governance toolchain): the load-bearing case.
  If any external consumer reads `implements:` from the registry, it
  is the justifying consumer.

Search surface:

```
git grep -nE "implements" -- '*.rs' '*.ts' '*.tsx' '*.js' '*.json' \
  '*.yaml' '*.yml' '*.md'
```

Filter against the relationship-graph fields (which also contain the
substring `implements` in spec bodies). The audit is specifically for
*field reads* — Rust struct fields named `implements`, JSON property
accesses `.implements`, frontmatter field reads, schema property
declarations, golden-file expected values.

**Halt at end of audit. Surface the consumer list to the operator.
The operator confirms:**

- (a) **No external consumers — total excision.** Remove `implements:`
  from the registry schema, spec-compiler emission, codebase-indexer
  derivation, lint validators, golden files, docs, and (if sourced in
  frontmatter) the 152 spec.md files. One coherent removal across the
  commit.
- (b) **External consumer X depends — migrate X in this commit.**
  Update X to read relationship fields directly. Then total excision
  per (a).
- (c) **Keep as derived view, document the justifying consumer in
  spec 130.** Last resort; only if migration of X is genuinely out of
  scope for this commit.

Default after audit: assume (a) unless the audit surfaces an external
consumer.

### Concern 2 — Full corpus annotation

All 152 specs declare their relationships explicitly. No spec carries
`origin: retroactive: true` *as a substitute for honest annotation* —
`origin: retroactive: true` is reserved for specs whose relationship
to code genuinely is retroactive bootstrap (spec 000 and similar
foundational instances), not for "we didn't get to it."

Annotation proceeds by cluster. Use the patterns the first surgery
established for the load-bearing clusters (registry-consumer,
spec-compiler, codebase-indexer, factory-engine, featuregraph,
src-tauri, Makefile, CI workflows) as calibration. Apply the same
discipline across the remaining ~100 specs.

**Cluster pattern guidance:**

- **MVP/establishment specs** — the first spec to bring a binary,
  module, or subsystem into existence: `establishes:` with the
  initial path set.
- **Time-series increments** — specs that add to a predecessor's
  surface without changing its behavior: `extends: <predecessor>`
  with `nature: additive`.
- **Wrapping increments** — specs that add a new layer or contract
  over a predecessor: `extends: <predecessor>` with `nature: wrapping`.
- **Cross-cutting aspect specs** — specs that tighten a single concern
  across many paths: `refines:` with `aspect: <tag>` and an optional
  `refines_specs:` list.
- **Replacement specs** — specs that fully or partially supersede a
  predecessor: `supersedes:` with `scope: full` or `scope: partial +
  paths:`.
- **Correction specs** — specs that patch a predecessor without
  replacement: `amends:` with `change_type:`.
- **Shared-resource specs** — specs that govern a section of a file
  also governed by others: `co_authority:` with `section:` and
  `with_specs:`.
- **Meta-governance specs** — specs that constrain how others may
  shape code (invariant freezes, policy specs): `constrains:` with
  `kind:`.
- **Genuine retroactive specs** — foundational bootstrap, specs
  documenting code that pre-existed the spec system:
  `origin: retroactive: true`. This is a small subset; most specs are
  not retroactive.

**Co-authority sections.** Define named anchors during this pass for
every shared resource encountered. Use the conventions established by
spec 152 (the path-co-authority spec from the first surgery):

- **Makefile** — anchor matches the make target group, e.g.
  `Makefile#spec-code-coupling`, `Makefile#supply-chain`,
  `Makefile#ci-fast`.
- **GitHub workflows** — anchor matches `jobs.<name>`, e.g.
  `.github/workflows/spec-conformance.yml#jobs.spec-conformance`.
- **Markdown rule files** — anchor matches the heading, e.g.
  `.claude/rules/governed-artifact-reads.md#governed-reads-policy`.
- **Rust/TypeScript source** — anchor matches `// region: <name>` /
  `// endregion` markers. If a co-authored source file doesn't yet
  have region markers, add them in this commit per the section
  scheme spec 152 documents.
- **Cargo.toml / package.json** — anchor matches the top-level table
  or section, e.g. `crates/factory-engine/Cargo.toml#dependencies`.

**Halt points during annotation.** Two mid-session halts allowed for
operator review:

- **Halt A** — after annotating the spec-compiler cluster (specs 001,
  003, 039, 091, 102, 132, 151). This cluster is the smallest
  structurally ambiguous one; reviewing it calibrates the pattern for
  the rest.
- **Halt B** — after annotating the codebase-indexer cluster
  (specs 101, 118, 129, plus the rewritten 133 in its new identity).
  This cluster's interaction with the rewritten 133 is the area where
  the relationship semantics may surface tension.

All other clusters proceed without halt against the calibrated
patterns. If a cluster surfaces structural ambiguity not covered by
the patterns, the agent halts and surfaces — but this is a real halt
for design input, not a checkpoint.

**Expected scope: 100–115 specs newly annotated** (in addition to the
~50 the first surgery touched), plus ~15 specs migrated from minimum
`origin: retroactive: true` annotation to honest annotation where the
genuine relationship was non-retroactive but the first surgery
deferred curation.

### Concern 3 — V-020 emission in spec-lint

The V-020 constant exists in `tools/shared/spec-types/src/lib.rs`.
Emission site does not. Add it:

- `tools/spec-lint/src/lib.rs` — emit V-020 when a spec has no
  relationship fields (none of `establishes`, `extends`, `refines`,
  `supersedes`, `amends`, `co_authority`, `constrains`) and does not
  have `origin: retroactive: true`.
- Regression test: a fixture spec with no relationship fields and no
  origin; assert V-020 fires.
- Regression test: a fixture spec with `origin: retroactive: true`
  only; assert V-020 does not fire.
- Regression test: a fixture spec with `extends:` only; assert V-020
  does not fire.

### Concern 4 — Section-matching runtime activation

Spec 152 documents the section-matching algorithm. The gate currently
falls back to whole-file authority for co-authored paths. Activate
the real semantics.

**Per-file-type anchor parsers** in `tools/spec-code-coupling-check`
(or a new shared crate if cleaner):

- **Makefile parser** — recognize target groups by header comments or
  blank-line-separated blocks; anchor name maps to the target group's
  name.
- **YAML workflow parser** — recognize `jobs.<name>` blocks via the
  yaml-rust crate (or serde_yaml); anchor name is the job name.
- **Markdown heading parser** — recognize ATX headings (`## Section`);
  anchor name is the kebab-cased heading text.
- **Source region parser** — recognize `// region: <name>` and
  `// endregion` (Rust, TypeScript) or `# region: <name>` and
  `# endregion` (Python, shell); anchor name is the region label.
- **TOML/JSON parser** — recognize top-level keys; anchor name is the
  key name.

**Diff-hunk-to-section attribution.** For each diff hunk:

1. Determine the file type (extension-based, with shebang fallback).
2. Parse the file's sections via the appropriate parser.
3. Determine which section(s) the hunk overlaps.
4. If the hunk crosses sections, attribute it to *all* overlapping
   sections (every overlapped section's authority must be satisfied).
5. If the hunk sits outside any named section, attribute it to a
   special "unsectioned" pseudo-section. The gate's empty-authority-
   by-rule patterns may apply to unsectioned hunks; otherwise the
   path's whole-file authority applies.

**Gate tightening.** Replace the whole-file fallback with the
section-scoped derivation per spec 152. The gate now requires:

- For each edited hunk H in path P:
  - If P has section-scoped co-authority claims:
    - Match H to overlapping sections.
    - For each overlapping section S, the `co_authority` spec claiming
      `P#S` must be edited or amended in the diff.
  - If P has no section-scoped claims (whole-file authority):
    - Per the existing gate v2 semantics.

**Second-wave annotation corrections.** After activating section
matching, run the gate against the corpus's annotated state. If any
co-authority claim points at a section that the parser cannot find,
or at a section that no hunk in the corpus's history overlaps, surface
the misalignment. Correct annotations in the same commit. This is the
audit doing its job — the section names declared in frontmatter
must match the section names the parsers see.

### Concern 5 — Query surface in registry-consumer

The graph is queryable. Add to `tools/registry-consumer`:

- **`--by-authority <path>`** — given a code path, output the current
  authority set for that path (after supersession resolution),
  optionally with `--section <name>` to scope to a co-authored
  section.
- **`--show-relationships <spec-id>`** — given a spec id, output the
  spec's outgoing and incoming relationships in a structured form.
  Outgoing: what this spec establishes, extends, refines, supersedes,
  amends, co-authors, constrains. Incoming: which other specs
  reference this spec via those relationships.
- **`--show-supersession-chain <spec-id>`** — given a spec id, output
  the full chain of supersession (back to the originating spec and
  forward to any current supersedor), per path if scope is partial.
- **`--show-constraints-on <spec-id>`** — given a spec id, output the
  set of `constrains:` claims that target this spec, with their
  `kind:` declarations.
- **`--validate-graph`** — output any structural problems in the
  relationship graph: cycles in extends/supersedes chains, dangling
  references to non-existent spec ids, supersession that targets a
  spec which supersedes the supersedor (paradox), etc.

All new queries support `--json` for machine-readable output, per the
existing CLI conventions.

Update the help text contracts (`tools/registry-consumer/tests/
fixtures/help_contract/expected/`) for the new verbs.

### Concern 6 — Full-corpus gate verification

After all five prior concerns are complete, exercise the full corpus
through the gate:

- Construct a synthetic change set that touches at least one path in
  every load-bearing cluster.
- Run the gate. Confirm output is precise (small authority sets),
  semantically correct (the right specs are named), and complete
  (no path slips through unattributed).
- Construct a synthetic change set that *violates* a constraint
  (`constrains:`). Confirm the gate reports the constraint failure
  with the constraining spec and the invariant violated.
- Construct a synthetic change set that edits a section of a
  co-authored file. Confirm the gate matches the section and demands
  the right co-authority.
- Construct a synthetic change set that edits a path with no current
  authority and does not match any empty-authority-by-rule pattern.
  Confirm the gate fails with "spec required."

Document the verification runs in a session-internal note (not a
deliverable file; the operator reviews at the squash step).

## Schema, parser, and tool changes

All in scope for this commit:

- **Registry schema** — remove `implements:` (per Concern 1's
  resolution).
- **`tools/spec-compiler/src/lib.rs`** — remove `implements:`
  derivation and emission. The spec-compiler now emits relationship
  fields directly to the registry.
- **`tools/codebase-indexer/src/spec_scanner.rs`** — remove
  `implements:` derivation. The scanner emits relationship-aware data
  to the index.
- **`tools/spec-lint/src/lib.rs`** — V-020 emission per Concern 3.
  Remove any `implements:`-specific validators (they're now dead).
- **`tools/spec-code-coupling-check`** — section-matching parsers
  and gate tightening per Concern 4. Remove any `implements:`-
  specific code paths.
- **`tools/registry-consumer/src/lib.rs` and `main.rs`** — query
  surface per Concern 5. Remove any `implements:`-specific output.
- **The 152 spec.md files** — if `implements:` is sourced in
  frontmatter, remove the field after annotation is complete. (If
  emitted-only, no change to spec.md files for excision.)
- **Tests, golden files, fixtures** — sweep for `implements:`
  references; remove or update per the field's excision.
- **Documentation** — sweep CLAUDE.md, AGENTS.md,
  `.claude/rules/*.md`, the constitution, and the four governance
  specs for references to `implements:`. Update to the relationship-
  graph framing.

## Constitution

`standards/spec/constitution.md` (post-I3) or
`.specify/memory/constitution.md` (pre-I3 — this commit lands before
I3) — the §Spec Relationship Graph section gains:

- A normative declaration that `implements:` is *removed* (or
  preserved as a derived view, per Concern 1's resolution) — the
  relationship graph is the single source of truth.
- The well-formedness expectation strengthened: every spec declares
  its relationships; `origin: retroactive: true` is reserved for
  genuine bootstrap, not absence.
- Section-scoped authority is normative: co-authored paths require
  section anchors; the gate enforces section-level coupling.

≤ 100 lines added.

## Operator decisions (resolve before firing)

| # | Decision | Resolution |
|---|---|---|
| 1 | Default for `implements:` excision (subject to audit outcome): (a) total excision if no external consumers; (b) migrate external consumer in this commit and excise; (c) keep as derived view with documented consumer. Default (a). | `correct` |
| 2 | Annotation halts: Halt A (post-spec-compiler cluster) and Halt B (post-codebase-indexer cluster) confirmed. Allow additional halts only on structural ambiguity. | `correct` |
| 3 | Section-matching parsers: build in `tools/spec-code-coupling-check` directly, or in a new shared crate `tools/shared/section-parser`. Recommended: new shared crate for reuse by registry-consumer's `--validate-graph` and future tooling. | `correct` |
| 4 | Region marker convention for source files: `// region: <name>` (VSCode-compatible) confirmed. Add markers to co-authored source files as part of this commit. | `correct` |
| 5 | `origin: retroactive: true` discipline post-annotation: this field is reserved for genuine bootstrap (spec 000 and similar foundational instances). Specs annotated as `retroactive` purely because the first surgery deferred them get migrated to honest annotation. | `correct` |
| 6 | New query verbs in registry-consumer: `--by-authority`, `--show-relationships`, `--show-supersession-chain`, `--show-constraints-on`, `--validate-graph`. Confirmed scope. | `correct` |
| 7 | Cross-spec relationship cycles (extends → supersedes → extends) — disposition: gate fails with `--validate-graph`; corpus must be cycle-free. Confirmed. | `correct` |
| 8 | Empty-authority-by-rule patterns from the first surgery: revisit during this commit? Recommended: yes, with full corpus annotated, audit whether the patterns are still correct (e.g., `Cargo.lock` tail-suffix matching may be over-broad now that section-scoped authority is live). | `correct` |
| 9 | Spec 152's empty-authority-by-rule section — update with any patterns refined during this commit. Confirmed. | `correct` |
| 10 | Deferred items beyond these six concerns: none. Anything else surfaced during this commit defers to follow-up unless it's a trip-wire halt requiring in-commit resolution. | `correct` |

## Execution model

One session, autonomous, with two planned mid-session halts (decisions
#2A and #2B above). Agent works in WIP commits during the session;
final landing is `git reset --soft <pre-commit-HEAD>` + one squashed
commit.

**Session phases:**

1. **`implements:` consumer audit.** Enumerate every consumer per
   Concern 1. **HALT** with the consumer list for operator
   confirmation. Operator selects (a), (b), or (c). Proceed
   accordingly.
2. **`implements:` excision** (if (a) or (b)). Remove field from
   schema, emission, derivation, validators, output, golden files,
   docs, and spec.md frontmatter. `cargo test --workspace` green
   after excision.
3. **Annotation: registry-consumer cluster.** (Already annotated by
   the first surgery; verify it survives excision intact. If
   `implements:` was load-bearing in the existing annotations,
   migrate.)
4. **Annotation: spec-compiler cluster.** Draft annotations.
   **HALT A** for operator review.
5. **Annotation: codebase-indexer cluster** (including the rewritten
   133's interaction with this cluster). Draft annotations. **HALT B**
   for operator review.
6. **Annotation: remaining clusters.** Apply calibrated patterns to
   the ~85 specs not in the first surgery's load-bearing core. No
   halts unless structural ambiguity surfaces.
7. **Annotation: minimum-annotation migration.** The ~15 specs the
   first surgery marked `origin: retroactive: true` as a stopgap get
   migrated to honest annotation. The genuinely retroactive subset
   (spec 000 and similar) keeps `origin: retroactive: true`.
8. **V-020 emission in spec-lint.** Concern 3.
9. **Section-matching activation.** Concern 4. Parsers, attribution,
   gate tightening, second-wave annotation corrections.
10. **Query surface.** Concern 5. New verbs in registry-consumer,
    help text contracts, JSON output support.
11. **Constitution update.** Normative declarations per the
    constitutional section above.
12. **Full-corpus verification.** Concern 6. Synthetic change sets;
    confirm gate correctness across the corpus.
13. **Squash.** `git reset --soft <pre-commit-HEAD>`; one commit per
    §Commit shape.

## Verification gate

The single landing commit must satisfy simultaneously:

- `cargo build --workspace --release` clean.
- `cargo test --workspace` clean.
- `make pr-prep` clean under the now-section-active gate. Recursive
  verification: the commit passes its own gate, which is *stricter*
  than the first surgery's gate (section matching is live).
- `/init` clean end-to-end.
- `spec-lint` clean: V-020 fires on no spec in the corpus (because
  every spec has honest annotation). V-020 emission verified via
  regression test fixtures.
- `registry-consumer --validate-graph` reports zero structural
  problems (no cycles, no dangling references, no paradoxes).
- The registry produced by the new spec-compiler validates against
  the updated registry schema (no `implements:` field present, per
  Concern 1's resolution).
- Manual operator review at the squash step: the constitution's
  §Spec Relationship Graph section, the four governance specs (127,
  130, 133, 152) if any received touch-ups, a spot-check of 5–10
  annotated specs across clusters, the commit message.

## Trip-wires

- **`implements:` consumer audit surfaces an unexpected external
  consumer** (e.g., the desktop app reads `implements:` from the
  registry for some UI surface): halt at audit phase per Concern 1's
  halt; operator selects (b) migrate-in-commit or (c) preserve-as-
  derived.
- **A cluster annotation requires a ninth relationship kind** not in
  the eight defined: halt; design surface tension, operator decides
  whether to extend the field set in this commit or defer.
- **Section-matching parser produces ambiguous attributions** (a hunk
  attributable to multiple sections in a non-overlapping way): halt;
  the section scheme for that file type needs operator clarification.
- **A `co_authority:` annotation points at a section the parser cannot
  find:** correct the annotation in-commit (this is the audit doing
  its job) — but if more than ~10 such corrections surface in one
  cluster, halt; the cluster's annotation pattern may be wrong.
- **Cycles in extends/supersedes chains surface during
  `--validate-graph`:** halt; the corpus is structurally
  contradictory and operator must resolve.
- **`make pr-prep` under the tightened gate fails on the commit
  itself:** halt. The commit cannot land if it doesn't pass its own
  stricter gate. Re-examine annotations.
- **A spec migration from `origin: retroactive: true` to honest
  annotation reveals the spec was retroactive for good reason** (its
  relationship to code genuinely is "this code existed before the
  spec"): keep `origin: retroactive: true`; this is not a failure,
  it's the audit being honest. Surface in commit message for
  transparency.
- **Squashed diff exceeds 50,000 lines:** halt for operator scale-
  check before push. The first surgery was 1,699/-884; this commit
  may be larger due to full corpus annotation. Not a failure, but a
  pause.

## Commit shape

Single squashed commit:

```
feat(spec-governance): corpus-wide relationship graph annotation;
section-matching activation; query surface; implements excision

The spec relationship graph is now the corpus's complete and active
governance representation. All 152 specs declare their relationships
honestly. Section-scoped authority is live in the gate, enforced via
per-file-type anchor parsers (Makefile, GitHub workflows, markdown,
Rust/TypeScript regions, TOML/JSON top-level keys). The graph is
queryable through registry-consumer's new verbs. The legacy
implements: field is excised; the relationship graph is the single
source of truth.

Corpus annotation: 100+ specs newly annotated across the spec-
compiler, codebase-indexer, factory-engine extended, featuregraph
extended, registry-consumer extended, src-tauri extended, Makefile
co-authority extended, CI workflows extended, spec-spine tooling,
constitutional/bootstrap, governance/policy, and assorted standalone
clusters. ~15 specs migrated from origin: retroactive: true to
honest annotation. ~<count> specs retain origin: retroactive: true
as genuine foundational bootstrap.

V-020 (spec lacks relationship fields) is now emitted by spec-lint.
New specs cannot accrete without declaring their relationships.

Section-matching: per-file-type parsers attribute diff hunks to
named sections; the gate's co-authority satisfaction is now
section-scoped, replacing the first surgery's whole-file fallback.
Second-wave annotation corrections (<count>) applied where co-
authority claims pointed at sections the parsers did not recognize.

Query surface: registry-consumer gains --by-authority,
--show-relationships, --show-supersession-chain, --show-constraints-on,
and --validate-graph. The graph is no longer data without a query
surface — humans and downstream tools can introspect it directly.

Excision: implements: removed from the registry schema, spec-
compiler emission, codebase-indexer derivation, spec-lint
validators, and the 152 spec.md frontmatter files. <External
consumer disposition per Concern 1 resolution.>

Constitution: §Spec Relationship Graph strengthened. The graph is
the single source of truth; origin: retroactive: true is reserved
for genuine bootstrap; section-scoped authority is normative.

This commit passes its own gate under section-matching: every edited
path's current section-scoped authority is also edited in this
commit. registry-consumer --validate-graph reports zero structural
problems.

The spec spine is at representational maturity.

Refs: docs/analysis/cleanup/cleanup-master-plan.md
Refs: docs/analysis/cleanup/side-quest-spec-relationship-graph.md
Refs: docs/analysis/cleanup/side-quest-ii-corpus-maturity.md
```

## Hard rules

- **One landing commit.** WIP commits in-session; squash before push.
- **The commit passes its own gate under section matching.** Recursive
  verification, stricter than the first surgery. This is the proof
  that the corpus and the model are now coherent.
- **No retirements.** Annotation only — the four governance specs from
  the first surgery (127, 130, 133, 152) are touched up if needed but
  not rewritten. Their identities are stable.
- **No new governance concepts.** The eight relationship fields are
  the eight relationship fields. If a ninth genuinely surfaces, halt.
- **`origin: retroactive: true` is honest.** Use only for genuine
  bootstrap. Not for "we didn't get to curating this."
- **The audit precedes the excision.** Do not remove `implements:`
  before the consumer audit halt. Operator confirmation gates the
  removal.
- **No scope creep beyond the six concerns.** Anything else surfaced
  during this commit defers to follow-up, unless it's a trip-wire.
- **Each spec touched gets honest annotation.** No drive-by minimum
  annotation. The spec spine is the crown jewel; the annotations
  reflect that.

## What success looks like

After the commit lands:

- The spec corpus is the relationship graph. There is no parallel
  legacy `implements:` field carrying ambiguous duplicate authority.
- Every spec declares its relationships honestly. `origin:
  retroactive: true` appears only where the relationship is genuinely
  retroactive bootstrap.
- Section-scoped authority is live. Editing the supply-chain section
  of the Makefile requires editing the supply-chain spec, not any of
  the other seven specs that also touch the Makefile.
- V-020 enforces the model on new spec authoring. Drift cannot
  accrete.
- The graph is queryable. Humans and tools can ask `who governs path
  X?`, `what does spec Y relate to?`, `what supersedes spec Z?`, `what
  constrains spec W?`, `is the graph well-formed?` — and get precise
  answers.
- `--validate-graph` reports zero structural problems across the
  corpus.
- The gate's output is small and precise on any future change.
  Multi-claim noise is gone; section-scoped failures name the right
  one or two specs.
- The spec spine is structurally extractable for the upcoming repo
  split with no further governance-model work needed.

The corpus deserves coherent governance, populated honestly. This is
that commit.

Begin by reading the canonical inputs (the first surgery commit, the
four governance specs in their current state, the registry schema,
the spec-compiler / spec-lint / coupling-check / registry-consumer /
codebase-indexer source). Confirm pre-conditions. Then proceed
through the session phases. Three halts are planned (audit, Halt A,
Halt B); any further halt is a real blocker.

The spec spine is the crown jewel of the project. This commit is what
makes it that.
