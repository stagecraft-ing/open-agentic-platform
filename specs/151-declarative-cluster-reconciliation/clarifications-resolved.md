# Spec 151 — Clarifications resolved

This document records the Phase 0 pin pass for spec 151. Each §Decision
matches the six-field schema declared in `spec.md` §Clarifications.

Phase 0 closure criteria (per spec.md §"Why this spec is filed as `draft`"):

- (a) Every §Decision below matches the six-field schema — **landed
  in this commit**.
- (b) SC-003 empirical bootstrap baseline measured and recorded in
  `execution/dr-baseline.md` — **pending operator action**.
- (c) Any placeholder pin (e.g. Clarification #9's password-manager
  vault path if substituted) committed to spec body verbatim —
  **§Decision 9 records the v1 1Password commit; substitution pending
  operator confirmation if different**.

When all three land, the spec lifecycle flips to `status: approved`
and plan.md + tasks.md follow in a sibling PR.

---

### §Decision 1 — reconciliation-tool

**Decision:** Flux v2 for declarative cluster reconciliation. Quoted
verbatim from spec.md §Clarification 1: "Recommend **Flux v2** for one
constitutional reason and a small set of tiebreakers. *Constitutional
reason:* Spec 087 establishes stagecraft as the platform's operator
surface. Argo CD ships a first-class operator dashboard that would
create a competing operator surface for cluster state — operators
would face two surfaces (stagecraft for tenant + governance, Argo for
cluster) instead of one. Flux v2 has no operator UI; its interface is
controller-set + cluster events + Prometheus metrics, leaving
stagecraft as the single operator surface per spec 087."

**Alternatives considered:**
- Argo CD — creates a competing operator surface to stagecraft (spec
  087); heavier runtime; multi-tenant features unused at our scale.
- Rancher Fleet — insufficient CNCF momentum / adoption to bet on for
  a long-lived control plane.
- Werf — insufficient CNCF momentum / adoption; not Helm-native as a
  first-class CRD.
- Do nothing (keep `setup.sh` imperative) — fails the FU-008 / FU-003
  seam-retirement requirement that motivates this spec.

**Rationale:** Constitutional alignment with spec 087's
single-operator-surface principle; Flux v2's interface (controllers +
cluster events + Prometheus metrics) leaves stagecraft as the only
operator UI, while Argo CD's dashboard would compete with it.

**Consequences:** If flipped to Argo CD post-implementation, spec 087
amends to acknowledge dual operator surfaces; stagecraft's roadmap
re-scopes around cluster-state-non-ownership; the Argo dashboard
itself becomes a governance surface that needs Rauthy OIDC
integration; the migration ordering (§Decision 8) reopens because
operational helpers might land differently.

**Review:** `single-author-self-pinned`

**Pinned:** 2026-05-17 by bart

---

### §Decision 2 — secrets-approach

**Decision:** SOPS with age for per-purpose Secrets. Quoted verbatim
from spec.md §Clarification 2: "Recommend **SOPS with age**: git-native,
no in-cluster controller dependency beyond Flux's built-in SOPS
support, no external Secret Manager required."

**Alternatives considered:**
- Sealed Secrets — requires a dedicated controller and per-cluster
  key management; complicates DR (the controller's signing key
  becomes a second master-secret problem).
- External Secrets Operator — pulls from external Secret Managers
  (Vault, AWS Secrets Manager, GCP Secret Manager); adds a hard
  external dependency we don't need at our scale for the FU-008 use
  case (M2M creds we already hold locally).
- Encrypt-in-cluster with rotating cluster keys — non-portable across
  clusters; doesn't satisfy C-001 (single source of truth in git).
- Do nothing (keep imperative `kubectl create secret` in setup.sh) —
  keeps the FU-008 / FU-003 seam open; defeats this spec's purpose.

**Rationale:** Lightest in-cluster dependency that satisfies M-002 +
C-001 + C-005; Flux's `kustomize-controller` has SOPS support built-in,
so adopting SOPS adds zero new controllers to the cluster.

**Consequences:** If flipped to ESO post-implementation, an external
Secret Manager becomes a hard dependency; DR runbook (FR-007, SC-003)
adds a "restore external Secret Manager" step that breaks SC-003's
30-min budget; multi-cloud portability (spec 072) becomes
cloud-Secret-Manager-coupled.

**Review:** `single-author-self-pinned`

**Pinned:** 2026-05-17 by bart

---

### §Decision 3 — repo-topology

**Decision:** Monorepo with `platform/gitops/` tree. Quoted verbatim
from spec.md §Clarification 3: "Recommend **monorepo with
`platform/gitops/` tree**. Aligns with constitution Principle V
(legacy inputs non-normative, one canonical truth) and the existing
spec/code coupling model. Separate gitops repo would split the
spec-to-code traceability the codebase-indexer relies on. Branch-based
gitops is fragile."

