---
id: "193-paired-release-cadence"
slug: paired-release-cadence
title: "Paired release cadence + version-consistency guard (amends 037, 086, 117)"
status: approved
implementation: complete
owner: bart
created: "2026-06-01"
approved: "2026-06-01"
kind: governance
domain: tooling
risk: medium
amends: ["037", "086", "117"]
depends_on:
  - "037"  # cross-platform-axiomregent (axiomregent release matrix)
  - "086"  # open-source-launch (release-tools fitness baseline)
  - "117"  # release-artifact-attestations (SBOM + provenance the guard reads)
  - "116"  # supply-chain-policy-gates (the lane the guard wires into)
  - "158"  # workflow-ref-sha-pinning-lint (shell-lint-in-tools/lint precedent)
code_aliases: ["RELEASE_CADENCE", "RELEASE_VERSION_GUARD"]
establishes:
  - unit: { kind: file, path: tools/lint/release-version-guard.sh }
  - unit: { kind: file, path: tools/lint/release-version-guard-test.sh }
refines:
  - aspect: "release-version-alignment"
    unit: { kind: file, path: .github/workflows/release-desktop.yml }
  - aspect: "release-version-alignment"
    unit: { kind: file, path: .github/workflows/release-axiomregent.yml }
  - aspect: "release-version-alignment"
    unit: { kind: file, path: .github/workflows/release-tools.yml }
  - aspect: "release-version-alignment"
    unit: { kind: file, path: .github/workflows/ci-supply-chain.yml }
  - aspect: "release-version"
    unit: { kind: file, path: product/apps/opc/src-tauri/tauri.conf.json }
  - aspect: "release-version"
    unit: { kind: file, path: product/apps/opc/package.json }
  - aspect: "release-version"
    unit: { kind: file, path: product/apps/opc/src-tauri/Cargo.toml }
  - aspect: "release-version"
    unit: { kind: file, path: crates/axiomregent/Cargo.toml }
compliance:
  - framework: "owasp-asi-2026"
    # ASI04 (supply-chain compromise). A release whose envelope version
    # disagrees with its artifact version is an integrity gap: the published
    # version string is unverifiable against the binary it labels. The guard
    # makes envelope == artifact == SBOM a merge- and publish-time contract.
    controls: ["ASI04"]
summary: >
  Establish a paired release cadence for OPC desktop and axiomregent, a
  product-prefixed tag grammar (opc-v*, axiomregent-v*), and a pre-publish
  version-consistency guard that refuses any release whose tag disagrees with
  its committed version sources or its SBOM. Flips the axiomregent publish
  path from live/immutable to draft-first so every publish is guard-gated AND
  human-gated. Closes the class of bug where a release envelope (tag) carried
  one version while the artifacts carried a stale committed version
  (OPC drafts v0.3.5/6/7 shipped 0.3.4 assets; axiomregent v0.3.0/v0.3.1
  published 0.2.0 binaries).
---

# 193 — Paired release cadence + version-consistency guard

> **Amendment record.** This spec amends **037** (cross-platform axiomregent
> release), **086** (open-source-launch release-tools), and **117** (release
> artifact attestations). It does not supersede them: their release mechanics
> stand. It adds (a) a version-consistency contract every release must satisfy,
> (b) a product-prefixed tag grammar, and (c) draft-first publishing for
> axiomregent. The companion investigation of every release-publish path is
> recorded in `docs/analysis/release-publish-paths-2026-06-01.md`.

## 1. Problem Statement

Two version-source families feed a release and nothing asserted they agree:

- The **release envelope** (GitHub Release title + tag) is taken from the git
  tag / `workflow_dispatch` input.
- The **artifacts** are named and stamped from the *committed* version sources
  — `tauri.conf.json` / `package.json` / `Cargo.toml` for OPC (Tauri reads the
  bundle version from `tauri.conf.json`), and `crates/axiomregent/Cargo.toml`
  for axiomregent (`CARGO_PKG_VERSION`).

Cutting a release without first bumping the committed sources produced a split:
the OPC drafts **v0.3.5 / v0.3.6 / v0.3.7** all carry **0.3.4** assets, and the
live axiomregent releases **v0.3.0 / v0.3.1** ship **0.2.0** binaries. Worse,
`release-axiomregent.yml` published a **live, immutable** release directly
(`gh release create` with no `--draft`), so the mismatch was burned publicly
the instant a `workflow_dispatch` ran.

## 2. Cadence policy

The two products release on a **paired cadence** anchored at a shared
`major.minor`:

- **minor / major bumps are train-scoped.** Both products move together to the
  same `major.minor.0`; patch resets to `.0`. The first train under this spec is
  `0.4.0` for both `opc` and `axiomregent`.
