# Side Quest II — Activation: Section-Scoped Coupling Gate

## What this is

A single-commit, branch-local activation pass. Side quest II
populated the spec relationship graph across the full corpus and
built the section-matching library API; the gate binary still falls
back to whole-file co-authority because the CLI is not yet wired to
the library. This commit wires it — expands parser coverage to match
the corpus's actual `co_authority:` annotations, plumbs git-diff
hunk parsing into the gate, makes the binary section-aware, exercises
the full corpus through synthetic verification scenarios, and tightens
the schema if vestigial `implements:` surfaces remain.

The maturity commit installed the data and the library. This commit
installs the *runtime*. After it lands, section-scoped authority is
the gate's actual behavior, not its documented behavior.

The commit lands on `cut-d/autonomous-run-20260519-025506`, ahead of
Epic 2. This is the last governance-model work before Epic 2 fires in
a fresh session.

## What this is NOT

- Not a third side quest. This is the runtime activation half of
  side quest II. The naming reflects that — this is finishing what
  was deliberately deferred for incremental staging.
- Not negotiable on the single-commit landing. WIP commits in-session;
  final landing is one squashed commit.
- Not a chance to revisit the relationship-graph model or the
  annotations. The eight fields are the eight fields; the corpus
  annotations are the corpus annotations. The activation may surface
  annotation corrections where a `co_authority:` claim points at a
  section the parsers can't find — those corrections land in this
  commit. But no annotation gets re-curated for taste; only for
  parser-alignment.
- Not a chance to expand the query surface, add new V-codes, or
  introduce new relationship semantics. The model is the model.

## Pre-conditions

- Branch: `cut-d/autonomous-run-20260519-025506`.
- `git status` clean.
- The maturity commit (`6e326463` — `feat(spec-governance): corpus-
  wide relationship graph annotation; section-matching activation;
  query surface; implements excision`) is present at HEAD or in the
  recent history.
- The first surgery commit (`8fc400d1`) precedes it.
- Epic 2 has not yet started. (If Epic 2 commits are present, halt.)
- `cargo test --workspace` clean.
- `make pr-prep` green on the baseline (under the whole-file fallback
  gate; this commit will tighten that).

## Scope — five phases

### Phase 1 — Parser coverage audit and expansion

Enumerate every `co_authority:` annotation in the corpus. For each,
extract the `(path-type, section-style)` pair. The Makefile parser
exists; build the remaining parsers to cover exactly the set the
corpus actually uses.

Audit command shape:

```
registry-consumer --show-relationships --json | \
  jq '.[] | .co_authority[]? | {path, section}' | sort -u
```

(or equivalent — the agent uses whatever surfaces the data
completely).

For each `(path-type, section-style)` pair the audit surfaces, build
the parser:

- **Makefile** — already exists. Verify it covers the annotated
  sections (the corpus may use anchors the existing parser doesn't
  recognize; expand if so).
- **GitHub workflow YAML** — `jobs.<name>` blocks. Use serde_yaml or
  yaml-rust. Anchor is the job name; section bounds are the job's
  line range.
- **Markdown** — ATX heading blocks (`## Section`). Anchor is the
  kebab-cased heading text; section bounds are heading line through
  next-heading-of-same-or-higher-level minus one.
- **Rust/TypeScript source** — `// region: <name>` / `// endregion`
  markers. Anchor is the region label; section bounds are the marker
  line through endregion line. If the corpus annotates a section in a
  source file that has no region markers, add the markers in this
  commit per the section scheme.
- **TOML/JSON** — top-level keys. Anchor is the key name; section
  bounds are the key's declaration through next-top-level-key minus
  one. Use the toml and serde_json crates.
- **Other** — only if the corpus audit surfaces a file type not in the
  above list. If it does, halt and surface — the section scheme for
  the new type needs operator input.

Each parser implements the section-matching library's parser trait
(established in the maturity commit). New parser tests follow the
existing 22-test pattern: section enumeration, boundary correctness,
edge cases (empty file, file with no sections, file with malformed
sections).

**Parser coverage location.** Per maturity-commit operator decision
#3, the parsers live in `tools/shared/section-parser` (or wherever
the maturity commit actually placed them — the agent confirms during
phase 1 read and follows the existing structure).

### Phase 2 — Git-diff hunk parsing and gate wiring

The gate binary (`tools/spec-code-coupling-check/src/main.rs`)
currently consumes a flat list of changed paths. Wire it to consume
hunks instead:

