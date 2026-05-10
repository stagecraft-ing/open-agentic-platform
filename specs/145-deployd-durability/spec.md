---
id: "145-deployd-durability"
slug: deployd-durability
title: "deployd-api durability chain — PVC + scrub-narrowing + Hiqlite backup/s3 + restore-on-startup"
status: draft
implementation: pending
owner: bart
created: "2026-05-10"
kind: platform-delivery
risk: medium
depends_on:
  - "073"  # axiomregent-unification (deployd-api-rs runtime carrier)
  - "086"  # open-source-launch (deployd-api role context)
  - "144"  # hiqlite default-features hygiene (companion; manifest discipline this spec inherits)
code_aliases: ["DEPLOYD_DURABILITY"]
implements:
  - path: platform/charts/deployd-api/values.yaml
  - path: platform/charts/deployd-api/values-hetzner.yaml
  - path: platform/charts/deployd-api/values-azure.yaml
  - path: platform/charts/deployd-api/values-aws.yaml
  - path: platform/charts/deployd-api/values-gcp.yaml
  - path: platform/charts/deployd-api/values-do.yaml
  - path: platform/charts/deployd-api/values-local.yaml
  - path: platform/charts/deployd-api/templates/deployment.yaml
  - path: platform/charts/deployd-api/templates/external-secret.yaml
  - path: platform/services/deployd-api-rs/Cargo.toml
  - path: platform/services/deployd-api-rs/Cargo.lock
  - path: platform/services/deployd-api-rs/src/main.rs
  - path: platform/services/deployd-api-rs/src/store.rs
  - path: platform/services/deployd-api-rs/src/config.rs
  - path: docs/runbooks/deployd-api-durability.md
summary: >
  The `deployments` and `deployment_events` tables in deployd-api-rs's
  Hiqlite store are the audit trail of who deployed what, when, with
  what scope, and what outcome — not reconstructable from K8s state for
  rolled-back releases, and not reconstructable at all for the
  append-only event log. Today three reinforcing layers of "off" make
  this data ephemeral on the actively-shipping Hetzner deploy: the env
  values file disables the chart's PVC, the container start-up
  command runs `rm -rf /var/lib/deployd/data/*` on every boot, and the
  Cargo manifest enables only `["sqlite"]` so there is no off-cluster
  durability path. This spec lands the four coupled fixes as one unit:
  flip persistence on, narrow the data-dir scrub back to the targeted
  stale-lock cleanup it replaced, enable Hiqlite `backup`+`s3`+`auto-heal`
  with operator-supplied S3 credentials, and add restore-on-startup so
  a fresh pod against an empty volume rehydrates from the most recent
  encrypted snapshot. Splitting the four lands a half-fix.
---

# 145 — deployd-api durability chain

## 1. Background

`deployd-api-rs` is the platform's Rust deployment-orchestration
service. Its Hiqlite store carries two governance-load-bearing tables
(`audit.md` Phase 4):

- **`deployments`** (`platform/services/deployd-api-rs/src/store.rs:39-77`)
  — one row per release; release SHA, artifact ref, status, endpoints.
  Partially reconstructable from the K8s API for *currently-deployed*
  releases, **not** reconstructable for rolled-back ones.
- **`deployment_events`** (`store.rs:39-77`) — append-only event log
  per deployment. The audit trail itself; not reconstructable from any
  other source.

Three layers of "off" stack to make this state ephemeral on the
actively-shipping Hetzner deploy (`verifications.md` Q1):

1. **The env values file overrides the chart default to disable the
   PVC.** `platform/charts/deployd-api/values-hetzner.yaml:34-38` sets
   `persistence.enabled: false` with the inline rationale "Stealth
   stage: use emptyDir for hiqlite state. Deployment history is lost
   on pod restart, which is acceptable pre-GA. Flip to true and set a
   size/class when persistent deployment history is required." The
   chart's top-level default is `true`
   (`platform/charts/deployd-api/values.yaml:22-25`); the other env
   files (`values-azure.yaml`, `values-aws.yaml`, `values-gcp.yaml`,
   `values-do.yaml`, `values-local.yaml`) inherit that default.
