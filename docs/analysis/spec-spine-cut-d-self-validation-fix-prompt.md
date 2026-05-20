# Spec-Spine Cut D — Asymmetric Self-Validation Fix

## What this is

A single targeted fix that closes the asymmetric-self-validation gap
identified by the architectural review (§Q5 grammar locus):

- `codebase-indexer` self-validates its emitted `codebase-index.json`
  against `schemas/codebase-index.schema.json` at compile time via
  `jsonschema`.
- `spec-compiler` does NOT self-validate its emitted `registry.json`
  against `specs/000-bootstrap-spec-system/contracts/registry.schema.json`.
  No `jsonschema` import. The only mention of `registry.schema.json` in
  `tools/spec-compiler/src/lib.rs` is a comment at line 1124.

After this fix, both generic spec-spine producers self-validate every
JSON artifact they emit against the schema that declares its shape.
This is a pre-launch correctness gate: a producer that ships its
schema as a normative reference but does not enforce it on its own
output is unsound at the standard-as-contract level.

This is **not** an autonomous run. The fix is small, mechanical, and
mirrors an existing pattern. Land it as a single commit on top of the
existing cut-d branch. Stop and ask if any step turns out to be more
than the mechanical change described.

## Pre-conditions

- Branch: `cut-d/autonomous-run-20260519-025506`. Confirm via
  `git log --oneline -10`: the protected base (16 W-unit + 1 chore +
  5 fix commits, plus `3612d734` architectural-review-prompt and
  `fef29499` architectural-review-report) must be intact in the
  history. The tip is the docs commit carrying this prompt; one or
  more later docs commits may sit above the architectural-review-prompt
  commit and below the tip, all of them prompt/report artifacts and
  none touching code.

## Pattern to mirror

Read the existing pattern in `codebase-indexer` FIRST. Do not write
any spec-compiler changes until you have a complete picture of how
codebase-indexer does it. The fix is "do the same thing, for
spec-compiler."

**Files to read:**
- `tools/codebase-indexer/Cargo.toml` — which `jsonschema` crate is
  used (there are multiple competing ones; use the same one), at what
  version, with what features.
- `tools/codebase-indexer/src/schema.rs` — the validation module
  structure: how the schema is loaded (compile-time via `include_str!`
  or runtime via file read), how the validator is compiled (once via
  `once_cell` / `LazyLock`, or per-call), what the public API looks
  like (e.g., `validate_against_schema(value: &Value) -> Result<…>`).
- `tools/codebase-indexer/src/lib.rs` around lines 300–304 — the call
  site: where in the emit path validation happens (before write, after
  hash), how errors are propagated (return Err, panic, print diagnostic).

**Capture these answers as your design baseline:**
1. Which `jsonschema` crate + version + features.
2. Schema loaded compile-time or runtime?
3. Validator compiled once or per-call?
4. Failure semantics: fail-fast (return Err) or warn-and-continue?
5. Validation runs before or after `write` to disk?
6. Validation runs before or after `compute_content_hash`?

Spec-compiler's fix must match codebase-indexer's answers point-for-point
unless there is a structural reason to diverge. If you find a reason
to diverge, STOP and surface it before proceeding.

## The fix

### Step 1: Schema currency check (dry run)

Before adding strict validation, verify the schema is in sync with the
emitted registry shape. Run the current spec-compiler against the live
spec corpus, capture the emitted `build/spec-registry/registry.json`,
and validate it manually against `specs/000-bootstrap-spec-system/contracts/registry.schema.json`.

**Method:**
```
make registry   # or: cargo build --release --manifest-path tools/spec-compiler/Cargo.toml \
                #     && ./tools/spec-compiler/target/release/spec-compiler compile

# Validate manually using ajv-cli, jsonschema (python), or a one-off
# Rust binary you can write at /tmp/. Whichever is fastest.
```

**Expected outcomes:**

- **A: Registry validates clean against the schema.** Proceed to
  Step 2.