**Alternatives considered:**
- Separate gitops repo — splits spec/code coupling traceability the
  codebase-indexer relies on; spec/code coupling gate (spec 127)
  cannot follow a cross-repo link without rework.
- Branch-based gitops (e.g. a `gitops` branch in this repo) — fragile;
  diverges over time; complicates PR review which expects main as the
  reconciliation target.
- Per-environment repos — multiplies governance surface and review
  load; per-environment differences belong in Kustomize structure,
  not in repository structure.

**Rationale:** Constitution Principle V (one canonical truth); the
codebase-indexer (`tools/codebase-indexer/`) and the spec/code coupling
gate (`tools/spec-code-coupling-check/`) both assume one repo for
spec-to-code mapping. Splitting the gitops tree out forks those
mappings.

**Consequences:** If flipped to a separate gitops repo, the
codebase-indexer needs to follow a cross-repo link via submodule or
remote reference; spec/code coupling gate's diff-scanning becomes
two-repo; CODEOWNERS spans two repos for cluster changes.

**Review:** `single-author-self-pinned`

**Pinned:** 2026-05-17 by bart

---

### §Decision 4 — bootstrap-shape

**Decision:** `flux bootstrap` invoked once from the shrunk `setup.sh`
after cluster creation, with this repo + path arg naming
`platform/gitops/clusters/hetzner-prod/`. Quoted verbatim from spec.md
§Clarification 4: "Recommend **`flux bootstrap`** as the canonical
Flux pattern, invoked once from the shrunk setup.sh after cluster
creation. Terraform-installed Flux would couple Flux lifecycle to
terraform state (over-coupling). setup.sh wrapper-with-helm-install
re-implements `flux bootstrap` worse."

**Alternatives considered:**
- Terraform-installed Flux — couples Flux lifecycle to terraform state;
  Flux upgrades become a terraform plan/apply cycle rather than a
  Flux-native upgrade.
- `setup.sh` wrapper that helm-installs Flux directly — re-implements
  `flux bootstrap` (which handles git-credentials + Flux-system
  namespace + GitRepository CR atomically) worse and partially.
- Manual `flux install` + per-step git config — diverges over operators
  (different operators run different command sequences).

**Rationale:** `flux bootstrap` is the canonical Flux pattern; it is
idempotent, handles the Flux-system → git-credentials handshake
atomically, and is the documented disaster-recovery path.

**Consequences:** If flipped to terraform-installed Flux,
disaster-recovery (FR-007, SC-003) becomes a terraform-apply cycle
rather than a `flux bootstrap` invocation; the 30-min DR budget
recomputes; Flux upgrades become a separate terraform plan rather
than a `flux upgrade` invocation.

**Review:** `single-author-self-pinned`

**Pinned:** 2026-05-17 by bart

---

### §Decision 5 — app-image-cd-relationship

