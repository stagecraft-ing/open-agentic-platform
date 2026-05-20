# Epic 2 — Implementation

## What this is

The implementation epic of the structural cleanup. One CC session
executes 14 phases of repository surgery (I0 pre-flight + I1–I13),
each ending in zero, one, or more commits. The phases are pre-planned
in `docs/analysis/cleanup/implementation-manifest.md` (synthesized
from the 10 discovery audits produced by Epic 1) and have been
reconciled against the post-activation section-scoped coupling gate
in commit `131392ff`. This prompt sets the meta-rules; the manifest
is the per-phase mechanical contract.

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
  mechanical path-string updates. Relationship-graph semantics
  (`establishes:`, `extends:`, `refines:`, `amends:`, `co_authority:`,
  any other path-bearing fields surfaced during I0) were locked by
  the activation commit `131392ff`; Epic 2 preserves them.

## Post-activation field model (orientation)

This block exists so the agent reads Epic 2 with the correct mental
model of where path references live post-activation. It is descriptive,
not prescriptive — I0 enumerates the actual corpus state authoritatively.

Pre-activation, list-form `implements:` carried path arrays. The
activation excised list-form `implements:` (V-014 errors on it) and
migrated path claims to the relationship-graph fields:

| Field | Carries | Authority shape |
|---|---|---|
| `establishes:` | paths arrays | whole-file |
| `extends:` | paths arrays | whole-file |
| `refines:` | paths arrays | whole-file |
| `amends:` | paths arrays | whole-file |
| `co_authority:` | path + section | section-aware |
| `constrains:` | enumerate at I0; may or may not carry paths | tbd |
| `implements:` | scalar spec-ID pointer only (e.g., `implements: "148-..."`) | n/a — no paths to sweep |

Path-string updates during Epic 2 sweep **all path-bearing
relationship-graph fields**, not just `co_authority:`. D4 was
authored pre-activation against list-form `implements:`; I0
reconciles by enumerating the relationship-graph fields and
producing an append-only appendix.

Section anchors exist only on `co_authority:` (path + section pairs).
The other path-bearing fields are whole-file. Cross-phase rule 9
(section-anchor resolution failure) therefore applies only to
`co_authority:`.

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

Epic 1 and the post-activation reconciliation surfaced the following
open questions. The operator resolves each before Epic 2 fires; the
agent reads these resolutions and proceeds accordingly. Recommended
defaults are provided where the discovery audits made one explicit.

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
| 14 | Relationship-graph field enumeration scope for I0 + per-phase sweeps | I0 in-flight discovery (post-activation) | Sweep all path-bearing relationship-graph fields surfaced by I0 enumeration, not just `co_authority:`. Starting hypothesis: `establishes:`, `extends:`, `refines:`, `amends:`, `co_authority:`; verify `constrains:` (and any other field surfaced) at I0 enumeration. I0 produces the definitive list as an append-only appendix to D4 | `correct`               |

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
4. **Section-scoped coupling gate at every commit.** Code moves land
   together with **all relationship-graph reference updates** in the
   same commit. The gate as of commit `131392ff` enforces authority
   via the relationship-graph fields (`establishes:`, `extends:`,
   `refines:`, `amends:`, `co_authority:`, and any other path-bearing
   field surfaced by the I0 enumeration). `co_authority:` adds
   section-aware enforcement on top of whole-file. A path moved
   without its corresponding relationship-graph references being
   updated will fail the gate on the next file touch that hits
   that path or section. The I0 appendix to D4 enumerates the
   per-phase relationship-graph reference updates. `make pr-prep`
   succeeds at every commit (modulo the two known Spec 151 W-codes
   per pre-conditions). No "I'll fix the spec refs in the next
   commit" — consult D4 + the I0 appendix before committing.
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
   which I-phase. D6 says which workflows and which Makefile lines.
   D1 catalogues every path reference. If a phase's manifest entry
   diverges from the audits, halt and surface.
