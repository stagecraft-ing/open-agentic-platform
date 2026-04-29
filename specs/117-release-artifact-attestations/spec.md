---
id: "117-release-artifact-attestations"
slug: release-artifact-attestations
title: Release Artifact Attestations — SBOM + build provenance for every shipped binary
status: draft
implementation: pending
owner: bart
created: "2026-04-28"
kind: governance
risk: medium
depends_on:
  - "000"  # bootstrap-spec-system
  - "037"  # cross-platform-axiomregent (release matrix precedent)
  - "086"  # open-source-launch (release-fitness baseline)
  - "104"  # makefile-ci-parity-contract
code_aliases: ["RELEASE_ATTESTATIONS"]
implements:
  - path: .github/workflows/release-axiomregent.yml
  - path: .github/workflows/release-desktop.yml
  - path: .github/workflows/release-tools.yml
summary: >
  Every binary shipped via release-{axiomregent,desktop,tools}.yml is paired
  with a CycloneDX SBOM and a GitHub-signed build provenance attestation.
  SBOMs and attestations are uploaded as release assets. The desktop installer
  SHA-256 sidecars (already present) are kept; the new attestations cover
  the binaries themselves and document the toolchain, dependencies, and
  build environment that produced them.
---

# 117 — Release Artifact Attestations

## 1. Problem Statement

The three release workflows ship binaries that downstream users will install
on their own machines:

- `release-axiomregent.yml` — standalone MCP server binaries for four target
  triples.
- `release-desktop.yml` — Tauri desktop installers (DMG, AppImage, NSIS) for
  three platforms, with sidecar binaries embedded.
- `release-tools.yml` — five Rust CLI tools per target triple, packaged as
  archives.

The desktop installer flow has already implemented SHA-256 sidecars (read by
the in-app updater). That covers **integrity** of the installer download
itself. It does not cover:

- **Provenance** — there is no signed claim asserting "this binary was built
  by the open-agentic-platform repo, by workflow X, at commit Y, by the
  GitHub-hosted runner whose attestor key signed it."
- **Inventory** — there is no SBOM listing the crates and JS packages that
  went into each binary. A downstream user cannot answer "is this affected
  by RUSTSEC-2026-XXXX?" without rebuilding from source.

GitHub now ships first-class primitives for both:

- `actions/attest-build-provenance@v2` produces an in-toto SLSA v1.0
  provenance attestation, signed by the runner's identity, anchored to a
  Sigstore transparency log.
- `anchore/sbom-action@v0` produces CycloneDX or SPDX SBOMs from a path or
  a Docker image.

For a project whose framing is **governed operating system**, shipping
unattested binaries is incongruent. This spec closes that gap on every
release surface in one pass.

## 2. Goals

- **Every release asset carries an attestation.** The four axiomregent
  binaries, the three desktop installers, and the per-platform tool
  archives each have a corresponding `*.intoto.jsonl` provenance attestation
  attached to the same GitHub Release.
- **Every release asset has an SBOM.** A `*.cdx.json` (CycloneDX) sibling
  per artifact lists every crate and version. For the desktop installer,
  the SBOM includes the bundled sidecar binaries' contents.
- **Verification is documentable.** A `RELEASE-VERIFICATION.md` doc at repo
  root explains the verification flow:
  `gh attestation verify <file> --repo <repo>`.
- **No new Makefile target.** Attestations are CI-only — they are produced
  by GitHub-hosted runner identity, not reproducible locally. The Makefile
  is unchanged; the three workflows gain steps; ci-parity-check skips the
  attestation steps via an allowlist.
- **Failure is loud.** A failed attestation step fails the release job. A
  silent skip is a regression.

## 3. Scope

### In scope

- `release-axiomregent.yml`: SBOM + provenance per binary in the `publish`
  job, after artifact download, before `gh release create`.
- `release-desktop.yml`: SBOM + provenance per installer artifact, after
  Tauri Action completes, before SHA-256 sidecar generation. Provenance is
  per-installer file (DMG, AppImage, NSIS).