2. **The container start-up command erases the data directory on every
   pod start.** `platform/charts/deployd-api/templates/deployment.yaml:39-43`:

   ```yaml
   command: ["/bin/sh", "-c"]
   args:
     - |
       rm -rf /var/lib/deployd/data/*
       exec /usr/local/bin/deployd-api
   ```

   Commit `3aa8893a` widened an earlier targeted stale-lock cleanup
   from commit `cd84f1e9` (which removed only
   `/var/lib/deployd/data/state_machine/lock`) into a full data-dir
   scrub. Even when a PVC is mounted, the Hiqlite data subtree under
   `/var/lib/deployd/data/*` is wiped at every boot, so the volume is
   durable from K8s's perspective but **not** from the application's.
3. **The Cargo manifest enables only `["sqlite"]`.**
   `platform/services/deployd-api-rs/Cargo.toml:17`:
   `default-features = false, features = ["sqlite"]`. `backup` and `s3`
   are off; there is no off-cluster durability path even if (1) and (2)
   were resolved. The single-replica, single-node Hiqlite cluster at
   `127.0.0.1:7001` (`store.rs:15-18`, `node_id: 1`) is the only copy
   of the data.

Phase 4 of `audit.md` named this an **OVERSIGHT**: deploy history is
governance-load-bearing data, not reconstructable from K8s state, and
the `cryptr → s3-simple` chain is already in
`platform/services/deployd-api-rs/Cargo.lock` as a Hiqlite transitive
(zero new direct deps to enable encrypted S3 backup). The
`verifications.md` Implications table promoted the original audit's
"S effort" recommendation to **M effort** and converted it from "an
audit recommendation" into "its own spec" — this is that spec.

### 1.1 Coordination with spec 146 (deployd-api memory hardening)

**Read this before implementing §2.** Spec 146
(`146-deployd-api-memory-hardening`, authored 2026-05-10) lands a
chart-default `resources` block on
`platform/charts/deployd-api/values.yaml` populating `limits.memory:
1Gi`, `requests.memory: 256Mi`, `requests.cpu: 100m`. Spec 146 closes
spec 143 FU-021 — the OOM driver was *cold-start hiqlite WAL pressure
against an unbounded cgroup* on the actively-shipping Hetzner deploy
(restartCount=3, exit 137, ~10 min lifetime). Spec 146 §2.4 and spec
143's §13 2026-05-10 ~17:00 / ~17:30 UTC entries flag this as a
coordination point with spec 145 for one reason:

> **If memory pressure during WAL init produces an OOM *before* spec
> 145's restore-on-startup path runs, the chart-default 1Gi cgroup
> spec 146 lands is the load-bearing safeguard that lets
> restore-on-startup run at all.**

Concretely, when this session lands §2.4 (restore-on-startup):

1. **Verify the cgroup floor is present.** `helm template` against
   `values.yaml` must render `resources.limits.memory` and
   `resources.requests.memory` non-empty in the deployd-api
   Deployment. Spec 146 already wires this; the assertion here is
   "spec 146 landed before spec 145's restore code path is
   exercised on cluster." If §2 of this spec ships ahead of spec
   146, the restore path runs against an unbounded cgroup and the
   cold-start OOM can fire *before* the restore logic gets to run.
2. **Decide absorb-vs-fork on WAL-pressure-aware scheduling.** If
   restore-on-startup itself drives non-trivial allocation under
   the 1Gi floor (e.g., decrypting a multi-MB snapshot in-memory),
   this session decides whether to:
   - **Absorb** — raise the cgroup default in spec 146 (amend
     §2.1 with budget math for the restore-decrypt allocation), or
   - **Fork** — file a follow-up spec on
     WAL-pressure-aware scheduling (e.g., stream decrypt to disk,
     or chunk restore by table).
   Spec 146 §2.4 names this absorb-vs-fork choice as deferred to
   this session — the floor is sufficient cover for the diagnosed
   FU-021 failure mode, but spec 145's restore allocation profile
   is the next pressure surface and this session has the data.
3. **Touch points.** Spec 146 claims the same `values.yaml` file
   under spec 130's any-claimant heuristic. Edits to the
   `resources:` block from this session require either the spec 146
   amendment path (cleanest) or a Spec-Drift-Waiver per spec 127
   FR-005. The two specs' edits to `values.yaml` are otherwise
   disjoint (146 adds `resources:`; 145 changes
   `persistence.enabled` and `command.args`).

