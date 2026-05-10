---
description: "Task list for spec 145 — deployd-api durability chain"
---

# Tasks: deployd-api durability chain

**Input**: `specs/145-deployd-durability/spec.md` + `plan.md`
**Prerequisites**: `plan.md` (required), `spec.md` (required for §-anchors), companion-spec audit + verifications: `specs/144-hiqlite-default-features/audit.md`, `specs/144-hiqlite-default-features/verifications.md`

**Tests**: included where feasible. Live AC-1 / AC-2 / AC-4 must be
validated against the actual Hetzner deploy — unit + integration tests
cannot substitute for real PVC and real S3 behaviour.

## Format: `[ID] [P?] [Phase] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Phase]**: Maps to `plan.md` phase (P0–P4)
- File paths in descriptions are exact

## Path Conventions

- Chart values: `platform/charts/deployd-api/values*.yaml`
- Chart templates: `platform/charts/deployd-api/templates/`
- Service Cargo: `platform/services/deployd-api-rs/`
- Service Rust src: `platform/services/deployd-api-rs/src/`
- Runbook: `docs/runbooks/`

---

## Phase 0: Pre-flight investigation

**Purpose**: Resolve `spec.md` §6 open questions and confirm
assumptions before writing code. Findings inform implementation
choices and may amend the spec if material divergences surface.

- [ ] T001 [P0] Confirm spec 145 frontmatter compiles cleanly:
      `./tools/spec-compiler/target/release/spec-compiler compile`
      and verify exit 0 + spec 145 appears in
      `build/spec-registry/registry.json` via
      `./tools/registry-consumer/target/release/registry-consumer show 145-deployd-durability`.
- [ ] T001a [P0] **Verify chart template inventory.**
      `ls platform/charts/deployd-api/templates/` — confirm
      `external-secret.yaml` and/or `secretproviderclass.yaml` exist
      and decide which file is the operator's primary projection
      mechanism for the new BackupConfig secrets. Spec 145 §2.3 / §2.5
      assume `external-secret.yaml` (ExternalSecrets operator); if
      the operator-side answer is SecretProviderClass instead, the
      `implements:` list and T035 target file change accordingly —
      AMEND the spec rather than silently flipping the target.