- **B: Registry validates with N errors.** The schema is stale
  relative to the registry shape — likely candidates: the
  `ImplementsField` polymorphism widened in W-05, the `compliance`
  field removed in W-06c, schema 2.0.0 bump in Cut D. STOP. Surface
  the validation errors and the suspected drift. Do NOT update the
  schema or the registry to make them agree — that is a separate scope
  (schema currency is a real concern but not this fix). Ask the
  operator for direction.

This step exists because adding strict validation on top of a stale
schema would break the build. The asymmetric-self-validation fix is
predicated on a current schema; if the schema is stale, the order is
"update schema first, then add self-validation."

Repeat the same dry-run check for `build/spec-registry/build-meta.json`
against `specs/000-bootstrap-spec-system/contracts/build-meta.schema.json`.

### Step 2: Add the validation module to spec-compiler

**File:** `tools/spec-compiler/Cargo.toml`.

Add `jsonschema` (same crate, version, features as codebase-indexer)
to `[dependencies]`. No other deps.

**File:** `tools/spec-compiler/src/schema.rs` (new file).

Mirror `tools/codebase-indexer/src/schema.rs` line-for-line in
structure. Same loading pattern (compile-time vs runtime), same
LazyLock / once_cell pattern, same public API shape. Two validators:

- `validate_registry_against_schema(value: &Value) -> Result<…>` —
  validates against
  `specs/000-bootstrap-spec-system/contracts/registry.schema.json`.
- `validate_build_meta_against_schema(value: &Value) -> Result<…>` —
  validates against
  `specs/000-bootstrap-spec-system/contracts/build-meta.schema.json`.

If codebase-indexer uses `include_str!` for its schema, do the same:
the schema path resolves at compile time, the binary carries the
schema. If codebase-indexer reads the schema at runtime from a known
filesystem path, do the same.

If codebase-indexer's schema lives at `schemas/` and the registry's
schema lives at `specs/000-…/contracts/`, that location asymmetry is
out of scope for this fix — both schemas stay where they are. The
fix imports each from its current location.

### Step 3: Wire validation into the emit path

**File:** `tools/spec-compiler/src/lib.rs`.

Locate the registry emit path (the function that writes
`registry.json`). Add a validation call between
`compute_content_hash` and the write-to-disk step, matching
codebase-indexer's order from Step 1 capture point 5–6.

Locate the build-meta emit path. Add the equivalent
`validate_build_meta_against_schema` call.

Match codebase-indexer's failure semantics from Step 1 capture point
4: if codebase-indexer fails-fast, spec-compiler fails-fast; if it
warns-and-continues, same.

Remove the comment at lib.rs:1124 that mentions `registry.schema.json`
if it now describes implemented behavior (replace with normal doc
comment) or is now stale (remove). Use judgment; do not add an
"updated" or "TODO removed" comment.

### Step 4: Add tests

**File:** `tools/spec-compiler/tests/schema_conformance.rs` (new file,
or extend an existing test file if `tools/spec-compiler/tests/` has
one named similarly — `ls tools/spec-compiler/tests/` first).

Mirror `tools/codebase-indexer/tests/schema_conformance.rs` (or
equivalent) if it exists. At minimum, two round-trip tests:

