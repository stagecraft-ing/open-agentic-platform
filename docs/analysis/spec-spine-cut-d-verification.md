# Spec-Spine Cut D — Verification

> Verifies the branch `cut-d/autonomous-run-20260519-025506` produced by
> the prior autonomous run, against the protocol in
> `docs/analysis/spec-spine-cut-d-verification-prompt.md`. Read-only
> review of the artifact (the branch); the prior run-report is treated
> as a claim to be checked, not a summary to be trusted.
>
> Pre-conditions resolved (single chore commit `cc7dd80b` at tip): the
> `registry-consumer` binary was rebuilt to read `specVersion 2.0.0`,
> and `build/codebase-index/index.json` was recompiled to match the
> post-W-10 input set. No W-unit commit was amended.
>
> Branch tip after the chore commit: `cc7dd80b`.
> Base for diff: `b41c02e7` (main HEAD at session start).
> 16 W-unit commits + 2 docs + 1 chore = 19 commits ahead of main.

## Per-question verdicts

| # | Subject | Verdict |
|---|---------|---------|
| Q1 | Build (workspace debug + release; standalone enrichers) | verified |
| Q2 | Tests (workspace pass, baseline drift, golden test on main) | **failed** |
| Q3a | KNOWN_KEYS `compliance` retention | verified |
| Q3b | featuregraph golden — pre-existing failure on `main` | verified with note |
| Q3c | `validate_spec_id_resolution` deferred wiring | verified with note |
| Q4a | KNOWN_KEYS factoring documented | verified with note |
| Q4b | ImplementsField polymorphism widening | verified |
| Q5 | End-state matches Phase 5 of the plan | verified |

## Verdict

**BLOCKED.** Q2 surfaced two test regressions in
`crates/axiomregent/tests/mcp_featuregraph_test.rs`
(`test_features_impact`, `test_gov_drift`) that pass on `main` at
`b41c02e7` and fail on the branch tip after Cut D. The W-05 commit body
documents the same class of fixture break (typed-reader rejects
`UnknownSchemaVersion("")` for fixtures missing the field) but only
updates featuregraph's own fixtures; the downstream axiomregent
fixture at line 23–25 still writes a registry without `specVersion` and
is now an undocumented silent failure introduced by Cut D. This is
not a pre-existing failure, not a documented deviation, and is not
disclosed in `docs/analysis/spec-spine-cut-d-run-report.md`.

The other findings are either clean (verified) or carry annotations
the operator should read but none are merge-blockers individually.

---

## Q1 — Does it build?

**Verdict: verified.**

Note on protocol deviation: there is no root `Cargo.toml`, so
`cargo clean` cannot be invoked once for "the workspace." Each Rust
workspace (`crates/` plus the per-tool manifests under `tools/`) was
cleaned in turn before rebuild. The bar interpreted from the prompt is:
debug and release build for each Rust workspace + standalone manifest
build for each new binary.

### Commands and results

| Stage | Command (essence) | Time | Warnings | Exit |
|-------|-------------------|------|----------|------|
| crates workspace debug | `cargo build --manifest-path crates/Cargo.toml` | 99 s | 0 | 0 |
| crates workspace release | `cargo build --release --manifest-path crates/Cargo.toml` | 5 m 12 s | 0 | 0 |
| All 13 tool manifests, debug | per-manifest loop | 148 s combined | 0 | 0 |
| All 13 tool manifests, release | per-manifest loop | 349 s combined | 0 | 0 |

Standalone manifest builds for the two new enrichers
(`tools/oap-registry-enrich/Cargo.toml`,
`tools/oap-code-index-enrich/Cargo.toml`) succeeded as part of the
per-manifest loop.

Zero new warning categories versus a pre-Cut-D baseline (the
crates-workspace debug build at `b41c02e7` also produced no warnings —
checked indirectly via the `cargo test` baseline on the worktree below,
which built the same crate set without warnings).