This subsection is informational, not contractual; it does not
expand spec 145's `implements:` list. The coordination flag exists
so this session's first read of spec 145 surfaces the spec 146
context without needing to dredge spec 143's §13 ledger.

## 2. Resolution

The four changes below land as one coherent unit. They are coupled
because:

- without (a), no Cargo feature change matters;
- without (b), a PVC is durable from K8s's perspective but not from
  the application's;
- without (c), on-cluster persistence still has no DR story;
- without (d), a fresh pod against an empty volume cannot rehydrate.

### 2.1 Chart — flip Hetzner persistence on; audit other env files

**File:** `platform/charts/deployd-api/values-hetzner.yaml:34-38`.

Set:

```yaml
persistence:
  enabled: true
  size: 1Gi
  storageClassName: hcloud-volumes
```

The inline "stealth stage" rationale is replaced with one sentence:
"Persistent deployment history required from spec 145; size pinned to
1Gi (deploy history is tens-of-rows scale today) and storage class
pinned to `hcloud-volumes` rather than the cluster default to avoid
silent re-binding if cluster StorageClass priorities are reordered."

**Caveat — Hetzner CSI minimum.** The hcloud CSI driver enforces a
minimum volume size (commonly 10 GiB at the time of writing). If the
1 GiB request is rejected by the CSI provisioner — or silently rounded
up — implementation MUST verify against `kubectl get pvc -n <ns>` of
the existing deployd-api PVC (or `hcloud volume list`) and AMEND
spec 145 to whatever Hetzner actually accepts (typically the 10 GiB
floor). Verification step lives in `tasks.md` T002.