- [ ] T001b [P0] **Coupling-gate dry-run.**
      `./tools/spec-code-coupling-check/target/release/spec-code-coupling-check`
      against the working tree as if spec 145's `implements:` paths
      were touched (synthetic stdin diff or `--paths` flag if
      supported, otherwise stage no-op edits to each target file
      locally and re-run). Surface any conflicts with specs 086 / 073
      / 072 (or any other current owner of those paths under spec
      130's primary-owner heuristic) **before** Phase 1 begins. If
      a Spec-Drift-Waiver is required, draft it now — surfacing it
      at handoff time is much better than mid-Phase-2.
- [ ] T002 [P0] **Verify Hetzner CSI accepts 1 GiB.** Resolved
      decision: `size: 1Gi`, `storageClassName: hcloud-volumes`.
      The hcloud CSI driver typically enforces a minimum (commonly
      10 GiB). Run
      `kubectl get pvc -n <ns>` against the existing deployd-api
      workload AND/OR `hcloud volume list` AND/OR
      `kubectl get storageclass hcloud-volumes -o yaml` to confirm
      1 GiB is accepted (or to observe what the provisioner actually
      allocates for a 1 GiB request). **If 1 GiB is rejected or
      silently rounded up, AMEND `spec.md` §2.1 and §6 to the actual
      minimum the driver accepts (likely 10 GiB) before T030 edits
      `values-hetzner.yaml`. Halt and surface to user.**
- [ ] T003 [P0] Choose scrub Option A vs Option B (P0.2): bring up a
      single-pod Hiqlite v0.13.1 cluster against a PVC, send
      `SIGKILL`, restart, observe whether init succeeds without
      intervention. If the lock file persists → Option A
      (narrow `rm -f` of just `/var/lib/deployd/data/state_machine/lock`).
      Otherwise → Option B (drop the wrapper shell entirely).
- [ ] T004 [P0] Cron defaults (resolved): record
      `schedule: "0 */6 * * *"`, `keep: 28` as the chart-level default
      to be set in T036. NFR-002 keeps both operator-configurable.
      No verification work required.
- [ ] T005 [P0] **Confirm chart-template projection path.** Both
      `platform/charts/deployd-api/templates/external-secret.yaml` and
      `platform/charts/deployd-api/templates/secretproviderclass.yaml`
      exist (verified in T001a). Decide which file is the operator's
      primary projection mechanism for the new BackupConfig secrets;
      spec 145 §2.3 / §2.5 assume ExternalSecrets. If the operator-
      side answer is SecretProviderClass, AMEND `spec.md`'s
      `implements:` list and §2.3 wording before T035 edits the
      template. The cryptr key itself is resolved as a long-lived
      operator-controlled Azure Key Vault entry (NFR-004); confirm
      the operator's Key Vault has capacity for three new entries
      (`backup-s3-access-key`, `backup-s3-secret-key`,
      `backup-cryptr-key`).
- [ ] T006 [P0] Other env files' persistence posture (resolved):
      `values-azure.yaml`, `values-aws.yaml`, `values-gcp.yaml`,
      `values-do.yaml` inherit chart default silently (no per-env
      override added in Phase 2). `values-local.yaml` opts out
      explicitly with `persistence.enabled: false`. T031 implements
      both. Verification: `grep -nE 'persistence:' platform/charts/deployd-api/values-*.yaml`
      confirms none of the managed-K8s files currently set
      `persistence.enabled` — they inherit from `values.yaml`. If a
      managed-K8s file already has an override, halt and surface
      before T031.
- [ ] T007 [P0] Read Hiqlite v0.13.1 restore API (P0.6): consult
      `hiqlite::NodeConfig` rustdoc + upstream restore example. Pin
      function name, arguments, error type. If the API materially
      differs from `spec.md` §2.4, AMEND the spec before writing
      wiring code.
- [ ] T008 [P0] Confirm BackupConfig field shape (P0.7) on Hiqlite
      v0.13.1: inspect upstream `NodeConfig` post-`backup`-feature
      shape (S3 endpoint, bucket, cron, retention, encryption key
      field names). Mirror in `src/config.rs`'s typed struct.
- [ ] T009 [P0] Confirm spec 144 timing (P0.8): is spec 144 already
      shipped, in-flight, or independent? Either way, spec 145 is
      unblocked because deployd-api-rs is a separate Cargo workspace.
      Document the timing relationship for the PR description.

**Checkpoint**: Open questions §6 are resolved (or AMEND'd into the
spec). Phase 1 cannot start until this checkpoint clears.

---

## Phase 1: Cargo + restore-on-startup

**Purpose**: enable Hiqlite features, wire `NodeConfig.backup_config`
from env, and implement restore-on-startup.

### Tests for Phase 1

> **TDD ordering note.** The three Phase 1 tests have different
> compile dependencies on the implementation, despite the shared `[P]`
> marker:
>
> - **T010 (`BackupConfig::from_env`)** can be authored RED-first
>   against a stub struct (declare the struct + the `from_env`
>   signature returning `unimplemented!()` in T022, then write the
>   test, then fill in T022). Stub-driven TDD is appropriate here
>   because the API surface is new and small.
> - **T011 (`init_db` without BackupConfig)** requires T023 to compile
>   — `init_db` only takes its new arg shape after T023 lands.
>   Author the test once T023's signature is stable.
> - **T012 (restore against test endpoint)** requires T024 to compile.
>   The test is `#[ignore]`-gated and only runs when a localstack /
>   minio endpoint is configured.

- [ ] T010 [P] [P1] Test:
      `platform/services/deployd-api-rs/src/config.rs` (or
      `tests/config_test.rs`): `BackupConfig::from_env` honours all
      env vars; returns `None` when no env keys are set; errors when
      partial config is supplied (e.g. endpoint set, secret key
      missing).
- [ ] T011 [P] [P1] Test (Rust integration):
      `platform/services/deployd-api-rs/tests/store_test.rs` —
      `init_db` against a temp dir without BackupConfig matches
      current behaviour.
- [ ] T012 [P] [P1] Test (Rust integration, `#[ignore]` by default):
      `platform/services/deployd-api-rs/tests/restore_test.rs` —
      restore against a known snapshot when an S3-compatible test
      endpoint is configured. Documented in the runbook for manual
      pre-merge runs.

### Implementation

- [ ] T020 [P1] Edit `platform/services/deployd-api-rs/Cargo.toml:17`
      to:
      ```toml
      hiqlite = { version = "~0.13", default-features = false, features = ["sqlite", "backup", "s3", "auto-heal"] }
      ```
- [ ] T021 [P1] Regenerate
      `platform/services/deployd-api-rs/Cargo.lock` via
      `cargo check --manifest-path platform/services/deployd-api-rs/Cargo.toml`
      (or `cargo generate-lockfile --manifest-path platform/services/deployd-api-rs/Cargo.toml`).
      Inspect diff: `cron` enters; `cryptr`, `s3-simple`, `deadpool`,
      `rusqlite` already present. Halt if any direct dep appears or
      the diff exceeds feature activation.
- [ ] T022 [P1] Add `BackupConfig` struct to
      `platform/services/deployd-api-rs/src/config.rs`. Fields per
      `plan.md` §Phase 1 step 3. Loader reads from env (`DEPLOYD_BACKUP_*`
      prefix); returns `None` when no env keys set; returns `Err` on
      partial config.
- [ ] T023 [P1] Wire `NodeConfig.backup_config` in
      `platform/services/deployd-api-rs/src/store.rs::init_db`
      (lines 13-33). Populate from `BackupConfig::from_env()` when
      `Some`; leave default when `None`.
- [ ] T024 [P1] Implement restore-on-startup in
      `platform/services/deployd-api-rs/src/main.rs:24-28` (or in
      `store.rs::init_db` if upstream supports inline). Algorithm:
      inspect data dir → if empty + BackupConfig → restore from
      latest snapshot → fail fast on restore error. Readiness probe
      stays not-Ready until `init_db` returns Ok.
- [ ] T025 [P1] Run T010 → green.
- [ ] T026 [P1] Run T011 → green.
- [ ] T027 [P1] Run
      `cargo build --manifest-path platform/services/deployd-api-rs/Cargo.toml`
      → exit 0.
- [ ] T028 [P1] Run
      `cargo clippy --manifest-path platform/services/deployd-api-rs/Cargo.toml --all-targets -- -D warnings`
      → exit 0 (warnings are errors).
- [ ] T029 [P1] Run
      `cargo test --manifest-path platform/services/deployd-api-rs/Cargo.toml`
      → all non-`#[ignore]` tests pass.

**Phase 1 exit:** deployd-api-rs builds clean with the new feature
posture; `BackupConfig::from_env` is a function the chart's env
wiring can target.

---

## Phase 2: Chart edits

**Purpose**: flip persistence on, narrow the scrub, wire BackupConfig
env entries, project the new secrets.

- [ ] T030 [P2] Edit
      `platform/charts/deployd-api/values-hetzner.yaml:34-38` per
      FR-001:
      ```yaml
      persistence:
        enabled: true
        size: 1Gi               # or T002-confirmed minimum (e.g. 10Gi)
        storageClassName: hcloud-volumes
      ```
      Drop the "stealth stage" comment. Replace with one-sentence
      rationale per `spec.md` §2.1.
- [ ] T031 [P2] Edit other env files per T006:
      `values-local.yaml` adds explicit `persistence.enabled: false`
      with a one-sentence inline rationale ("Dev loop only — emptyDir
      is fine; restore-on-startup is opt-in via env-supplied
      BackupConfig which is unset in this profile").
      `values-azure.yaml`, `values-aws.yaml`, `values-gcp.yaml`,
      `values-do.yaml` are **not** edited — they inherit the chart
      default silently per the resolved decision in T006.
- [ ] T032 [P2] Edit
      `platform/charts/deployd-api/templates/deployment.yaml:39-43`
      per FR-003 and the T003 decision (Option A or B).
- [ ] T033 [P2] Extend
      `platform/charts/deployd-api/templates/deployment.yaml`
      `env:` block with BackupConfig non-sensitive fields:
      `DEPLOYD_BACKUP_S3_ENDPOINT`, `DEPLOYD_BACKUP_S3_BUCKET`,
      `DEPLOYD_BACKUP_S3_PATH_PREFIX`, `DEPLOYD_BACKUP_CRON_SCHEDULE`,
      `DEPLOYD_BACKUP_KEEP_COUNT`, sourced from `.Values.backup.*`.
- [ ] T034 [P2] Extend
      `platform/charts/deployd-api/templates/deployment.yaml`
      `env:` block with BackupConfig sensitive fields:
      `DEPLOYD_BACKUP_S3_ACCESS_KEY`,
      `DEPLOYD_BACKUP_S3_SECRET_KEY`, `DEPLOYD_BACKUP_CRYPTR_KEY`,
      sourced from `secretRef` against the secret projected by
      `external-secret.yaml`.
- [ ] T035 [P2] Edit
      `platform/charts/deployd-api/templates/external-secret.yaml`
      to add three new keys: `backup-s3-access-key`,
      `backup-s3-secret-key`, `backup-cryptr-key`. Names match the
      operator-side secret store layout from T005.
- [ ] T036 [P2] Update
      `platform/charts/deployd-api/values.yaml` to declare new
      chart-level keys under `backup:`:
      ```yaml
      backup:
        endpoint: ""           # operator-supplied per env
        bucket: ""             # operator-supplied per env
        pathPrefix: ""         # optional
        schedule: "0 */6 * * *"  # NFR-002 default; operator-overridable
        keep: 28                 # NFR-002 default; operator-overridable
      ```
      Sensitive keys (access key, secret key, cryptr key) are NOT
      declared in values; they live in the secret only. The
      operator-side per-env values files override `endpoint` and
      `bucket` (and optionally `pathPrefix`, `schedule`, `keep`).
- [ ] T037 [P2] Helm-render smoke:
      `helm template platform/charts/deployd-api -f
      platform/charts/deployd-api/values-hetzner.yaml` → exit 0.
      Inspect rendered Deployment for BackupConfig env entries
      (sensitive via secretKeyRef, non-sensitive via value) and the
      narrowed startup args.
- [ ] T038 [P2] Helm-render smoke for one other env file (e.g.
      `values-azure.yaml`) to confirm the chart default flow still
      works.

**Phase 2 exit:** chart edits clean; `helm template` green; AC-6
baseline established.

---

## Phase 3: Runbook

**Purpose**: capture the operational contract.

- [ ] T040 [P3] Author `docs/runbooks/deployd-api-durability.md`
      per `spec.md` §2.5. Sections: Prerequisites; Secret store
      layout (the keys added in T035); Helm values (the keys added
      in T036); Backup verification (how to confirm a snapshot
      succeeded); DR restore procedure; Key-rotation considerations
      (NFR-004 — surface only, implementation deferred).
- [ ] T041 [P3] Operator review: walk the runbook through a fresh-
      cluster scenario; capture sign-off (or revisions) in the PR
      description. (AC-7.)

**Phase 3 exit:** runbook merged in working tree; operator sign-off
captured.

---

## Phase 4: Live validation + spec close

**Purpose**: validate AC-1 through AC-9 against the running Hetzner
deploy, refresh registries, mark spec implementation complete.

> **Hands-on ownership note.** T050–T055 are live-cluster ops:
> `kubectl delete pod`, `kubectl delete pvc`, querying live data,
> waiting on real readiness probes. The implementation agent **MUST
> pause and request explicit ack from the user before each
> destructive `kubectl` invocation** (pod delete, pvc delete) and
> before each AC-validation step. Auto mode does not authorise
> autonomous destructive ops against a live cluster. The agent's
> role in Phase 4 is to prepare commands, surface diagnostics, and
> walk the operator through them — not to drive them.

- [ ] T050 [P4] **Pre-deploy.** Add the three new secret keys to the
      operator-side secret store (Azure Key Vault entries per T005).
      Confirm `kubectl get secret -n <ns>` shows the projected
      ExternalSecret has refreshed with the new keys.
- [ ] T051 [P4] Deploy to Hetzner: `make deploy-hetzner` (or the
      equivalent chart-apply path). Pod comes up Ready.
- [ ] T052 [P4] **AC-1 — pod eviction.** Insert (or pick) a known
      row in `deployments` and a known row in `deployment_events`.
      `kubectl delete pod -n <ns> deployd-api-...`. Wait for
      replacement Ready. Re-query — both rows present.
- [ ] T053 [P4] **AC-3 — cron snapshot emission.** Wait one cron
      cycle (or trigger manually if upstream supports). Verify a new
      object appears in S3 at the configured prefix. Decrypt with
      the cryptr key on a workstation as a smoke check.
- [ ] T054 [P4] **AC-2 — fresh-PVC restore.** With AC-3 confirmed,
      `kubectl delete pvc -n <ns> <pvc-name>` then force pod
      restart. Watch readiness probe — pod stays NotReady through
      the restore window. On Ready, query — rowset matches the most
      recent snapshot.
- [ ] T055 [P4] **AC-4 — scrub no longer deletes data.** Pod restart
      against the (now repopulated) PVC. Confirm `deployments` and
      `deployment_events` rowsets unchanged.
- [ ] T056 [P4] **AC-5.** Re-run
      `cargo build / check / clippy / test --manifest-path
      platform/services/deployd-api-rs/Cargo.toml` → exit 0.
- [ ] T057 [P4] **AC-6.**
      `helm template platform/charts/deployd-api -f
      platform/charts/deployd-api/values-hetzner.yaml` renders
      cleanly (re-confirmed against the post-PR working tree).
- [ ] T058 [P4] **AC-9.** `make ci` (warm) → exit 0.
- [ ] T059 [P4] **AC-8.**
      `./tools/spec-code-coupling-check/target/release/spec-code-coupling-check`
      → no warnings against spec 145's `implements:` list.
- [ ] T060 [P4] Recompile spec registry + codebase index:
      `./tools/spec-compiler/target/release/spec-compiler compile`
      and
      `./tools/codebase-indexer/target/release/codebase-indexer compile && render`.
- [ ] T061 [P4] Update spec 145 frontmatter:
      `implementation: complete`, `closed: "<today>"`. Recompile
      registry. Confirm `registry-consumer status-report` reflects
      the change.
- [ ] T062 [P4] Open PR. Title:
      `feat(spec-145): deployd-api durability chain — PVC + scrub-narrow + s3 backup + restore-on-startup`.

**Phase 4 exit:** AC-1 through AC-9 in `spec.md` §3.3 pass; PR open.

---

## Acceptance criteria mapping

| AC | Tasks |
|---|---|
| AC-1 (pod eviction durability) | T030, T032, T051, T052 |
| AC-2 (fresh-PVC restore) | T020, T024, T034, T035, T053, T054 |
| AC-3 (encrypted snapshots in S3) | T020, T022, T023, T033, T053 |
| AC-4 (scrub does not delete data) | T032, T055 |
| AC-5 (Cargo build/test green; SBOM clean) | T020, T021, T027–T029, T056 |
| AC-6 (helm template green; rendered Deployment correct) | T030–T037, T057 |
| AC-7 (runbook sufficient) | T040, T041 |
| AC-8 (coupling gate clean) | T059 |
| AC-9 (`make ci` warm green) | T058 |

---

## Quick reference — key file:line anchors

| File | Lines | Phase | Action |
|---|---|---|---|
| `platform/charts/deployd-api/values-hetzner.yaml` | 34-38 | 2 | persistence.enabled true + size + class |
| `platform/charts/deployd-api/values-azure.yaml` | (decision per T006) | 2 | inherit or explicit |
| `platform/charts/deployd-api/values-aws.yaml` | (decision per T006) | 2 | inherit or explicit |
| `platform/charts/deployd-api/values-gcp.yaml` | (decision per T006) | 2 | inherit or explicit |
| `platform/charts/deployd-api/values-do.yaml` | (decision per T006) | 2 | inherit or explicit |
| `platform/charts/deployd-api/values-local.yaml` | (decision per T006) | 2 | inherit or explicit |
| `platform/charts/deployd-api/values.yaml` | (new `backup:` block) | 2 | declare chart-level backup keys |
| `platform/charts/deployd-api/templates/deployment.yaml` | 39-43 | 2 | narrow rm scope (Option A/B) |
| `platform/charts/deployd-api/templates/deployment.yaml` | (env block) | 2 | wire BackupConfig env entries |
| `platform/charts/deployd-api/templates/external-secret.yaml` | (new keys) | 2 | three new sensitive keys |
| `platform/services/deployd-api-rs/Cargo.toml` | 17 | 1 | enable backup, s3, auto-heal |
| `platform/services/deployd-api-rs/Cargo.lock` | (regenerated) | 1 | cron enters; no new direct deps |
| `platform/services/deployd-api-rs/src/config.rs` | (new struct) | 1 | typed BackupConfig + env loader |
| `platform/services/deployd-api-rs/src/store.rs` | 13-33 | 1 | populate NodeConfig.backup_config |
| `platform/services/deployd-api-rs/src/main.rs` | 24-28 | 1 | restore-on-startup; readiness gate |
| `docs/runbooks/deployd-api-durability.md` | (new file) | 3 | operational contract |
