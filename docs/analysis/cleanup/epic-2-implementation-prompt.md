# Epic 2 — Implementation

## What this is

The implementation epic of the structural cleanup. One CC session
executes 14 phases of repository surgery (I0 pre-flight + I1–I13),
each ending in zero, one, or more commits. The phases are pre-planned
in `docs/analysis/cleanup/implementation-manifest.md` (synthesized
from the 10 discovery audits produced by Epic 1) and have been
reconciled against the post-activation section-scoped coupling gate
in commit `131392ff`, with the relationship-graph field set locked
by the v3 revision against spec 130 §1's eight-field model and the
indexer's `spec_scanner.rs` read surface. This prompt sets the
meta-rules; the manifest is the per-phase mechanical contract.

**Read first, in order:**

1. `docs/analysis/cleanup/cleanup-master-plan.md` — locked target
   layout, cross-epic invariants, the 13-phase index.
2. This file (`epic-2-implementation-prompt.md`) — phase-by-phase
   procedure, invariants, halt conditions, post-activation
   refinements.
3. `docs/analysis/cleanup/implementation-manifest.md` — synthesized
   manifest with per-phase pre-conditions, operations, verification,
   trip-wires.
4. The 9 audit deliverables (`reference-audit.md`,
   `cargo-workspace-inventory.md`, `typescript-workspace-inventory.md`,
   `spec-implements-inventory.md`, `schema-duplication-audit.md`,
   `workflow-makefile-inventory.md`, `vcode-emission-audit.md`,
   `render-path-decomposition.md`, `protocol-drift-resolutions.md`) on
   demand, when the phase you are executing references them.

The implementation manifest is canonical for mechanical detail. This
prompt is canonical for procedure, invariants, and halt conditions.
Where they conflict, halt and surface.

## What this is NOT

- Not feature work. No new spec is authored. No new behavior is
  introduced. The only `fix(cleanup)` commit (I12) corrects a single
  permissive V-code emission to match the V-007 pattern; this is
  audit-driven, not new policy.
- Not an "improve as you go" pass. If a phase's audited path turns out
  to be slightly worse than something you spot mid-execution, halt and
  surface — do not improvise. The locked target layout is the locked
  target layout.
- Not an opinion pass. The operator has resolved every blocking
  decision (see §Operator decisions below) before firing this prompt.
  If you encounter a decision point that isn't covered, halt.
- Not a chance to re-litigate the manifest. If a phase's manifest
  entry seems wrong, halt and surface; do not silently deviate.
- Not a chance to re-curate relationship-graph annotations beyond
  mechanical path-string updates. Relationship-graph semantics were
  locked by the activation commit `131392ff` and the spec 130
  eight-field model; Epic 2 preserves them.

## Post-activation field model (orientation)

This block is descriptive, not prescriptive. The I0 enumeration
produces the definitive corpus state. This orientation reflects the
locked state as of v3.

Spec 130 §1 enumerates the eight relationship-graph fields:

> Eight frontmatter fields — establishes, extends, refines,
> supersedes, amends, co_authority, constrains, origin — encode
> how specs relate to code and to each other.

Of those eight, the corpus actually carries paths in six. The
remaining two (`amends`, `origin`) are spec-ID lists or metadata.

The indexer (`tools/codebase-indexer/src/spec_scanner.rs:142–215`)
reads four of the six path-bearing fields into `implementing_paths`,
which the coupling gate consumes. The other two are path-bearing in
the spec model but are not yet wired into the gate — they require
manual corpus-consistency checks.

| Field | Shape | Enforcement |
|---|---|---|
| `establishes:` | flat path strings | **gate-enforced** (whole-file) |
| `extends:` | `{paths: [...]}` items | **gate-enforced** (whole-file) |
| `refines:` | `{paths: [...]}` items | **gate-enforced** (whole-file) |
| `co_authority:` | `{paths, section, with_specs}` | **gate-enforced** (section-aware) |
| `supersedes:` | `{spec, scope, paths}` (scope: partial carries paths; scope: full is ID-only desugaring) | **corpus-consistency only** (not in `implementing_paths`) |
| `constrains:` | `{kind: invariant-freeze}` carries paths; `{kind: delivery-sequencing / sequencing-plan}` carries target_specs (out of scope for path sweeps) | **corpus-consistency only** (not in `implementing_paths`) |
| `amends:` | spec-ID list only | not path-bearing |
| `origin:` | `{retroactive: true, ...}` metadata | not path-bearing |

**Gate-enforced fields** fail `make pr-prep` on stale references.
**Corpus-consistency fields** do not — a missed sweep leaves stale
paths in the spec corpus without triggering a test or gate failure.
Both categories must be swept at each move phase; both categories
must be verified at each move phase; only one category fails loud
if you miss it.

The I0 enumeration is the source of truth for which fields actually
appear in the corpus at execution time; the table above is the
hypothesis the I0 step verifies against spec 130 and
`spec_scanner.rs`.

## Pre-conditions

Confirm before starting:

- Branch: `cut-d/autonomous-run-20260519-025506`.
- `git status` clean. No uncommitted changes.
- `git log --oneline -20` shows, in order from HEAD: the activation
  commit `131392ff` (`feat(spec-governance): activate section-scoped
  coupling gate`), the maturity commit `6e326463`
  (`feat(spec-governance): corpus-wide relationship graph
  annotation; ...`), the first surgery commit `8fc400d1`, then the
  10–12 Epic 1 `docs(cleanup): ...` commits. If any are missing or
  out of order, halt.
- `cargo test --workspace` clean.
- `/init` runs without surfacing path-resolution errors.
- `make pr-prep` clean **modulo two known Spec 151 W-codes** that
  surfaced during activation and are deferred per operator decision
  #11:
    - `platform/charts/rauthy/values-hetzner.yaml` (file does not
      exist; sibling stagecraft/deployd-api variants do).
    - `specs/137-tenant-environment-access-gates/tasks.md` section
      `phase2-migration` (anchor does not resolve to any heading
      slug).

  These two W-codes are baseline noise, not regressions. If
  `make pr-prep` surfaces *any other* W-codes or *any* V-codes,
  halt.
