---
id: "127-spec-code-coupling-gate"
slug: spec-code-coupling-gate
title: "Spec/Code Coupling Gate — fail PRs that change implements: paths without spec.md"
status: approved
implementation: complete
amended: "2026-05-02"
amendment_record: "130-spec-coupling-primary-owner"
owner: bart
created: "2026-05-02"
approved: "2026-05-02"
kind: governance
risk: medium
depends_on:
  - "000"  # bootstrap-spec-system
  - "001"  # spec-compiler-mvp (registry)
  - "101"  # codebase-index-mvp (index.json)
  - "103"  # init-protocol-governed-reads (governed-read discipline)
  - "104"  # makefile-ci-parity-contract (Makefile mirror)
  - "118"  # workflow-spec-traceability (`# Spec:` header)
code_aliases: ["SPEC_CODE_COUPLING"]
implements:
  - path: tools/spec-code-coupling-check
  - path: .github/workflows/ci-spec-code-coupling.yml
  - path: Makefile
summary: >
  PR-time gate that fails when a diff touches paths declared in any spec's
  `implements:` list but does not also modify that spec's `spec.md`. Closes
  the largest engineered-drift surface: the ability to ship code change
  without an accompanying spec change. Reads the compiled codebase-index
  via designated-consumer typed deserialization (spec 103). Bypasses for
  pure infrastructure paths (`.github/`, `docs/`, root README/AGENTS) live
  in a checked-in allowlist; explicit waivers via `Spec-Drift-Waiver:`
  line in the PR body are surfaced in CI logs.
---

# 127 — Spec/Code Coupling Gate

## 1. Problem Statement

Spec 101 (codebase-index-mvp) compiles every `implements:` declaration into
`build/codebase-index/index.json` Layer 2 (`Traceability.mappings`). The
mapping is consumed by the init protocol and `CODEBASE-INDEX.md`, but it
is not enforced at PR time.

Concretely: a contributor (human or agent) can edit `crates/orchestrator/`
without touching `specs/044-multi-agent-orchestration/spec.md`. Nothing in
CI fails. The drift surface is: code moves while the spec stays frozen at
its prior shape, and reviewers may not notice the misalignment if the diff
spans dozens of files.

This is the largest single defensive gap in the spec spine — every other
hardening lever (lifecycle lint, governance docs, advisories) lights up
on inputs the contributor authored, while an `implements:` declaration is
a passive index entry that no current gate guards.

## 2. Goals

- **Symmetry, not size.** Failing on any unmatched coupling — even a
  one-line module change — is the point. The gate is binary; nuance lives
  in the spec edit.
- **Governed reads only.** The gate reads `build/codebase-index/index.json`
  through typed deserialization shared with `codebase-indexer` (spec 103
  exception: a consumer binary parses its own artifact). No `jq`, no `awk`.
- **Allowlist pure infrastructure.** Workflow files, docs, and root
  governance markdown change without owing a spec. The bypass list lives
  in source (not in PR body) so review can challenge it.
- **Explicit waiver path.** A reviewer who intentionally decouples (e.g.
  emergency hotfix) writes `Spec-Drift-Waiver: <reason>` in the PR body.
  The gate surfaces the waiver in its log output and exits 0; reviewers
  see the waiver text in the CI summary.
- **Self-test.** The gate verifies its own PR — adding spec 127 itself
  changes both `tools/spec-code-coupling-check/` and
  `specs/127-spec-code-coupling-gate/spec.md`, satisfying the rule.

## 3. Scope

### In scope

- A new Rust binary `tools/spec-code-coupling-check/` that:
  - Reads `build/codebase-index/index.json` via typed `serde_json::from_reader`
    against `codebase-indexer`'s exported types (the consumer-binary exception
    in spec 103).
  - Computes the diff path set against a base ref (default `origin/main`).
  - Determines, for each diff path, the set of spec IDs whose `implements:`
    list claims it (exact path or prefix).
  - Fails when an owing spec's `spec.md` is not also in the diff.
  - Honours a checked-in bypass list and a PR-body waiver.
- A new GitHub Actions workflow `.github/workflows/ci-spec-code-coupling.yml`
  with a `# Spec: 127-spec-code-coupling-gate` header (spec 118).
- A Makefile target `ci-spec-code-coupling` composed into `make ci`.
- Inclusion in `tools/ci-parity-check`'s `ENFORCING_WORKFLOWS` (spec 104).

### Out of scope

- Mutating the index. The gate is read-only; cross-reference build is
  `codebase-indexer compile`'s job.
