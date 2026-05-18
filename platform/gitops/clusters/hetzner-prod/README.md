# Hetzner production cluster — Flux v2 GitOps tree

This tree is the declared state for the OAP Hetzner production cluster (`oap-hetzner` per [`platform/infra/hetzner/cluster.yaml`](../../../infra/hetzner/cluster.yaml)). Flux v2 runs in-cluster, watches this path, and reconciles HelmReleases, Kustomizations, Certificates, and (post-spec-153) SOPS-encrypted Secrets continuously.

> **Spec spine:** authority for what lands here is [spec 151 — declarative cluster reconciliation](../../../../specs/151-declarative-cluster-reconciliation/spec.md). The plan-time split decision is in [`plan.md`](../../../../specs/151-declarative-cluster-reconciliation/plan.md): chart-contract + CD git-write is spec 152; per-purpose Secret migration is spec 153.

## Version pair (operator-facing pin)

**Phase 1 prep state (this PR):**

| Component | Current pin | Target for T-007 closure |
|---|---|---|
| K3s | `v1.31.4+k3s1` | `v1.33.11+k3s1` (latest stable 1.33.x as of 2026-05-18) |
| Flux v2 | not bootstrapped | `v2.8.7` |

`cluster.yaml`'s `k3s_version` lives at the current pin; the bump pairs atomically with `flux bootstrap` in T-007's PR per [spec 151 `execution/dr-baseline.md` §F4](../../../../specs/151-declarative-cluster-reconciliation/execution/dr-baseline.md) — landing the bump alone would risk an operator-initiated cluster recreation between PRs producing a Flux-less v1.33 cluster.

Bumping either side post-T-007: open a PR that updates the matching edits and re-runs the DR Stage 2 exercise (spec 151 SC-003).

## Layout

```
platform/gitops/clusters/hetzner-prod/
├── README.md                  (this file — version pin + tree map)
├── flux-system/               (omitted from VCS until `flux bootstrap`
│                               creates gotk-components.yaml + gotk-sync.yaml
│                               + kustomization.yaml on T-007)
├── infrastructure/            (operational HelmReleases — spec 151 Phases 2–4)
├── manifests/                 (raw Kubernetes manifests — Certificates,
│                               ClusterIssuers, namespace bootstraps)
└── secrets/                   (SOPS-encrypted Secret manifests — placeholder
                                until spec 153 lands FR-005 end-to-end)
```

The `flux-system/` directory is intentionally absent from the initial scaffold. `flux bootstrap github --owner=stagecraft-ing --repo=open-agentic-platform --path=platform/gitops/clusters/hetzner-prod` populates it itself on first bootstrap; pre-creating it would conflict with the bootstrap-generated kustomization.

## Single-cluster v1 shape

Per spec 151 §Clarification 6, the v1 layout is **flat single-cluster**, not the canonical Flux `clusters/<env>/` ↔ `infrastructure/<env>/` ↔ `apps/<env>/` overlay shape. Kustomize-compatible naming throughout — extracting overlays later is a refactor, not a redesign. Spec 151 §FR-010 records the constraint.

## Phase mapping

| Phase | Tasks | What lands here |
|---|---|---|
| 1 (this PR) | T-001…T-004 (scaffold only) | Directory README placeholders; `flux-system/` deferred to T-007 |
| 2 | T-008…T-013 | `infrastructure/reflector.yaml` + `manifests/tenants-wildcard-certificate.yaml` (unblocks spec 137 Phase 6) |
| 3 | T-014…T-018 | `infrastructure/cert-manager.yaml`, `manifests/cert-manager-clusterissuers.yaml`, `infrastructure/ingress-nginx.yaml` |
| 4 | T-019…T-021 | `infrastructure/rauthy.yaml` |
| 5 | T-022…T-026 | drift-detection + DR-runbook evidence (no manifests; the cluster is already declarative) |

Application HelmReleases (stagecraft, deployd-api, tenant-hello) land in spec 152 under a sibling `apps/` subtree to be created in 152's first PR. SOPS-encrypted application Secrets land in spec 153.

## Adding a new manifest

1. Drop the YAML into the appropriate subdirectory.
2. Update that subdirectory's README index table.
3. Run `kubectl --dry-run=client -k platform/gitops/clusters/hetzner-prod/<subdir>/` locally if the subdirectory carries a `kustomization.yaml`; otherwise `kubectl --dry-run=client -f <file>` per manifest.
4. PR the change. Flux's CI gate (spec 116 `cargo-deny` / `pnpm audit` / `npm audit` family does **not** cover Kubernetes manifests; a Kustomize-build gate is queued as a future hardening — see spec 151 §future-hardening once filed).
5. After merge, `flux reconcile kustomization <name> --with-source` on the operator workstation to short-circuit the reconcile interval.

## Out of scope

- `flux-system/` content (owned by `flux bootstrap`).
- Per-purpose application Secret YAMLs (spec 153).
- Application image rollouts (spec 152 — CD writes image-tag commits, never touches the cluster).