- `release-tools.yml`: SBOM + provenance per archive (`oap-tools-*.tar.gz`,
  `oap-tools-*.zip`).
- `RELEASE-VERIFICATION.md` documenting the verification commands.
- `ci-parity-check` allowlist update for the attestation step names so the
  parity gate doesn't flag them as missing-from-Makefile.

### Out of scope

- Cosign signing of binaries. GitHub's attest-build-provenance uses
  Sigstore under the hood; a separate cosign workflow is unnecessary for
  v1.
- Reproducible builds (bit-for-bit identical rebuilds). A future spec.
- Container image attestations. Stagecraft and deployd-api images already
  flow through `cd-{stagecraft,deployd-api-rs}.yml`; image attestations are
  a candidate follow-up but out of scope here (this spec covers release
  binaries only).
- Distribution-time trust anchors (Homebrew, winget). Out of scope.

## 4. Workflow Shape

### 4.1 release-axiomregent.yml (publish job)

After `actions/download-artifact` and before `gh release create`, insert:

```yaml
- name: Generate SBOMs (CycloneDX)
  uses: anchore/sbom-action@<pinned-sha>  # v0
  with:
    path: dist/
    format: cyclonedx-json
    output-file: dist/sbom-axiomregent.cdx.json

- name: Attest build provenance
  uses: actions/attest-build-provenance@<pinned-sha>  # v2
  with:
    subject-path: 'dist/axiomregent-*'
```

The attestation files are uploaded by the action to GitHub's attestation
store and surfaced as release assets via:

```yaml
- name: Attach attestations + SBOMs to release
  run: |
    gh attestation download --repo "$GITHUB_REPOSITORY" --predicate-type \
      'https://slsa.dev/provenance/v1' dist/axiomregent-* --output-dir dist/
    gh release upload "$TAG" dist/sbom-axiomregent.cdx.json dist/*.intoto.jsonl \
      --repo "$GITHUB_REPOSITORY" --clobber
```

### 4.2 release-desktop.yml (per-target release job)

Inserted after the Tauri Action and before the SHA-256 sidecar generator:

```yaml
- name: Generate installer SBOMs
  uses: anchore/sbom-action@<pinned-sha>
  with:
    path: apps/desktop/src-tauri/target
    format: cyclonedx-json
    output-file: apps/desktop/src-tauri/target/sbom-desktop-${{ matrix.target }}.cdx.json

- name: Attest desktop installer provenance
  uses: actions/attest-build-provenance@<pinned-sha>
  with:
    subject-path: |
      apps/desktop/src-tauri/target/**/release/bundle/dmg/*.dmg
      apps/desktop/src-tauri/target/**/release/bundle/appimage/*.AppImage
      apps/desktop/src-tauri/target/**/release/bundle/nsis/*.exe
```

### 4.3 release-tools.yml (publish job)

After archives are packaged and before `gh release upload`:

```yaml
- name: Generate tool SBOMs
  uses: anchore/sbom-action@<pinned-sha>
  with:
    path: dist/
    format: cyclonedx-json
    output-file: release/sbom-tools.cdx.json

- name: Attest tool provenance
  uses: actions/attest-build-provenance@<pinned-sha>
  with:
    subject-path: 'release/oap-tools-*'
```

## 5. Verification Flow (RELEASE-VERIFICATION.md)

A standalone markdown doc at repo root, linked from README.md and from the
release notes template, documents:

```bash
# Verify provenance
gh attestation verify path/to/axiomregent-aarch64-apple-darwin \
  --repo stagecraft-ing/open-agentic-platform

# Inspect SBOM
jq '.components | length' sbom-axiomregent.cdx.json
```

Plus the equivalent flow without `gh` (using `cosign verify-blob` against
the Sigstore Rekor log).

## 6. Acceptance Criteria

- **AC-1:** A push of `axiomregent-v0.0.1` (or `workflow_dispatch`)
  produces a GitHub Release with: per-target binaries, per-target
  `*.intoto.jsonl` attestations, and a `sbom-axiomregent.cdx.json`.