`apps/desktop/src-tauri` and `platform/services/deployd-api-rs` are
outside the spec-spine Cut D scope (the plan does not enumerate them in
Phase 5's crate list) and were not built.

---

## Q2 — Does it test?

**Verdict: failed.**

### Workspace test counts on branch tip (`cc7dd80b`)

Run: `cargo test --manifest-path crates/Cargo.toml --no-fail-fast`.

- **Crates workspace:** 1026 pass, 3 fail, 0 ignored (across 84 test
  binaries that produced result lines).
- The 3 failures are
  `axiomregent::tests::mcp_featuregraph_test::test_features_impact`,
  `axiomregent::tests::mcp_featuregraph_test::test_gov_drift`, and
  `featuregraph::tests::golden::test_golden_graph`.
- Of those three: the first two are new regressions introduced by
  Cut D (see "Silent loss" below). The third is the pre-existing
  failure on `main` documented in the run-report and verified
  independently in §Q3b.
- **All tool tests pass:** 249 pass / 0 fail / 0 ignored, summed
  across 13 tool manifests (spec-compiler, spec-lint,
  registry-consumer, codebase-indexer, policy-compiler,
  spec-code-coupling-check, oap-registry-enrich,
  oap-code-index-enrich, stakeholder-doc-lint,
  assumption-cascade-check, adapter-scopes-compiler,
  ci-parity-check, shared/spec-types).

### Baseline comparison vs `main` at `b41c02e7` (worktree)

Independent run inside a fresh worktree at `b41c02e7`:

- `axiomregent::tests::mcp_featuregraph_test`: **2 pass / 0 fail.**
  Same two tests pass on main; fail on branch tip. Evidence:
  worktree run output shows `test result: ok. 2 passed; 0 failed`
  for that binary at main.
- `featuregraph::tests::golden::test_golden_graph`: **FAIL.**
  Confirmed pre-existing on main, identical actual fingerprint as on
  the branch tip (`sha256:febf01350aa6bcafe421aa95fbb36df6fe6ca0fb4ed52da82a05d1551e96ed43`).
  This is the pre-existing failure the run-report flagged.

### Removed tests, identity and documented justification

1. `tools/codebase-indexer/tests/exit_codes.rs::check_exits_nonzero_on_blocking_diagnostic`
   removed in W-07c (`ebb81abd`). Justification documented in commit
   body and visible as an explicit comment in
   `tools/codebase-indexer/tests/exit_codes.rs:91`
   ("Cut D W-07c: `check_exits_nonzero_on_blocking_diagnostic`
   removed."). I-105 (workflow-without-spec-header) moved to the OAP
   enricher in W-07a; the test's domain moved with it. Mechanism
   preserved: `BLOCKING_DIAGNOSTIC_CODES` remains as an empty slice
   at `tools/codebase-indexer/src/lib.rs:65` (used at line 349). This
   is the documented case.
2. Four `cross_crate_check` tests in `tools/shared/spec-types/`
   (`shape_table_pairs_appear_in_both_duplicates`,
   `known_keys_appear_in_spec_compiler`,
   `valid_kinds_appear_in_spec_compiler`,
   `conventional_categories_appear_in_spec_lint`) added in W-01
   (`3c2890f9`) and removed in W-02 (`0aa6b5be`). Confirmed via
   `git show` on each commit. 7 tests at W-01; 3 frontmatter tests
   remain in W-02 (`splits_required_frontmatter`,
   `missing_frontmatter_returns_err`,
   `optional_returns_none_when_absent`). Matches the run-report
   claim exactly.

### Silent loss (the failure)

`crates/axiomregent/tests/mcp_featuregraph_test.rs:23-25` writes a
test registry without a `specVersion` field:

```rust
std::fs::write(
    registry_dir.join("registry.json"),
    r#"{"features":[{"id":"test-feature", ... }]}"#,
)
```

- The file was **not** modified by any of the 16 W-unit commits
  (verified: `git log --oneline b41c02e7..HEAD -- crates/axiomregent/tests/mcp_featuregraph_test.rs` returns empty).
- On `main` at `b41c02e7`, the test passes because featuregraph's
  pre-W-05 local `CompiledRegistry` parser accepted missing
  `specVersion` (registered "1.0" by default).
- After W-05 (`7ede4d93`), featuregraph's `load_registry_records`
  delegates to `spec_registry_reader::load`. The typed reader's
  schema-version peek in
  `tools/registry-consumer/src/lib.rs::peek_schema_version` (and the
  documented W-05 fixture update in
  `crates/featuregraph/src/registry_source.rs` lines 84-88) returns
  `UnknownSchemaVersion("")` for fixtures missing the field. The
  test failure message is exactly that:
  `Tool failed: unsupported registry specVersion: ""`.
- The W-05 commit body acknowledges this class of fixture break for
  featuregraph's own tests:
  > "Test fixtures in registry_source.rs updated to include explicit
  > `specVersion: '1.5.0'`. Pre-W-05 featuregraph's local parser was
  > permissive (no version dispatch); the typed-reader rejects
  > `UnknownSchemaVersion('')` for fixtures missing the field."
- The downstream consumer (`axiomregent::mcp_featuregraph_test`,
  which goes through `featuregraph` to load the registry) was not
  audited or updated. The run-report does not disclose this regression.

This is two test failures introduced by Cut D, undocumented and not
listed under "Tests not yet wired" or "Three changes most likely to
need reviewer attention" in the run-report. The mechanical fix is
one line in the test fixture (add `"specVersion":"1.5.0"` to the
emitted JSON), but verification does not perform that fix per the
prompt's "Do not fix it." rule.

---

## Q3 — Do the three flagged risk sites hold?

### Q3a — KNOWN_KEYS retention (`tools/shared/spec-types/src/lib.rs:75-115`)

**Verdict: verified.**

- The 8 specs with `^compliance:` frontmatter (via
  `grep -l '^compliance:' specs/*/spec.md`):
  `047-governance-control-plane`, `067-tool-definition-registry`,
  `068-permission-runtime`, `069-lifecycle-hook-runtime`,
  `102-governed-excellence`, `116-supply-chain-policy-gates`,
  `121-claim-provenance-enforcement`, `147-spec-kind-grammar`.
- Trimmed registry: `grep -c '"compliance"' build/spec-registry/registry.json` = 0.
- Enricher overlay: `grep -c '"compliance"' build/spec-registry/registry-oap.json` = 8.
  Matches the 8 specs.
- `tools/spec-compiler/src/lib.rs:1373` (`yaml_scalar_to_json`)
  rejects `Mapping(_) | Tagged(_)` and only accepts scalar
  sequences (`Sequence` with `x.as_str()?` per-element, line 1388).
  The 8 specs carry `compliance` as a list of mappings
  (`- framework: ...; controls: [...]`), which would V-002 on the
  per-element `as_str()` returning `None`. The V-002 emit site is
  `tools/spec-compiler/src/lib.rs:1247-1255`.
- The W-06c commit body (`460f5bde`) defends the deviation with the
  same rationale the run-report gives (full quoted text from the
  commit body in §"Decisions made autonomously"). The rationale lives
  *in code* as a 9-line doc comment on `KNOWN_KEYS` in
  `tools/shared/spec-types/src/lib.rs:65-74` explaining the "permitted
  frontmatter allowlist" vs "fields the compiler emits" factoring.

### Q3b — featuregraph golden — pre-existing failure

**Verdict: verified with note.**

- Reproduced the failure on `main` at `b41c02e7` inside a fresh
  worktree: `git worktree add --detach /tmp/cut-d-main b41c02e7`,
  rebuilt the spec-compiler at main, ran
  `./tools/spec-compiler/target/release/spec-compiler compile`,
  then `cargo test -p featuregraph --test golden test_golden_graph`.
  Result: `FAILED` on main with the same actual fingerprint the
  branch tip produces.
- Actual scanner fingerprint pre- and post-W-05: identical
  (`sha256:febf01350aa6bcafe421aa95fbb36df6fe6ca0fb4ed52da82a05d1551e96ed43`).
  `impl_files()` semantics are byte-preserved.
- Stored golden fingerprint (in `crates/featuregraph/tests/golden/features_graph.json`):
  `sha256:15ca2c6d7cf649cb2b389eb888f7c782bdb91532aa92705dce983d830c802868`.
- **Note: labels inverted in both the run-report and the W-05 commit
  body.** Both documents claim "the golden file's
  `graph_fingerprint` is `febf01350aa6...` but the live spec corpus
  computes `15ca2c6d...`". The empirical evidence is the opposite:
  the golden file contains `15ca2c6d...`, the scanner produces
  `febf01350aa6...`. The substantive claim (pre-existing on main,
  byte-preserved by W-05, golden is stale) is correct; only the
  fingerprint→role labeling is wrong. The commit body's
  stash-and-rerun verification is documented; the labels appear to
  be a transcription slip rather than a verification error.

### Q3c — `validate_spec_id_resolution` not wired to callers

**Verdict: verified with note.**

- `validate_spec_id_resolution` (`crates/factory-engine/src/governance_certificate.rs:759`)
  is `pub`. `write_validation_warnings` (line 788) is `pub`.
- Unit tests at lines 872, 881, 890, 904, 913, 923 — all pass under
  `cargo test -p factory-engine` (counted in the crates workspace
  185-pass total for that binary).
- Grep over `crates/factory-engine/src/`,
  `crates/factory-engine/src/bin/`, `tools/`, `apps/desktop/src-tauri/`:
  no caller outside the test module of `governance_certificate.rs`.
  Confirmed neither `generate_certificate` (line 511) nor
  `verify_certificate` (line 645) calls the W-10 helpers.
- Call sites where wiring would land (binaries declared in
  `crates/factory-engine/Cargo.toml`'s `[[bin]]` entries):
  - `crates/factory-engine/src/bin/build_certificate.rs:110`
    (`generate_certificate`)
  - `crates/factory-engine/src/bin/factory_run.rs:52`
    (`generate_certificate`)
  - `crates/factory-engine/src/bin/verify_certificate.rs:49`
    (`verify_certificate`)
- **Note: run-report's binary list is off by one.** The run-report's
  Q3c disclosure lists "`factory-harness`, `factory-run`,
  `build-certificate`" as the three binaries needing the wiring.
  `factory-harness` (`crates/factory-engine/src/bin/harness.rs`)
  does not directly call `generate_certificate` or
  `verify_certificate`; the third binary is `verify-certificate`,
  not `factory-harness`. The substantive claim (three binaries with
  ~5 LoC per caller for `repo_root` threading + a call + sibling
  write) is plausible; the binary set membership is slightly wrong.

---

## Q4 — Are the autonomous deviations consistent with plan intent?

### Q4a — KNOWN_KEYS retention factoring

**Verdict: verified with note.**

- The "permitted frontmatter allowlist" vs "fields the compiler
  emits" distinction is documented in code as a 9-line `///` doc
  comment on `KNOWN_KEYS` in `tools/shared/spec-types/src/lib.rs:65-74`.
- Full rationale + the 8-spec corpus enumeration lives in the W-06c
  commit body (`460f5bde`).
- **Note:** there is no companion documentation outside the in-code
  comment and the commit body. No `README` update, no `CLAUDE.md`
  rule, no spec amendment explaining the factoring at narrative
  scale. A reader who arrives via `grep KNOWN_KEYS` will find the
  comment; a reader who arrives via "what does the spec spine say
  about `compliance`?" will not. Not a code defect, but the
  documentation surface is thinner than the plan's intent (W-06c
  in Phase 1 simply said "drop `compliance` from `KNOWN_KEYS`" — the
  realized choice is more nuanced and the prose footprint for that
  nuance is small).

### Q4b — ImplementsField polymorphism widened in W-05

**Verdict: verified.**

- `tools/registry-consumer/src/lib.rs:339-367` defines the widened
  enum `ImplementsField::Scalar(String) | List(Vec<Value>)` with
  `paths()` (line 350; Scalar → `Vec::new()`; List → `path` field
  extraction) and `as_scalar()` (line 363; Scalar → `Some(&str)`;
  List → `None`).
- Spec 147 (`specs/147-spec-kind-grammar/spec.md`) at line 26 carries
  `compliance:` as a sequence-of-mappings, confirming the live
  corpus shape that motivated the widening. Spec 147's `implements:`
  contract (the "scalar is a spec-id, list is `{path, primary?}`
  items" semantic) is referenced in
  `tools/spec-compiler/src/lib.rs:1366-1371` (`parse_implements`
  doc comment cites spec 147 and reserves Scalar for
  `kind: capability`, list for path items).
- W-05 commit body documents the semantic correction. The W-03 test
  for `paths()` was updated in W-05 to encode the corrected
  semantic; per the run-report this is the one place a prior PR's
  test changed in a later PR (verified via `git show 7ede4d93 -- tools/registry-consumer/src/lib.rs`).
- Downstream consumers checked:
  - `crates/featuregraph/src/registry_source.rs:54-58` —
    `impl_files` is populated from `paths()`, with an explicit doc
    comment ("Scalar implements: contributes nothing to impl_files
    — those claims are spec-to-spec references, not code paths").
    Aligned with the corrected semantic.
  - `apps/desktop/src-tauri/src/commands/analysis.rs` —
    `srr::load` consumed but the `implements` field is not
    dereferenced as a path anywhere. No scalar-as-path treatment.
  - `crates/axiomregent/src/feature_tools.rs` — consumes
    `FeatureNode.impl_files` (the post-`paths()` form) via
    featuregraph. Receives byte-preserved semantics.
- No downstream consumer is broken by the corrected `paths()`
  semantic. The widening + correction are consistent with the plan
  intent and the spec 147 contract.

---

## Q5 — Does the end-state match Phase 5 of the plan?

**Verdict: verified.**

### Final crate list

All 8 spec-spine + OAP-side crates from Phase 5's table are present:

| Plan position | Path | Cargo `name =` | Present |
|---|---|---|---|
| 1 | `tools/shared/spec-types/` | `open_agentic_spec_types` | yes |
| 2 | `tools/spec-compiler/` | `open_agentic_spec_compiler` | yes |
| 3 | `tools/spec-lint/` | `open_agentic_spec_lint` | yes |
| 4 | `tools/registry-consumer/` | `open_agentic_spec_registry_reader` | yes |
| 5 | `tools/codebase-indexer/` | `open_agentic_codebase_indexer` | yes |
| 6 | `tools/spec-code-coupling-check/` | `open_agentic_spec_code_coupling_check` | yes |
| 7 | `tools/oap-registry-enrich/` | `open_agentic_registry_enrich` | yes |
| 8 | `tools/oap-code-index-enrich/` | `open_agentic_code_index_enrich` | yes |

Adjacent OAP-internal tools (unchanged per the plan):
`tools/policy-compiler/`, `tools/stakeholder-doc-lint/`,
`tools/assumption-cascade-check/`, `tools/adapter-scopes-compiler/`,
`tools/ci-parity-check/`. All present, all build, all test.
`tools/schema-parity-check/` is JavaScript and is correctly not in
the Rust inventory.

`tools/shared/frontmatter/` is deleted, as the plan required (W-01
absorbed its contents into shared-types). Confirmed:
`ls tools/shared/` shows only `spec-types/`.

### Dependency direction

Each spec-spine crate's `[dependencies]`, read from its `Cargo.toml`:

- `tools/shared/spec-types`: `serde`, `serde_yaml`. **Hard leaf**, no
  other crates. Matches plan intent ("depends only on `serde` /
  `serde_yaml`").
- `tools/spec-compiler`: `open_agentic_spec_types` (path),
  `chrono`, `clap`, `serde`, `serde_json`, `serde_yaml`, `walkdir`,
  `jsonschema`, `tempfile`. No OAP-specific dependencies.
- `tools/spec-lint`: `open_agentic_spec_types` (path), `clap`,
  `serde_yaml`, `tempfile`. No OAP-specific.
- `tools/registry-consumer`: `clap`, `serde`, `serde_json`,
  `tempfile`. No OAP-specific (notably no `open_agentic_spec_types`
  dependency — the typed reader is shape-invariant and reuses
  upstream serde definitions).
- `tools/codebase-indexer`: `open_agentic_spec_types` (path),
  `chrono`, `clap`, `jsonschema`, `serde`, `serde_json`,
  `serde_yaml`, `toml`, `walkdir`, `tempfile`. No OAP-specific.
- `tools/spec-code-coupling-check`:
  `open_agentic_codebase_indexer` (path), `clap`, `serde_json`,
  `tempfile`. No OAP-specific.

OAP-side enrichers depend on spec-spine crates (the correct
direction):

- `tools/oap-registry-enrich`: `open_agentic_spec_registry_reader`
  (path), `open_agentic_spec_types` (path), plus generic crates.
- `tools/oap-code-index-enrich`: `open_agentic_codebase_indexer`
  (path), `open_agentic_spec_types` (path), plus generic crates.

`crates/factory-engine` and `apps/desktop/src-tauri` consume
`open_agentic_spec_registry_reader` (post W-10 and W-12
respectively); both are OAP-side consumers, so the direction is
correct.

No leaf-discipline violation found.

### Release bundle

`.github/workflows/release-tools.yml:107-123` builds exactly four
binaries via `cargo build --release --target ${{ matrix.target }}
--manifest-path tools/<name>/Cargo.toml`:

1. `spec-compiler`
2. `registry-consumer`
3. `spec-lint`
4. `codebase-indexer`

Lines 116-120 document the W-09 deletion of `policy-compiler` from
the bundle with the rationale ("It remains an OAP-internal tool
built+tested in spec-conformance.yml; external consumers should
build it from source").

The staging loop at line 146 — `for tool in spec-compiler
registry-consumer spec-lint codebase-indexer` — confirms exactly
those four binaries are archived. Neither `oap-registry-enrich` nor
`oap-code-index-enrich` appears in the workflow.

### Schema versions

- `SPEC_VERSION = "2.0.0"` at `tools/spec-compiler/src/lib.rs:52`.
  Diff against `b41c02e7`: single line change
  (`-const SPEC_VERSION: &str = "1.5.0";` →
  `+const SPEC_VERSION: &str = "2.0.0";`). W-06c.
- `SCHEMA_VERSION = "2.0.0"` at
  `tools/codebase-indexer/src/types.rs:26`. Diff against
  `b41c02e7`: single line change (`1.4.0 → 2.0.0`). W-07c.

Other `*_SCHEMA_VERSION` constants exist in
`crates/factory-contracts/` (`KNOWLEDGE_SCHEMA_VERSION`,
`PROVENANCE_SCHEMA_VERSION`, `STAKEHOLDER_DOC_SCHEMA_VERSION`) but
all remain at `"1.0.0"` and are not part of Cut D. The two bumps
above are the only two version bumps in the run, as the plan
required.

---

## Notes

(Annotations the operator should read before deciding next steps.
None individually severe enough to block, but provided for context.)

1. **Run-report and W-05 commit body invert the
   golden-vs-actual fingerprint labels** (Q3b). The substantive
   verification (pre-existing on main, byte-preserved by W-05) is
   correct, but a reader following the labels would conclude
   "scanner computes `15ca2c6d`" which is the opposite of the
   evidence.

2. **Q3c binary list is off by one** (Q3c). The run-report names
   `factory-harness` as one of the three wiring sites; the actual
   third site is `verify-certificate`. The deferred wiring cost
   estimate (~5 LoC per caller, three binaries) holds, just on a
   different binary set.

3. **KNOWN_KEYS factoring documentation is in-code only** (Q4a).
   The 9-line doc comment on `KNOWN_KEYS` is sufficient for a
   reader who arrives via grep; no narrative companion exists in
   `README`, `CLAUDE.md`, or a spec amendment. The W-06c commit
   message is the durable explanation.

4. **`build/codebase-index/index.json` was stale on the source
   branch** (Pre-conditions). The W-07c commit committed an index
   that reflected the post-W-07c source state; W-10 added a new
   `open_agentic_spec_registry_reader` dep on `factory-engine`,
   which changed the input set without a follow-up index commit.
   The chore commit `cc7dd80b` at the verification tip refreshes
   the index. `codebase-indexer check` returns 0 after the chore.

5. **registry-consumer binary at `tools/registry-consumer/target/`
   was stale** (Pre-conditions). The pre-Cut-D binary was compiled
   against `specVersion 1.5.0` and rejected the post-W-06c
   `2.0.0` registry. Rebuild restores the typed-reader's
   schema-version dispatch.

---

## Failures

1. **Q2: silent test regression in
   `crates/axiomregent/tests/mcp_featuregraph_test.rs`.**

   - `test_features_impact` and `test_gov_drift` pass on `main` at
     `b41c02e7` (independently verified via worktree); both fail on
     the branch tip after Cut D with
     `Tool failed: unsupported registry specVersion: ""`.
   - Root cause is W-05's switch of featuregraph's registry parser
     to the typed reader (which requires a non-empty `specVersion`)
     combined with the test fixture at
     `crates/axiomregent/tests/mcp_featuregraph_test.rs:23-25` that
     writes a registry without `specVersion`.
   - The W-05 commit body acknowledges this exact class of fixture
     break for featuregraph's own tests but does not audit
     downstream consumers; the run-report does not disclose this
     regression at all. Neither "Tests not yet wired" nor "Three
     changes most likely to need reviewer attention" mentions it.
   - The mechanical fix is one line in the test fixture (add
     `"specVersion":"1.5.0"` to the JSON literal). Per the
     verification-prompt's "Do not fix it" rule the fix is not
     applied in this pass.

This is one verification failure rooted in a discoverable code
defect that the autonomous run did not surface. It bears on
operator merge confidence because the run-report's "branch ready
for operator review" line is contradicted by an undocumented
regression.