- **Hunk extraction.** Run `git diff --no-color -U0 <base>..<head>`
  and parse the unified-diff output. Each hunk header has the form
  `@@ -<old-start>,<old-count> +<new-start>,<new-count> @@ [context]`.
  Capture per-path the list of `(new-start, new-count)` ranges.
- **Hunk-to-section attribution.** For each hunk:
  1. Detect the file type (extension-based, with shebang fallback for
     extensionless files).
  2. Invoke the matching parser to enumerate the file's sections.
  3. Determine which section(s) the hunk's line range overlaps.
  4. Multi-section hunks: attribute to all overlapped sections (every
     overlapped section's authority must be satisfied).
  5. Unsectioned hunks (file has parser support but hunk sits outside
     all named sections): attribute to a synthetic "unsectioned"
     section. Empty-authority-by-rule patterns may apply; otherwise
     the path's whole-file authority applies.
  6. No-parser files: fall back to whole-file authority unchanged.
- **Gate decision.** Replace the existing whole-file decision logic
  with section-scoped per the maturity commit's library API
  (`check_coupling_section_aware`). The library is canonical for the
  satisfaction algorithm; the binary's job is hunk extraction,
  section attribution, and library invocation.
- **Output format.** When the gate fails, output names the precise
  `(path, section)` pair and the current authority set for that pair.
  Whole-file failures continue to output the path with its whole-file
  authority set. No "claimed by N specs" noise.

The whole-file fallback path is preserved for paths without parser
support and for paths whose authority is genuinely whole-file (no
`co_authority:` annotations). The fallback is not removed — it's
the correct behavior for that case.

### Phase 3 — Second-wave annotation corrections

Once the binary is section-aware, run `make pr-prep` (or the
underlying gate invocation directly) against a synthetic full-corpus
change. For any `co_authority:` annotation pointing at a section the
parsers cannot find, surface the misalignment.

Two correction kinds:

- **Annotation typo.** The annotation names a section that doesn't
  exist in the file (typo in the anchor, outdated section name).
  Correct the annotation to match the file's actual sections.
- **Missing section marker.** The annotation names a section that
  *should* exist but the file doesn't yet have the marker (e.g., a
  source file annotated with a region but no `// region:` marker
  present). Add the marker to the file.

Per the maturity prompt's trip-wire: more than ~10 corrections in one
cluster halts. If a cluster surfaces wholesale misalignment, the
cluster's annotation pattern may need operator review before
proceeding.

For sections that exist but have semantic mismatches (the anchor
*name* is right but the file's content under that section doesn't
match what the annotation claims), do *not* correct in this commit.
Surface them; the operator decides whether the annotation is wrong or
the file's content is wrong, and the resolution lands as a follow-up.
This commit is for parser-alignment, not semantic re-curation.

### Phase 4 — Synthetic verification

Three flavors, all three executed. The maturity prompt anticipated
this; it gets its real exercise here.

**Flavor A — mechanical coverage.** Construct a synthetic change set
that touches at least one path in every cluster identified by the
maturity commit's annotation passes (registry-consumer, spec-compiler,
codebase-indexer, factory-engine, featuregraph, src-tauri, Makefile,
CI workflows, spec-spine tooling, constitutional/bootstrap,
governance/policy, plus any standalone clusters). Also touch at least
one example of each relationship kind in use (`establishes`,
`extends`, `refines`, `supersedes`, `amends`, `co_authority`,
`constrains`, `origin`). Run the gate. Confirm:

- For every touched path, the gate identifies the correct authority
  set (small, precise).
- For paths with `co_authority:`, the gate matches the touched hunk
  to the correct section and demands the right co-authority spec.
- No path slips through unattributed.
- Output is small (per-path one or two specs typically, not lists of
  tens).

**Flavor B — adversarial.** Construct change sets the gate *should*
reject, with precise expected reasons:

- A change to a co-authored section without touching that section's
  authority spec → expect rejection naming the section and the
  authority.
- A change that violates a `constrains:` invariant → expect rejection
  naming the constraining spec and the invariant.
- A change to a path with no current authority that doesn't match an
  empty-authority-by-rule pattern → expect rejection with "spec
  required."
- A change to a section that exists in the parser but isn't claimed
  by any spec → expect rejection or empty-authority-by-rule
  acceptance, depending on the path.
- A change that touches multiple sections in one file → expect
  rejection if any one section's authority is unsatisfied.