- **AC-2:** A push of `v0.0.1` produces a GitHub Release with: three
  desktop installers, three matching `*.intoto.jsonl` files, three
  per-target SBOMs, AND the existing `*.sha256` sidecars (regression
  guard — the new flow does not displace the updater integrity hashes).
- **AC-3:** `release-tools.yml` follow-up run produces a `sbom-tools.cdx.json`
  and per-archive attestations, attached to the same release as `release-desktop`
  (sequencing is handled in spec H Phase 3.2 / M8 of the parent plan).
- **AC-4:** `gh attestation verify` against any released binary returns
  `Verification succeeded` with provenance pointing to the correct
  workflow, repo, and commit SHA.
- **AC-5:** `make ci-parity` does not flag the new workflow steps as
  missing-from-Makefile (the parity allowlist explicitly exempts steps
  whose name matches `^(Generate|Attest).*` in release-* workflows).
- **AC-6:** SBOM components count > 0 for every emitted SBOM
  (sanity check that the action ran against a populated artifact dir).

  **Lessons from the smoke runs (2026-04-29):**

  1. Pointing `anchore/sbom-action` at the staged `dist/` of stripped
     release binaries yields a zero-component SBOM because syft cannot
     recover crate metadata from stripped Rust binaries. Scope `path:` to
     the source tree (e.g. `crates/axiomregent`, `apps/desktop`, `tools/`)
     where `Cargo.toml` and `Cargo.lock` give syft something to
     enumerate.
  2. Source-tree scope still returned 0 components on
     `axiomregent-v0.0.1-attestation-smoke` because `anchore/sbom-action@v0.17.8`
     ships syft 1.11.1, whose Rust cataloger silently no-ops on `Cargo.lock`
     in some directory shapes. Local syft 1.43.0 against the same path
     returned 661 components. Action MUST be pinned to v0.24.0+ (ships
     syft 1.30+).

## 7. Risks and Mitigations

- **Risk:** `attest-build-provenance` requires
  `permissions: id-token: write, attestations: write, contents: write`.
  Adding these to the release jobs widens permission surface.
  **Mitigation:** scoped per-job, not workflow-wide. Release workflows are
  already write-scoped (`contents: write`) so the marginal escalation is
  the id-token + attestations grants, both narrowly used.

- **Risk:** SBOM generation against a Tauri target dir produces a very
  large JSON (every transitive crate + every npm package).
  **Mitigation:** acceptable size cost (single MB scale). Compression
  applied via `gh release upload`'s default. If size becomes a problem,
  switch SBOM scope from `path: target/` to `format: spdx-json` + a path
  filter.

- **Risk:** Sigstore transparency log is unreachable, breaking the
  attestation step.
  **Mitigation:** known historical incidents resolve in minutes;
  attest-build-provenance retries internally. If sustained, the release
  job fails — preferable to silently shipping unattested binaries.

- **Risk:** A consumer running an older `gh` CLI without `gh attestation
  verify` cannot validate.
  **Mitigation:** RELEASE-VERIFICATION.md documents both `gh` and direct
  `cosign verify-blob` paths. The `gh` requirement is `>= 2.50`.

## 8. Sequencing With M8 (release-tools workflow_run trigger)

The parent plan's Phase 3.2 (M8) converts `release-tools.yml`'s trigger
from `push: tags` to `workflow_run` on `Release Desktop` completion. This
spec's release-tools changes (§4.3) MUST land alongside that trigger
change in a single commit, so the SBOM/attestation steps run against the
correct release object. The parity-check allowlist update (§4 last bullet)
is included.

## 9. Pre-Public-Release Posture

The parent plan flags this spec as "Phase 3 before a public release."
Concretely: no `vN.0.0` GitHub Release tag is pushed until this spec is
implemented end-to-end and AC-1 through AC-4 are demonstrated on a test
tag (`v0.0.0-attestation-smoke` or similar). The pre-public-release
ordering is the load-bearing constraint of this spec.
