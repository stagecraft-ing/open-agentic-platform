---
id: "151-declarative-cluster-reconciliation"
title: "Declarative cluster reconciliation — GitOps for cluster-side state via Flux v2"
status: draft
implementation: pending
owner: bart
created: "2026-05-17"
kind: platform
risk: high
depends_on:
  - "087"  # unified-workspace-architecture (stagecraft is the operator surface; this defines how its operator actions reach the cluster)
  - "143"  # presigned-upload-public-endpoint (FU-008 names the setup.sh-monolith seam this spec retires)
code_aliases: ["GITOPS_RECONCILIATION"]
implements: []  # populated as phases land
summary: >
  Replace `platform/infra/hetzner/setup.sh`'s imperative cluster-mutation
  monolith with a declarative GitOps reconciliation layer. Flux v2 runs
  in-cluster, watches `platform/gitops/`, and reconciles HelmReleases,
  Kustomizations, Certificates, and SOPS-encrypted Secrets continuously.
  setup.sh shrinks to one-time bootstrap. Application image rollouts stay
  on the existing CD workflow (image-push → `kubectl rollout restart`).
  Phase 2 unblocks spec 137 Phase 6 by landing the reflector + wildcard-cert
  annotations as the first declarative reconciliation under the new tree.
---

# Feature Specification: Declarative cluster reconciliation via Flux v2 GitOps

**Feature Branch**: `151-declarative-cluster-reconciliation`
**Created**: 2026-05-17
**Status**: Draft
**Input**: When PR #157 added a Helm install (`kubernetes-reflector`) and a
Certificate-manifest annotation change, neither reached the cluster on
merge — the canonical path is "re-run setup.sh," a monolith that interleaves
cluster create, helm installs, kubectl creates, secret materialisation,
stagecraft rollouts, and GitHub Actions sync. Re-running it has known
side effects (FU-009 CronJob clobber, force-roll of stagecraft-api).
Spec 143 FU-008 already named this as Hit #2 of 4 on the same structural
seam. The fix is not "decompose setup.sh into smaller imperative scripts"
— that just makes a smaller monolith. The fix is to invert the dependency:
cluster mutations depend on declarative state in git, and an in-cluster
controller reconciles continuously. PR-merge → cluster-converged becomes
the loop, not "PR-merge → operator re-runs script."

## Purpose and charter

Application image deploys already flow through CD (image-push triggers
`kubectl rollout restart` against the existing Deployment). That loop
is tight and we are keeping it. The gap is **cluster declarative state**
— Helm releases, Custom Resources, Certificates, ConfigMaps, the
per-purpose Secrets the FU-008 / FU-003 follow-up class struggles with —
which today is materialised by `platform/infra/hetzner/setup.sh` only when
an operator re-runs it. This spec defines the inversion: declarative state
lives in `platform/gitops/`, Flux v2 reconciles continuously from there,
and setup.sh stops being modified for every infrastructure delta.

The principle is **declarative over imperative for cluster state**.
Imperative scripts may exist for one-time bootstrap (cluster creation,
Flux installation, the git-secret handshake) but not for ongoing
reconciliation. The git ref is the cluster's intended state.

**Explicitly in scope:**

- Stand up Flux v2 (`source-controller`, `helm-controller`,
  `kustomize-controller`, `notification-controller`) on the existing
  Hetzner cluster, bootstrapped against this repository.
- A `platform/gitops/` tree of HelmRelease, HelmRepository, Kustomization,
  Certificate, and SOPS-encrypted Secret manifests. The complete declared
  cluster state is readable from this tree at any git ref.
- Migrate existing cluster-side concerns (reflector — new; cert-manager —
  existing; ingress-nginx — existing; rauthy chart — existing; stagecraft
  chart — existing; deployd-api chart — existing; tenant-hello chart
  hosting; per-purpose Secrets) into the gitops tree, one PR per
  concern, with `setup.sh` shrinking as each lands.
