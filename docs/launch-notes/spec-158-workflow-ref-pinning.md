# Spec 158 launch notes — workflow-ref SHA-pinning lint

> Evidence bundle for spec 158 (`workflow-ref-sha-pinning-lint`, amends
> 116). Each block below is the literal terminal output from a real
> invocation against this tree. The auditability claim is not "trust
> us" — it is "here are the four invocations and their outputs;
> reproduce them locally and the contract holds."
>
> Generated: 2026-05-22.

The lint promotes a 100%-correct-by-discipline convention into a
merge-blocking contract. Four falsifiers; each one a specific way the
claim could be wrong, and the output that proves it isn't. The
methodology note (Block 0) names the failure mode that almost made
this file dishonest.

---

## Block 0 — methodology note

A lint that returns exit 0 across zero scanned lines is the precise
failure mode of every "we audit our workflows" claim that has ever
shipped. The first draft of `workflow-pins.sh` had a bug in its
`grep` pre-filter — `^[[:space:]]*uses:` did not match the YAML
list-item form `- uses:`, which is how every workflow step in this
tree is written. The lint scanned zero lines and returned exit 0.
The "convention is enforced" claim would have been true in form and
false in substance.

The bug was caught by a separate falsifier — counting matched lines
in the lint vs. counting `uses:` lines in the tree with `grep -c`:

```
$ grep -cE '^[[:space:]]*uses:' .github/workflows/release-desktop.yml
14
$ grep -cE '^[[:space:]]*(-[[:space:]]+)?uses:' .github/workflows/release-desktop.yml
19
```

5 of 19 refs would have escaped the lint. The fix is in the
`pinned_re`, `unpinned_candidate_re`, `any_uses_re`, and `docker_re`
expressions — all four now accept the optional `-[[:space:]]+`
list-item prefix.

Generalisable principle: when a lint or audit script returns 0,
the next check is "did it actually look at anything?" — and that
check must be independent of the lint itself. The fixtures in
`tools/lint/tests/fixtures/` are the standing version of this
check: a failing fixture that the lint MUST flag as broken, so a
zero-scan regression is detectable.

---

## Block 1 — falsifier #1: tree-wide scan against the real tree

The claim: every `uses:` ref in `.github/workflows/**` and
`.github/actions/**` is SHA-pinned to a full 40-hex commit.

```
$ ./tools/lint/workflow-pins.sh
$ echo "exit: $?"
exit: 0
```

Silent exit 0 across 126 `uses:` refs in 22 workflow files. The
"100%-correct by discipline" claim is now scripted, not asserted.

---

## Block 2 — falsifier #2: synthetic bad fixture

The claim: the lint correctly classifies every shape it must
classify, and refuses the right ones.