**Decision:** Image tags become declarative state in git. CD's
per-service workflow builds + pushes the image to GHCR first, then
commits the new `image.tag` value into the relevant HelmRelease values
file under `platform/gitops/clusters/hetzner-prod/` and pushes to main.
CD never touches the cluster. The full pin (eight sub-pins) is quoted
verbatim in spec.md §Clarification 5: chart-contract via
`cd-managed-images.yaml` (sub-pin i, with pinned schema and worked
example in spec body), failure-mode ordering (push image first, then
commit — sub-pin ii), first-image-deploy baseline as migration PR's
responsibility (sub-pin iii), per-service migration atomicity with
layered enforcement (sub-pin iv: reviewer + CODEOWNERS first-claim,
spec 127's coupling gate post-migration, future hardening named),
break-glass rollback semantics with mandatory 24h follow-up PR (sub-pin
v), commit-signing for CD-bot commits explicitly out of scope v1
(sub-pin vi, verified against spec 116 §3 in-scope list), CI/CD
permissions scoped via CODEOWNERS to values files only (sub-pin vii),
and rationale-over-driftDetection.ignore (sub-pin viii).

**Alternatives considered:**
- `spec.driftDetection.ignore` on image-field path — patch-over-conflict;
  models the field as "shared ownership" between Flux and CD with a
  polite truce rather than dissolving the dual-writer.
- Flux image automation (`image-reflector-controller` +
  `image-automation-controller`) — strictly stronger but adds
  ImagePolicy CRDs and tag-discovery complexity not needed for v1;
  named as a future spec.
- Mutable tags (`:latest`, `:main`) + `imagePullPolicy: Always` —
  sacrifices commit-pinned audit trail (immutable image tag per commit).
- Do nothing (keep current `helm upgrade --set image.tag=...`) —
  incompatible with Flux owning the HelmRelease (dual-writer fight).

**Rationale:** Single-writer pattern (Flux owns the image field via
HelmRelease values; CD is the upstream content producer that writes
to git) consistent with this spec's principle that declarative state
in git is the cluster's intended state. Dissolves the dual-writer
problem entirely instead of mediating it.

**Consequences:** If flipped to image-automation post-implementation,
the chart-contract evolves to declare ImagePolicy CRDs; CD loses
git-push permission to main; spec 116 supply-chain surface changes
(image-policy events become signing surface); spec 102's cert pipeline
reads image-policy events. If flipped back to imperative `helm upgrade
--set`, FR-004 and M-001 reopen and the FU-002 dual-writer hazard
re-fires per service.

**Review:** `single-author-self-pinned`

**Pinned:** 2026-05-17 by bart

---

### §Decision 6 — multi-cluster-topology

**Decision:** Flat single-cluster v1.
`platform/gitops/clusters/hetzner-prod/` holds the full declared state
directly, no `base/` + `overlays/` split. Quoted verbatim from spec.md
§Clarification 6: "Recommend **flat single-cluster v1**:
`platform/gitops/clusters/hetzner-prod/` holds the full declared state
directly, no `base/` + `overlays/` split. Kustomize is still the
renderer (Flux's `Kustomization` CR) but with a flat tree. Building
overlay machinery before a second cluster exists is the premature
abstraction the OAP discipline rejects elsewhere."

**Alternatives considered:**
- Kustomize overlays with `base/` + per-cluster overlays now — premature
  abstraction; we have one cluster and three similar lines beats a
  speculative abstraction.
- Per-cluster branches — diverge over time; merge conflicts grow with
  cluster count.
- Per-cluster repos — multiplies governance surface; conflicts with
  §Decision 3's monorepo pin.

**Rationale:** OAP discipline against premature abstraction (rejected
elsewhere — three similar lines is better than a speculative
abstraction); spec 072 brings the concrete second cluster target when
overlay extraction lands, driven by an actual second instance with
real differences to capture.

**Consequences:** If a second cluster appears before spec 072
implements overlays, this clarification reopens with the empirical
second-cluster shape (real differences in domain / replicas / region /
provider-specific values inform the `base/` extraction). If spec 072
lands first, the deferred extraction is mechanical because the flat
tree was kept kustomize-compatible per FR-010.

**Review:** `single-author-self-pinned`

**Pinned:** 2026-05-17 by bart

---

### §Decision 7 — drift-surfacing

**Decision:** Cluster events + Prometheus metrics for v1. Stagecraft
UI surfacing and Slack/PagerDuty hooks are follow-ups, not gates on
this spec's closure. Quoted verbatim from spec.md §Clarification 7:
"Recommend **cluster events + Prometheus for v1**. These are the Flux
defaults and require no additional integration. Stagecraft UI
surfacing is a follow-up that uses the same Prometheus metrics;
Slack/PagerDuty hooks into the existing notification stack are also a
follow-up. Don't gate this spec on observability ergonomics."

**Alternatives considered:**
- Stagecraft UI surfacing now — adds work not gated on Phase 0;
  deferred to a follow-up that uses the same Prometheus metrics.
- Slack / PagerDuty hooks now — adds notification stack integration
  work; deferred to a follow-up.
- All-of-the-above before spec closure — gates this spec on
  observability ergonomics, which it should not be.

**Rationale:** Flux defaults work; downstream surfacings (stagecraft
UI, Slack) all build on the same Prometheus signals; no reason to gate
v1 on layered observability that adds no new evidence beyond what
Prometheus already exposes.

**Consequences:** If cluster events prove insufficient operationally,
follow-up specs light up the same Prometheus signals on stagecraft UI
or Slack with no architectural rework. If Prometheus is replaced by
another metrics backend, the cluster-events fallback still surfaces
reconciliation outcomes.

**Review:** `single-author-self-pinned`

**Pinned:** 2026-05-17 by bart

---

### §Decision 8 — migration-ordering

**Decision:** Reverse-risk ordering. Lowest-stakes first (reflector,
new and standalone), then operational helpers (cert-manager,
ingress-nginx), then identity (rauthy), then app-charts (stagecraft,
deployd-api, tenant-hello), then per-purpose Secrets (highest stakes,
gated on SOPS path being solid). Each migration is one PR; setup.sh
shrinks monotonically. Quoted verbatim from spec.md §Clarification 8.

**Alternatives considered:**
- Forward-risk ordering (per-purpose Secrets first) — increases blast
  radius if SOPS bootstrap path has bugs; lower-stakes items don't
  inform the higher-stakes ones.
- Alphabetical / arbitrary — doesn't prioritise risk learning; each
  migration teaches less than it could.
- All-at-once mega-PR — violates FR-009 (incremental migration);
  blast radius is the whole cluster on a single PR.

**Rationale:** Lowest-risk items teach the migration shape (HelmRelease
authoring, Kustomization ordering, CRD-before-CR sequencing); higher-
risk items inherit that learning. Per-PR rollback granularity preserved.

**Consequences:** If a higher-risk item reveals an architectural
problem mid-migration (e.g. rauthy's Hiqlite storage needing a
StatefulSet-aware Kustomization), the lower-risk items are already
declarative and continue to serve as the cluster's reconcile surface;
rollback is per-PR with no cascading impact.

**Review:** `single-author-self-pinned`

**Pinned:** 2026-05-17 by bart

---

### §Decision 9 — sops-key-custody

**Decision:** Five sub-decisions pinned for SOPS key custody, quoted
verbatim from spec.md §Clarification 9:

- **(a) Pre-bootstrap location:** operator-host filesystem at
  `~/.config/sops/age/keys.txt` (mode `0600`, operator-only).
- **(b) Backup custody location:** **1Password — vault
  `OAP / Cluster Keys`, item `sops-age-hetzner-prod`, attachment
  `keys.txt`**. If the operator-of-record uses Bitwarden or a
  different password manager, this pin records the substitution with
  vault + item path verbatim. **Substitution pending operator
  confirmation before Phase 0 actually closes** — see Phase 0 criterion
  (c) at top of this document.
- **(c) Post-bootstrap location:** `sops-age` Secret in `flux-system`
  namespace on the live cluster (Flux's convention); runtime form,
  not a third copy of authority.
- **(d) Rotation:** OUT OF SCOPE v1; future spec "SOPS key rotation
  tooling" adds `make rotate-sops-key` that automates the re-encryption
  sweep. v1 compromise-recovery path is "fresh cluster + manual
  re-encryption sweep" (~1-day operation).
- **(e) Single-operator model:** v1 assumes single operator-of-record.
  Multi-operator custody (recipients list + offboarding contract) is
  a future spec "multi-operator SOPS custody."

**Alternatives considered:**
- Hardware key with `age-plugin-yubikey` — adds hardware dependency;
  v1 does not need that level of attestation.
- Cloud KMS (Hetzner-hosted? AWS KMS via boundary instance?) — adds
  external KMS dependency; doesn't satisfy C-001 (single source of
  truth in git) without a delegation layer that complicates the model.
- HashiCorp Vault — operates Vault in addition to Flux; adds a second
  long-lived stateful service to the platform stack. Out of proportion
  to v1's surface.
- Plain age key in git encrypted by repo-deploy-key — re-creates the
  bootstrap-secret problem at the deploy-key level without solving it.

**Rationale:** One custody decision (one named primary location + one
named backup) at smaller surface than N per-purpose credentials; the
bootstrap-secret problem is bounded to a single age key whose loss has
a bounded ~1-day manual recovery path.

**Consequences:** If the v1 age key is compromised, recovery is a
~1-day manual sweep (fresh cluster + re-encrypt every Secret). If
multi-operator becomes needed, a future spec adds a recipients list
and offboarding contract — until then, single-operator-only. If KMS
becomes needed (e.g. for regulatory attestation), a future spec
migrates; until then, operator-host + password-manager combo is the
v1 commit.

**Review:** `single-author-self-pinned`

**Pinned:** 2026-05-17 by bart

---

## Phase 0 outstanding items

- **(b) SC-003 empirical baseline** — requires running the four-step
  DR sequence (SOPS restore → terraform/setup.sh → `flux bootstrap`
  → reconcile) against a throwaway Hetzner cluster and recording
  per-step timings in `execution/dr-baseline.md`. Operator action;
  cannot be performed in-session.
- **(c) Password-manager vault path substitution** — §Decision 9(b)
  records 1Password vault `OAP / Cluster Keys`, item
  `sops-age-hetzner-prod` as the v1 commit. If the operator-of-record
  uses a different password manager, this pin amends in-place with
  the actual vault + item path before Phase 0 closes.
- **Single-author-self-pinned cert-pipeline surfacing** — per
  §Decision schema, the cert pipeline (spec 102) MUST surface
  `single-author-self-pinned` distinctly. If spec 102 has not added
  the surfacing by Phase 0 close, a stub follow-up filed against
  spec 102 is acceptable Phase 0 evidence (per spec.md schema preamble).

When all three outstanding items land, the spec lifecycle flips to
`status: approved` (with `approved: <date>` frontmatter) and plan.md
+ tasks.md follow in a sibling PR.