- SOPS-encrypted Secret manifests as the canonical path for per-purpose
  M2M credentials. The cluster holds the SOPS age private key; git holds
  encrypted ciphertext only.
- Drift detection: Flux events + Prometheus metrics surface drift.
  Stagecraft UI surfacing of drift is a follow-up, not this spec.
- Disaster recovery: re-bootstrapping Flux against the same git ref
  reconverges the cluster to declared state.

**Explicitly out of scope:**

- Application image rollouts (stays on the existing per-service CD
  workflows: `cd-stagecraft.yml`, `cd-deployd-api-rs.yml`,
  `cd-tenant-hello.yml`). Image tags do not flow through git.
  Flux Image Reflector / Image Update Automation may be adopted later
  as a separate spec; this one keeps the image-push → rollout-restart
  loop unchanged.
- Tenant resources (Deployments, Services, Ingresses rendered by
  deployd-api at tenant-deploy time). Those remain deployd-api owned;
  this spec governs only platform infrastructure.
- Multi-cluster federation (one Flux instance per cluster; the gitops
  tree supports Kustomize overlays for per-cluster differences but each
  cluster pulls independently).
- Stagecraft UI for editing cluster state. Operators still author YAML
  in PRs; the PR review is the governance surface.
- Replacing terraform for cluster creation (terraform stays the entry
  point; Flux takes over after `kubectl` is reachable).
- Cross-region disaster recovery (single-region per cluster for v1).

## Current state vs intent

**Current state (2026-05-17):**