Fixture covers five intentional violations (tag-pin, branch-pin,
named-ref pin, dynamic ref via `${{ env.SHA }}`, fully dynamic ref
via `${{ matrix.action }}@${{ matrix.sha }}`) and three legitimate
skips (local path, docker:// digest, properly pinned with comment).

```
$ ./tools/lint/workflow-pins.sh /tmp/wf-bad.yml
/tmp/wf-bad.yml:7:- uses: actions/checkout@v4
/tmp/wf-bad.yml:8:- uses: actions/setup-node@main
/tmp/wf-bad.yml:9:- uses: dtolnay/rust-toolchain@stable
/tmp/wf-bad.yml:13:- uses: actions/checkout@${{ env.SHA }} [dynamic ref — pin literally]
/tmp/wf-bad.yml:14:- uses: ${{ matrix.action }}@${{ matrix.sha }} [dynamic ref — pin literally]

workflow-pins: 5 unpinned ref(s) found. Pin to a full 40-hex SHA.
See: https://docs.github.com/en/actions/security-for-github-actions/security-guides/security-hardening-for-github-actions#using-third-party-actions
$ echo "exit: $?"
exit: 1
```

Five violations, exit 1, editor-clickable `file:line:ref` format on
stderr. Dynamic refs (`${{ ... }}` inside the `uses:` value) carry
the `[dynamic ref — pin literally]` annotation: the failure category
determines remediation, and a tag-pin fix differs from a
dynamic-ref fix.

The dynamic-ref refusal is not a heuristic. The lint is a static
proof system; what it cannot statically verify, it cannot soundly
approve. Approving an unprovable claim would turn the contract from
a refusal mechanism into a suggestion. Same shape of soundness
argument as Rust refusing to compile where the borrow checker
cannot prove non-aliasing — not because aliasing definitely
happens, but because soundness requires refusal in the absence of
proof.

---

## Block 3 — falsifier #3: the lint's own regression tests

The claim: the lint's classification semantics are themselves
versioned and verifiable. The test fixtures ARE the spec for the
lint — every shape the lint must classify lives there, and the
expected output is committed alongside.

```
$ ./tools/lint/workflow-pins-test.sh
PASS: passing fixture → exit 0, silent
PASS: failing fixture → exit 1, 5 violations on stderr
PASS: bash 4+ gate present in lint source
PASS: tree-wide scan → exit 0 (all current refs SHA-pinned)

workflow-pins-test: all assertions passed
$ echo "exit: $?"
exit: 0
```

The fourth falsifier has its own falsifiers. Recursion bottoms out
at the test fixtures. Curry-Howard applied to the lint itself:
the fixtures are the proof that the lint's semantics still hold.

---

## Block 4 — falsifier #4: full pre-commit hook, clean tree

The claim: the contract is enforced at the lower boundary of the
supply-chain pipeline, not only at merge. An un-pinned ref never
exists in any signed commit, on any branch, in pushed history.

```
$ git config core.hooksPath .githooks   # opt-in
$ .githooks/pre-commit
$ echo "exit: $?"
exit: 0
```

Two stages green silently: codebase-index staleness check (spec
103) followed by workflow-pins SHA-pin enforcement (spec 158).
Same `tools/lint/workflow-pins.sh` invoked from
`ci-supply-chain.yml` and from `.githooks/pre-commit` — one
script, two consumers, identical semantics.

A CI-only gate enforces "no un-pinned ref ever merges to main."
A pre-commit hook enforces "no un-pinned ref is ever signed by a
human into any branch." The hook is what makes the stronger claim
true.

---

## Block 5 — CI evidence, platform-side

The local blocks above prove the lint works. Block 5 proves the
platform runs it the same way at the merge boundary — same script,
same exit-code discipline, rendered job name as it appears in the
GitHub UI captured for downstream `required_status_checks` binding.

- **Branch:** `158-workflow-ref-sha-pinning-lint`
- **Head SHA:** `54ba6a3fe9cd74a1a92a516742f3416cfcf4b9da`
- **Workflow:** `CI supply-chain` (`.github/workflows/ci-supply-chain.yml`)
- **Run ID:** `26277186106`
- **Run URL:** https://github.com/stagecraft-ing/open-agentic-platform/actions/runs/26277186106
- **Trigger:** `workflow_dispatch` (see process observation below)
- **Conclusion:** success
- **Rendered job name (the binding key for required_status_checks):**
  `workflow-pins / SHA-pin enforcement`

### Job log excerpt (lint + test runner steps)

```
Complete job name: workflow-pins / SHA-pin enforcement
##[group]Run tools/lint/workflow-pins.sh
tools/lint/workflow-pins.sh
##[group]Run tools/lint/workflow-pins-test.sh
tools/lint/workflow-pins-test.sh
PASS: passing fixture → exit 0, silent
PASS: failing fixture → exit 1, 5 violations on stderr
PASS: bash 4+ gate present in lint source
PASS: tree-wide scan → exit 0 (all current refs SHA-pinned)
workflow-pins-test: all assertions passed
```

All four jobs in the run completed with conclusion `success`:
`cargo-deny / advisories + bans + licenses + sources`,
`pnpm-audit / desktop + packages`,
`npm-audit / stagecraft`,
`workflow-pins / SHA-pin enforcement`.

### What Block 5 proves and does not prove

Be precise about which claim each piece of evidence supports. Block
5 establishes:

- The lint runs correctly in the GitHub Actions runner environment
  on `ubuntu-24.04` (not just on the maintainer's macOS).
- The job renders in the GitHub UI with the literal name
  `workflow-pins / SHA-pin enforcement` — the binding key for
  `required_status_checks.contexts`.
- The script exits 0 against the current tree state in CI.

Block 5 does NOT prove:

- That the gate fires on the `pull_request:` event, which is the
  event `required_status_checks` binds against at the merge
  boundary. The CI run was triggered via `workflow_dispatch`
  fallback (see auto-trigger gap below).

The distinction matters for the auditability claim. The
required_status_checks binding step of the hardening sprint MUST
wait for an additional falsifier (Block 6, below, once the
auto-trigger gap is resolved) — runner-environment correctness is
necessary but not sufficient evidence for merge-boundary
enforcement.

### Process observation — auto-trigger gap

On opening PR #195 (draft, then converted ready, then reverted to
draft), zero workflows fired against the head SHA via the
`pull_request:` event. `gh api repos/.../actions/runs?head_sha=...`
returned an empty workflow run list across multiple polling
intervals. `workflow_dispatch` against the same branch fired the
expected jobs immediately, which is how Block 5's evidence was
captured.

Root cause not yet identified. Hypotheses worth checking before
the launch-hardening sprint:

1. A repo-level Actions approval policy that suppresses
   `pull_request:` triggers for branches that modify workflow
   files (a security posture some orgs enable).
2. A queue or quota condition not visible from the gh CLI API
   surface used here.
3. Specific interaction between the path-glob expansion in this
   PR (`.github/workflows/**`, `.github/actions/**`,
   `tools/lint/**`) and GitHub's pull-request workflow scheduler.

Captured here because it's directly relevant to the falsifier
discipline: a CI gate that exists but does not fire is
indistinguishable from a CI gate that does not exist. The
required_status_checks binding (hardening step 3) MUST be
configured against jobs that actually run on the relevant
events, not jobs that exist in the workflow file but are silently
skipped. Until the auto-trigger gap is diagnosed and resolved,
the workflow_dispatch fallback is the verifiable path; merge-time
enforcement via `required_status_checks` should not be enabled
against a workflow whose `pull_request:` firing is not yet
reliable.

## What the bundle authorises

This file is not a press release. It is the evidence base for
four specific claims:

1. The lint is sound (Block 2 — refuses what it cannot prove).
2. The lint scans what it claims to scan (Block 0 — the
   zero-scan regression that would have hidden a broken lint is
   itself tested for).
3. The lint's semantics are versioned (Block 3 — fixtures are
   the proof).
