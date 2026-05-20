# Spec-Spine Cut D — Gap Fixes

## What this is

Five small fixes that close the gaps the verification pass surfaced
(`docs/analysis/spec-spine-cut-d-verification.md`). After these land,
the cut-d branch is merge-ready.

This is **not** an autonomous run. Each fix is small, mechanical, and
reviewable. Land them one per commit, in the order below. You can
stop and check in at any point.

## Pre-conditions

- Branch: `cut-d/autonomous-run-20260519-025506`. Same branch the
  verification ran against. Tip should be `cc7dd80b`
  (`chore(cut-d): refresh derived artifacts post-W-07c`).
- The verification report is the source of truth for what to fix.
  Cite it in commit messages where helpful.
- Hard rule unchanged from prior sessions: do not modify any of the
  16 W-unit commits. Do not push the branch. Do not modify `main`.
  Every fix is a new commit on top.

## The five fixes

### Fix 1 — axiomregent test fixture (the blocking defect)

**File:** `crates/axiomregent/tests/mcp_featuregraph_test.rs`,
lines 23–25.

**Change:** the test fixture writes a registry without `specVersion`.
Add `"specVersion":"1.5.0"` to the JSON literal.

**Verify after:** `cargo test -p axiomregent --test mcp_featuregraph_test`
must pass both `test_features_impact` and `test_gov_drift`.

**Commit message:**
```
fix(cut-d): add specVersion to axiomregent test fixture

W-05 routed featuregraph's registry parser through
spec_registry_reader, which requires a non-empty specVersion. The
W-05 commit body updated featuregraph's own fixtures but did not
audit downstream consumers; this fixture in axiomregent was missed,
producing two silent test regressions
(test_features_impact, test_gov_drift) that the autonomous run
report did not disclose.

Verification reference: docs/analysis/spec-spine-cut-d-verification.md
§Q2 "Silent loss" and §Failures item 1.
```

### Fix 2 — audit for other missing-specVersion fixtures

**Scope:** the verification found one site. Make sure it's the only
one.

**Method:**
```
git grep -n 'registry.json' -- '*.rs'
git grep -n '"features"' -- '*.rs'
```
Look for any other test that constructs a `registry.json` content
without a `specVersion` field. For each hit, decide:

- **Test code writes a registry literal:** apply the same fix as
  Fix 1 (add `"specVersion":"1.5.0"`).
- **Test reads a real fixture file on disk:** verify the fixture
  file under `tests/fixtures/` has `specVersion`.
- **Non-test reference:** ignore.

If you find zero additional sites, commit a brief note (see
commit message template); the audit itself is the deliverable.
If you find one or more sites, fix them in a single commit
following Fix 1's pattern.

**Verify after:** `cargo test --workspace` (or per-crate equivalents
as the verification report describes). The only remaining test
failure should be `featuregraph::tests::golden::test_golden_graph`,
which is pre-existing and addressed by Fix 4.

**Commit message (if additional fixes found):**
```
fix(cut-d): add specVersion to N additional test fixtures

Audit per docs/analysis/spec-spine-cut-d-verification.md §Failures
item 1. Found <N> additional sites where test registries were
constructed without specVersion. Same one-line fix applied.

Sites:
- <file:line>
- <file:line>
```

**Commit message (if no additional fixes found):**
```
chore(cut-d): audit for missing-specVersion test fixtures

Per docs/analysis/spec-spine-cut-d-verification.md §Failures item 1,
audited the codebase for additional test fixtures constructing
registries without specVersion. Found none beyond the
axiomregent fixture fixed in the previous commit.

Method: `git grep -n '"features"' -- '*.rs'` and inspection of
each result for registry-literal construction.
```

### Fix 3 — wire validate_spec_id_resolution into call sites

**Files:**
- `crates/factory-engine/src/bin/build_certificate.rs:110`
- `crates/factory-engine/src/bin/factory_run.rs:52`
- `crates/factory-engine/src/bin/verify_certificate.rs:49`

**Change:** thread `repo_root` through each binary's cert generation
or verification path. After `generate_certificate` / `verify_certificate`
runs, call `validate_spec_id_resolution`, then
`write_validation_warnings` to emit the sibling JSON file.