1. **`test_emitted_registry_validates`** — compile a small fixture
   corpus (use `tempfile` per spec-compiler's existing test patterns),
   parse the emitted `registry.json`, call
   `validate_registry_against_schema`, assert Ok.
2. **`test_emitted_build_meta_validates`** — same for build-meta.

Drift-detection tests (deliberately mutate output to break validation)
are nice-to-have, not required. If codebase-indexer has them, mirror;
otherwise skip.

### Step 5: Verify

**Build:**
```
cargo build --workspace --release
```
Must succeed without warnings beyond the pre-fix baseline.

**Test:**
```
cargo test --workspace
```
Must pass. Test count must be ≥ baseline by exactly the number of new
tests added in Step 4.

**Smoke:**
```
make registry
# or: ./tools/spec-compiler/target/release/spec-compiler compile
```
Must succeed and produce a registry.json that the spec-compiler's own
validation (now wired in Step 3) accepts. Same for build-meta.json.

**Independent check:** validate the freshly-emitted `registry.json`
against `registry.schema.json` using an external tool (`ajv-cli`,
`check-jsonschema`, etc.), same way you did in Step 1. Output must
agree with the now-built-in validation: both clean.

### Step 6: Commit

Single commit on top of the branch tip. Single concern: spec-compiler
now self-validates registry.json and build-meta.json.

**Commit message:**
```
feat(cut-d): spec-compiler self-validates registry.json and build-meta.json

Closes the asymmetric-self-validation gap identified by the
architectural review (§Q5 grammar locus).

Pre-fix state: codebase-indexer self-validates its emitted
codebase-index.json against schemas/codebase-index.schema.json at
compile time via jsonschema. spec-compiler did not self-validate
registry.json or build-meta.json against their schemas under
specs/000-bootstrap-spec-system/contracts/. The schemas existed as
normative documentation; the producer did not enforce them on its
own output.

Post-fix state: spec-compiler validates both emitted artifacts
against their declared schemas before write. Failure semantics
mirror codebase-indexer's: <fail-fast / warn-and-continue, fill in
from Step 1 capture>.

Schema currency confirmed by Step 1 dry-run: the live corpus's
emitted registry.json and build-meta.json validate clean against
their schemas as of this commit.

Architectural review reference: docs/analysis/spec-spine-cut-d-architectural-review.md
§Q5 grammar locus reservation.
```

If the failure-semantics decision was a divergence from codebase-indexer
(per Step 2's escape hatch), document that divergence in the commit
body with one sentence of rationale.

## What this fix does NOT cover

- **Schema location reorganization.** `registry.schema.json` and
  `build-meta.schema.json` stay under `specs/000-…/contracts/`.
  `codebase-index.schema.json` stays under `schemas/`. The asymmetric
  location is a separate concern flagged in the architectural review
  §Q4 split-readiness.
- **OAP enricher self-validation.** `oap-registry-enrich` emits
  `registry-oap.json`; `oap-code-index-enrich` emits
  `codebase-index-oap.json`. Whether either self-validates against
  `*-oap.schema.json` is out of scope. Surface as a follow-up if you
  notice the same gap exists, but do not fix in this commit.
- **Schema generator from spec-types.** The architectural review's Q5
  also flagged the absence of a generator emitting JSON Schemas from
  the Rust types in `tools/shared/spec-types/`. Out of scope.
- **Schema versioning policy.** The SemVer policy for schema 3.0.0+
  is out of scope.

## End-state

After this fix:
- 16 W-unit commits + 1 chore + 5 fix commits + 1 architectural-review
  prompt commit + 1 NEW self-validation commit on top.
- `cargo test --workspace` clean.
- spec-compiler imports `jsonschema`, owns `src/schema.rs`, validates
  registry.json and build-meta.json on emission.
- The branch remains merge-ready and is in a state where the `/init`
  trace mission can run against a corrected spec-compiler.

## Hard rules

- Single commit. Do not split across multiple commits.
- Do not modify any prior commit on the branch.
- Do not push the branch. Do not open a PR. Do not modify `main`.
- Mirror codebase-indexer's pattern point-for-point. If a structural
  reason forces divergence, STOP and surface it; do not unilaterally
  decide to do it differently.
- Do not change codebase-indexer's existing validation pattern even
  if you spot improvements. This fix is one-directional: bring
  spec-compiler up to codebase-indexer's parity, not the other way
  around.
- Do not move schema files. Do not rename. Do not reorganize
  `schemas/` vs `specs/000-…/contracts/`.
- Do not "while I'm here" refactor or modernize.
- If Step 1 reveals schema-currency drift, STOP. Do not patch the
  schema and do not patch the registry. Surface to the operator and
  wait for direction.
- If the existing codebase-indexer self-validation turns out to be
  silently broken (e.g., never actually runs, or runs but ignores
  errors), STOP and surface. Do not fix codebase-indexer as part of
  this commit; the asymmetric fix presumes the existing pattern is
  load-bearing as documented.

Begin with the pattern read (no edits): `tools/codebase-indexer/Cargo.toml`,
then `tools/codebase-indexer/src/schema.rs`, then the call site in
`tools/codebase-indexer/src/lib.rs`. Capture the six baseline answers
listed in "Pattern to mirror" before any spec-compiler edits.
