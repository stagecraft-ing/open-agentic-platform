---
id: "225-deployd-selfprovision-rbac"
title: "deployd Self-Provisioned Per-Namespace RBAC (drop the cluster-wide workloads grant)"
feature_branch: "225-deployd-selfprovision-rbac"
status: approved
implementation: complete  # Mechanism (ClusterRole split + ensure_workload_rbac + env/grants) shipped in #503 under a Spec-Drift-Waiver; this spec is its design home and lands the validated flip: the chart template now suppresses the cluster-wide workloads ClusterRoleBinding when rbac.selfProvision is true, and values-hetzner.yaml enables it. The bind-verb escalation was validated on the real oap-hetzner-master1 K3s cluster on 2026-07-04 before the fallback-drop was wired (see Validation).
kind: platform
domain: platform
created: "2026-07-04"
authors: ["open-agentic-platform"]
language: en
summary: >
  deployd-api creates tenant/preview namespaces on demand and runs
  `helm upgrade --install` inside them. Its workload permissions
  (secrets, deployments, services, ...) were granted CLUSTER-WIDE via a
  single ClusterRoleBinding, so deployd could touch workloads in every
  namespace on the cluster, not just the ones it manages. #503 split the
  RBAC into a cluster-scoped `deployd-controller-namespaces` role (namespace
  CRUD only) and a namespaced `deployd-controller-workloads` role, and added
  an opt-in self-provisioning mechanism (`DEPLOYD_SELF_PROVISION_RBAC`): when
  on, deployd grants its own ServiceAccount the workloads role via a
  RoleBinding it creates in each target namespace, right before helm runs
  there. #503 deliberately left the cluster-wide fallback in place because
  dropping it was gated on real-cluster validation of the `bind`-verb
  escalation. This spec is the design home for that mechanism and lands the
  validated flip: with `rbac.selfProvision: true` the chart no longer renders
  the cluster-wide workloads ClusterRoleBinding, so deployd's standing
  workload authority is reduced from every namespace to only the namespaces
  it actually deploys into. The chart default is unchanged (fallback intact);
  the flip is enabled per-environment via values.
code_aliases: ["DEPLOYD_SELF_PROVISION_RBAC"]
depends_on:
  - "136-tenant-hello-demo-service"  # deployd's helm-upgrade deploy path (Phase 3 cluster-validation-gating precedent)
  - "145-deployd-durability"  # co-owns values-hetzner.yaml (persistence); this spec refines it with the rbac block
establishes:
  - unit: { kind: file, path: platform/charts/deployd-api/templates/rbac.yaml }
refines:
  # Add the rbac.selfProvision block to the Hetzner overlay. Additive concern
  # (RBAC scoping) on a file 143 refines (sweeper secret) and 145 extends
  # (persistence); this refines a distinct aspect, no overlap.
  - aspect: "deployd-selfprovision-rbac"
    unit: { kind: file, path: platform/charts/deployd-api/values-hetzner.yaml }
extends:
  # A new spec adds a node to the featuregraph golden (same precedent as
  # specs 214, 222, 223, 224); claimed additively against 034.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
references:
  - role: mechanism-implementation
    unit: { kind: file, path: platform/services/deployd-api-rs/src/rbac.rs }
  - role: mechanism-wiring
    unit: { kind: file, path: platform/services/deployd-api-rs/src/routes.rs }
---

# Feature Specification: deployd Self-Provisioned Per-Namespace RBAC