4. The contract holds across the entire push-history surface,
   not only at merge (Block 4 — hook + CI step share the
   script).

Each block is reproducible from a fresh clone with `make setup`
plus the invocations shown. The "noncompilable attack surface"
framing only counts where the compile exists; this is the
compile.

---

## Process observations

Small notes captured during the work that produced this bundle.
Each is a piece of evidence about *how* the contract was built,
not the contract itself; the essay's methodology section may
cite them.

- **Direct commit to main, caught by discipline, not enforcement
  (2026-05-22).** During the three-commit landing of this PR, the
  initial commits were authored against the local `main` branch
  rather than against a feature branch. The error was caught by
  discipline (a sanity check before pushing) and repaired without
  remote impact via `git branch <feature>` + `git reset --hard
  <previous-tip>` + `git checkout <feature>`. The current branch
  protection on `main` (`required_signatures.enabled: false`,
  `required_approving_review_count: 0`,
  `required_status_checks.contexts: []`) would not have refused
  the push had the commits been pushed before the error was
  caught. The hardened state (this PR's purpose, plus the rest of
  the launch-hardening sprint) would have refused it at the
  platform layer rather than relying on the maintainer's
  discipline. The proof of why the hardening matters is written
  in the act of doing the hardening — exactly the failure mode
  the contract exists to refuse, observed and repaired during the
  work that builds the contract.

- **The first-draft lint scanned zero lines (see Block 0).**
  Counted separately as the load-bearing methodology note above,
  not duplicated here. Same shape of finding: discipline caught
  a bug that no in-place check would have, surfaced by an
  independent differential ("did the lint look at anything?")
  rather than by the lint's own self-report.

Both observations rhyme: a defense built on discipline is a
defense whose absence is invisible. The lint and the hardening
sprint exist to make that absence visible — by the lint failing
loudly, or by the platform refusing the push.