2. **`git mv`, not `cp` + `rm`.** Path moves preserve history; the
   reviewer (and the operator) can `git log --follow` through the
   cleanup.
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
      the discovery doc that drove the change (e.g., "Per
      D5/schema-duplication-audit.md §Group X").
5. **No autonomous resolution of new ambiguity.** If during a phase
   you discover a reference, file, or dependency that the audits
   missed, halt. Do not heuristically guess where it belongs.
6. **No "improvements" to anything touched.** If you move a file and
   notice a stale comment, leave it. Stale comments are tracked as
   follow-up; this branch is for structural restructure only.
7. **`platform/` is read-only.** Master plan §Locked target layout
   keeps platform/ internal structure unchanged. The only allowed
   touches to platform/ are reference-updates inside files
   (e.g., a doc that mentions a moved path) — never file moves,
   never directory restructure.
8. **The 9 audit docs themselves are append-only.** If, during a
   phase, you discover a reference D1 missed, add a "Discovered in
   I-phase execution" appendix to the relevant audit doc in the same
   commit. Do not silently update the audit's main tables. I0's
   appendix to D4 is created via the same append-only mechanism.
9. **Section-anchor resolution failure halts.** If after a `git mv`
   (in I3, I5, I6, I7, or I9) the section-aware gate reports a
   `co_authority:` annotation whose `path:` resolves to the new
   location but whose `section:` anchor no longer resolves inside
   the file, halt. (Section anchors exist only on `co_authority:`;
   the other relationship-graph fields are whole-file and this rule
   does not apply to them.) The expected cause is one of:
   (a) the move was accompanied by an unintended content edit
   (forbidden by invariant 6),
   (b) the section anchor was already broken pre-move (a
   Spec 151–class baseline issue — append to D4's I0 refresh
   appendix and surface; do not "fix" the annotation in scope), or
   (c) the parser for that path-type drifted (escalate to operator;
   do not improvise a parser fix in Epic 2 scope).

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
Relationship-graph refs updated: establishes=<n>, extends=<n>,
  refines=<n>, amends=<n>, co_authority=<n>, <other>=<n>
Verification gate: PASS
Trip-wires encountered: <none | list>
Audit-doc appendices added: <none | list>
```

Omit zero-count fields from the relationship-graph line. If no
relationship-graph fields were touched in the phase (e.g., I1, I2),
state `Relationship-graph refs updated: none`.

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
relationship-graph fields. V-014 now errors on list-form
`implements:` stragglers; scalar `implements: "<spec-id>"` survives
as a parent-spec pointer that carries no paths.

Two gaps result, both reconciled in I0:

1. **Shape gap.** D4's tables reference list-form `- path:` entries
   under `implements:`. Those entries no longer exist verbatim. The
   same path strings now live under relationship-graph fields in
   the same specs.
2. **Coverage gap.** D4 does not enumerate the relationship-graph
   fields. Per operator decision #14, the post-activation
   load-bearing surface for path references is the full set of
   path-bearing relationship-graph fields. I0 enumerates this set
   authoritatively against the corpus.

## Discovery reference

D4 (`spec-implements-inventory.md`), activation commit `131392ff`,
operator decision #14.

## Operations

1. Enumerate the relationship-graph field surface against
   post-activation HEAD. Starting hypothesis: `establishes:`,
   `extends:`, `refines:`, `amends:`, `co_authority:`. Verify
   `constrains:` and any other path-bearing field by reading
   `tools/spec-compiler/src/lib.rs` around line 247–250 (the
   compiler's relationship-graph source-field list) and
   cross-checking against `git grep -nE "^(establishes|extends|refines|amends|co_authority|constrains):" specs/*/spec.md`.

   Lock the final field list at the top of the I0 appendix
   (described below). The list is the definitive scope for I3–I9
   sweeps.

2. For each enumerated field, extract every `(spec, line, path,
   [section])` tuple. For `co_authority:` entries, include the
   `section:` value; for the other path-bearing fields, omit.

3. Cross-reference each tuple against D1 Groups A–O (the same
   path-group taxonomy D4 uses). Classify by I-phase per the master
   plan.

4. Append to D4 under three new headings (append-only; do not
   rewrite D4's main tables):

   ```
   ## I0 refresh corrections (post-activation reconciliation)
   ```

   For any D4 row whose underlying entry no longer resolves to a
   real line (the shape gap). May be empty if D4's rows are
   accidentally still resolvable.

   ```
   ## relationship-graph reference enumeration (Epic 2 in-scope)
   ```

   The full enumerated set from step 2, scoped to entries whose
   `path:` matches a moving target (D1 Groups A–O). Columns:
   `spec | line | field | path | section | I-phase`.

   ```
   ## relationship-graph fields locked at I0
   ```

   A single bulleted list of the field names that I0 enumerated.
   I3–I9 sweeps reference this list verbatim.

5. Commit `docs(cleanup): I0 inventory refresh appendix to D4` if
   any of the three headings carries content. If all three are
   empty (corpus shape exactly matches D4 + only `implements:` is
   path-bearing — very unlikely post-activation), skip the commit
   and note in the I0 completion report that no appendix was
   needed.

## Verification

- `git status` clean post-commit.
- `make pr-prep` clean modulo the two known Spec 151 W-codes.
- `cargo test --workspace` unchanged (no code touched).
- The `relationship-graph fields locked at I0` list contains, at
  minimum, every field for which the I0 enumeration found at least
  one path entry. If a field is in the starting hypothesis but the
  enumeration finds no path entries (e.g., `constrains:` is
  spec-ID-only), it is listed under a sub-heading
  `## relationship-graph fields enumerated but not path-bearing` for
  documentation completeness.

## Trip-wires

- If the enumeration surfaces a path-bearing relationship-graph
  field that was not in the starting hypothesis (i.e., a sixth or
  seventh field): halt, surface. This indicates the field model
  has drifted from operator decision #14's starting hypothesis;
  operator confirms before I3 begins.
- If the enumeration surfaces a relationship-graph reference whose
  target I-phase is ambiguous (e.g., a path that crosses two
  groups): halt. Do not heuristic-classify.
- If the enumeration surfaces a relationship-graph reference whose
  `path:` points at a path **not** catalogued in any D1 group:
  halt. This is a coverage gap in D1 that needs operator review.
- If `git grep` surfaces list-form `implements:` entries (which
  V-014 should have prevented from existing): halt. The activation
  either did not fully migrate the corpus or the gate has
  regressed.

## Commit

`docs(cleanup): I0 inventory refresh appendix to D4`
(or no commit if all three appendix headings are empty — very
unlikely).

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

D2 (`cargo-workspace-inventory.md`) — all manifests, all path-deps,
all lockfile locations.

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
  (standalone workspace preserved)
- `cargo build` from `platform/services/deployd-api-rs/` succeeds
  independently
- `make pr-prep`
- Workflows referencing `manifest-path crates/<x>/Cargo.toml` or
  `manifest-path tools/<x>/Cargo.toml` continue to work (manifest-path
  invocations are workspace-compatible)

## Trip-wires

- If two members declare incompatible versions of the same direct
  dep (e.g., `serde = 1.0.150` in one, `serde = 1.0.200` in another)
  and the workspace cannot unify without breaking a public API:
  halt, surface, operator decides which version becomes canonical.
- If a member's tests pass standalone but fail under the unified
  workspace (transitive feature unification): halt, surface.

## Commit

`refactor(cleanup): consolidate Rust crates into single workspace`

---

# Phase I2 — Create target directory skeleton

## Scope

Create the empty directory structure for the target layout. No file
moves yet. This phase exists to make I3–I9's `git mv` operations
land cleanly into pre-existing destination dirs (which makes diffs
readable and avoids "directory created as a side effect of move"
noise).

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
2. Update `.gitignore` to add `.derived/` (entire tree gitignored —
   it replaces `build/`'s ignored content). Leave the I9 cutover of
   the existing `build/` paths to I9; for I2, both `build/` and
   `.derived/` are gitignored, but only `.derived/` carries the
   `.gitkeep` placeholders.

## Verification

- `git status` shows only the new directory placeholders + .gitignore
  diff.
- `cargo test --workspace` unchanged from I1 (skeleton creation does
  not affect Rust build).
- `make pr-prep` clean.

## Trip-wires

None expected. If `.gitignore` already contains conflicting `.derived/`
or `build/` rules, halt and surface.

## Commit

`refactor(cleanup): create target directory skeleton`

---

# Phase I3 — Standards content graduation

## Scope

`.specify/` Spec Kit content is graduated to `standards/spec/`.
Specifically:

- `.specify/memory/constitution.md` → `standards/spec/constitution.md`
- `.specify/contract` (file) → `standards/spec/contract.md`
- `.specify/templates/` → `standards/spec/templates/`
- New placeholder files: `standards/spec/spec-format.md`,
  `standards/spec/semver-policy.md` (both with TODO content per
  master plan §Out of scope).

`.specify/scripts/` is *not* moved here; it's evaluated for deletion
in I13. `.specify/init-options` is evaluated in I13.

## Discovery reference

D1 Group A (`.specify/` paths), D4 Group A, I0 appendix
"relationship-graph reference enumeration (Epic 2 in-scope)"
Group A rows.

## Operations

1. `git mv .specify/memory/constitution.md standards/spec/constitution.md`
2. `git mv .specify/contract standards/spec/contract.md`
3. `git mv .specify/templates standards/spec/templates`
4. Create `standards/spec/spec-format.md` and `standards/spec/semver-policy.md`
   with TODO bodies.
5. Update every reference catalogued in D1 Group A (code-import,
   path-literal, doc-prose, spec-implements, gitignore-rule
   categories — all in scope here except those marked for I10 protocol
   alignment, which I10 handles).
6. **Sweep all relationship-graph references per the I0 appendix,
   Group A rows.** For every field listed in `relationship-graph
   fields locked at I0` whose entries point at `.specify/...`,
   update `path:` values to `standards/spec/...` in the same commit.
   Section anchors on `co_authority:` entries are byte-identical
   post-`git mv`; do not adjust the `section:` values.
7. **Per operator decision #13**: after the commit, run
   `spec-code-coupling-check --base origin/main --head HEAD` over
   the I3 diff. If any `co_authority:` annotation reports a
   resolution failure, the cause is a path-string mismatch (not
   anchor drift) — amend the commit to fix the path string in the
   same phase. If anchor itself fails to resolve, this triggers
   cross-phase rule 9 (halt).

## Verification

- `cargo test --workspace`
- `make pr-prep`
- `/init` resolves the new `standards/spec/constitution.md` path
  (caveat: if `/init` also reads `.specify/contract` via init.md,
  that's an I10 concern and may show transient — surface if so).
- `spec-code-coupling-check --base origin/main --head HEAD` clean
  modulo the two known Spec 151 W-codes.

## Trip-wires

- If a path-literal in production Rust code reads `.specify/contract`
  and the post-cleanup path isn't substitutable (e.g., binary file
  embedded via `include_str!` at compile time and the offset matters):
  halt, surface.
- If a spec's relationship-graph row references a `.specify/` path
  that the I0 appendix missed: halt, surface, file an appendix to
  D1 / the I0 appendix.
- Cross-phase rule 9 (section-anchor resolution failure) applies
  to `co_authority:` entries.

## Commit

`refactor(cleanup): graduate standards content from .specify/`

---

# Phase I4 — Schema co-location

## Scope

All authored schemas (the ones humans write, not the ones generators
produce) move to `standards/schemas/<group>/` per the master plan
§Locked target layout. The duplicate identified in D5 is resolved
per operator decision #3.

## Discovery reference

- D5 (`schema-duplication-audit.md`) — every authored schema with
  its current location and target location.
- D1 Groups B, D, E, F — reference paths.
- D4 + I0 appendix Groups B, D, E, F — spec coupling references.

## Operations

For each authored schema:

1. `git mv` the schema file to its `standards/schemas/<group>/`
   destination.
2. Update every `include_str!("...")` referencing the schema. Updates
   are atomic per crate (do all of one crate's includes in one commit
   if multiple, so the crate is never half-broken). If multiple
   crates' includes are mutually independent, batch into a single
   commit per the manifest.
3. Update every JSON Schema `$ref` between schemas that crossed
   directories.
4. Update runtime file-reads (the loaders in `standards-loader/`,
   `agent-frontmatter/`, etc.) to the new paths.
5. Resolve the standard.schema.json / coding-standard.schema.json
   duplicate per operator decision #3: keep
   `packages/yaml-standards-schema/schema/standard.schema.json` (which
   I7 later moves into `product/packages/...`); delete
   `standards/schema/standard.schema.json`; update all refs to
   `standards/schemas/coding/standard.schema.json` to point at the
   kept npm-side copy (or, if the kept copy is also being re-co-located
   into `standards/schemas/coding/`, harmonize via a single canonical
   path — operator decision #3 should specify which).
6. Address `crates/agent/src/schemas/` per operator decision #4 — if
   delete: `git rm` the directory and any dead references; if move:
   `git mv` into `standards/schemas/agent/`.
7. **Sweep all relationship-graph references per the I0 appendix,
   Groups B / D / E / F rows.** For every field listed in
   `relationship-graph fields locked at I0` whose entries point at
   the moving schemas, update `path:` values to
   `standards/schemas/<group>/...`. Section anchors inside JSON
   schemas are uncommon; if any `co_authority:` annotation has a
   `section:` for a schema, surface for operator review (likely an
   annotation error).

## Verification

- `cargo build --workspace --release` — every `include_str!` resolves.
- `cargo test --workspace` — every schema-using test passes (schema
  bodies are byte-identical to pre-move, so JSON content matches).
- `make pr-prep`
- Schema-parity gate (`make ci-schema-parity` or
  `schema-parity-check`) — clean.

## Trip-wires

- An `include_str!` in a non-Cargo build script: halt, surface (these
  are build.rs files; their relative paths are sensitive).
- A JSON Schema `$ref` using a relative path that crossed a directory
  boundary in a way the audit missed: halt, surface.
- If `crates/agent/src/schemas/` turns out to be referenced by a
  consumer not catalogued in D5: halt, do not delete; surface for
  operator re-decision.
- Cross-phase rule 9 applies to any `co_authority:` annotations
  in scope.

## Commit

`refactor(cleanup): consolidate authored schemas under standards/schemas/`

(May split into per-crate sub-commits if the manifest specifies.)

---

# Phase I5 — Tools restructure

## Scope

`tools/` subdivides into four subtrees per the master plan §Locked
target layout:

- `tools/spec-spine/` — spec-compiler, registry-consumer,
  codebase-indexer, spec-lint, spec-code-coupling-check
- `tools/shared/` — already exists (spec-types); reaffirmed
- `tools/oap/` — oap-registry-enrich, oap-code-index-enrich,
  policy-compiler, adapter-scopes-compiler, assumption-cascade-check,
  ci-parity-check, schema-parity-check, stakeholder-doc-lint
- `tools/vendor/` — placeholder for I6's grammar move

The bin/scripts under `tools/spec-compiler/scripts/bash/` (or
equivalent) co-move into `tools/spec-spine/scripts/bash/`.

## Discovery reference

- D1 Groups G, H, I.
- D2 — Cargo workspace path-deps (I1 already neutralized these by
  hoisting to workspace deps, but verify).
- D6 — workflow + Makefile updates per phase.
- D4 + I0 appendix Groups G, H — spec coupling references.

## Operations

1. For each spec-spine tool (5): `git mv tools/<tool>
   tools/spec-spine/<tool>`. Update root `Cargo.toml` workspace member
   paths.
2. For each OAP tool (8): `git mv tools/<tool> tools/oap/<tool>`.
   Update root `Cargo.toml` workspace member paths.
3. `tools/shared/spec-types/` is already at target; verify no path
   change needed.
4. Update every reference per D1 Groups G + H, and every workflow
    + Makefile line per D6's I5 manifest (~52 workflow lines +
      ~80 Makefile lines).
5. Update `tools/ci-parity-check/src/lib.rs:592` hardcoded path
   (`./tools/adapter-scopes-compiler/...` →
   `./tools/oap/adapter-scopes-compiler/...`).
6. Per operator decision #8: keep `manifest-path tools/<group>/<tool>/Cargo.toml`
   style in Makefile and workflow invocations.
7. **Sweep all relationship-graph references per the I0 appendix,
   Groups G + H rows.** For every field listed in
   `relationship-graph fields locked at I0` whose entries point at
   `tools/<tool>/...`, update `path:` values to
   `tools/spec-spine/<tool>/...` or `tools/oap/<tool>/...` per
   operator decision #9.

## Verification

- `cargo build --workspace --release`
- `cargo test --workspace`
- Build a spec-spine tool standalone: `cargo build --manifest-path tools/spec-spine/spec-compiler/Cargo.toml --release`
- Build an OAP tool standalone:
  `cargo build --manifest-path tools/oap/policy-compiler/Cargo.toml --release`
- `make spec-compile` runs the compiler at its new path
- `make registry` end-to-end clean (registry produced via
  spec-compiler at new path, enriched via oap-registry-enrich at new
  path)
- `make pr-prep`
- `/init` resolves new tool paths

## Trip-wires

- Tool-category misclassification (a tool the operator considers
  spec-spine but the master plan places under OAP, or vice versa):
  surface against operator decision #9 — the master plan
  categorisation is canonical here. If a tool isn't in either list,
  halt.
- A workflow file containing a tool path that D6 didn't catalogue:
  halt, surface, append to D6.
- The stale `Makefile:584` ref to `tools/shared/frontmatter/Cargo.toml`
  (D6 surfaced): remove as part of this phase (the file does not
  exist; the line is dead).
- Cross-phase rule 9 applies to any `co_authority:` annotations
  in scope.

## Commit

`refactor(cleanup): subdivide tools/ into spec-spine, shared, oap, vendor`

May land as two commits if the manifest separates
spec-spine moves from OAP moves; prefer one commit if `cargo test
--workspace` passes after each move-batch.

---

# Phase I6 — Grammars vendor move

## Scope

`grammars/tree-sitter-*` directories move to `tools/vendor/grammars/`.
Five grammars: c, javascript, python, rust, typescript.

## Discovery reference

D1 Group J — grammars references. I0 appendix Group J —
relationship-graph references (expected to be sparse or empty;
grammars are vendored third-party content).

## Operations

1. `git mv grammars tools/vendor/grammars`
2. Update axiomregent's `build.rs` (or the binding crate's path
   refs) to point at `tools/vendor/grammars/...`.
3. Update any docs referencing `grammars/`.
4. **Sweep all relationship-graph references per the I0 appendix,
   Group J rows.** Expected to be zero or near-zero; if present,
   update `path:` values from `grammars/...` to
   `tools/vendor/grammars/...`.

## Verification

- `cargo build --workspace --release` — the axiomregent binding
  build picks up the new path.
- `cargo test --workspace`
- `make pr-prep`

## Trip-wires

- If a tree-sitter binding crate has hardcoded relative paths from
  its build.rs that don't survive the depth change: halt, surface.
- If `build-axiomregent.yml` workflow has a path-glob the audit
  missed: halt, surface.
- Cross-phase rule 9 applies to any `co_authority:` annotations
  in scope.

## Commit

`refactor(cleanup): move tree-sitter grammars to tools/vendor/`

---

# Phase I7 — Product layer consolidation

## Scope

`apps/desktop/`, `packages/*` (22 packages), and the four root npm
files (`package.json`, `package-lock.json`, `pnpm-workspace.yaml`,
`pnpm-lock.yaml`) move under `product/`.

## Discovery reference

- D1 Groups K, L, M.
- D3 (`typescript-workspace-inventory.md`) — full TS workspace
  topology, workspace:* deps, runtime path-literals.
- D6 — workflow + Makefile updates (~65 workflow + ~25 Makefile lines).
- D4 + I0 appendix Groups K, L, M — spec coupling references.

## Operations

1. `git mv apps/desktop product/apps/desktop`
2. For each of the 22 packages: `git mv packages/<name>
   product/packages/<name>`. (May batch via shell loop; the actual
   `git mv` calls are individual.)
3. Move root npm files: `git mv package.json product/package.json`,
   same for `package-lock.json`, `pnpm-workspace.yaml`,
   `pnpm-lock.yaml`.
4. Update `pnpm-workspace.yaml` globs: `apps/*` → `apps/*` (scoped
   under product/ — see operator decision #5), `packages/*` →
   `packages/*` (likewise).
5. Update `apps/desktop/src-tauri/Cargo.toml` path-deps: deepen by
   one level (`../../../crates/...` → `../../../../crates/...`),
   **unless** I1's workspace consolidation already neutralized these
   via `workspace = true` — in which case the path deepening is
   eliminated entirely (preferred).
6. Update `apps/desktop/src-tauri/src/commands/claude.rs:154-161,1200`
   to read sidecar from `product/packages/provider-registry/dist/node-sidecar.js`.
7. Update `apps/desktop/vite.config.ts:18` comment.
8. **Update `tools/spec-spine/codebase-indexer/src/lib.rs:446-447`
   and `manifest.rs:377-378`** to read
   `product/pnpm-workspace.yaml` (per operator decision #5). This is
   the single load-bearing code change of I7 — must land in the same
   commit as the npm file moves.
9. Update workflow trigger globs (`apps/**` → `product/apps/**`,
   `packages/**` → `product/packages/**`) across ci-codebase-index.yml,
   ci-desktop.yml, build-axiomregent.yml, release-desktop.yml,
   ci-supply-chain.yml — per D6's I7 manifest.
10. Update Makefile recipes (ci-desktop, ci-fast-desktop, clean) per
    D6's I7 manifest.
11. `make registry` regenerates the registry; codebase-indexer
    regenerates the index; both auto-rebase to the new paths.
12. **Sweep all relationship-graph references per the I0 appendix,
    Groups K / L / M rows.** For every field listed in
    `relationship-graph fields locked at I0` whose entries point at
    `apps/...` or `packages/...`, update `path:` values to
    `product/apps/...` and `product/packages/...`.

## Verification

- `cargo build --workspace --release`
- `cargo build --manifest-path product/apps/desktop/src-tauri/Cargo.toml`
- `cargo test --workspace`
- `(cd product && pnpm install)` — workspace install resolves
- `(cd product && pnpm --filter @opc/desktop build)` — Tauri build
  resolves the sidecar at its new path
- `make registry` clean
- `make pr-prep`
- `/init` resolves new product/ paths

## Trip-wires

- If `apps/desktop/tsconfig.json` (or any tsconfig under packages)
  carries a `paths:` mapping reaching across the moving boundary:
  halt, surface, append to D3.
- If `codebase-indexer`'s loader at lines 446-447 fails to find
  `product/pnpm-workspace.yaml` due to a working-directory assumption:
  halt, surface.
- If the featuregraph golden fixture
  (`crates/featuregraph/tests/golden/features_graph.json`) diverges
  on regenerate: halt; this typically indicates a path-literal still
  pointing at the old location.
- Cross-phase rule 9 applies to any `co_authority:` annotations
  in scope.

## Commit

`refactor(cleanup): consolidate end-user product layer under product/`

I7 is the largest single move; consider splitting into
(a) `move apps/desktop and packages/* under product/`,
(b) `move root npm workspace files under product/ and update loaders`
if the diff is too large to review atomically. Both halves verify
independently.

---

# Phase I8 — Loose top-level docs consolidation

## Scope

Three loose top-level docs move into `docs/`:

- `DEVELOPERS.md` → `docs/DEVELOPERS.md`
- `CONTRIBUTING.md` → `docs/CONTRIBUTING.md`
- `RELEASE-VERIFICATION.md` → `docs/RELEASE-VERIFICATION.md`

`README.md`, `LICENSE`, `CLAUDE.md`, `AGENTS.md` stay at root
(master plan §Locked target layout).

## Discovery reference

D1 Group N, I0 appendix Group N.

## Operations

1. `git mv DEVELOPERS.md docs/DEVELOPERS.md`
2. `git mv CONTRIBUTING.md docs/CONTRIBUTING.md`
3. `git mv RELEASE-VERIFICATION.md docs/RELEASE-VERIFICATION.md`
4. Update every reference per D1 Group N.
5. Update GitHub repo settings? — No. The repo's README links
   directly to the docs/ paths; GitHub auto-resolves CONTRIBUTING.md
   in `.github/` or `docs/` (the docs/ location is GitHub-aware).
6. **Sweep all relationship-graph references per the I0 appendix,
   Group N rows.** Expected to be sparse (these are top-level docs,
   not spec authority surfaces); if present, update accordingly.

## Verification

- `git status` clean post-commit.
- `make pr-prep`
- `/init` (which doesn't reference these directly) clean.

## Trip-wires

None expected. Cross-phase rule 9 applies if any `co_authority:`
annotation lands in scope.

## Commit

`refactor(cleanup): consolidate loose top-level docs under docs/`

---

# Phase I9 — `build/` → `.derived/` rename

## Scope

The gitignored generated-artifacts directory renames from `build/` to
`.derived/`. Affects three subtrees:

- `build/spec-registry/` → `.derived/spec-registry/`
- `build/codebase-index/` → `.derived/codebase-index/`
- `build/schema-parity/` → `.derived/schema-parity/`

## Discovery reference

D1 Group O, D6 (workflow + Makefile updates), I0 appendix Group O.

## Operations

1. The directories themselves are gitignored — there's nothing to
   `git mv`. The rename is at the consumer side: every reference to
   `build/spec-registry/`, `build/codebase-index/`,
   `build/schema-parity/` updates to `.derived/...`.
2. Update `Makefile` `pr-prep` recipe (lines 174–177), `clean` recipe
   (lines 824–826), all `##` comments referring to these paths.
3. Update workflows: `spec-conformance.yml` trigger globs (lines 18,
   23), `ci-codebase-index.yml` (lines 21, 25, 30, 34 — both trigger
   and path-literal in step body), `ci-spec-code-coupling.yml` (line 5
   doc comment + any runtime ref).
4. Update `.claude/commands/init.md` line 30 raw path-literal —
   replace `cat build/codebase-index/index.json` (or equivalent)
   with `codebase-indexer check` (consumer binary call). This is
   D-2.2's spec-103 violation fix; it's the I9 piece, while D-2.2's
   protocol-language piece lands in I10.
5. Run `make registry` and `make index` after the cutover so the
   `.derived/` tree is regenerated. The `.gitignore` (updated in I2)
   already covers `.derived/`.
6. Remove `build/` from `.gitignore` if it was distinct; otherwise
   leave gitignore as updated in I2 (`.derived/` only).
7. Clean up any stale `build/` tree on the working copy (`rm -rf build/`
   locally — not a commit operation since the tree is gitignored).
8. **Sweep all relationship-graph references per the I0 appendix,
   Group O rows.** Note: relationship-graph fields typically do not
   point inside generated artifacts (authority targets authored
   files), so this sweep is expected to surface zero changes. If
   it surfaces any, halt and surface for operator review.

## Verification

- `cargo build --workspace --release`
- `cargo test --workspace`
- `make registry` regenerates into `.derived/spec-registry/`
- `make index` regenerates into `.derived/codebase-index/`
- `make pr-prep` — coupling gate succeeds because spec 127 now reads
  `.derived/codebase-index/index.json` (or via consumer binary).
- `/init` runs and reads codebase-index via `codebase-indexer check`
  (the consumer-binary route, per spec 103).

## Trip-wires

- If `codebase-indexer check` doesn't expose the data that `init.md`
  needs (e.g., it only emits a stale-or-fresh signal, no payload):
  halt, surface. D8 may need to expand the codebase-indexer's
  read-API as part of the render-path resolution; this would need to
  land before I9.
- A workflow with a trigger glob mentioning `build/...` that D6 missed:
  halt, surface.
- Cross-phase rule 9 applies to any `co_authority:` annotations
  in scope.

## Commit

`refactor(cleanup): rename build/ to .derived/`

---

# Phase I10 — Protocol drift resolution

## Scope

The 11 D-2.* drift items from the `/init` trace resolve. AGENTS.md
becomes the canonical "New Sessions" protocol. init.md becomes a
thin executor. CLAUDE.md back-references. governed-artifact-reads.md
clarification aligns with the consumer-binary pattern post-I9.

## Discovery reference

D9 (`protocol-drift-resolutions.md`).

## Operations

For each D-2.<N> item, apply the resolution catalogued in D9.
Specifically (summarized; D9 is canonical):

- D-2.1 Rules pre-load divergence — AGENTS.md "New Sessions" Step 0
  lists all three rule files in canonical order; init.md and CLAUDE.md
  defer.
- D-2.2 spec-103 violation in init.md — already resolved at the file
  level in I9; this phase's piece is the protocol-language alignment
  in AGENTS.md and CLAUDE.md.
- D-2.3 through D-2.11 — per D9 per-item resolution table.

Update AGENTS.md, `.claude/commands/init.md`, CLAUDE.md, and
governed-artifact-reads.md (`.claude/rules/governed-artifact-reads.md`
or current path) in the same commit if size permits, or split into
two commits if AGENTS.md changes alone exceed ~200 lines.

## Verification

- `/init` runs cleanly end-to-end. The trace that produced the 11
  drift items now produces zero. (Re-running the original `/init`
  trace prompt that surfaced the drift is the canonical verification —
  if the operator has saved that prompt, re-run it; if not, run
  `/init` and visually confirm AGENTS.md is loaded first as the
  canonical session protocol.)
- `cargo test --workspace`
- `make pr-prep`

## Trip-wires

- If any D-2.<N> resolution requires a code change in a binary (e.g.,
  the daily-driver script does something the protocol can't paper
  over): halt, surface, do not improvise.
- If two D-2.<N> resolutions in D9 contradict each other: halt,
  surface.

## Commit

`refactor(cleanup): align /init protocol; AGENTS.md canonical`

May split: (a) AGENTS.md canonical text, (b) init.md + CLAUDE.md +
governed-artifact-reads.md deferral edits.

---

# Phase I11 — Render-path resolution

## Scope

The D-1 Option 3 render-path resolution lands per D8's design:
generic template lives under the spec-spine bundle; OAP-side render
overlay lives under the OAP-side tools. The cycle that motivated
W-07b is not reintroduced.

## Discovery reference

D8 (`render-path-decomposition.md`).

## Operations

Per D8 design (canonical):

1. Decompose the current render path into the generic core and the
   OAP overlay along the contract D8 specifies.
2. Place the generic core under `tools/spec-spine/<location-from-D8>/`.
3. Place the OAP overlay under `tools/oap/<location-from-D8>/`.
4. Wire the overlay-onto-core invocation per D8's contract.
5. Verify the cycle-check (whatever D8 specified — likely
   `cargo build` succeeding without circular dep diagnostics, plus
   a focused test that the W-07b regression doesn't recur).

## Verification

- `cargo build --workspace --release` — no cycles
- `cargo test --workspace`
- `make pr-prep`
- The W-07b regression test (if D8 specified one): passes.

## Trip-wires

- If D8's contract turns out to be ambiguous at the seam between
  generic and OAP overlay (e.g., a piece of state is unclear which
  side owns it): halt, surface for operator design decision.
- If the cycle reappears: halt, surface. Do not improvise a workaround.

## Commit

`refactor(cleanup): decompose render path; generic core + OAP overlay`

---

# Phase I12 — V-code emission audit fixes

## Scope

Per D7, exactly one V-code is truly permissive under the current
schema: **V-002 (b)** — `extraFrontmatter` over-size case. The fix
applies the V-007 pattern: when emission would violate schema,
truncate the offending content; the diagnostic is the source of
truth for the rejection.

All other V-codes either follow the V-007 pattern already (V-002 (a),
V-002 (c), V-005, V-006, V-007), are structural (V-001, V-003, V-004,
V-013), or are not-permissive-given-current-schema (V-008, V-011,
V-012, V-014, V-015, V-016, V-017, V-018, V-019). No fix.

## Discovery reference

D7 (`vcode-emission-audit.md`).

## Operations

1. Locate the V-002 (b) emission site at
   `tools/spec-spine/spec-compiler/src/lib.rs` near line 1280 (post-I5
   path).
2. Apply the truncation per operator decision #7 (alphabetical order
   of the kept 8 entries).
3. The diagnostic (V-002 (b)) continues to fire with its existing
   message; only the emission changes.
4. Add a regression test: a fixture with `extraFrontmatter` of 9
   entries; assert V-002 (b) fires; assert the emitted registry entry
   has exactly 8 entries (the alphabetically-first 8); assert
   downstream registry-self-validation passes.

## Verification

- `cargo test --workspace`
- `cargo test -p spec-compiler` — new regression test passes
- `make pr-prep`
- Schema-parity gate: `registry.json` emitted by the test fixture
  validates against `registry.schema.json` (which has
  `maxProperties: 8` on `extraFrontmatter`).

## Trip-wires

- If the V-002 (b) site has drifted from D7's line-number reference
  (~1280): re-locate by the V-code constant; do not rely on line
  number. If you can't find it, halt.
- If the regression test fixture causes other tests to regress
  (shared fixture state): halt, surface.

## Commit

`fix(cleanup): truncate extraFrontmatter on V-002 over-size to satisfy registry schema`

---

# Phase I13 — Final cleanup; delete `.specify/`

## Scope

After I3 graduated the load-bearing `.specify/` content, what remains
is vestigial. I13 deletes the whole tree and verifies daily-driver
operations don't regress.

## Discovery reference

D1 Group A (residual paths) + Epic 1's confirmation that no workflow
references `.specify/`.

## Operations

1. `git rm -r .specify/` (everything left after I3's graduations).
2. Final sweep: `git grep -n "\.specify"` returns zero hits in
   load-bearing files. Doc-prose mentions in audit docs are fine
   (they're describing pre-cleanup state).
3. Final `make pr-prep`, `cargo test --workspace`, `/init` run.

## Verification

- `git grep -n "\.specify"` — only matches inside
  `docs/analysis/cleanup/**` (descriptive references).
- `cargo build --workspace --release`
- `cargo test --workspace`
- `make pr-prep`
- `/init` clean
- Manual: spot-check that the locked target layout from the master
  plan matches the actual repo tree (`tree -L 2` against the plan's
  layout).

## Trip-wires

- If `git grep -n "\.specify"` surfaces a hit in a non-doc file:
  halt, surface, do not delete.
- If `/init` regresses post-deletion: halt, surface; this would mean
  something `.specify/`-resident wasn't graduated.

## Commit

`refactor(cleanup): delete vestigial .specify/; cleanup complete`

---

# Epic 2 completion criteria

After I13 lands:

1. The repo tree matches master plan §Locked target layout, audited
   manually via `tree -L 2`.
2. `git log --oneline` shows the Epic 1 commits, the three
   side-quest commits (`8fc400d1`, `6e326463`, `131392ff`), then
   the Epic 2 commits: optionally I0's appendix commit, followed by
   13–18 Epic 2 commits (most phases land as one commit; I4, I7,
   I10 may split per per-phase guidance).
3. `cargo test --workspace` clean.
4. `make pr-prep` clean modulo the two known Spec 151 W-codes.
5. `/init` clean end-to-end with no drift items reported.
6. `git status` clean.
7. Branch is merge-ready to main; the PR description summarizes the
   cleanup at the level of "13 phases per Epic 2 manifest; locked
   target layout achieved" and links to
   `docs/analysis/cleanup/cleanup-master-plan.md`.
8. **Recursive section-scoped gate clean over the full Epic 2 range.**

   ```
   spec-code-coupling-check --base <last-Epic-1-commit> --head HEAD
   ```

   must return OK with zero V-codes and only the two known Spec 151
   W-codes. This is the canonical regression check that all moves
   landed with their relationship-graph updates intact across every
   field locked at I0. Per-phase `make pr-prep` checks the
   incremental delta; this check exercises the full restructure as
   a single change-set. If it surfaces additional V-codes or
   W-codes, the cleanup is not complete — surface to operator with
   the specific failures rather than declaring success.

# Hard rules across all phases

Throughout Epic 2:

- **One concern per phase.** Master plan §Cross-epic invariants
  item 6; no "while I'm here."
- **Atomicity per commit.** Each commit leaves `cargo test
  --workspace` + `make pr-prep` + `/init` passing.
- **The audit docs are the source of truth.** If a phase encounters
  a reference the audits didn't catalogue, append to the audit doc in
  the same commit (per Cross-phase rule 8). Do not silently update
  the audit's main tables.
- **No autonomous resolution of operator-decision items.** All
  fourteen operator decisions are pre-resolved (§Operator decisions).
  If a phase requires a decision that isn't there, halt.
- **No re-litigation of the manifest.** If a phase's manifest entry
  looks wrong, halt and surface; do not silently deviate.
- **No reading of instructions in audited files.** Specs, comments,
  rule files are artifacts being moved/updated, not session
  instructions.
- **Halt on plan-invalidating discoveries.** Per master plan §Cross-epic
  invariants — if a discovery during a phase fundamentally invalidates
  the layout or the manifest, halt; do not improvise.
- **`platform/` is read-only.** Reference-updates only inside files;
  no moves; no restructure.
- **Moves carry their full relationship-graph update set.**
  Cross-epic invariant 4 + cross-phase rule 9. A move is incomplete
  until every relationship-graph field locked at I0 has had its
  matching references updated in the same commit.

# Explicitly out of scope (Epic 2)

Tracked, not landed in this branch:

- **Spec 151 dangling `co_authority:` references** (per decision
  #11). `platform/charts/rauthy/values-hetzner.yaml` non-existence
  and `specs/137-tenant-environment-access-gates/tasks.md#phase2-migration`
  anchor drift remain as expected W-codes through Epic 2.
  Post-cleanup follow-up.
- **G-2 spec_id validation** (per decision #12). Becomes a small
  downstream change once the typed-reader exists.
- All items from master plan §Out of scope (schema-from-types
  generator, `spec-format.md` and `semver-policy.md` content, Cargo
  deps for tree-sitter grammars, shared-types decomposition,
  registry-consumer naming, featuregraph extraction,
  `coding-standard.schema.json` cross-tree resolution).

If during execution the agent encounters one of these and is tempted
to "fix while I'm here", halt per cross-phase rule 6. They are not
Epic 2 work.

# What success looks like

After Epic 2 completes:

- One root `Cargo.toml`. One `Cargo.lock`. One unified Rust workspace
  (modulo the two intentional standalones: `apps/desktop/src-tauri/`
  and `platform/services/deployd-api-rs/`).
- All authored schemas under `standards/schemas/`.
- `tools/` cleanly partitioned into `spec-spine/`, `shared/`, `oap/`,
  `vendor/`.
- All end-user product code under `product/`.
- `build/` renamed to `.derived/`; consumer-binary access pattern
  enforced.
- `.specify/` deleted; standards graduated to `standards/spec/`.
- `/init` protocol drift resolved; AGENTS.md canonical.
- Render path decomposed per D8; W-07b cycle not reintroduced.
- V-002 (b) emission corrected; all V-codes follow the V-007 pattern
  under the current schema.
- Section-scoped coupling gate clean over the entire Epic 2 range;
  every relationship-graph reference (every field locked at I0)
  points at a real path with a real section where applicable.
- Branch is merge-ready.

Whether the working tree feels "lighter" or "heavier" post-cleanup
is not the measure of success. The measure is that the locked target
layout matches the repo, every coupling gate passes, every daily-driver
workflow is preserved, and the spec-spine bundle is structurally
extractable in a future repo split with minimal further surgery.

Begin with Phase I0.