- Heuristic coupling (e.g. "a spec change touched by a doc edit suffices").
  False positives are acceptable; false negatives are the threat model.
- Verifying spec frontmatter validity. That is `spec-compiler` + `spec-lint`.

## 4. Functional Requirements

- **FR-001 — runs on every PR.** The workflow triggers on `pull_request`
  (no path filter — every diff is candidate). It also runs on
  `workflow_dispatch` for manual replay.
- **FR-002 — governed read.** The binary deserializes
  `build/codebase-index/index.json` through `codebase-indexer`'s public
  types crate. It does not invoke `jq`, `python`, or text-based parsers.
  Schema-version mismatch is a hard error with a directive to recompile.
- **FR-003 — coupling detection.** For each diff path P that is not
  bypass-listed, the gate computes the set of spec IDs S such that some
  `mapping.implementing_paths[].path` is exactly P or a path prefix of P
  (slash-anchored). The gate fails when **no** `s ∈ S` has `specs/<s>/spec.md`
  in the diff (per spec 130's primary-owner heuristic, amends 2026-05-02).
  The original FR-003 required *every* `s ∈ S` to be in the diff — see
  the `## Amendment record` section below for the rationale.
- **FR-004 — error shape.** A coupling violation prints, per offending
  spec, a multi-line block: spec ID + each offending path on its own
  indented line. Exit code 1.
- **FR-005 — waiver.** A line `Spec-Drift-Waiver: <reason>` (case-sensitive
  keyword, free-form reason) anywhere in the PR body causes the gate to
  exit 0 with a `::warning::` annotation that echoes the reason. The
  waiver applies to the entire PR; per-violation waivers are out of scope.
- **FR-006 — bypass allowlist.** A checked-in list of path prefixes
  (`.github/`, `docs/`, root `README.md`/`CLAUDE.md`/`DEVELOPERS.md`/
  `LICENSE`/`CHANGELOG.md`/`CODEOWNERS`/`.gitignore`/`.gitattributes`)
  suppresses coupling checks. Notably absent: `Makefile` and `AGENTS.md`
  — both are claimed by multiple specs (104/105/116 and 103 respectively)
  and changes route through the affected owners. Workflow files under
  `.github/workflows/` are bypass-listed because spec 118's
  `# Spec: NNN-slug` header convention governs workflow ownership
  separately and orthogonally; layering the diff-based check on top would
  duplicate enforcement and amplify churn.
- **FR-007 — self-test.** A test in `tools/spec-code-coupling-check/tests/`
  exercises the rule against a synthetic index + diff fixture: a path
  declared by spec X with no spec X edit in the diff produces a violation;
  same diff plus the spec edit produces no violation; a bypass-listed
  path never produces a violation; a waiver-bearing PR body suppresses
  violations and emits the warning annotation marker.

## 5. Implementation Shape

### Binary

`tools/spec-code-coupling-check/` declares
`[package.metadata.oap].spec = "127-spec-code-coupling-gate"`. It depends
on `open_agentic_codebase_indexer` as a path dependency for the typed
schema (`SCHEMA_VERSION` constant + `CodebaseIndex` deserialize).

Subcommands (single binary, flags only):

```
spec-code-coupling-check [--repo <path>] [--base <ref>] [--head <ref>]
                         [--paths-from <file>] [--pr-body <file>]
                         [--index <path>]
```

- `--base/--head` default to `origin/main` and `HEAD`.
- `--paths-from` accepts a newline-delimited file (override for testing
  and for the workflow's `gh pr diff --name-only` redirect).
- `--pr-body` reads waiver text from a file (default: `$GITHUB_PR_BODY`
  env var or empty).
- `--index` defaults to `build/codebase-index/index.json`.

### Workflow

`.github/workflows/ci-spec-code-coupling.yml`:

```yaml
# Spec: 127-spec-code-coupling-gate
name: CI spec/code coupling
on: [pull_request, workflow_dispatch]
permissions: { contents: read, pull-requests: read }
jobs:
  coupling-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<sha> with: { fetch-depth: 0 }
      - name: Build the gate
      - name: Compile codebase-index
      - name: Run gate
        env:
          BASE_REF: ${{ github.event.pull_request.base.sha }}
          HEAD_REF: ${{ github.event.pull_request.head.sha }}
          PR_BODY: ${{ github.event.pull_request.body }}
        run: |
          ./tools/spec-code-coupling-check/target/release/spec-code-coupling-check \
              --base "$BASE_REF" --head "$HEAD_REF" --pr-body <(echo "$PR_BODY")
```

### Makefile

```makefile
ci-spec-code-coupling:
\tcargo build --release --manifest-path tools/spec-code-coupling-check/Cargo.toml
\t./tools/spec-code-coupling-check/target/release/spec-code-coupling-check \\
\t    --base $(or $(BASE_REF),origin/main) --head $(or $(HEAD_REF),HEAD)
```

`ci` composes `ci-spec-code-coupling` after `ci-tools` (which produces
the index) and before `ci-supply-chain`.

## 6. Acceptance Criteria

- **AC-1.** A synthetic diff that touches `crates/orchestrator/src/lib.rs`
  without changing `specs/044-multi-agent-orchestration/spec.md` exits 1
  with a violation block naming spec 044 and the offending path.
- **AC-2.** The same diff with the spec.md added exits 0 silently.
- **AC-3.** A diff touching only `docs/ARCHITECTURE.md` exits 0.
- **AC-4.** A diff with a coupling violation but a `Spec-Drift-Waiver:
  hotfix for incident OPS-123` line in the PR body exits 0 with a
  warning annotation echoing the waiver reason.
- **AC-5.** `make ci` includes `ci-spec-code-coupling` and exits 0 in
  this PR (self-test).
- **AC-6.** `ci-parity-check` confirms the new workflow's `run:` blocks
  are mirrored in the Makefile.

## 7. Risks and Mitigations

- **Risk:** The bypass list grows into an escape hatch.
  **Mitigation:** The list is a checked-in source file; every addition
  shows up as a normal diff line. Reviewers can challenge.

- **Risk:** Schema drift between `codebase-indexer` and the gate.
  **Mitigation:** The gate depends on the indexer crate as a path
  dependency, so a schema change rebuilds both in lockstep. The gate
  also asserts `SCHEMA_VERSION` at runtime against the file's
  `schemaVersion` field.

- **Risk:** Waivers normalised by repeat use.
  **Mitigation:** Waiver text is echoed as `::warning::` in the workflow
  log; CI summaries show every waiver. Pattern review is a manual
  governance practice, not a tool feature in v1.

## 8. Worked Example

A PR that adds a function to `crates/orchestrator/src/dag.rs` without
amending any of the 12 specs that claim `crates/orchestrator`:

```
spec-code-coupling-check: 1 path(s) lack a claimant edit.

  crates/orchestrator/src/dag.rs (claimed by 12 specs)
    004-spec-to-execution-bridge-mvp
    043-agent-organizer
    052-state-persistence
    079-scheduling
    082-artifact-integrity-platform-hardening
    090-governance-non-optionality
    094-unified-artifact-store
    098-governance-enforcement-stitching
    099-workspace-scoped-persistence
    102-governed-excellence
    119-project-as-unit-of-governance
    044-multi-agent-orchestration

To resolve, amend ANY ONE claimant's spec.md (per spec 130
primary-owner heuristic), or add 'Spec-Drift-Waiver: <reason>'
to the PR body.
```

Adding any one of the listed claimants' `spec.md` to the diff clears
the violation. The intent is friction in the right place — some owner
must review — not coercive scale across every claimant.

## Defect log

**2026-05-02 — test isolation: `GITHUB_PR_BODY` env leak.**
The `tests/cli.rs` integration tests built the gate binary via
`Command::new(cli_bin())`, which inherits the parent process
environment. When `make ci-spec-code-coupling` runs in a shell that
already has `GITHUB_PR_BODY` set (e.g. when verifying a waiver
locally), the inherited value contains a `Spec-Drift-Waiver:` line
and AC-1's "violation must exit 1" assertion silently degrades to
exit 0. Fix: `tests/cli.rs::run` now adds
`.env_remove("GITHUB_PR_BODY")` before spawning. No production code
change; failure mode was test-only.

## Amendment record

**Amendment 2026-05-02 (record: 130-spec-coupling-primary-owner).**
The original FR-003 required *every* claimant of a diff path to have
its `spec.md` in the same diff. Real-corpus exercise during spec 129's
demo step revealed cascade behaviour: `crates/orchestrator` is claimed
by 12 specs, so a single-file change to that crate would have demanded
12 cosmetic spec amendments. The current FR-003 wording captures the
post-amendment rule (any one claimant's edit covers the path); spec
130 documents the rationale, the alternate paths considered (refining
`implements:` declarations; adding explicit `primary: true` flags),
and OQ-1 noting when an explicit-primary mechanism would replace the
heuristic. The renderer also changes shape: violation blocks are
path-centric (not per-spec), and every claimant is named so reviewers
can sanity-check that the right one was edited.
