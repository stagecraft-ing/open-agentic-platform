# Cut D — W-05 commit body corrigendum

> Durable record of a documentation defect in commit `7ede4d93`
> ("W-05: featuregraph consumes typed-reader"). The commit itself
> cannot be amended (hard rule: do not modify any of the 16 W-unit
> commits), so the correction lives here.

## What is wrong in the W-05 commit body

The W-05 commit body inverts the golden-file vs scanner-output
fingerprint labels when documenting the pre-existing
`crates/featuregraph/tests/golden.rs::test_golden_graph` failure.
The body states:

> "the golden file's `graph_fingerprint` is `febf01350aa6...`
> but the live spec-corpus computes `15ca2c6d...`"

> "actual fingerprint is identical pre- and post-W-05
> (`15ca2c6d...`)"

Empirically the labels are swapped. The golden file at
`crates/featuregraph/tests/golden/features_graph.json` (before the
Cut D Fix 4 refresh in commit `78641fd3`) contained the
fingerprint `sha256:15ca2c6d7cf649cb2b389eb888f7c782bdb91532aa92705dce983d830c802868`,
and the scanner against the live spec corpus produces
`sha256:febf01350aa6bcafe421aa95fbb36df6fe6ca0fb4ed52da82a05d1551e96ed43`.

## Why the substantive claim is unchanged

The substantive verification W-05 carried — that the failure
predates W-05 and that `impl_files()` semantics are byte-preserved
across the W-05 refactor — is independently confirmed by the
verification pass
(`docs/analysis/spec-spine-cut-d-verification.md` §Q3b and §Notes 1).
The stash-and-rerun check on the W-04 baseline shows the
scanner-produced fingerprint is identical pre- and post-W-05;
only the labels in the commit body's prose are wrong.

## What was done about it

Cut D Fix 5 corrects the same label inversion in
`docs/analysis/spec-spine-cut-d-run-report.md` (§W-05) and adds an
inline pointer back to this corrigendum. Cut D Fix 4
(`chore(cut-d): refresh featuregraph golden fingerprint`, commit
`78641fd3`) refreshes the golden file to the empirically-observed
scanner output, closing the test gap that the inverted labels
referred to. The W-05 commit body itself remains uncorrected per
the no-history-rewrites rule.
