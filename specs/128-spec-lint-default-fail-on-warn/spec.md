---
id: "128-spec-lint-default-fail-on-warn"
slug: spec-lint-default-fail-on-warn
title: "spec-lint default to fail-on-warn — repo opts into the strict posture"
status: approved
implementation: complete
amends: ["006"]
owner: bart
created: "2026-05-02"
approved: "2026-05-02"
kind: governance
risk: low
depends_on:
  - "006"  # conformance-lint-mvp (the surface being amended)
  - "104"  # makefile-ci-parity-contract (the integration point)
code_aliases: ["SPEC_LINT_STRICT"]
implements:
  - path: Makefile
summary: >
  Spec 006 defines `spec-lint` as advisory by default with an opt-in
  `--fail-on-warn` flag for repos that choose the strict posture. This
  spec ratifies OAP's policy choice: the strict posture is on. `make ci`
  now invokes spec-lint with `--fail-on-warn` and propagates non-zero exit
  codes (the previous `|| true` is dropped). Behaviour-preserving for the
  current corpus — zero W-codes fire against the 128-spec corpus as of
  the 2026-05-02 audit — but closes the silent-drift channel for future
  changes.
---

# 128 — spec-lint default to fail-on-warn

## 1. Problem Statement

Spec 006 §"Purpose and charter" defines two postures for `spec-lint`:

> - **CLI** that prints warnings to **stderr** and exits **0** by default …
> - **Optional** `--fail-on-warn` (or equivalent) for **strict** CI

…and §"Normative dependency" closes with:

> 006 warnings are **advisory** unless a repo policy opts into
> `--fail-on-warn`.

OAP's `make ci` does NOT currently opt in:

```makefile
./tools/spec-lint/target/release/spec-lint || true   # warnings non-blocking (matches CI)
```

The `|| true` swallows any future warnings into `make ci`'s green log.
A contributor could land a `W-002` (superseded spec missing replacement
pointer) or `W-005` (mixed task notation) without CI noticing.

This spec is the policy choice that spec 006 line 46 anticipated.

## 2. Decision

**OAP opts into `--fail-on-warn`.** No CLI default is changed; spec-lint
itself remains advisory by design. The amendment is purely the
operational integration in `make ci`.

This is the amendment-style change the spec-spine bootstrap (spec 000)
designed for: refining narrative without superseding. The `amends:
["006"]` frontmatter records the link; spec 006 carries
`amended: 2026-05-02` and `amendment_record:
"128-spec-lint-default-fail-on-warn"` plus an in-body callout.

## 3. Scope

### In scope

- Drop `|| true` from `make ci-tools`'s spec-lint line.
- Add `--fail-on-warn` to that invocation.
- Update spec 006 with the amendment frontmatter and in-body callout.
- Inventory spec-lint warnings before the flip and document the
  baseline in `/tmp/spec_lint_inventory.md` (saved as a session artefact,
  not a checked-in file).

### Out of scope

- Adding new W-codes — that's a follow-on to spec 006.
- Changing the CLI default (`spec-lint` without flags still exits 0) —
  this is a per-repo policy choice and other consumers may differ.
- Changing the test surface of `spec-lint` itself.

## 4. Inventory baseline (2026-05-02)

Pre-flip inventory:

```
$ ./tools/spec-lint/target/release/spec-lint
$ echo $?
0
```

Zero W-codes fire across all 128 specs. Below the Unit 3 inventory
thresholds (>20 total or any single spec >3) that would have routed the
flip to a separate hygiene-sweep PR. The amendment is purely mechanical.

If a future amendment to spec 006 adds new heuristics that increase the
fire rate, the policy choice can be reconsidered via another amendment.

## 5. Functional Requirements

- **FR-001 — make ci is strict.** `make ci-tools` invokes
  `spec-lint --fail-on-warn`, no `|| true`.
- **FR-002 — CLI default unchanged.** Running `spec-lint` without flags
  still exits 0 on warnings. Other consumers (a future docs build, a
  pre-commit hook) opt in or out independently.
- **FR-003 — amendment annotation.** Spec 006 carries
  `amended: 2026-05-02` and `amendment_record:
  "128-spec-lint-default-fail-on-warn"` plus a brief in-body callout
  pointing at this spec.

## 6. Acceptance

- **AC-1.** `make ci-tools` invocation contains `--fail-on-warn` and no
  `|| true` on the spec-lint line.
- **AC-2.** `make ci` exits 0 (current corpus is clean).
- **AC-3.** Spec 006 frontmatter includes the amendment annotation.
- **AC-4.** A synthetic test that introduces a `W-002` violation
  (superseded spec without replacement pointer) is rejected by
  `make ci-tools` post-flip. Verified manually during landing; not
  encoded as a permanent test fixture (would require a fixture spec
  that never compiles cleanly).