- **patch bumps are product-scoped.** Either product may ship `major.minor.PATCH`
  independently to fix a defect without dragging the other.
- **Boundary invariant.** At every minor/major boundary, both products share
  `major.minor.0`. Formally: `opc.major == axiomregent.major &&
  opc.minor == axiomregent.minor`, and on a minor/major release both patch
  components are `0`. Patch divergence (`opc 0.4.1` while `axiomregent 0.4.0`)
  is permitted within a train; major/minor divergence is not.

This spec does not automate the bump; it defines the invariant the guard and
reviewers enforce. The bump is an honest committed edit (§4), never a
build-time injection — the source tree always tells the truth about the
version it will ship.

## 3. Tag grammar (naming unification)

All release tags are **product-prefixed**: `<product>-v<semver>`.

| Product | Tag | Trigger workflow |
|---------|-----|------------------|
| OPC desktop | `opc-v<semver>` (e.g. `opc-v0.4.0`) | `release-desktop.yml` |
| axiomregent | `axiomregent-v<semver>` (e.g. `axiomregent-v0.4.0`) | `release-axiomregent.yml` |

OPC's bare `v*` grammar is **retired**. Each workflow's resolver derives the
product from the prefix (`${TAG%-v*}`) and the version from the suffix
(`${TAG##*-v}`), and rejects a tag whose product does not match the workflow.
The dependent `release-tools.yml` `workflow_run` gate moves from
`startsWith(head_branch, 'v')` to `startsWith(head_branch, 'opc-v')` so the
tool-archive chain keeps firing for renamed desktop releases.

## 4. Version-consistency guard (the contract)

`tools/lint/release-version-guard.sh <product> [expected-version] [sbom]` asserts
the version is identical across every source:

```
tag == tauri.conf.json == package.json == Cargo.toml == Cargo.lock ( == SBOM component )
```

- **Internal-consistency mode** (`<product>` only): all committed sources agree
  with one another. Runs in the supply-chain lane (`ci-supply-chain.yml`) on
  every PR, catching a half-done bump before it can reach a release tag.
- **Expected mode** (`<product> <version> [sbom]`): all sources — and the SBOM
  component — equal the resolved tag version. Runs in the release workflows.

Exit `0` = eligible; `1` = mismatch (release NOT eligible); `2` = usage error.
SBOM-component **mismatch** is fatal; SBOM-component **absence** is a warning
(syft component naming is scanner-version dependent and outside this repo's
control). The guard ships with a fixture test
(`release-version-guard-test.sh`) that is its own spec, run in the same lane —
the spec-158 precedent.

## 5. Publish gating

- **OPC desktop** already builds a **draft** (`tauri-action releaseDraft: true`);
  publishing is human-gated. This spec adds a **pre-build** guard (fail-fast,
  before any artifact or draft exists — no version burned on mismatch) and a
  **post-build** SBOM assertion that discards the draft + tag on mismatch.
- **axiomregent** flips from **live publish** to **draft-first**: the guard runs
  **before** `gh release create --draft`, so a mismatch publishes nothing and
  burns no tag. Promoting the draft to a published release is a separate,
  human-gated action.

The result: no release — desktop or axiomregent — can publish with a version
the artifacts do not actually carry, and no release publishes without a human.

## 6. Acceptance criteria

- **AC-1.** `release-version-guard.sh opc` and `… axiomregent` exit `0` on the
  committed tree (both at `0.4.0`); the fixture test passes. *(verified)*
- **AC-2.** Both release workflows resolve `<product>` and `<version>` from a
  prefixed tag and reject a mismatched-product tag.
- **AC-3.** `release-desktop.yml` triggers on `opc-v*`; `release-tools.yml`'s
  `workflow_run` gate keys on `opc-v`.
- **AC-4.** `release-axiomregent.yml` creates a **draft** and runs the guard
  before creation.
- **AC-5.** `ci-supply-chain.yml` runs the internal-consistency guard for both
  products and the fixture test on every PR.
- **AC-6.** Committed versions: `opc = 0.4.0` across tauri.conf.json /
  package.json / Cargo.toml / Cargo.lock; `axiomregent = 0.4.0` across
  Cargo.toml / Cargo.lock. *(verified under `cargo metadata --locked`)*

## 7. Out of scope (human-gated, deferred)

- Cutting and **publishing** the `opc-v0.4.0` / `axiomregent-v0.4.0` releases.
  This spec lands the mechanism; the first paired publish is an operator action,
  additionally blocked until the release-publish-path investigation
  (`docs/analysis/release-publish-paths-2026-06-01.md`) is resolved and the
  publish path is controlled.
- Cleanup of the existing stale drafts (OPC `v0.3.5/6/7`) and reconciliation of
  the already-live `axiomregent-v0.3.0/v0.3.1` — remediation, tracked
  separately.