- `docs/analysis/cleanup/implementation-manifest.md` exists and matches
  the committed Epic 1 output. If the operator hand-edited it after
  Epic 1, that edited version is canonical.
- §Operator decisions below is fully filled in (every item resolved,
  no `<pending>` placeholders).

If any precondition fails, halt and surface.

## Operator decisions (fill in before firing this prompt)

| # | Decision | Discovery ref | Recommended default | **Operator resolution** |
|---|---|---|---|-------------------------|
| 1 | `apps/desktop/src-tauri/` Cargo workspace isolation | D2 OQ-1, D3 | Keep `src-tauri/` as a standalone workspace (preserves SQLite isolation, Tauri build idioms); I1 consolidates everything *except* `src-tauri/` | `correct`               |
| 2 | `platform/services/deployd-api-rs/` standalone disposition | D2 OQ-2 | Leave standalone (platform layer is structurally untouched per master plan §Locked target layout) | `correct`               |
| 3 | Coding-standard schema duplicate resolution | D5 | Keep `packages/yaml-standards-schema/schema/standard.schema.json` (npm-side consumer); delete `standards/schema/standard.schema.json`; update remaining refs to point at the kept copy | `correct`               |
| 4 | `crates/agent/src/schemas/` dormancy | D5 OQ | If unreferenced by any consumer, delete in I4; if referenced, co-locate under `standards/schemas/agent/` per master plan | `correct`               |
| 5 | `pnpm-workspace.yaml` runtime read location | D3 OQ-1, D6 OQ-4 | Workspace YAML lives at `product/pnpm-workspace.yaml` (per master plan §Locked target layout); `codebase-indexer` loader (`tools/codebase-indexer/src/lib.rs:446-447`, `manifest.rs:377-378`) updates to read from `product/pnpm-workspace.yaml` in the same I7 commit | `correct`               |
| 6 | V-010 dormancy in `spec-types` | D7 OQ-1 | Out of scope for Epic 2; surface as follow-up. Leave the constant in place; do not remove or change emission semantics | `correct`               |
| 7 | V-002 (b) truncation order for `extraFrontmatter` over-size | D7 OQ-2 | Alphabetical key order for the kept 8 entries (deterministic, predictable) | `correct`               |
| 8 | I1 root workspace style — `manifest-path` vs `--package` | D6 OQ-1 | Keep existing `manifest-path tools/<tool>/Cargo.toml` style in Makefile and workflows; root workspace consolidation does not require rewriting invocations | `correct`               |
| 9 | I5 per-tool spec-spine vs OAP categorisation | D6 OQ-2 | Use the master plan §Locked target layout categorisation verbatim (spec-spine: spec-compiler, registry-consumer, codebase-indexer, spec-lint, spec-code-coupling-check; OAP: oap-registry-enrich, oap-code-index-enrich, policy-compiler, adapter-scopes-compiler, assumption-cascade-check, ci-parity-check, schema-parity-check, stakeholder-doc-lint) | `correct`               |
| 10 | `@opc/root` package `oap.spec` field at root `package.json` | D3 OQ-3 | Do not add; root `package.json` is a workspace orchestrator and not subject to spec 127 coupling | `correct`               |
| 11 | Spec 151 dangling `co_authority:` references (the two surfaced during activation `131392ff`) | Activation summary | Defer to post-cleanup. Expected W-codes during Epic 2; not corrected in scope. Agent treats them as baseline noise. | `correct`               |
| 12 | G-2 spec_id validation work | Memory: "G-2 decoupled, small downstream change once typed reader exists" | Defer to post-cleanup. Out of Epic 2 scope. | `correct`               |
| 13 | `co_authority:` section-anchor resolution after I3 constitution.md graduation | New (post-activation) | Verify post-move with `spec-code-coupling-check --base origin/main --head HEAD` over the I3 diff range; if any anchor fails to resolve after the byte-identical `git mv`, the cause is a path-string mismatch (not anchor drift) — fix in the same I3 commit | `correct`               |
| 14 | Relationship-graph field path-bearing scope for I0 + per-phase sweeps | I0 enumeration discovery (v3 lock-in) | Lock the 6-field path-bearing set: `establishes`, `extends`, `refines`, `co_authority` (gate-enforced); `supersedes`, `constrains` (corpus-consistency only, not in `implementing_paths`). `amends`, `origin` are enumerated-but-not-path-bearing. Authority: spec 130 §1 eight-field model; `tools/codebase-indexer/src/spec_scanner.rs:142-215` for the gate-enforced subset | `correct`               |

If any `<pending>` remains when Epic 2 fires, halt immediately and
surface — do not infer.

## Execution model

Phases run sequentially **I0 → I1 → … → I13**. I0 is a read-only
pre-flight that may land zero or one commit; I1–I13 are the
substantive restructure phases. The agent works autonomously through
all phases without operator input between phases, subject to the
trip-wire halt conditions defined per-phase and globally below.
Within a phase, the agent may make multiple commits if the manifest
specifies sub-commits or if atomicity (e.g., per-crate `include_str!`
updates in I4) demands them.

At the boundary of each phase the agent runs the verification gate
(below) and proceeds only if it passes. Failure halts and surfaces.

## Cross-epic invariants (every commit, every phase)

These are restated from the master plan and are non-negotiable:

1. **Branch append-only.** No commit prior to the cleanup is modified.
   Rebase, squash, amend of pre-cleanup history is forbidden.
2. **`cargo test --workspace` clean at every commit.** If a phase
   cannot achieve this, halt at the last clean state, do not commit
   the broken state. Surface.
3. **`/init` works at every commit.** Daily-driver capability is
   preserved throughout. Within a phase the agent may temporarily
   reference moved paths during in-flight edits, but the phase's
   final commit must leave `/init` clean.