- `platform/infra/hetzner/setup.sh` (≈400 lines, growing) interleaves:
  cluster create via `hcloud-cli` + `kubeone`, helm installs (cert-manager,
  ingress-nginx, rauthy, reflector as of #157), `kubectl create secret`
  calls (rauthy-secrets, deployd-api-secrets, per-purpose M2M creds),
  stagecraft-api rollout, deployd-api helm-managed rollout, GitHub Actions
  secret sync. One sequential, partially-idempotent script.
- `platform/infra/hetzner/post-create.sh` (called by setup.sh) carries
  additional concerns including the FU-009 CronJob clobber at lines
  419-422 that fires on every re-run.
- Three application-level CD workflows handle image rollouts cleanly:
  `cd-stagecraft.yml`, `cd-deployd-api-rs.yml`, `cd-tenant-hello.yml`.
  These work and are not the seam this spec addresses.
- `platform/charts/` holds Helm charts for stagecraft, deployd-api,
  rauthy, tenant-hello, oauth2-proxy-gate. They are referenced from
  setup.sh by helm CLI invocation.
- `platform/infra/hetzner/manifests/` holds raw Kubernetes manifests
  (currently the wildcard Certificate from spec 106 §12.4 / spec 137 T076).
  Applied by `kubectl apply` from setup.sh.
- Every cluster-side delta in a PR (Helm chart change, new manifest,
  new Secret) requires an operator to re-run setup.sh after merge.
  PR merge does not converge the cluster.

**Intent (this spec's aspiration):**

- `platform/gitops/` tree of declarative manifests. The complete intended
  state of the Hetzner cluster (and any future cluster) is readable from
  this tree at any git ref.
- A Flux v2 instance running in `flux-system` namespace, bootstrapped
  against this repository, watching `platform/gitops/clusters/hetzner/`
  (or the equivalent cluster-specific path).
- Every PR merge that touches `platform/gitops/` triggers Flux
  reconciliation within minutes. No operator intervention to converge.
- `platform/infra/hetzner/setup.sh` reduced to a thin one-time-bootstrap
  script: `hcloud-cli` cluster create → kubeconfig → `flux bootstrap`
  with this repo + path → done. Target line count: under 100 lines.
- A drifted cluster (e.g. someone hand-edits a Deployment) is reverted
  by Flux within the reconciliation interval.
- Disaster recovery: re-create cluster → `flux bootstrap` against the
  same git ref → cluster reconverges to declared state. No "rebuild
  setup.sh state machine" path.
- Spec 143 FU-008 closes by reference: per-purpose Secrets materialise
  via SOPS-encrypted YAML in git; the setup.sh-monolith seam is retired
  for the secret-sync class.

## Cluster mutation contract *(normative)*

Any cluster mutation that this platform owns MUST follow one of three
paths. The path is determined by the kind of mutation, not by operator
preference.

- **M-001 (application image rollouts):** Image tag bumps for
  `stagecraft-api`, `deployd-api`, `tenant-hello`, and any future
  in-tree application service MUST flow through the existing per-service
  CD workflow (image-push to GHCR → `kubectl rollout restart` against
  the existing Deployment). These mutations DO NOT pass through Flux.
  Rationale: tight loop, immutable image-tag-per-commit, no benefit
  from declarative reconciliation in the inner loop.
- **M-002 (declarative cluster state):** Helm releases (versions,
  values), Kubernetes manifests (Certificates, ConfigMaps, RBAC, CRDs,
  Custom Resources), and SOPS-encrypted Secrets MUST be expressed as
  YAML under `platform/gitops/` and reconciled by Flux. No `helm install`
  or `kubectl apply` for these mutations from any operator-side script
  after bootstrap. PR merge is the only mutation channel.
- **M-003 (one-time cluster bootstrap):** Cluster creation,
  kubeconfig handoff, Flux installation, and the initial git-secret
  handshake MAY run imperatively from `platform/infra/<cloud>/setup.sh`.
  Once Flux is running and watching the repo, the bootstrap script's
  job is done; further mutations follow M-002.

Per-purpose Secrets (the FU-008 class) fall under **M-002** via SOPS-
encrypted manifests, not under M-003. The setup.sh imperative
`kubectl create secret` path is retired once the SOPS migration lands.

Constraints on the contract:

- **C-001 (single source of truth):** The complete intended cluster
  state is derivable from one git ref + `platform/gitops/` + the
  SOPS age key held in-cluster. No state lives in operator workstations,
  shared drives, or out-of-band notes.
- **C-002 (drift recovery):** Flux MUST revert manual cluster mutations
  to the declared state within one reconciliation interval. Operators
  changing the cluster outside the gitops tree is a recoverable error,
  not a permanent fork.
- **C-003 (idempotent reconciliation):** Applying the current git state
  to a clean cluster MUST produce identical resources to applying it to
  a partially-converged cluster. Flux's reconciliation model already
  guarantees this; the spec records the requirement so future
  contributions don't introduce non-idempotent CRDs or post-install
  hooks that violate it.
- **C-004 (no cluster credentials in CI):** Flux pulls from git; CI does
  not push to the cluster (except for M-001 image rollouts, which use
  scoped per-service kubeconfigs already in CI). Adding cluster-admin
  kubeconfig to GitHub Actions for ad-hoc apply is explicitly
  prohibited by this contract.

## Functional Requirements *(MVP)*

- **FR-001:** A Flux v2 instance MUST run in the `flux-system` namespace
  on every cluster the platform operates. Components: `source-controller`,
  `kustomize-controller`, `helm-controller`, `notification-controller`.
  Pinned to a specific Flux version; bumps are deliberate edits.
- **FR-002:** `platform/gitops/` MUST contain the complete declarative
  cluster state for every cluster the platform operates. Per-cluster
  differences MUST be expressible as Kustomize overlays under
  `platform/gitops/clusters/<cluster-name>/` without forking the base
  tree.
- **FR-003:** `platform/infra/hetzner/setup.sh` MUST reduce to under
  100 lines covering only one-time bootstrap (cluster create, kubeconfig,
  `flux bootstrap`). All ongoing reconcile concerns MUST migrate into
  the gitops tree before this spec closes.
- **FR-004:** Application image rollouts MUST remain on the existing
  per-service CD workflow. This spec MUST NOT modify
  `cd-stagecraft.yml`, `cd-deployd-api-rs.yml`, or `cd-tenant-hello.yml`
  in ways that change the image-tag rollout path. Spec 137 Phase 6
  closure does not depend on Image Automation.
- **FR-005:** Per-purpose Secrets MUST materialise via SOPS-encrypted
  manifests under `platform/gitops/`. The age private key MUST be held
  in `flux-system` namespace at bootstrap time and not stored in git
  in any form. `kubectl create secret` MUST NOT appear in setup.sh or
  post-create.sh after Phase 5 lands.
- **FR-006:** Flux drift detection MUST emit cluster events for every
  reconciliation; Prometheus metrics MUST expose reconciliation success
  rate, drift count, and last-reconcile-time per resource. Stagecraft
  surfacing of these signals is a follow-up, not this spec.
- **FR-007:** Disaster recovery MUST be expressible as a runbook of:
  recreate cluster via terraform/setup.sh → `flux bootstrap` against
  the same git ref → cluster converges to declared state. The runbook
  MUST be exercised at least once before this spec closes.
- **FR-008:** The reflector Helm release and the spec 137 wildcard-cert
  annotations MUST be the first migrations under the new tree (Phase 2).
  This unblocks spec 137 Phase 6 evidence collection.
- **FR-009:** Migration of existing cluster state into gitops MUST be
  incremental — one PR per concern (cert-manager, ingress-nginx, rauthy,
  stagecraft chart, deployd-api chart, tenant-hello chart, per-purpose
  Secrets). Each PR MUST be independently mergeable and the cluster
  MUST remain operational after each merge.
- **FR-010:** Multi-cluster support MUST be achievable by adding a new
  `platform/gitops/clusters/<cluster>/` overlay without modifying the
  base manifests. Spec 072 (multi-cloud-k8s-portability) benefits from
  this without itself being a precondition.

## Success Criteria

- **SC-001:** Modifying a HelmRelease YAML under
  `platform/gitops/clusters/hetzner/` and merging to main results in
  Flux reconciling the change within 5 minutes without operator
  intervention. Demonstrated via the reflector or cert-annotation
  Phase 2 deltas.
- **SC-002:** `platform/infra/hetzner/setup.sh` line count drops from
  the current ≈400 to under 100 lines, covering only bootstrap. The
  reduction is verified by `wc -l` at spec closure.
- **SC-003:** A second cluster can be bootstrapped from scratch via
  the runbook in under 30 minutes of operator wall-clock time,
  arriving at a converged state matching the declared gitops tree.
  Evidence: timed disaster-recovery exercise on a throwaway cluster.
- **SC-004:** No `kubectl create secret`, `helm upgrade --install`, or
  `kubectl apply -f` invocations remain in `setup.sh` /
  `post-create.sh` for runtime cluster state. Verified by grep at spec
  closure.
- **SC-005:** A manual `kubectl edit deployment` against a Flux-managed
  resource is reverted within one reconciliation interval. Evidence:
  a deliberate drift test recorded in `execution/verification.md`.
- **SC-006:** Spec 143 FU-008 closes by reference to SC-006 here:
  per-purpose Secret materialisation no longer requires re-running
  setup.sh; the seam is retired structurally for the M2M Secret class.
- **SC-007:** Spec 137 Phase 6 evidence E1–E6 is collected against a
  cluster where the spec 137 reflector + cert annotations were
  reconciled by Flux from `platform/gitops/`, not applied by setup.sh.

## Out of scope (MVP)

- **Image automation in Flux** — image-push to GHCR continues to drive
  app rollouts via the existing CD workflows. Adopting
  `image-reflector-controller` + `image-automation-controller` is a
  future spec, not this one.
- **Tenant resource reconciliation** — deployd-api remains the renderer
  for tenant Deployments / Services / Ingresses at tenant-deploy time.
  Migrating tenant rendering to Flux is a different conversation.
- **Replacing terraform** — cluster creation stays on terraform / hcloud-cli
  / kubeone. Flux installs after `kubectl` is reachable.
- **Stagecraft UI for declared state** — operators continue to edit YAML
  in PRs; PR review is the governance surface. A future spec may add a
  stagecraft view over the gitops tree, but authoring stays in git.
- **Cross-region disaster recovery** — single-region per cluster for v1.
  Multi-region is a different problem class.
- **Multi-tenant Flux** — one Flux instance per cluster with cluster-admin
  scope. Tenant-scoped Flux (per-namespace reconciliation policies) is
  out of scope; tenants don't operate clusters.

## Clarifications

The clarifications below are real open decisions; each is load-bearing
and Phase 0 closes when all are pinned. Recommended answers are recorded
inline as starting points for reviewer discussion, not as decisions.

### Session 2026-05-17

1. **Reconciliation tool: Flux v2 vs Argo CD vs other?** Recommend
   **Flux v2**: lighter-weight, Helm-native (HelmRelease is a first-class
   CRD), no dashboard server to operate, aligns with our minimal-binary
   aesthetic. Argo CD is heavier, multi-tenant-oriented, and would add a
   UI surface that overlaps stagecraft's role. Other tools (Rancher Fleet,
   Werf) are off the table — not enough adoption / CNCF momentum.
2. **Secrets approach: SOPS vs Sealed Secrets vs External Secrets Operator?**
   Recommend **SOPS with age**: git-native, no in-cluster controller
   dependency beyond Flux's built-in SOPS support, no external Secret
   Manager required. Sealed Secrets requires a dedicated controller and
   per-cluster key management that complicates disaster recovery.
   External Secrets Operator pulls Anthos/Vault/AWS Secrets Manager —
   useful long-term but adds a hard external dependency we don't need
   for the FU-008 use case (M2M creds we already hold locally).
3. **Repo topology: monorepo (this repo) vs separate gitops repo vs
   branch?** Recommend **monorepo with `platform/gitops/` tree**.
   Aligns with constitution Principle V (legacy inputs non-normative,
   one canonical truth) and the existing spec/code coupling model.
   Separate gitops repo would split the spec-to-code traceability the
   codebase-indexer relies on. Branch-based gitops is fragile.
4. **Bootstrap shape: `flux bootstrap` vs Terraform-installed Flux vs
   setup.sh wrapper?** Recommend **`flux bootstrap`** as the canonical
   Flux pattern, invoked once from the shrunk setup.sh after cluster
   creation. Terraform-installed Flux would couple Flux lifecycle to
   terraform state (over-coupling). setup.sh wrapper-with-helm-install
   re-implements `flux bootstrap` worse.
5. **App image CD relationship: keep imperative `kubectl rollout restart`
   vs Flux Image Automation?** Recommend **keep imperative for v1**.
   Image rollouts are a tight inner loop where image-tag-per-commit
   immutability + a CD workflow that restarts is faster and simpler
   than Flux image-automation-controller pulling tag updates. Migrating
   to Flux image automation is a future spec if multi-cluster makes the
   current pattern brittle.
6. **Multi-cluster topology: Kustomize overlays vs per-cluster
   directories vs per-cluster branches?** Recommend **Kustomize overlays
   under `platform/gitops/clusters/<cluster>/`** with a shared base tree.
   Per-cluster directories duplicate; per-cluster branches diverge over
   time. Overlays let cluster-specific values (domain, replicas, region)
   override the base without forking.
7. **Drift surfacing: cluster events + Prometheus only vs stagecraft UI
   vs Slack/PagerDuty?** Recommend **cluster events + Prometheus for v1**.
   These are the Flux defaults and require no additional integration.
   Stagecraft UI surfacing is a follow-up that uses the same Prometheus
   metrics; Slack/PagerDuty hooks into the existing notification stack
   are also a follow-up. Don't gate this spec on observability ergonomics.
8. **Migration ordering: which existing concerns migrate first vs last?**
   Recommend **reverse-risk ordering**: lowest-stakes first (reflector,
   which is new and standalone), then operational helpers (cert-manager,
   ingress-nginx), then identity (rauthy), then app-charts (stagecraft,
   deployd-api), then per-purpose Secrets (highest stakes, gated on
   SOPS path being solid). Each migration is one PR; setup.sh shrinks
   monotonically.

## Risks

- **R-001 (Flux as new SPOF):** Flux itself becomes critical
  infrastructure. If Flux is mis-bootstrapped or its controllers crash
  loop, cluster state stops converging. Mitigation: Flux has been
  battle-tested in production at large scale (CNCF graduated 2024-04);
  pin to a known-good version; the disaster recovery runbook covers
  Flux re-bootstrap.
- **R-002 (SOPS key custody):** The age private key in `flux-system`
  is the cluster's master decryption key. If lost, all SOPS-encrypted
  Secrets become inaccessible (existing values continue to work since
  Flux already decrypted them, but rotation breaks). Mitigation: the
  age key is also stored in a separate operator-controlled
  vault/keychain at bootstrap; FR-007 disaster recovery exercises the
  re-bootstrap path. A future spec may move to a managed KMS for the
  age key.
- **R-003 (migration mid-state operational risk):** While Phases 3–5
  are in flight, some concerns are managed by Flux and some by
  setup.sh. A re-run of setup.sh during this window would conflict
  with Flux-managed state. Mitigation: each migration PR includes the
  setup.sh edit to retire the relevant imperative block in the same
  commit; setup.sh cannot re-create what Flux owns.
- **R-004 (CRD ordering):** Flux installs HelmReleases that themselves
  install CRDs (cert-manager Issuer, Certificate; rauthy CRDs if any).
  CRD-before-CR ordering must be respected. Mitigation: Kustomization
  `dependsOn` and HelmRelease ordering primitives handle this; the
  Phase 3 cert-manager migration exercises the pattern first.
- **R-005 (spec 137 timing):** This spec exists in part to unblock
  spec 137 Phase 6. If Phases 0–2 here take longer than expected,
  spec 137 evidence is delayed. Mitigation: Phase 2 is intentionally
  scoped narrowly (reflector + cert annotations only — the two
  spec 137 deltas) so the unblock is concrete and bounded; full
  migration continues in parallel without blocking 137.

## Cross-references

- **Spec 087 (unified-workspace-architecture):** Stagecraft is the
  operator surface; this spec defines how stagecraft's operator
  actions (and PRs from any author) reach the cluster.
- **Spec 143 (presigned-upload-public-endpoint) FU-008:** This spec
  is the structural fix that retires the FU-008 seam. SC-006 here
  satisfies FU-008's intent.
- **Spec 137 (tenant-environment-access-gates):** Phase 6 evidence
  collection is unblocked by this spec's Phase 2.
- **Spec 105 (scripts-to-binaries-migration):** Same lineage — moving
  imperative scripts to governed structures. Spec 105 migrated tools
  to Rust binaries or Makefile recipes; spec 151 migrates cluster
  mutations to declarative manifests.
- **Spec 072 (multi-cloud-k8s-portability):** Beneficiary, not
  dependency — the gitops tree's overlay model makes multi-cloud
  cluster bootstrap consistent across providers.
- **Spec 089 (governed-convergence-plan):** Aligns with the
  convergence direction (governance non-optional; cluster mutations
  governed by spec-bound PRs, not operator-side scripts).
- **Spec 116 (supply-chain-policy-gates):** SOPS-encrypted Secrets in
  git are within the supply-chain envelope; the encryption discipline
  must satisfy 116's gates.

## Why this spec is filed as `draft`

The 8 clarifications above are load-bearing decisions that benefit from
one reviewer pass before lock-in. The recommendations are starting
points; Phase 0 closes when each is pinned (or amended with an
empirically-validated alternative). Until Phase 0 closes, the
`platform/gitops/` directory and the Flux installation MUST NOT be
created — the spec's body drives the implementation, not the other
way around (CONST-005).