For each adversarial scenario, the gate's rejection reason must match
the expected reason exactly. Mismatch indicates a bug in the
attribution or satisfaction logic.

**Flavor C — historical replay.** Select 10–20 actual commits from
the branch's history (including the surgery commit, the maturity
commit, and a sample of pre-surgery commits). For each, run the new
gate against the commit's diff as if it were a fresh PR. Record:

- Which the new gate would have accepted.
- Which the new gate would have rejected, and the rejection reason.
- Whether the rejection reason is *correct* (the commit genuinely
  was under-coupled) or *spurious* (the commit was well-coupled but
  the new gate's annotation knowledge is incomplete).

Spurious rejections in historical replay indicate annotation gaps —
specs that should claim a path but don't. Correct them in this commit
if they're in scope (the annotation simply missed the path); surface
them as follow-up if they require operator judgment (the annotation
needs new relationship semantics or cluster-level review).

Document the verification runs in a session-internal report. The
operator reviews the report at the squash step.

### Phase 5 — Schema tightening audit

Determine whether the registry schema still accepts list-form
`implements:` on input (i.e., a spec.md frontmatter with
`implements: [path1, path2]` would parse without error, even though
nothing emits it). If yes, decide:

- **Drop the input acceptance.** The schema rejects `implements:` on
  input; spec-compiler emits a parse error if it sees the field. This
  is the clean outcome.
- **Keep the input acceptance with a deprecation V-code.** spec-lint
  fires a new V-code (V-021 or next free) when it sees `implements:`
  on input, advising migration to relationship fields. Less clean;
  only chosen if the schema's input surface is consumed by external
  tooling we cannot migrate in-commit.

Audit by checking whether the spec-compiler's parser still has a code
path for list-form `implements:` consumption. If yes, the schema
accepts it. If the parser was already cleaned in the maturity commit
(no input path for `implements:`), this phase is a no-op verification.

Default: drop the input acceptance if the audit shows it's still
accepted but nothing requires it.

## Schema, parser, and tool changes

In scope for this commit:

- **`tools/shared/section-parser`** (or wherever the maturity commit
  placed it) — expanded parser coverage per Phase 1.
- **`tools/spec-code-coupling-check/src/main.rs`** — hunk extraction,
  section attribution, library invocation per Phase 2.
- **`tools/spec-code-coupling-check/src/lib.rs`** — if any
  refactoring needed to expose the library API cleanly for the
  binary; otherwise unchanged from the maturity commit.
- **The 152 spec.md files** — annotation corrections per Phase 3.
  Limited to anchor-name corrections and added region markers.
- **Co-authored source files** — added `// region:` / `# region:`
  markers per Phase 3 corrections.
- **Registry schema** — `implements:` input rejection per Phase 5 (if
  audit indicates).
- **`tools/spec-compiler/src/lib.rs`** — removal of `implements:`
  input parsing path per Phase 5 (if audit indicates).
- **`tools/spec-lint/src/lib.rs`** — V-021 emission per Phase 5 (only
  if the operator chooses the deprecation-V-code path over clean
  rejection; default path is no spec-lint change).
- **Tests, fixtures, golden files** — expanded per the new parser
  tests and any annotation corrections; updated golden files for any
  CLI output format changes in the gate binary.

## Constitution

No constitutional changes in this commit. The maturity commit's
§Spec Relationship Graph section already declares section-scoped
authority as normative. This commit makes that declaration
operationally true; it does not re-declare it.

## Operator decisions (resolve before firing)

| # | Decision | Resolution |
|---|---|---|
| 1 | Parser coverage scope: build exactly the parsers the corpus audit surfaces, no speculative parsers for file types nothing co-authors. Confirmed. | `correct` |
| 2 | If the audit surfaces a `(path-type, section-style)` not in the prompt's enumerated list (Makefile, YAML jobs, Markdown headings, source regions, TOML/JSON keys): halt for operator section-scheme input. Confirmed. | `correct` |
| 3 | Schema tightening default: drop list-form `implements:` input acceptance if audit shows it's still accepted but unused. Confirmed. | `correct` |
| 4 | Adversarial verification scenario set: the five listed in Phase 4 Flavor B are the minimum; agent may add more for coverage. Confirmed. | `correct` |
| 5 | Historical replay sample: 10–20 actual commits, including surgery commit, maturity commit, and a representative pre-surgery sample. Confirmed. | `correct` |
| 6 | Spurious-rejection corrections during historical replay: in-scope if annotation gap (missed path), out-of-scope if requiring operator judgment (re-curation, new semantics). Confirmed. | `correct` |
| 7 | Semantic-mismatch surfacing during Phase 3 (section anchor right but content wrong): surface and defer, do not correct in this commit. Confirmed. | `correct` |
| 8 | Output format change: the gate's failure output names `(path, section)` pairs and authority sets; "claimed by N specs" is eliminated. Confirmed. | `correct` |
| 9 | Annotation correction cluster halt threshold: more than ~10 corrections in one cluster halts for operator review. Confirmed. | `correct` |
| 10 | Single planned halt this session: after Phase 4 verification report, before Phase 5 schema tightening, for operator review of verification findings. Additional halts only on trip-wire. Confirmed. | `correct` |

## Execution model

One session, autonomous, with one planned mid-session halt. Agent
works in WIP commits during the session; final landing is `git reset
--soft <pre-commit-HEAD>` + one squashed commit.

**Session phases:**

1. **Read.** The maturity commit, the existing section-parser library
   structure, the gate binary's current main.rs, the corpus's
   `co_authority:` annotations.
2. **Phase 1 — Parser coverage audit.** Enumerate the `(path-type,
   section-style)` pairs the corpus uses. Build parsers to match.
   `cargo test --workspace` green after each parser lands.
3. **Phase 2 — Gate wiring.** Hunk extraction, section attribution,
   library invocation in main.rs. `cargo test --workspace` green.
4. **Phase 3 — Second-wave annotation corrections.** Run gate against
   synthetic full-corpus change. Correct anchor typos and add missing
   markers. Surface semantic mismatches without correcting.
5. **Phase 4 — Synthetic verification.** Three flavors: mechanical,
   adversarial, historical. Document the verification report.
6. **HALT** for operator review of verification report.
7. **Phase 5 — Schema tightening.** Audit `implements:` input
   acceptance; drop if appropriate.
8. **Final verification.** `cargo test --workspace`, `make pr-prep`
   under the now-active section-scoped gate (the commit must pass its
   own gate under the tightened semantics — this is the real
   recursive verification), `/init`, `spec-lint`, `registry-consumer
   validate-graph`.
9. **Squash.** `git reset --soft <pre-commit-HEAD>`; one commit per
   §Commit shape.

## Verification gate

The single landing commit must satisfy simultaneously:

- `cargo build --workspace --release` clean.
- `cargo test --workspace` clean (new parser tests + any existing
  test updates for output format changes).
- `make pr-prep` clean **under section-scoped semantics** — the
  commit passes its own gate under the tightened rules. This is the
  real recursive verification: the maturity commit passed under
  whole-file fallback; this commit must pass under section matching.
- `/init` clean.
- `spec-lint` clean.
- `registry-consumer validate-graph` — zero structural problems.
- All three synthetic verification flavors produce expected results
  (per the verification report).
- Manual operator review at the squash step: the verification report,
  any annotation corrections that landed, the commit message, a
  spot-check of the gate's new output format on 2–3 example failures
  (constructed for the spot-check; not real failures).

## Trip-wires

- **A `(path-type, section-style)` pair surfaces that isn't in the
  prompt's enumerated parser list:** halt; operator inputs the
  section scheme for the new type.
- **Phase 3 annotation corrections exceed ~10 in one cluster:** halt;
  the cluster's annotation pattern needs review.
- **Phase 4 historical replay surfaces spurious rejections requiring
  operator judgment** (not just annotation gaps): halt; operator
  decides scope-in vs scope-out for in-commit correction.
- **Phase 4 adversarial scenarios produce mismatched rejection
  reasons:** halt; bug in attribution or satisfaction logic. Fix
  before proceeding.
- **`make pr-prep` under section-scoped semantics fails on the commit
  itself:** halt. Re-examine annotation corrections; the commit
  cannot land if it fails its own stricter gate.
- **Phase 5 audit shows `implements:` input is still consumed by
  something:** halt; operator decides between in-commit migration
  and deprecation-V-code path.
- **A `co_authority:` annotation references a file type with no
  existing parser and no clear section scheme** (e.g., a binary
  file, an XML file with no obvious anchor convention): halt;
  operator inputs the section scheme or removes the annotation.
- **Squashed diff exceeds 30,000 lines:** halt for operator scale
  check. Expected size is smaller than the maturity commit (parser
  expansion + binary wiring + corrections is probably 3,000–8,000
  lines).

## Commit shape

Single squashed commit:

```
feat(spec-governance): activate section-scoped coupling gate

The spec relationship graph's section-scoped authority is now the
gate's runtime behavior. The maturity commit installed the corpus
annotations and the section-matching library API; this commit wires
the gate binary to consume them, expands parser coverage to match
the corpus's actual co_authority: annotations, and exercises the
full corpus through mechanical, adversarial, and historical
verification scenarios.

Parser coverage: <N> parsers covering <list>. Each parser implements
the section-matching library's parser trait with <count> tests.

Gate binary: spec-code-coupling-check now extracts git-diff hunks,
attributes each hunk to its containing section(s) via the
appropriate parser, and invokes check_coupling_section_aware for
satisfaction. Output names (path, section) pairs and authority
sets; the "claimed by N specs" output of the legacy gate is fully
eliminated.

Annotation corrections: <count> corrections across <count> clusters
where co_authority: anchors pointed at sections the parsers could
not find. Corrections are limited to anchor-name fixes and added
region markers; semantic mismatches were surfaced for follow-up
operator review.

Synthetic verification: mechanical coverage across all clusters
and all relationship kinds (passing); adversarial scenarios across
<count> failure modes (each producing precise expected rejection
reasons); historical replay across <count> actual commits (no
spurious rejections requiring re-curation).

Schema tightening: <result of Phase 5 — either "list-form
implements: input rejected; schema and parser cleaned" or
"verified no input acceptance; no-op">.

This commit passes its own gate under section-scoped semantics:
every edited path's section-scoped current authority is also
edited in this commit. The recursive verification is now tight
against the model's full semantics.

The spec spine's section-scoped governance is operationally live.

Refs: docs/analysis/cleanup/cleanup-master-plan.md
Refs: docs/analysis/cleanup/side-quest-spec-relationship-graph.md
Refs: docs/analysis/cleanup/side-quest-ii-corpus-maturity.md
Refs: docs/analysis/cleanup/side-quest-ii-activation.md
```

## Hard rules

- **One landing commit.** WIP commits in-session; squash before push.
- **The commit passes its own gate under section-scoped semantics.**
  This is the strictest recursive verification yet — the proof that
  the runtime matches the model.
- **No annotation re-curation.** Corrections are parser-alignment
  only (anchor name fixes, added region markers). Semantic
  mismatches surface for follow-up.
- **Parser coverage matches corpus need exactly.** No speculative
  parsers for file types nothing co-authors.
- **The library is canonical for satisfaction logic.** The binary's
  job is hunk extraction, section attribution, and library
  invocation. If a discrepancy surfaces between binary behavior and
  library expectations, fix the binary, not the library.
- **No model changes.** Eight relationship fields, V-020 in
  spec-lint, current set of constrains: kinds — all unchanged. V-021
  emission only as a Phase 5 fallback if total schema tightening is
  infeasible.
- **The verification report is part of the commit's review surface.**
  Operator reads it at the squash step. It is not a deliverable file
  (no docs/ entry) but it is structured well enough to read.

## What success looks like

After the commit lands:

- The gate binary's runtime behavior matches the model the maturity
  commit declared. Section-scoped authority is enforced; whole-file
  fallback is the genuine fallback (for paths without `co_authority:`
  annotations and for file types without parser support), not the
  default.
- Editing the supply-chain section of the Makefile demands editing
  the supply-chain spec, not any of the other Makefile-touching
  specs. The Makefile-class problem is operationally solved.
- The gate's failure output is precise: `(path, section)` pairs and
  small authority sets. No "claimed by N specs" lists.
- The corpus's annotations are parser-aligned: every `co_authority:`
  anchor names a section the parsers actually find.
- The graph has been exercised: mechanical coverage proved correct
  attribution; adversarial scenarios proved precise failure
  semantics; historical replay calibrated the model against real
  changes.
- The schema is at minimum surface: `implements:` is gone from input
  acceptance (or its remaining acceptance is documented and
  justified).
- Epic 2 starts in a fresh session against a fully-active gate. Any
  annotation issues Epic 2's structural moves surface are fixable
  one-at-a-time in the moves' own commits.

The spec spine is operationally complete. The next session does
structural cleanup against it.

Begin by reading the maturity commit and the section-parser library's
current state. Confirm pre-conditions. Then proceed through the
session phases. One halt is planned (post-verification, pre-schema-
tightening); any further halt is a real blocker.

This is the commit that makes the model true.