4. **Section-scoped coupling gate + corpus consistency at every
   commit.** Code moves land together with **all six locked
   relationship-graph reference updates** (per decision #14) in the
   same commit:
    - **Gate-enforced** (`establishes`, `extends`, `refines`,
      `co_authority`): `make pr-prep` fails loud if missed.
      `co_authority:` adds section-aware enforcement on top of
      whole-file.
    - **Corpus-consistency only** (`supersedes`, `constrains`):
      `make pr-prep` does NOT fail on missed sweeps. The phase
      verification gate adds an explicit grep-based cross-check
      (below).

   A path moved without all six fields being updated leaves either
   a gate failure (gate-enforced) or silent corpus rot
   (corpus-consistency). The I0 appendix to D4 enumerates the
   per-phase updates for both categories. No "I'll fix the spec
   refs in the next commit" — consult D4 + the I0 appendix before
   committing.
5. **Spec 103 governed-artifact-reads observed.** No ad-hoc parsing
   of `build/**` (or `.derived/**` post-I9). Consumer binaries
   (`codebase-indexer check`, `registry-consumer ...`) only. If a
   phase needs to read an artifact, it uses the consumer.
6. **One concern per phase.** No "while I'm here" scope creep.
   Cross-phase concerns wait for their own phase, even if the touched
   file is also touched in a later phase.
7. **Reversibility.** Each phase's commits can be `git revert`'d
   without cascading damage to earlier phases. If a planned operation
   would couple two phases irreversibly, halt and surface.
8. **No new specs.** New files (`standards/spec/spec-format.md`,
   `standards/spec/semver-policy.md`) are placeholders with TODO
   content (master plan §Out of scope). Substantive authoring is
   deferred.

## Verification gate (run at the end of every phase)

After the last commit of each phase, run in sequence:

```
git status                                    # must be clean
cargo build --workspace --release             # must succeed
cargo test --workspace                        # must pass
make pr-prep                                  # must pass (section-scoped gate + spec 103 gate; two known Spec 151 W-codes are baseline)
```

**Corpus-consistency cross-check** for any phase whose move-set
overlaps `supersedes:` or `constrains:` targets per the I0 appendix:

```
git grep -nE "^(supersedes|constrains):" specs/*/spec.md
```

Then verify (manually or via a small awk/jq pipeline) that no entry
still references a pre-move path from the phase's move set. If any
entry does, the sweep was incomplete — halt at the failing commit,
amend with the missed update, re-verify. The end-of-phase report
records the check result as `Corpus-consistency check
(supersedes/constrains): PASS` or `FAIL`.

For phases that touch the daily-driver protocol (I10) or the consumer
binaries (I4, I5, I9), additionally:

```
/init                                         # must complete without path errors
```

For phases that touch CI workflows (I1, I5, I7, I9), additionally
audit the affected workflow file by hand: every line touched is
either correctly updated to the new path or correctly preserved.
There is no automated workflow-syntax check beyond `actionlint` if
the repo carries it.

If any check fails, halt at the failing commit. Do not push.

## Cross-phase rules

Throughout all phases:

1. **Read the relevant audit before touching a path.** D4 + the I0
   appendix say which specs' relationship-graph rows update in
   which I-phase, distinguishing gate-enforced from
   corpus-consistency fields. D6 says which workflows and which
   Makefile lines. D1 catalogues every path reference. If a phase's
   manifest entry diverges from the audits, halt and surface.
2. **`git mv`, not `cp` + `rm`.** Path moves preserve history.
3. **Update references in the same commit as the move.** A move-only
   commit followed by a fix-refs commit leaves an in-between state
   where `cargo build` or `/init` is broken. Forbidden.
4. **Commit messages match the master plan shapes:**
    - `refactor(cleanup): ...` for move/restructure phases (I1–I11,
      I13).
    - `fix(cleanup): ...` for I12 (V-code emission audit fixes).
    - `docs(cleanup): ...` for I0 (inventory refresh appendix, if it
      lands a commit).
    - The first line ≤ 72 chars; body explains *what* and *why*; cite
      the discovery doc that drove the change.
5. **No autonomous resolution of new ambiguity.** If during a phase
   you discover a reference, file, or dependency that the audits
   missed, halt. Do not heuristically guess where it belongs.
6. **No "improvements" to anything touched.** If you move a file and
   notice a stale comment, leave it.
7. **`platform/` is read-only.** The only allowed touches to
   `platform/` are reference-updates inside files — never file
   moves, never directory restructure. (Note: `supersedes:`
   references pointing into `platform/` from non-platform specs do
   not require sweeps because the platform targets don't move.)
8. **The 9 audit docs themselves are append-only.** I0's appendix
   to D4 is created via the same append-only mechanism. If a phase
   discovers a reference D1 missed, append to the relevant audit
   doc in the same commit.
9. **Section-anchor resolution failure halts.** If after a `git mv`
   (in I3, I5, I6, I7, or I9) the section-aware gate reports a
   `co_authority:` annotation whose `path:` resolves to the new
   location but whose `section:` anchor no longer resolves inside
   the file, halt. (Section anchors exist only on `co_authority:`;
   the other relationship-graph fields are whole-file and this rule
   does not apply to them.) Expected causes:
   (a) the move was accompanied by an unintended content edit
   (forbidden by invariant 6),
   (b) the section anchor was already broken pre-move (a
   Spec 151–class baseline issue — append to the I0 appendix and
   surface; do not "fix" the annotation in scope), or
   (c) the parser for that path-type drifted (escalate to operator;
   do not improvise a parser fix in Epic 2 scope).
10. **Corpus-consistency sweep miss is a failure even if `make
    pr-prep` passes.** Per invariant 4 — `supersedes:` and
    `constrains:` are not gate-enforced. A missed sweep is silent
    rot. The end-of-phase corpus-consistency cross-check (above) is
    the catch; honour its FAIL exactly as you honour a `make
    pr-prep` FAIL.

## Reporting protocol

At the start of each phase:

```
=== Phase I<N> — <name> ===
Manifest reference: docs/analysis/cleanup/implementation-manifest.md
  §I<N>
Discovery references: D<X>, D<Y>
Pre-conditions verified: <list>
Planned commits: <count>
```

At the end of each phase:

```
=== Phase I<N> complete ===
Commits landed: <hashes>
Files touched: <count>
Relationship-graph refs updated (gate-enforced):
  establishes=<n>, extends=<n>, refines=<n>, co_authority=<n>
Relationship-graph refs updated (corpus-consistency):
  supersedes=<n>, constrains=<n>
Verification gate: PASS
Corpus-consistency check (supersedes/constrains): PASS
Trip-wires encountered: <none | list>
Audit-doc appendices added: <none | list>
```

Omit zero-count fields. If no relationship-graph refs were touched
(e.g., I1, I2), state `Relationship-graph refs updated: none` and
`Corpus-consistency check: N/A`.

If a trip-wire halts the phase mid-execution:

```
=== Phase I<N> HALT ===
Reason: <one sentence>
Last clean commit: <hash>
What was attempted: <one paragraph>
Operator decision needed: <one sentence question>
```

Surface the HALT and stop. Do not proceed to the next phase. Do not
attempt corrective improvisation.

---

# Phase I0 — Inventory refresh (pre-flight, read-only)

## Scope

D4 (`spec-implements-inventory.md`) was authored 2026-05-19, before
the side-quest activation. The activation (commit `131392ff`)
migrated path claims from list-form `implements:` to the
relationship-graph fields. Scalar `implements: "<spec-id>"` survives
as a parent-spec pointer that carries no paths.

Per operator decision #14, the post-activation load-bearing path
surface comprises six fields locked at I0 (four gate-enforced, two
corpus-consistency only). I0 produces the definitive corpus
enumeration as an append-only appendix to D4.

## Discovery reference

D4 (`spec-implements-inventory.md`), activation commit `131392ff`,
spec 130 §1 (eight-field model),
`tools/codebase-indexer/src/spec_scanner.rs:142-215` (gate-enforced
subset), operator decision #14.

## Operations

1. Verify the locked field set against authoritative sources:
    - Read `tools/codebase-indexer/src/spec_scanner.rs:142-215`;
      confirm it reads exactly `establishes`, `extends`, `refines`,
      `co_authority` into `implementing_paths`. If it reads a fifth
      field, halt — the gate-enforcement set has drifted from
      decision #14 and needs operator review.
    - Read `specs/130-*/spec.md` §1; confirm it enumerates exactly
      the eight fields named in §Post-activation field model. If
      spec 130 enumerates a ninth field, halt — the spec model has
      drifted.

2. Enumerate corpus state for each of the eight spec 130 fields:

   ```
   git grep -nE "^(establishes|extends|refines|supersedes|amends|co_authority|constrains|origin):" specs/*/spec.md
   ```

   For each match, extract `(spec, line, field, [path], [section])`
   tuples. For fields where the value is structured
   (`{spec, scope, paths}` etc.), parse the YAML to extract path
   values. For ID-only fields (`amends`, `origin`, scope: full
   `supersedes`), confirm no path values present.

3. Cross-reference each path-bearing tuple against D1 Groups A–O.
   Classify by I-phase per the master plan.

4. Append to D4 under these new headings (append-only; do not
   rewrite D4's main tables):

   ```
   ## relationship-graph fields locked at I0
   ```

   Single bulleted list with two sub-sections:
    - `### Gate-enforced (in implementing_paths)`: `establishes`,
      `extends`, `refines`, `co_authority`.
    - `### Corpus-consistency only (not in implementing_paths)`:
      `supersedes` (paths in scope: partial), `constrains` (paths
      in kind: invariant-freeze).
    - `### Enumerated but not path-bearing`: `amends`, `origin`.

   Cite spec 130 §1 and `spec_scanner.rs:142-215` as the
   authorities. I3–I9 reference this list verbatim.

   ```
   ## I0 refresh corrections (post-activation reconciliation)
   ```

   For any D4 row whose underlying entry no longer resolves to a
   real line (the shape gap from the list-form → relationship-graph
   migration). May be empty if D4's rows accidentally still
   resolve.

   ```
   ## relationship-graph reference enumeration (Epic 2 in-scope)
   ```

   The full enumerated set from step 2, scoped to entries whose
   `path:` matches a moving target (D1 Groups A–O). Columns:
   `spec | line | field | path | section | I-phase |
   enforcement`. The `enforcement` column carries `gate-enforced`
   or `corpus-consistency`.

5. Commit `docs(cleanup): I0 inventory refresh appendix to D4` if
   any of the three appendix headings carries content. If all three
   are empty (very unlikely post-activation), skip the commit and
   note in the I0 completion report.

## Verification

- `git status` clean post-commit.
- `make pr-prep` clean modulo the two known Spec 151 W-codes.
- `cargo test --workspace` unchanged (no code touched).
- The `relationship-graph fields locked at I0` list matches
  decision #14 exactly: four gate-enforced, two corpus-consistency,
  two not-path-bearing.

## Trip-wires

- If `spec_scanner.rs:142-215` reads a fifth field into
  `implementing_paths` that decision #14 didn't anticipate: halt.
  Gate-enforcement set has drifted.
- If spec 130 §1 enumerates a ninth field or fails to enumerate
  one of the eight named in §Post-activation field model: halt.
  Spec model has drifted.
- If the enumeration surfaces a corpus path entry under a field
  not in the locked six (e.g., paths appear under `amends:`):
  halt. Operator confirms whether the field promotes to
  path-bearing or whether the corpus entry is malformed.
- If the enumeration surfaces a relationship-graph reference whose
  target I-phase is ambiguous: halt. Do not heuristic-classify.
- If the enumeration surfaces a relationship-graph reference whose
  `path:` points at a path **not** catalogued in any D1 group:
  halt. Coverage gap in D1 needs operator review.
- If `git grep` surfaces list-form `implements:` entries (which
  V-014 should reject): halt. Activation did not fully migrate or
  the gate has regressed.

## Commit

`docs(cleanup): I0 inventory refresh appendix to D4`

---

# Phase I1 — Root Cargo workspace consolidation

## Scope

Today 16 separate `Cargo.lock` files exist across `crates/`,
`tools/spec-compiler/`, `tools/registry-consumer/`,
`tools/codebase-indexer/`, `tools/spec-lint/`,
`tools/spec-code-coupling-check/`, `tools/oap-registry-enrich/`,
`tools/oap-code-index-enrich/`, `tools/policy-compiler/`,
`tools/adapter-scopes-compiler/`, `tools/assumption-cascade-check/`,
`tools/ci-parity-check/`, `tools/schema-parity-check/`,
`tools/stakeholder-doc-lint/`, `tools/shared/spec-types/`,
`apps/desktop/src-tauri/`. After I1, one root `Cargo.toml`
declares a workspace covering all Rust members **except**
`apps/desktop/src-tauri/` (per operator decision #1) and
`platform/services/deployd-api-rs/` (per operator decision #2).

## Discovery reference

D2 (`cargo-workspace-inventory.md`).

## Operations

1. Author root `Cargo.toml` with `[workspace]` listing all 18
   `crates/*` plus the 13 in-scope `tools/*` members. Use
   `resolver = "2"`.
2. Hoist common deps to `[workspace.dependencies]` (per D2's
   shared-dep table). Member manifests adopt `dep = { workspace = true }`.
3. Remove the now-redundant `Cargo.lock` files (16 total in scope);
   one root `Cargo.lock` remains.
4. Update each member `Cargo.toml` to drop standalone `[workspace]`
   tables where they exist.
5. `apps/desktop/src-tauri/Cargo.toml` retains its standalone
   workspace; its path-deps continue to reference `../../../crates/...`
   verbatim (no path-depth change in I1 — that is I7's concern).
6. Update `rust-toolchain.toml` if not already at workspace root.
7. Update `deny.toml` to cover the unified workspace.

## Verification

- `cargo build --workspace --release`
- `cargo test --workspace`
- `cargo build` from `apps/desktop/src-tauri/` succeeds independently
- `cargo build` from `platform/services/deployd-api-rs/` succeeds
  independently
- `make pr-prep`
- Workflows referencing `manifest-path crates/<x>/Cargo.toml` or
  `manifest-path tools/<x>/Cargo.toml` continue to work
- Corpus-consistency check: N/A (no relationship-graph refs touched)

## Trip-wires

- Incompatible dep versions across members: halt, operator decides.
- Member tests pass standalone but fail under unified workspace:
  halt, surface.

## Commit

`refactor(cleanup): consolidate Rust crates into single workspace`

---

# Phase I2 — Create target directory skeleton

## Scope

Create the empty directory structure for the target layout. No file
moves yet.

## Discovery reference

Master plan §Locked target layout.

## Operations

1. Create empty directories with `.gitkeep` placeholders:
    - `standards/spec/grammar/`
    - `standards/spec/codes/`
    - `standards/spec/templates/`
    - `standards/schemas/spec-spine/`
    - `standards/schemas/frontmatter/`
    - `standards/schemas/factory/stage-outputs/`
    - `standards/schemas/agent/`
    - `standards/schemas/coding/`
    - `tools/spec-spine/scripts/bash/`
    - `tools/shared/`
    - `tools/oap/`
    - `tools/vendor/grammars/`
    - `product/`
    - `docs/contracts/`
    - `.derived/spec-registry/`
    - `.derived/codebase-index/`
    - `.derived/schema-parity/`
2. Update `.gitignore` to add `.derived/`.

## Verification

- `git status` shows only the new directory placeholders +
  `.gitignore` diff.
- `cargo test --workspace` unchanged.
- `make pr-prep` clean.
- Corpus-consistency check: N/A.

## Trip-wires

If `.gitignore` already contains conflicting `.derived/` or `build/`
rules, halt and surface.

## Commit

`refactor(cleanup): create target directory skeleton`

---

# Phase I3 — Standards content graduation

## Scope

`.specify/` Spec Kit content is graduated to `standards/spec/`:

- `.specify/memory/constitution.md` → `standards/spec/constitution.md`
- `.specify/contract` (file) → `standards/spec/contract.md`
- `.specify/templates/` → `standards/spec/templates/`
- New placeholder files: `standards/spec/spec-format.md`,
  `standards/spec/semver-policy.md`.

`.specify/scripts/` and `.specify/init-options` are evaluated in I13.

## Discovery reference

D1 Group A, D4 Group A, I0 appendix "relationship-graph reference
enumeration" Group A rows.

## Operations

1. `git mv .specify/memory/constitution.md standards/spec/constitution.md`
2. `git mv .specify/contract standards/spec/contract.md`
3. `git mv .specify/templates standards/spec/templates`
4. Create `standards/spec/spec-format.md` and
   `standards/spec/semver-policy.md` with TODO bodies.
5. Update every reference catalogued in D1 Group A (code-import,
   path-literal, doc-prose, spec-implements, gitignore-rule
   categories — all in scope here except those marked for I10
   protocol alignment).
6. **Sweep all six locked relationship-graph fields per the I0
   appendix, Group A rows.** Both gate-enforced (`establishes`,
   `extends`, `refines`, `co_authority`) and corpus-consistency
   (`supersedes`, `constrains`) updates land in the same commit.
   For every entry whose `path:` is `.specify/...`, update to
   `standards/spec/...`. Section anchors on `co_authority:`
   entries are byte-identical post-`git mv`; do not adjust.
7. **Per operator decision #13**: after the commit, run
   `spec-code-coupling-check --base origin/main --head HEAD` over
   the I3 diff. If any `co_authority:` annotation reports a
   resolution failure, the cause is a path-string mismatch — amend
   the commit. Anchor-resolution failures trigger cross-phase rule
   9 (halt).
8. Run the corpus-consistency cross-check (see Verification gate):

   ```
   git grep -nE "^(supersedes|constrains):" specs/*/spec.md
   ```

   Confirm no entry still references a pre-move `.specify/...`
   path. FAIL halts per cross-phase rule 10.

## Verification

- `cargo test --workspace`
- `make pr-prep`
- `/init` resolves the new `standards/spec/constitution.md` path
- `spec-code-coupling-check --base origin/main --head HEAD` clean
  modulo the two known Spec 151 W-codes.
- Corpus-consistency check (supersedes/constrains): PASS.

## Trip-wires

- `include_str!` of `.specify/contract` with offset-sensitive
  content: halt.
- Relationship-graph row referencing a `.specify/` path that the
  I0 appendix missed: halt, append to the I0 appendix.
- Cross-phase rule 9 applies to `co_authority:` entries.
- Cross-phase rule 10 applies to `supersedes:` / `constrains:`
  entries.

## Commit

`refactor(cleanup): graduate standards content from .specify/`

---

# Phase I4 — Schema co-location

## Scope

All authored schemas move to `standards/schemas/<group>/` per master
plan. Duplicate identified in D5 resolved per decision #3.

## Discovery reference

D5, D1 Groups B/D/E/F, D4 + I0 appendix Groups B/D/E/F.

## Operations

1. `git mv` each authored schema to its `standards/schemas/<group>/`
   destination.
2. Update every `include_str!("...")` referencing the schema.
   Atomic per crate.
3. Update every JSON Schema `$ref` between schemas that crossed
   directories.
4. Update runtime file-reads (loaders in `standards-loader/`,
   `agent-frontmatter/`, etc.) to the new paths.
5. Resolve standard.schema.json duplicate per decision #3.
6. Address `crates/agent/src/schemas/` per decision #4.
7. **Sweep all six locked relationship-graph fields per the I0
   appendix, Groups B / D / E / F rows.** Gate-enforced and
   corpus-consistency updates land together. Note: I0 surfaced
   `constrains:` entries in specs 130/132 pointing at
   `specs/000-bootstrap-spec-system/contracts/registry.schema.json`
   — these update to `standards/schemas/spec-spine/registry.schema.json`
   in this commit. (Specific spec/path tuples are in the I0
   appendix; this prose is for orientation.)
8. Run the corpus-consistency cross-check post-commit.

## Verification

- `cargo build --workspace --release` — every `include_str!`
  resolves.
- `cargo test --workspace`.
- `make pr-prep`.
- Schema-parity gate clean.
- Corpus-consistency check (supersedes/constrains): PASS.

## Trip-wires

- `include_str!` in build.rs: halt.
- JSON Schema `$ref` crossing a boundary the audit missed: halt.
- `crates/agent/src/schemas/` referenced by a consumer D5 missed:
  halt.
- Cross-phase rules 9 and 10 apply.

## Commit

`refactor(cleanup): consolidate authored schemas under standards/schemas/`

(May split per-crate.)

---

# Phase I5 — Tools restructure

## Scope

`tools/` subdivides per master plan:

- `tools/spec-spine/` — spec-compiler, registry-consumer,
  codebase-indexer, spec-lint, spec-code-coupling-check
- `tools/shared/` — spec-types
- `tools/oap/` — oap-registry-enrich, oap-code-index-enrich,
  policy-compiler, adapter-scopes-compiler,
  assumption-cascade-check, ci-parity-check, schema-parity-check,
  stakeholder-doc-lint
- `tools/vendor/` — placeholder for I6

## Discovery reference

D1 Groups G/H/I, D2, D6, D4 + I0 appendix Groups G/H.

## Operations

1. For each spec-spine tool: `git mv tools/<tool>
   tools/spec-spine/<tool>`. Update root `Cargo.toml`.
2. For each OAP tool: `git mv tools/<tool> tools/oap/<tool>`.
   Update root `Cargo.toml`.
3. `tools/shared/spec-types/` already at target.
4. Update every reference per D1 Groups G + H, every workflow +
   Makefile line per D6's I5 manifest.
5. Update `tools/ci-parity-check/src/lib.rs:592` hardcoded path.
6. Per decision #8: keep `manifest-path tools/<group>/<tool>/Cargo.toml`
   style.
7. **Sweep all six locked relationship-graph fields per the I0
   appendix, Groups G + H rows.** Per decision #9 for the
   spec-spine/OAP split.
8. Run the corpus-consistency cross-check post-commit.

## Verification

- `cargo build --workspace --release`.
- `cargo test --workspace`.
- Standalone tool builds (spec-spine + OAP) per manifest-path.
- `make spec-compile`, `make registry` end-to-end clean.
- `make pr-prep`.
- `/init` resolves new tool paths.
- Corpus-consistency check (supersedes/constrains): PASS.

## Trip-wires

- Tool category mismatch: master plan §Locked target layout is
  canonical; halt if a tool isn't in either list.
- Workflow tool path missed by D6: halt, append.
- Stale `Makefile:584` ref: remove in this phase.
- Cross-phase rules 9 and 10 apply.

## Commit

`refactor(cleanup): subdivide tools/ into spec-spine, shared, oap, vendor`

May split spec-spine vs OAP if manifest specifies.

---

# Phase I6 — Grammars vendor move

## Scope

`grammars/tree-sitter-*` → `tools/vendor/grammars/`.

## Discovery reference

D1 Group J, I0 appendix Group J (expected sparse).

## Operations

1. `git mv grammars tools/vendor/grammars`.
2. Update axiomregent's `build.rs` paths.
3. Update docs referencing `grammars/`.
4. **Sweep all six locked relationship-graph fields per the I0
   appendix, Group J rows.** Expected near-zero.
5. Run the corpus-consistency cross-check post-commit.

## Verification

- `cargo build --workspace --release`.
- `cargo test --workspace`.
- `make pr-prep`.
- Corpus-consistency check: PASS.

## Trip-wires

- tree-sitter binding crate hardcoded relative paths: halt.
- `build-axiomregent.yml` path-glob missed: halt.
- Cross-phase rules 9 and 10 apply.

## Commit

`refactor(cleanup): move tree-sitter grammars to tools/vendor/`

---

# Phase I7 — Product layer consolidation

## Scope

`apps/desktop/`, `packages/*`, root npm files → `product/`.

## Discovery reference

D1 Groups K/L/M, D3, D6, D4 + I0 appendix Groups K/L/M.

## Operations

1. `git mv apps/desktop product/apps/desktop`.
2. For each of the 22 packages: `git mv packages/<name>
   product/packages/<name>`.
3. Move root npm files into `product/`.
4. Update `pnpm-workspace.yaml` globs.
5. Update `apps/desktop/src-tauri/Cargo.toml` path-deps (eliminate
   if `workspace = true` from I1 neutralized them).
6. Update `apps/desktop/src-tauri/src/commands/claude.rs:154-161,1200`
   sidecar path.
7. Update `apps/desktop/vite.config.ts:18` comment.
8. **Update `tools/spec-spine/codebase-indexer/src/lib.rs:446-447`
   and `manifest.rs:377-378`** to read
   `product/pnpm-workspace.yaml` (decision #5). Same commit as the
   npm moves.
9. Update workflow trigger globs across ci-codebase-index.yml,
   ci-desktop.yml, build-axiomregent.yml, release-desktop.yml,
   ci-supply-chain.yml.
10. Update Makefile recipes per D6's I7 manifest.
11. `make registry` and codebase-indexer regenerate; both
    auto-rebase to new paths.
12. **Sweep all six locked relationship-graph fields per the I0
    appendix, Groups K / L / M rows.** Note: I0 surfaced `supersedes:`
    entries in spec 073 pointing at
    `apps/desktop/src-tauri/src/commands/{titor,search}.rs` — these
    update to `product/apps/desktop/src-tauri/src/commands/...` in
    this commit. (Specific tuples in the I0 appendix.)
13. Run the corpus-consistency cross-check post-commit.

## Verification

- `cargo build --workspace --release`.
- `cargo build --manifest-path product/apps/desktop/src-tauri/Cargo.toml`.
- `cargo test --workspace`.
- `(cd product && pnpm install)`.
- `(cd product && pnpm --filter @opc/desktop build)`.
- `make registry` clean.
- `make pr-prep`.
- `/init` resolves new product/ paths.
- Corpus-consistency check (supersedes/constrains): PASS.

## Trip-wires

- tsconfig `paths:` mapping crossing the boundary: halt.
- `codebase-indexer` loader fails to find
  `product/pnpm-workspace.yaml`: halt.
- featuregraph golden fixture diverges on regenerate: halt.
- Cross-phase rules 9 and 10 apply.

## Commit

`refactor(cleanup): consolidate end-user product layer under product/`

May split into (a) `move apps/desktop and packages/* under
product/`, (b) `move root npm workspace files under product/ and
update loaders`.

---

# Phase I8 — Loose top-level docs consolidation

## Scope

- `DEVELOPERS.md` → `docs/DEVELOPERS.md`
- `CONTRIBUTING.md` → `docs/CONTRIBUTING.md`
- `RELEASE-VERIFICATION.md` → `docs/RELEASE-VERIFICATION.md`

## Discovery reference

D1 Group N, I0 appendix Group N.

## Operations

1. `git mv` each of the three docs.
2. Update every reference per D1 Group N.
3. **Sweep all six locked relationship-graph fields per the I0
   appendix, Group N rows.** Expected sparse.
4. Run the corpus-consistency cross-check post-commit.

## Verification

- `git status` clean.
- `make pr-prep`.
- `/init` clean.
- Corpus-consistency check: PASS.

## Trip-wires

- None expected. Cross-phase rules 9 and 10 apply if any
  relationship-graph annotation in scope.

## Commit

`refactor(cleanup): consolidate loose top-level docs under docs/`

---

# Phase I9 — `build/` → `.derived/` rename

## Scope

Gitignored generated-artifacts dir rename. Consumer-side update only.

## Discovery reference

D1 Group O, D6, I0 appendix Group O.

## Operations

1. Update every reference to `build/spec-registry/`,
   `build/codebase-index/`, `build/schema-parity/` → `.derived/...`.
2. Update Makefile `pr-prep` and `clean` recipes; comments.
3. Update workflows: spec-conformance.yml, ci-codebase-index.yml,
   ci-spec-code-coupling.yml.
4. Update `.claude/commands/init.md` line 30 raw path-literal → use
   `codebase-indexer check` consumer binary (closes D-2.2 spec-103
   violation file-level).
5. Run `make registry` + `make index` to regenerate `.derived/`.
6. Remove `build/` from `.gitignore` if distinct; otherwise leave
   as updated in I2.
7. `rm -rf build/` locally.
8. **Sweep all six locked relationship-graph fields per the I0
   appendix, Group O rows.** Expected zero (authority targets
   authored files, not generated artifacts). Halt if any surface.
9. Run the corpus-consistency cross-check post-commit.

## Verification

- `cargo build --workspace --release`.
- `cargo test --workspace`.
- `make registry` regenerates into `.derived/spec-registry/`.
- `make index` regenerates into `.derived/codebase-index/`.
- `make pr-prep`.
- `/init` reads codebase-index via `codebase-indexer check`.
- Corpus-consistency check (supersedes/constrains): PASS.

## Trip-wires

- `codebase-indexer check` doesn't expose data init.md needs:
  halt, surface.
- Workflow trigger glob `build/...` D6 missed: halt.
- Cross-phase rules 9 and 10 apply.

## Commit

`refactor(cleanup): rename build/ to .derived/`

---

# Phase I10 — Protocol drift resolution

## Scope

11 D-2.* drift items from the `/init` trace resolve. AGENTS.md
canonical; init.md thin executor; CLAUDE.md back-references;
governed-artifact-reads.md aligned.

## Discovery reference

D9.

## Operations

For each D-2.<N> item, apply D9's resolution. Summary:

- D-2.1 Rules pre-load divergence → AGENTS.md "New Sessions" Step
  0 lists all three rule files; init.md and CLAUDE.md defer.
- D-2.2 spec-103 violation → file-level fix landed in I9; this
  phase's piece is protocol-language alignment in AGENTS.md and
  CLAUDE.md.
- D-2.3 through D-2.11 → per D9 per-item resolution table.

## Verification

- `/init` runs cleanly end-to-end; original trace reproduces zero
  drift items.
- `cargo test --workspace`.
- `make pr-prep`.
- Corpus-consistency check: N/A (no relationship-graph paths
  touched; protocol drift is rule-file content, not spec
  authority).

## Trip-wires

- A D-2.<N> resolution requires binary code change: halt.
- Two D-2.<N> resolutions contradict: halt.

## Commit

`refactor(cleanup): align /init protocol; AGENTS.md canonical`

May split.

---

# Phase I11 — Render-path resolution

## Scope

D-1 Option 3 render-path lands per D8: generic core under
spec-spine; OAP overlay under OAP tools. W-07b cycle not
reintroduced.

## Discovery reference

D8.

## Operations

1. Decompose per D8's contract.
2. Place generic core under `tools/spec-spine/<location-from-D8>/`.
3. Place OAP overlay under `tools/oap/<location-from-D8>/`.
4. Wire overlay-onto-core per D8.
5. Verify cycle-check per D8.

## Verification

- `cargo build --workspace --release` — no cycles.
- `cargo test --workspace`.
- `make pr-prep`.
- W-07b regression test (if D8 specified): passes.
- Corpus-consistency check: N/A (no relationship-graph paths
  touched).

## Trip-wires

- D8's seam ambiguous: halt.
- Cycle reappears: halt.

## Commit

`refactor(cleanup): decompose render path; generic core + OAP overlay`

---

# Phase I12 — V-code emission audit fixes

## Scope

V-002 (b) `extraFrontmatter` over-size truncation per V-007 pattern.

## Discovery reference

D7.

## Operations

1. Locate V-002 (b) emission at
   `tools/spec-spine/spec-compiler/src/lib.rs` near line 1280
   (post-I5 path).
2. Apply alphabetical truncation per decision #7.
3. Diagnostic message unchanged.
4. Add regression test: 9-entry fixture; assert V-002 (b) fires;
   assert emitted registry has 8 entries; downstream
   registry-self-validation passes.

## Verification

- `cargo test --workspace`.
- `cargo test -p spec-compiler` — regression test passes.
- `make pr-prep`.
- Schema-parity: emitted `registry.json` validates against
  `registry.schema.json` (`maxProperties: 8`).
- Corpus-consistency check: N/A.

## Trip-wires

- V-002 (b) site drifted from D7's line: re-locate by constant.
- Regression fixture causes other test regressions: halt.

## Commit

`fix(cleanup): truncate extraFrontmatter on V-002 over-size to satisfy registry schema`

---

# Phase I13 — Final cleanup; delete `.specify/`

## Scope

Delete the vestigial `.specify/` tree post-I3 graduation.

## Discovery reference

D1 Group A residual.

## Operations

1. `git rm -r .specify/`.
2. Final sweep: `git grep -n "\.specify"` returns zero hits in
   load-bearing files.
3. Final verification run.

## Verification

- `git grep -n "\.specify"` — only matches inside
  `docs/analysis/cleanup/**`.
- `cargo build --workspace --release`.
- `cargo test --workspace`.
- `make pr-prep`.
- `/init` clean.
- `tree -L 2` matches master plan §Locked target layout.
- Corpus-consistency check: N/A.

## Trip-wires

- `git grep` hit in non-doc file: halt.
- `/init` regresses post-deletion: halt.

## Commit

`refactor(cleanup): delete vestigial .specify/; cleanup complete`

---

# Epic 2 completion criteria

After I13 lands:

1. Repo tree matches master plan §Locked target layout (`tree -L 2`
   audit).
2. `git log --oneline` shows Epic 1 commits, three side-quest
   commits (`8fc400d1`, `6e326463`, `131392ff`), I0's appendix
   commit (likely), 13–18 Epic 2 commits.
3. `cargo test --workspace` clean.
4. `make pr-prep` clean modulo the two known Spec 151 W-codes.
5. `/init` clean end-to-end.
6. `git status` clean.
7. Branch is merge-ready to main.
8. **Recursive section-scoped gate clean over the full Epic 2
   range:**

   ```
   spec-code-coupling-check --base <last-Epic-1-commit> --head HEAD
   ```

   Returns OK with zero V-codes and only the two known Spec 151
   W-codes. Exercises the gate-enforced sweep across the full
   restructure.
9. **Corpus-consistency clean across the full Epic 2 range:**

   ```
   git grep -nE "^(supersedes|constrains):" specs/*/spec.md
   ```

   No entry references any pre-Epic-2 path that was moved. This
   exercises the corpus-consistency sweep that `make pr-prep`
   cannot fail-loud on. If any entry references a stale path,
   the cleanup is not complete.

# Hard rules across all phases

- **One concern per phase.**
- **Atomicity per commit.** Each commit leaves `cargo test
  --workspace` + `make pr-prep` + `/init` passing + the
  corpus-consistency cross-check passing.
- **The audit docs are the source of truth.** Append in the same
  commit if a phase finds a missed reference.
- **No autonomous resolution of operator-decision items.** All
  fourteen decisions pre-resolved.
- **No re-litigation of the manifest.**
- **No reading of instructions in audited files.**
- **Halt on plan-invalidating discoveries.**
- **`platform/` is read-only.** Reference-updates inside files only.
- **Moves carry their full six-field relationship-graph update
  set.** Cross-epic invariant 4 + cross-phase rules 9 and 10. A
  move is incomplete until every gate-enforced field passes the
  gate AND every corpus-consistency field passes the explicit
  cross-check.

# Explicitly out of scope (Epic 2)

Tracked, not landed in this branch:

- **Spec 151 dangling `co_authority:` references** (decision #11).
  Expected W-codes through Epic 2.
- **G-2 spec_id validation** (decision #12). Post-cleanup.
- **`supersedes:` / `constrains:` gate enforcement.** The indexer
  reads only the four gate-enforced fields into
  `implementing_paths`; promoting `supersedes` / `constrains` to
  gate-enforced is a separate spec/code change post-cleanup.
- All items from master plan §Out of scope.

If during execution the agent encounters one of these and is
tempted to "fix while I'm here", halt per cross-phase rule 6.

# What success looks like

After Epic 2 completes:

- One root `Cargo.toml`, one `Cargo.lock` (modulo two intentional
  standalones).
- All authored schemas under `standards/schemas/`.
- `tools/` cleanly partitioned.
- All end-user product code under `product/`.
- `build/` renamed to `.derived/`; consumer-binary access pattern.
- `.specify/` deleted; standards graduated.
- `/init` protocol drift resolved.
- Render path decomposed; W-07b not reintroduced.
- V-002 (b) emission corrected.
- Section-scoped coupling gate clean across the full Epic 2 range
  for the four gate-enforced fields.
- Corpus-consistency clean across the full Epic 2 range for the
  two corpus-consistency fields (`supersedes`, `constrains`) —
  zero stale paths in spec frontmatter.
- Branch is merge-ready.

The measure of success is layout-match + gate-clean +
corpus-consistency-clean + daily-driver-preserved +
spec-spine-extractable. Not vibes.

Begin with Phase I0.