**Feature Branch**: `225-deployd-selfprovision-rbac`
**Created**: 2026-07-04
**Status**: Approved
**Input**: Give the deployd self-provisioning RBAC mechanism (shipped mechanism-only in #503) a spec home, and land the cluster-validated flip that drops the cluster-wide workloads grant.

## Context

A security review (recorded in the `rbac.yaml` header) found that
deployd-api held **cluster-wide** CRUD on `secrets` and every other workload
resource: every namespace on the cluster (`kube-system`, `monitoring`,
`rauthy-system`, ...), not just the tenant namespaces it manages. #503 split
the single ClusterRole into two:

- **`deployd-controller-namespaces`** (cluster-scoped, always bound
  cluster-wide): only `namespaces` CRUD, which cannot be narrowed below
  cluster scope because `Namespace` is not a namespaced resource.
- **`deployd-controller-workloads`** (namespaced): everything deployd needs
  inside a target namespace to run `helm upgrade --install`.

The workloads role was still bound **cluster-wide** by default, because
deployd creates tenant/preview namespaces on demand and had no way to grant
itself workload rights in a namespace it just created. #503 closed that gap
with an opt-in mechanism (`DEPLOYD_SELF_PROVISION_RBAC`,
`deployd-api-rs/src/rbac.rs`): when on, deployd creates a RoleBinding
referencing `deployd-controller-workloads` in each target namespace, right
before helm runs there. Creating that RoleBinding requires the Kubernetes
privilege-escalation guards `create` on `rolebindings` **and** the `bind`
verb on that specific ClusterRole, both granted by the chart when
self-provisioning is enabled.

#503 shipped the mechanism opt-in and **default-off**, and deliberately did
**not** drop the cluster-wide fallback: doing so was gated on validating the
`bind`-verb escalation on a real cluster (only kind-proven at the time, per
the `rbac.yaml` header and the spec 136 Phase 3 precedent). #503 handled all
its coupling with Spec-Drift-Waivers rather than a spec. This spec is the
design home for the mechanism and lands the now-validated flip.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - deployd's blast radius is bounded to the namespaces it manages (Priority: P1)

An operator runs deployd on a shared cluster that also hosts identity
(`rauthy-system`), monitoring, and other unrelated workloads. deployd should
be able to install tenant charts in the namespaces it creates, and nowhere
else. Before this flip, a compromised or buggy deployd could read/modify
`secrets` in every namespace on the cluster.

**Why this priority**: This is the security posture the RBAC split was for.
The split alone did not reduce standing authority (the workloads role was
still bound cluster-wide); only dropping that binding does. The mechanism
that makes dropping it safe (self-provisioning) is what needed real-cluster
validation.

**Independent Test**: Render the chart with `rbac.selfProvision: true` and no
`rbac.namespaces`; assert no cluster-wide `ClusterRoleBinding` for
`deployd-controller-workloads` is produced. On a live cluster, assert
deployd's ServiceAccount cannot list secrets in a namespace it has not
deployed into, but self-provisions access on its next deploy there.

**Acceptance Scenarios**:

1. **Given** `rbac.selfProvision: true` and an empty `rbac.namespaces`,
   **When** the chart renders, **Then** the cluster-wide workloads
   `ClusterRoleBinding` is NOT produced (only the `namespaces`
   ClusterRoleBinding remains), and the deployment carries
   `DEPLOYD_SELF_PROVISION_RBAC=true`.
2. **Given** deployd processes a deploy into namespace `N` with
   self-provisioning on, **When** the deploy handler runs, **Then** deployd
   ensures `N` exists and creates the `deployd-controller-workloads`
   RoleBinding in `N` (idempotent, 409-tolerant) before `helm upgrade
   --install`, so helm has workload rights in `N`.
3. **Given** the chart default (`rbac.selfProvision: false`,
   `rbac.namespaces: []`), **When** the chart renders, **Then** the
   cluster-wide workloads `ClusterRoleBinding` IS produced, exactly as before
   this spec, preserving the working fallback untouched.
4. **Given** an explicit `rbac.namespaces` allowlist, **When** the chart
   renders, **Then** one per-namespace `RoleBinding` is produced for each
   listed namespace and no cluster-wide binding, regardless of
   `rbac.selfProvision`.
5. **Given** BOTH `rbac.selfProvision: true` AND a non-empty
   `rbac.namespaces` (the transitional-cutover configuration), **When** the
   chart renders and deployd runs, **Then** the chart produces the static
   per-namespace `RoleBinding`s from the allowlist (the `if .namespaces`
   branch wins) AND the deployment still carries
   `DEPLOYD_SELF_PROVISION_RBAC=true`, so deployd also self-provisions at
   deploy time. The two coexist safely: the self-provision RoleBinding create
   is 409-tolerant, so a namespace that already has a static binding is a
   no-op. This is the supported transitional state the Cutover section
   recommends for operators who cannot redeploy every tenant at cutover.

### Edge Cases

- **Existing namespace at cutover.** When the fallback is dropped, deployd
  loses standing workload authority in namespaces it created *before* the
  flip (they have no self-provisioned RoleBinding yet). This is benign:
  deployd only touches workloads during a deploy, and `ensure_workload_rbac`
  runs before helm on every deploy, so the FIRST deploy to such a namespace
  self-heals it (the RoleBinding create is idempotent). Cutover procedure:
  optionally redeploy existing tenants once after the flip to pre-provision
  their RoleBindings, or let them self-heal on next deploy.
- **`bind` verb missing.** If the escalation grant is absent, the RoleBinding
  create returns Forbidden ("attempting to grant RBAC permissions not
  currently held"); the deploy handler fails fast with that specific cause
  (`routes.rs`) instead of an opaque mid-helm "forbidden". This is the
  load-bearing check the negative control on kind proved.
- **Namespace deployd did not create.** `ensure_workload_rbac` creates the
  RoleBinding regardless of who created the namespace (namespace create is
  409-tolerant), so an externally-created target is self-provisioned on first
  deploy too.
- **Teardown path does not self-provision (residual, FR-007).** Only the
  deploy handler calls `ensure_workload_rbac`; `delete_deployment`
  (`routes.rs`) shells `helm uninstall` / `uninstall_with_gate` directly. So
  a DELETE of a namespace that was created before the flip and never
  redeployed (hence never self-provisioned) would hit `Forbidden` on
  uninstall. The delete path treats uninstall as best-effort and ignores the
  failure to stay idempotent, so the DB row is marked deleted while the
  namespace's k8s resources are orphaned rather than a hard error. Mitigation
  is the cutover procedure below; the durable fix is FR-007.

## Requirements *(mandatory)*

### Functional Requirements: mechanism (shipped in #503, documented here)

- **FR-001**: The chart MUST split deployd's permissions into a
  cluster-scoped `deployd-controller-namespaces` ClusterRole (namespace CRUD
  only, always bound cluster-wide) and a namespaced
  `deployd-controller-workloads` ClusterRole (the helm workload verb set).
- **FR-002**: When `rbac.selfProvision` is true, the
  `deployd-controller-namespaces` ClusterRole MUST additionally grant
  `create`/`patch` on `rolebindings` and the `bind` verb on
  `deployd-controller-workloads`, and the deployment MUST project
  `DEPLOYD_SELF_PROVISION_RBAC=true` plus the ServiceAccount name, pod
  namespace (downward API), and workloads-ClusterRole-name env the code reads.
- **FR-003**: When `DEPLOYD_SELF_PROVISION_RBAC` is on, deployd-api-rs MUST,
  before `helm upgrade --install` in a target namespace, ensure that
  namespace exists and create a RoleBinding referencing
  `deployd-controller-workloads` binding its own ServiceAccount (idempotent,
  409-tolerant), and MUST fail the deploy fast with a clear cause if that
  RBAC step is rejected (`ensure_workload_rbac`, wired in `routes.rs`).

### Functional Requirements: the flip (this PR)

- **FR-004**: When `rbac.selfProvision` is true and no explicit
  `rbac.namespaces` allowlist is supplied, the chart MUST NOT render the
  cluster-wide `deployd-controller-workloads` ClusterRoleBinding. deployd's
  standing workload authority is then exactly the set of namespaces it has
  self-provisioned, not the whole cluster.
- **FR-005**: The chart default MUST leave `rbac.selfProvision: false` and
  `rbac.namespaces: []`, preserving the cluster-wide fallback untouched. The
  flip is enabled per-environment via values (`values-hetzner.yaml` sets
  `rbac.selfProvision: true`), never by changing the chart default.
- **FR-006**: The `bind`-verb escalation MUST be validated on the real target
  cluster before the fallback-drop is enabled for that environment. An
  operator MUST NOT enable `rbac.selfProvision` on a cluster where deployd's
  ServiceAccount has not been shown to successfully self-provision a
  workloads RoleBinding (see Validation for the Hetzner evidence).

### Functional Requirements: staged (NOT delivered this PR)

- **FR-007** *(staged, code follow-up)*: The teardown path
  (`delete_deployment` in `routes.rs`) SHOULD call `ensure_workload_rbac` for
  the target namespace before `helm uninstall` / `uninstall_with_gate`, the
  same way the deploy handler does, so a namespace that was never
  self-provisioned can still be torn down cleanly once the cluster-wide
  fallback is gone. Deferred here because this PR is chart + spec only (the
  chosen "template change + spec" path) and requires a deployd-api-rs code
  change plus an image rebuild to take effect. Until it lands, the Cutover
  procedure covers existing tenants.

## Key Entities

- **`deployd-controller-namespaces`** / **`deployd-controller-workloads`**:
  the two ClusterRoles the RBAC split produced.
- **`rbac.selfProvision`** (chart value, default `false`): master switch for
  the self-provisioning grants + env and (per FR-004) the fallback-drop.
- **`DEPLOYD_SELF_PROVISION_RBAC`**: the env the running deployd reads
  (`SelfProvisionConfig::from_env`) to decide whether to self-provision.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `helm template` with `rbac.selfProvision: true` + empty
  `rbac.namespaces` renders exactly one `ClusterRoleBinding` (the namespaces
  one); the chart default renders two (namespaces + workloads fallback); an
  explicit allowlist renders one per-namespace `RoleBinding` per entry.
- **SC-002**: On the real target cluster, deployd's ServiceAccount can create
  the `deployd-controller-workloads` RoleBinding in a namespace it creates
  (the `bind`-gated operation succeeds). Validated 2026-07-04 on
  `oap-hetzner-master1` (see Validation).
- **SC-003**: The coupling gate and spec-lint are green with spec 225
  claiming the new/changed paths (`rbac.yaml`, `values-hetzner.yaml`).
- **SC-004**: A tenant namespace created before the flip regains deployd
  workload access on its next deploy with no manual RBAC step (self-heal via
  the idempotent `ensure_workload_rbac`).

## Cutover

When the flip reaches a cluster (CD deploy of this chart with
`rbac.selfProvision: true`), helm deletes the cluster-wide workloads
ClusterRoleBinding. From that instant, deployd's standing workload authority
is only the namespaces it has already self-provisioned (initially none). This
is safe for the deploy path (self-heals per FR-003) but leaves the teardown
gap (FR-007). The recommended cutover, ordered:

1. Merge and let CD deploy the flip.
2. Immediately redeploy each currently-live tenant once (there is exactly one
   on Hetzner today: `oap-stagecraft-ing-single-simple-dev`). Each redeploy
   runs `ensure_workload_rbac`, provisioning that namespace's RoleBinding, so
   both future deploys AND deletes have workload rights there.
3. Until FR-007 lands, avoid deleting a tenant that has not been redeployed
   since the flip (its uninstall would be silently best-effort and orphan
   resources).

An operator who cannot redeploy all tenants at cutover can instead list them
in `rbac.namespaces` as a transitional allowlist (the chart still renders
per-namespace RoleBindings from it even with `selfProvision: true`), then
remove them once each has been redeployed.

## Validation (2026-07-04, oap-hetzner-master1)

The flip was gated on validating the `bind`-verb escalation on real
Kubernetes (kind had proved it; the production K3s target had not). On
2026-07-04, with `rbac.selfProvision: true` applied to the live deployd
release (rev 80, image `sha-918b0c7`, the #503 binary), the exact operation
`ensure_workload_rbac` performs was exercised by impersonating deployd's
ServiceAccount (`system:serviceaccount:deployd-system:deployd-api-sa`):

1. Created a throwaway namespace as the SA (tests the `namespaces` grant): OK.
2. Created a RoleBinding referencing `deployd-controller-workloads` in it as
   the SA (tests `rolebindings` create + the `bind` escalation): OK.
3. Verified the RoleBinding's roleRef/subject; deleted the namespace.

The `bind`-gated create succeeded, confirming the grant authorizes deployd to
self-provision on this cluster. The kind negative control (no `bind` verb ->
Forbidden) established the grant is load-bearing. The deployd Rust code path
is unchanged from #503, so the mechanism validated on kind runs identically
here. This satisfies FR-006 for the Hetzner environment.

## Review follow-ups (2026-07-04)

Acknowledged from the automated review of this PR; none block the flip, all
tracked here so they are not lost:

- **Teardown-gap severity elevates post-flip (FR-007).** Before the flip a
  teardown `Forbidden` was masked by the cluster-wide grant; after it, a
  never-self-provisioned namespace's `helm uninstall` fails and is swallowed
  as best-effort, silently orphaning resources. The cutover is manual and
  unenforced. FR-007 (self-provision in the teardown path) is the durable
  fix; until then the Cutover procedure is the mitigation. Low current risk:
  no tenant exists that predates the flip once the clean-slate cluster is the
  starting point.
- **Self-provision subject comes from env, not a literal.**
  `workload_rolebinding` takes the RoleBinding subject from
  `DEPLOYD_SERVICE_ACCOUNT` / `DEPLOYD_POD_NAMESPACE` (downward API). The
  `bind` grant is scoped to `deployd-controller-workloads` only, so a
  poisoned projection could at worst grant THAT role to an unintended
  subject, not arbitrary escalation. Hardening follow-up: assert the pod's
  own downward-API namespace is authoritative rather than trusting a literal
  env override.
- **No stagecraft `audit_log` row for self-provisioned RoleBindings.** They
  appear only in the Kubernetes audit log. If the OAP audit trail is treated
  as the authority for privilege grants, this is a completeness gap. Candidate
  follow-on FR: emit a `deployd.rbac.self_provisioned` audit row.
- **Feature-annotation traceability.** The mechanism's code (`rbac.rs`,
  `routes.rs`) carries no `// Feature: DEPLOYD_SELF_PROVISION_RBAC`
  annotations (it shipped under #503's waiver), so the featuregraph records
  `impl_files: []` for this spec even though it is `implementation: complete`
  on the chart surface it establishes. Adding those annotations is the
  natural companion to retiring #503's Spec-Drift-Waiver; the `code_aliases`
  entry is aligned to the env-var token so a future annotation matches.