**Pattern (illustrative — adapt to each binary's actual signature):**
```rust
let cert = generate_certificate(...)?;
let warnings = validate_spec_id_resolution(&cert, &repo_root)?;
if !warnings.is_empty() {
    write_validation_warnings(&warnings, &output_dir)?;
}
```

**Verify after:**
- `cargo test -p factory-engine`: still passes.
- Manual smoke: build each binary with `cargo build --release
  --manifest-path crates/factory-engine/Cargo.toml`, run with a
  test cert that has a known-bad `intent.spec_id`, confirm a
  `validation-warnings.json` is written next to the cert.

**Commit message:**
```
fix(cut-d): wire validate_spec_id_resolution into cert callers (W-10)

W-10 introduced validate_spec_id_resolution and write_validation_warnings
as public helpers but did not thread them into the three cert-emitting/
verifying binaries. This closes the gap.

Verification reference: docs/analysis/spec-spine-cut-d-verification.md
§Q3c "validate_spec_id_resolution not wired to callers."

The run-report named factory-harness as the third site; the actual
third site is verify-certificate (also corrected in the next commit).

Sites:
- crates/factory-engine/src/bin/build_certificate.rs (generate_certificate caller)
- crates/factory-engine/src/bin/factory_run.rs (generate_certificate caller)
- crates/factory-engine/src/bin/verify_certificate.rs (verify_certificate caller)
```

### Fix 4 — refresh the featuregraph golden

**File:** `crates/featuregraph/tests/golden/features_graph.json`
(or whatever path the test references for the stored fingerprint).

**Change:** the stored golden fingerprint is stale relative to the
live spec corpus. The verification report identifies the empirical
actual fingerprint as
`sha256:febf01350aa6bcafe421aa95fbb36df6fe6ca0fb4ed52da82a05d1551e96ed43`
(this is the scanner's current output). The stored value
`sha256:15ca2c6d7cf649cb2b389eb888f7c782bdb91532aa92705dce983d830c802868`
is the old one.

**Method:** regenerate the golden with the test's own update
mechanism if one exists (often via an env-var like
`UPDATE_GOLDEN=1 cargo test ...`). If no update mechanism exists,
re-run the test, capture the actual output, and overwrite the
golden file.

**Verify after:** `cargo test -p featuregraph --test golden
test_golden_graph` passes.

**Commit message:**
```
chore(cut-d): refresh featuregraph golden fingerprint

The stored golden fingerprint in features_graph.json predates the
current spec corpus and was already stale on main at b41c02e7.
Cut D's W-05 preserved impl_files() semantics byte-for-byte
(scanner output unchanged pre- and post-W-05), so the regression
is corpus drift, not Cut D drift.

Verification reference: docs/analysis/spec-spine-cut-d-verification.md
§Q3b "featuregraph golden — pre-existing failure".

Empirically observed scanner fingerprint, now stored:
sha256:febf01350aa6bcafe421aa95fbb36df6fe6ca0fb4ed52da82a05d1551e96ed43
```

### Fix 5 — correct disclosure errors in run report and W-05 commit body

The run-report and one commit body have three documentation defects
the verification surfaced. These are corrections to historical
artifacts; the choice is whether to amend or to add a corrigendum.

**Hard rule:** the 16 W-unit commits cannot be amended. That includes
the W-05 commit. So the corrections land as a new doc commit, not as
history rewrites.

**Files to update:**

1. `docs/analysis/spec-spine-cut-d-run-report.md` — three errors:
   - The fingerprint labels are inverted (run-report says
     `febf01350aa6...` is the golden file's value and `15ca2c6d...`
     is the scanner's; empirically the reverse). Fix by swapping
     the labels.
   - The Q3c-equivalent disclosure names `factory-harness` as one
     of the three binaries; the third binary is actually
     `verify-certificate`. Fix the binary list.
   - The KNOWN_KEYS factoring is described as "permitted frontmatter
     allowlist distinct from emitted fields"; that's accurate, but
     the run-report frames it as a deviation while the realized
     factoring is more defensible than that framing implies. Soften
     to "deviation from plan letter, refined factoring documented
     in-code."

2. Add a new file
   `docs/analysis/spec-spine-cut-d-run-report-corrigendum.md`
   that documents the W-05 commit body's fingerprint label inversion.
   The commit itself cannot be amended; the corrigendum is the
   durable record. Three short paragraphs are enough.

**Commit message:**
```
docs(cut-d): correct disclosure errors in run report; add W-05 corrigendum

Three documentation defects identified by the verification pass:

1. Fingerprint labels inverted in the run report (substantive
   claim correct; labels swapped).
2. Run report's third-binary disclosure named factory-harness;
   actual binary is verify-certificate.
3. KNOWN_KEYS factoring framed as deviation; the realized
   factoring is more defensible than that framing.

The W-05 commit body has the same fingerprint label inversion but
cannot be amended (per the no-history-rewrites rule). A new file,
spec-spine-cut-d-run-report-corrigendum.md, captures the W-05
correction.

Verification reference: docs/analysis/spec-spine-cut-d-verification.md
§Notes 1, 2, 3.
```

## End-state

After all five fixes:

- 16 W-unit commits unchanged.
- 1 chore commit at `cc7dd80b` (the verification's derived-artifact
  refresh).
- 5 fix commits on top (or 4 if Fix 2 finds no additional sites and
  is folded into Fix 1's commit — your call).
- `cargo test --workspace` clean.
- Run-report and W-05 corrigendum reflect the empirical truth.
- Branch is merge-ready.

## What this prompt does NOT cover

- Architectural review of the resulting shape. Separate pass.
- Risk-surface review (external consumers, schema 2.0.0 migration
  story, policy-compiler bundle removal impact). Separate pass.
- Merge to main. Operator decision after the other reviews land.

## Hard rules

- Do not modify any of the 16 W-unit commits.
- Do not push the branch. Do not open a PR. Do not modify `main`.
- Do not bundle fixes across commits except where the prompt
  explicitly says so (Fix 1 and Fix 2 may combine if Fix 2 finds
  no additional sites).
- Do not "while I'm here" refactor or modernize.
- Stop and ask if any fix turns out to be more than the mechanical
  change described.

Begin with Fix 1.