**Other env files.** `values-azure.yaml`, `values-aws.yaml`,
`values-gcp.yaml`, `values-do.yaml` inherit the chart default
(`persistence.enabled: true`, `size: 1Gi`,
`storageClassName: ""` → cluster default) silently. The chart-level
default at `values.yaml:22-25` is correct for managed-K8s targets;
no per-env override is added unless the operator's evidence flags a
specific cluster's StorageClass needs an explicit pin. `values-local.yaml`
(kind / minikube / k3d) sets `persistence.enabled: false` explicitly
with a one-sentence inline rationale ("Dev loop only — emptyDir is
fine; restore-on-startup is opt-in via env-supplied BackupConfig
which is unset in this profile"). FR-002 records the decisions
explicitly.

The chart `templates/pvc.yaml:1-15` and `templates/deployment.yaml:99-100,
:115-121` already implement the PVC + volume-mount machinery and need
no edits — the gate is `.Values.persistence.enabled`.

### 2.2 Chart — narrow the container-start `rm` scope

**File:** `platform/charts/deployd-api/templates/deployment.yaml:39-43`.

Two acceptable resolutions:

**Option A — narrow to the original targeted cleanup** (commit
`cd84f1e9` form):

```yaml
command: ["/bin/sh", "-c"]
args:
  - |
    rm -f /var/lib/deployd/data/state_machine/lock
    exec /usr/local/bin/deployd-api
```

Removes only the stale Raft state-machine lock that motivated the
original cleanup. Preserves WAL, snapshots, and the SQLite database
across pod restarts.

**Option B — eliminate the cleanup entirely.** If Hiqlite 0.13.1's
first-boot semantics no longer require the lock-file workaround
(verify against upstream changelog and the `state_machine` directory
behaviour), the container `command`/`args` can drop back to the
default entrypoint and remove the wrapper shell.

Implementation chooses A or B; the spec does not pre-commit. Whichever
lands MUST be validated against the original failure that motivated
commit `3aa8893a` ("wipe emptyDir on container start to unblock
hiqlite") — i.e. simulate a pod restart against the new persistence
posture and confirm Hiqlite first-boot completes without manual
intervention.

### 2.3 Cargo — enable `backup`, `s3`, `auto-heal`; wire NodeConfig

**File:** `platform/services/deployd-api-rs/Cargo.toml:17`.

Becomes:

```toml
hiqlite = { version = "~0.13", default-features = false, features = ["sqlite", "backup", "s3", "auto-heal"] }
```

`backup` and `s3` mutually-imply per upstream (`s3 = ["backup"]`,
`backup` activates the cron-driven snapshot path that emits to S3 via
`cryptr → s3-simple`). `auto-heal` enables WAL self-repair for the
single-node deploy — cheap reliability, mirrors spec 144's posture
on the orchestrator + axiomregent crates.

**SBOM impact.** The `cryptr` and `s3-simple` chain is already in
`platform/services/deployd-api-rs/Cargo.lock` as a Hiqlite transitive
(`audit.md` Phase 2.3, Phase 3b). No new direct deps. `cron` enters
the lockfile (it is feature-gated by `backup` upstream). No other
crates should appear; the diff is feature-flag-driven.

**NodeConfig wiring.** `platform/services/deployd-api-rs/src/store.rs:13-33`
constructs the `hiqlite::NodeConfig`. Add:

- `backup_config` — populated from env (S3 endpoint URL, bucket, access
  key id, secret key, optional path prefix, encryption key for `cryptr`,
  cron schedule string, retention/keep-count). Keys land in
  `src/config.rs` (existing or new) as a typed
  `BackupConfig` struct loaded once at startup. Operator semantics
  in §3.4 (NFRs).

The exact struct shape on the Hiqlite side is `NodeConfig.backup_config`
(or whatever upstream `0.13.1` names it — implementation MUST consult
`hiqlite/Cargo.toml` v0.13.1 and the `NodeConfig` rustdoc rather than
assuming the spec's name). The contract this spec sets is on the
operator-visible env surface and behaviour, not on the upstream API.

**Helm wiring.** `platform/charts/deployd-api/templates/deployment.yaml`
gains env entries for the BackupConfig fields. Sensitive fields
(access key, secret key, encryption key) MUST be sourced from a
Kubernetes Secret via the existing
`platform/charts/deployd-api/templates/external-secret.yaml`
ExternalSecrets path, never from `values.yaml`. New keys are added to
the operator-supplied secret store (Azure Key Vault, AWS Secrets
Manager, etc.) and projected through `external-secret.yaml`. Non-sensitive
fields (endpoint, bucket, prefix, schedule, retention) MAY come from
chart values.

### 2.4 Restore-on-startup

**File:** `platform/services/deployd-api-rs/src/main.rs:24-28`,
`src/store.rs:13-33`.

On startup, before `start_node` (or as part of its configuration if
upstream supports it), `init_db` MUST:

1. Inspect the data dir at `data_dir` (default
   `/var/lib/deployd/data`). If it is non-empty and contains a valid
   Hiqlite WAL/state, proceed with normal start.
2. If the data dir is empty (fresh PVC, post-DR migration), and a
   `BackupConfig` is configured, attempt a Hiqlite restore from the
   most recent S3 snapshot. The exact API is Hiqlite v0.13.1's
   restore primitive (consult `NodeConfig` / restore docs at that
   version — not pre-committed in the spec).
3. If restore fails (no snapshots, decryption error, network failure),
   fail fast with a clear log line. Do not silently start with an
   empty database — that would erase audit history without operator
   awareness.
4. If `BackupConfig` is **not** configured (e.g. `values-local.yaml`
   for dev), proceed to start with the empty data dir as today —
   restore-on-startup is opt-in via the operator's choice to populate
   the backup env vars.

Pod readiness gating: the existing `/healthz` (or readiness probe)
MUST not flip to Ready until `init_db` returns successfully —
restore-in-progress is a not-Ready state.

### 2.5 Operational runbook

**File:** `docs/runbooks/deployd-api-durability.md` (new).

Documents:

- Required S3 prerequisites (bucket creation, IAM/access policy,
  encryption-key generation for `cryptr`, retention policy).
- Required Helm values + Kubernetes Secret keys (operator-side
  inventory).
- DR restore procedure (what an operator runs to force a restore from
  a specific snapshot, troubleshooting, rollback).
- Key-rotation considerations (out of scope for implementation but
  surfaced for ops awareness — see NFR-004).
- How to confirm a backup succeeded (log line / metric / S3 listing).

## 3. Requirements

### 3.1 Functional requirements

- **FR-001** — `platform/charts/deployd-api/values-hetzner.yaml`:
  `persistence.enabled: true`, with explicit `size` and
  `storageClassName` set to values appropriate for the Hetzner block
  storage class. The "stealth stage" rationale comment is removed.
- **FR-002** — Other env values files: `values-azure.yaml`,
  `values-aws.yaml`, `values-gcp.yaml`, `values-do.yaml` inherit the
  chart default `persistence.enabled: true` silently (no per-env
  override); `values-local.yaml` carries an explicit
  `persistence.enabled: false` with a one-sentence inline rationale
  for the dev-loop context (kind / minikube / k3d). Implementation MAY
  add an explicit per-env override on a managed-K8s file if the
  operator's evidence flags a specific cluster's StorageClass need;
  the default posture is silent inheritance.
- **FR-003** — `platform/charts/deployd-api/templates/deployment.yaml:39-43`:
  the data-dir scrub is narrowed to the original targeted stale-lock
  cleanup (Option A) OR eliminated entirely (Option B), with the
  decision validated against the Hiqlite first-boot scenario commit
  `3aa8893a` was working around.
- **FR-004** — `platform/services/deployd-api-rs/Cargo.toml:17` enables
  `backup`, `s3`, and `auto-heal` alongside the existing `sqlite`,
  with `default-features = false` retained.
- **FR-005** — `platform/services/deployd-api-rs/src/store.rs::init_db`
  populates `NodeConfig`'s backup configuration from env: S3 endpoint
  URL, bucket, access key id, secret access key, optional path prefix,
  encryption key for `cryptr`, cron schedule string, and
  retention/keep-count. Keys load through a typed config struct in
  `src/config.rs`.
- **FR-006** — `platform/services/deployd-api-rs/src/main.rs` wires
  restore-on-startup: a fresh pod against an empty volume rehydrates
  from the most recent S3 snapshot before pod readiness flips to
  Ready. Failure to restore (when restore is configured) fails fast
  with a clear log line; the service does not silently start with an
  empty database.
- **FR-007** — `cargo build --manifest-path platform/services/deployd-api-rs/Cargo.toml`
  produces no new direct dependencies. The lockfile diff is limited
  to feature-flag-driven activations (`cron` enters; `cryptr`,
  `s3-simple` were already present).
- **FR-008** — `docs/runbooks/deployd-api-durability.md` exists and
  covers S3 prerequisites, env/secret inventory, DR restore procedure,
  and key-rotation considerations.

### 3.2 Non-functional requirements

- **NFR-001** — Restore-on-startup completes within 60 seconds for the
  steady-state snapshot size (today's `deployments` +
  `deployment_events` corpus) OR fails fast with the reason logged.
  Steady-state snapshot size and the actual bound are pinned during
  implementation; if 60s is unattainable for the current corpus,
  the implementation surfaces the measured number and the spec
  amends.
- **NFR-002** — Backup cron schedule and retention policy are
  operator-configurable via Helm values (with secret material via
  Kubernetes Secret). Hard-coded schedules are forbidden — the
  operator owns the cadence/retention contract for their environment.
  Chart-level defaults: `schedule: "0 */6 * * *"` (every 6 hours),
  `keep: 28` (28 days of snapshots retained). These match the
  governance-audit RPO target (≤ 6 hours) and provide a
  ~one-month rolling window for DR rollback.
- **NFR-003** — S3 credentials (access key, secret key) and the
  `cryptr` encryption key come from a Kubernetes Secret loaded via
  ExternalSecrets (`templates/external-secret.yaml`) — confirm
  against `templates/secretproviderclass.yaml` during P0.4 in case
  the operator uses the CSI Secret Store path instead. They are
  never present in `values.yaml`, never baked into the container
  image, and never logged.
- **NFR-004** — The `cryptr` encryption key is a **long-lived,
  operator-controlled secret** stored in the operator's Azure Key
  Vault entry (or equivalent operator-controlled secret store) —
  **not** generated per-cluster at install time. This is the only
  posture compatible with cross-cluster DR: snapshots survive
  cluster rebuilds and remain decryptable under the operator's
  retained key. Per-cluster generation would mean a cluster rebuild
  loses decryption capability for snapshots taken under the prior
  key, which defeats the off-cluster encrypted-backup contract.
  Key rotation is a documented operator concern; implementation
  is **out of scope** for this spec (rotation requires snapshot
  re-encryption or a multi-key decryption window neither feature
  offers today), but the runbook surfaces the constraint and how
  to perform a key-change with a full snapshot rotation under
  operator control.
- **NFR-005** — Pod restart against an existing populated PVC, with
  the narrowed scrub from FR-003, completes in the same time class
  as the pre-spec startup (no new latency on the steady-state pod
  restart path).

### 3.3 Acceptance criteria

- **AC-1** — Hetzner deploy survives pod eviction (`kubectl delete
  pod -n <ns> deployd-api-...`) without losing rows from
  `deployments` or `deployment_events`. Validation: pre-eviction
  query, post-eviction same query, identical rowsets.
- **AC-2** — A fresh pod started against a freshly-provisioned
  (empty) PVC rehydrates from the most recent S3 snapshot before
  becoming Ready. Validation: drop the PVC, force pod restart, watch
  the readiness probe — pod Ready only after restore log line; query
  `deployments` / `deployment_events` and confirm rowset matches the
  pre-restart snapshot.
- **AC-3** — Backup cron emits snapshots in S3 at the configured
  cadence; snapshots are encrypted at rest under the operator-supplied
  `cryptr` key and decryptable on a fresh pod (validated by AC-2).
- **AC-4** — The container-start data-dir scrub no longer deletes
  rows from `deployments` or `deployment_events` across pod restarts.
  Validation: pod restart against a populated PVC, query both tables,
  confirm rowsets unchanged.
- **AC-5** — `cargo build / check / clippy / test --manifest-path
  platform/services/deployd-api-rs/Cargo.toml` is green; no new
  direct deps; lockfile diff is feature-flag-driven only.
- **AC-6** — `helm template platform/charts/deployd-api -f
  values-hetzner.yaml` renders without error and the rendered
  Deployment carries the post-FR-003 startup args, the env entries
  for BackupConfig (sensitive ones from secretRef), and the PVC
  mount.
- **AC-7** — `docs/runbooks/deployd-api-durability.md` is reviewed
  by the operator on duty and confirms it is sufficient for first-time
  S3 setup, day-2 monitoring, and DR restore.
- **AC-8** — Spec-code coupling gate accepts the change against
  this spec's `implements:` list with no warnings. The coupling
  surface is intentionally cross-layer (chart + service + runbook);
  spec 130's primary-owner heuristic places primary ownership on
  spec 145 since this is the spec that motivates the coupled change.
- **AC-9** — `make ci` (warm) is green.

## 4. Out of scope

- **Multi-replica deployd-api.** `store.rs:15` keeps `node_id: 1`
  and the single-replica posture. Hiqlite `dlock` for write
  coordination across replicas is a future spec (audit Phase 5
  "future-spec" cluster, open question 1).
- **`listen_notify` SSE wiring** to stream deploy events into
  stagecraft's audit_log table. Future spec; not motivated by any
  current failure (today's stagecraft path polls `/v1/deployments/.../status`).
- **axiomregent durability.** Hiqlite-level S3 backup at axiomregent
  was rejected by the audit (durable copy lives in stagecraft
  Postgres). The "axiomregent offline audit buffering" question
  (audit open question 2) is its own future spec.
- **orchestrator changes.** Spec 144 covers manifest hygiene there.
  The orchestrator distributed-mode backup posture is academic per
  `verifications.md` Q2 (the build path is dead-on-disk).
- **Deployd-api-rs spec-pin hygiene.**
  `platform/services/deployd-api-rs/Cargo.toml:31` declaring
  `package.metadata.oap.spec = "073-axiomregent-unification"` is a
  separate hygiene pass (audit open question 3) — not this spec.
- **Key rotation implementation.** The runbook surfaces the
  operator-side constraint; a feature-level rotation path is not
  delivered here.
- **Multi-cloud env directory instantiation.** Spec 072
  (`multi-cloud-k8s-portability`) owns environment instantiation for
  AWS / GCP / DO. This spec audits those env files for persistence
  consistency but does not stand up new env directories.
- **`cron` schedule format / library swap.** Hiqlite's upstream cron
  semantics define the schedule string; this spec does not introduce
  a custom scheduler.

## 5. Provenance

- **`audit.md` Phase 2.3** — deployd-api-rs feature analysis;
  load-bearing case for `backup` + `s3` enablement.
- **`audit.md` Phase 4** — "OVERSIGHT" verdict on deployd-api-rs
  durability; classifies `deployments` and `deployment_events` as
  governance-load-bearing.
- **`audit.md` Phase 5** — original "S effort" recommendation row.
- **`verifications.md` Q1** — PVC posture investigation. PVC template
  exists; `values-hetzner.yaml` overrides the chart default to
  `false`; container start-up command runs `rm -rf
  /var/lib/deployd/data/*` on every boot (commit `3aa8893a` widened
  the earlier targeted lock cleanup from commit `cd84f1e9`).
- **`verifications.md` Implications table** — the four-step
  coupling rationale: chart persistence + scrub narrowing + S3
  enablement + restore-on-startup land together or land a half-fix.
- **`verifications.md` Next actions §2** — promotes the audit's
  "S effort" recommendation to **M**, and from "an audit
  recommendation" to "its own spec".
- **Chart anchors:**
  `platform/charts/deployd-api/templates/pvc.yaml:1-15`,
  `templates/deployment.yaml:39-43`, `:99-100`, `:115-121`.
- **Values anchors:**
  `platform/charts/deployd-api/values-hetzner.yaml:34-38`,
  `platform/charts/deployd-api/values.yaml:22-25`.
- **Service anchors:**
  `platform/services/deployd-api-rs/src/store.rs:13-33`, `:35-77`,
  `src/main.rs:24-28`.
- **Cargo anchors:**
  `platform/services/deployd-api-rs/Cargo.toml:17`,
  `Cargo.lock` (hiqlite deps block; `cryptr` + `s3-simple`
  already present).
- **CONST-005 framing.** This spec adds a new contract; it does not
  edit prior specs to retroactively justify a code change. Spec 145
  is authored before any chart, Cargo, or service-code edit lands.

## 6. Decision log + open questions

Operator decisions resolved 2026-05-10 are folded directly into the
relevant Resolution sections and the FR/NFR contracts above. They
remain listed here as a decision audit trail.

### Resolved 2026-05-10

1. **Hetzner storage class + size.** `size: 1Gi`,
   `storageClassName: hcloud-volumes`. Pinning the class explicitly
   prevents silent re-binding if cluster StorageClass priorities
   are reordered. Caveat: the hcloud CSI provisioner enforces a
   minimum (commonly 10 GiB); if 1 GiB is rejected at apply time,
   AMEND this spec to whatever the CSI driver actually accepts
   rather than letting implementation drift silently. Verification
   step lives at `tasks.md` T002. (See §2.1.)
3. **Backup cron schedule + retention defaults.**
   `schedule: "0 */6 * * *"`, `keep: 28`. Chart-level default in
   `values.yaml`; operator-configurable per NFR-002. (See §3.2 NFR-002.)
4. **`cryptr` key provisioning.** Long-lived operator-controlled
   secret in Azure Key Vault (or equivalent). NOT per-cluster
   generation — that posture would lose decryption capability across
   cluster rebuilds and defeat the off-cluster backup contract.
   (See §3.2 NFR-004.)
5. **Other env files' persistence posture.** `values-azure.yaml`,
   `values-aws.yaml`, `values-gcp.yaml`, `values-do.yaml` inherit
   the chart default silently. `values-local.yaml` opts out
   explicitly with `persistence.enabled: false` and a one-sentence
   inline rationale for the dev-loop context. (See §2.1 + §3.1
   FR-002.)

### Still open (implementation-time decisions)

2. **Scrub Option A vs Option B.** §2.2 leaves the choice to
   implementation pending a Hiqlite v0.13.1 first-boot validation.
   Concretely: does Hiqlite v0.13.1 still leave a stale
   `state_machine/lock` after an unclean shutdown? If yes → Option A
   (narrow). If no → Option B (eliminate the wrapper shell
   entirely). Test plan in `tasks.md` T003.
6. **Hiqlite restore API at v0.13.1.** §2.4 names the behaviour but
   not the upstream API. Implementation MUST consult
   `hiqlite::NodeConfig` and the v0.13.1 restore docs/example before
   writing the wiring; if the restore API materially differs from
   what this spec assumes, the spec amends rather than the
   implementation drifts. Verification step at `tasks.md` T007.
