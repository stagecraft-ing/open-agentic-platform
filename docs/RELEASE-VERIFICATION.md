# Release Artifact Verification

Every binary published from this repository ships with two governed
attestations:

1. **CycloneDX SBOM** (`*.cdx.json`) — a complete inventory of every
   crate, npm package, and grammar that went into the artifact.
2. **SLSA build-provenance attestation** — a Sigstore-signed claim
   asserting "this binary was built by `<repo>`, by workflow `<name>`,
   from commit `<sha>`, on a GitHub-hosted runner."

These are produced by `release-axiomregent.yml`, `release-desktop.yml`,
and `release-tools.yml`. Specification: [`117-release-artifact-attestations`](specs/117-release-artifact-attestations/spec.md).

## Verifying provenance

Requires `gh` CLI version 2.50 or later.

```bash
# Verify a specific binary
gh attestation verify path/to/axiomregent-aarch64-apple-darwin \
  --repo stagecraft-ing/open-agentic-platform

# Expected output:
# Loaded digest sha256:... for file://...
# Loaded 1 attestation from GitHub API
# - Verification succeeded!
#
# The following policy criteria will be enforced:
# - Source Repository Owner URI: https://github.com/stagecraft-ing
# - Source Repository URI:       https://github.com/stagecraft-ing/open-agentic-platform
# - Predicate type:              https://slsa.dev/provenance/v1
# - Subject Alternative Name:    https://github.com/stagecraft-ing/open-agentic-platform/.github/workflows/release-axiomregent.yml@refs/tags/...
```

## Verifying without `gh`

The attestations live in the public Sigstore Rekor transparency log.

```bash
# Compute the artifact digest
DIGEST=$(sha256sum axiomregent-aarch64-apple-darwin | awk '{print $1}')

# Find the Rekor entry
rekor-cli search --sha "sha256:$DIGEST"

# Inspect the entry
rekor-cli get --uuid <uuid-from-search> --format json
```

## Inspecting the SBOM

```bash
# How many components?
jq '.components | length' sbom-axiomregent.cdx.json

# What licenses?
jq -r '.components[] | "\(.name) \(.version) \(.licenses[0].license.id // "unknown")"' \
  sbom-axiomregent.cdx.json | sort -u
```

## Updater integrity (desktop only)

The desktop installer flow additionally publishes per-installer
SHA-256 sidecars (`*.dmg.sha256`, `*.exe.sha256`, `*.AppImage.sha256`).
These are consumed by the in-app updater
(`apps/desktop/src-tauri/src/commands/updater.rs`) — they are an
integrity check, not a provenance check, and exist for offline
update validation. They coexist with the SLSA attestations.
