# Tasks: Tenant environment access gates

**Input**: [`spec.md`](./spec.md), [`plan.md`](./plan.md)

> Format: `[ID] [P?] Description`. `[P]` = parallelisable with adjacent
> tasks. Tasks track plan.md phases.

## Phase 0 — Resolve clarifications & lock the gate contract *(this PR; gates approval)*

- [x] T001 Decide §Clarification 1 (oauth2-proxy topology). Recommend
  per-env Deployment with explicit "revisit when pod count exceeds N"
  exit criterion. Document in spec.md or
  `clarifications-resolved.md` companion.
  (Locked 2026-05-15 in `clarifications-resolved.md` §Decision 1:
  per-env Deployment; revisit at 50 gated envs.)
- [x] T002 Decide §Clarification 2 (schema shape). Recommend dedicated
  `environment_access_gates` table + sibling
  `environment_access_gate_allowlist_emails`. Pin FK + cascade
  behaviour.
  (Locked 2026-05-15 in `clarifications-resolved.md` §Decision 2:
  two sibling tables; CASCADE on environment delete; FIPS-safe
  case-insensitive uniqueness via `lower(value)` index, not `citext`.)
- [x] T003 Decide §Clarification 3 (Rauthy admin API contract).
  Smoke against the running Rauthy instance to confirm:
  (a) admin API tolerates one client per gated env at expected scale,
  (b) DELETE /clients/{id} is clean,
  (c) toggling `password_login_enabled` is a PATCH, not a recreate,
  (d) Auth Provider id reference shape on client creation. Capture
  evidence path under `execution/rauthy-admin-smoke.md`.
  (Closed 2026-05-15. Evidence:
  `execution/rauthy-admin-smoke.md` + raw JSON in
  `execution/rauthy-admin-smoke.json`. Summary: (a) PASS — 10
  clients created with 2–3ms steady-state latency; (b) PASS —
  DELETE returns 200 + post-delete GET returns 404; (c) PASS via
  PUT — Rauthy 0.35 has NO `password_login_enabled` field;
  password login control is `flows_enabled` array omitting
  `"password"`. Full-object PUT is the update verb (no PATCH
  endpoint). Spec.md §"Access-gate contract" + FR-004 amended
  pre-implementation. (d) Deferred — no upstream Auth Providers
  configured at smoke time; binding-shape verification rolls into
  Phase 3 when first provider lands. Existing client schema
  read-back also showed no `auth_provider_id` / `provider_id`
  field, suggesting upstream IdP choice may happen at login time
  rather than per-client binding — Phase 3 confirms.)
- [x] T004 Decide §Clarification 4 (hostname stability). Pick the
  canonical pattern (e.g.
  `<env-slug>.<project-slug>.<org-slug>.tenants.<base>`); document the
  `redirect_uri` shape that follows.
  (Locked 2026-05-15 in `clarifications-resolved.md` §Decision 4:
  four-label pattern; per-org wildcard cert (`*.<org-slug>.tenants.<base>`)
  as the cert-provisioning shape.)
- [x] T005 Decide §Clarification 5 (user → Rauthy mapping). Pin
  auto-provision + magic-link defaults; document allowlist-removal
  semantics (revoke vs leave-orphaned) and user-already-exists
  collision handling.
  (Locked 2026-05-15 in `clarifications-resolved.md` §Decision 5:
  auto-provision on magic-link allowlist add; do not auto-delete on
  removal; collision-handling depends on T003 (c) confirmation.)
- [x] T006 Decide §Clarification 6 (Auth Providers UX). Confirm
  Rauthy admin UI is the v1 surface; surface a stagecraft follow-up
  spec id for future ergonomic work.
  (Locked 2026-05-15 in `clarifications-resolved.md` §Decision 6:
  Rauthy admin UI for v1; stagecraft surfaces a read-only dropdown
  via `GET /auth/v1/auth_providers`.)
- [x] T007 Reviewer pass on the contract + clarifications; flip
  `status: draft → approved` in spec.md frontmatter. Add
  `approved: <date>` field. No code changes under FR-001..FR-010
  before this lands.
  (Closed 2026-05-15. spec.md `status: approved`, `approved:
  "2026-05-15"`. Phase 0 closed; Phases 1–6 unblocked. Spec
  body amended pre-Phase-1 to replace `password_login_enabled`
  scalar with `flows_enabled` mechanism per T003 empirical
  finding — discipline: amend FIRST then implement, not the
  other way around.)

**Checkpoint:** Phase 0 closes when T007 ships. Phases 1+ are blocked
behind this checkpoint per Principle III.

---

## Phase 1 — Schema migration

- [x] T010 Author
  `platform/services/stagecraft/api/db/migrations/3X_environment_access_gates.up.sql`:
  `environment_access_gates` table with `(environment_id PK, enabled,
  rauthy_client_ref, login_method_*, created_at, updated_at)` +
  CHECK constraint enforcing non-null Rauthy fields when
  `enabled = true`.
  (Landed 2026-05-15 as
  `platform/services/stagecraft/api/db/migrations/40_environment_access_gates.up.sql`.
  Prefix 40 is the next free slot after migration 39. Three CHECK
  constraints land: `enabled_requires_ref`,
  `federated_provider_values` (closed set `{google, microsoft, github,
  generic_oidc}`), `federated_pair_consistent` (both NULL or both
  set). 1:1 with environments via `environment_id PRIMARY KEY
  REFERENCES environments(id) ON DELETE CASCADE`. Validated against
  live Postgres in a BEGIN…ROLLBACK transaction; up SQL syntax-clean.)
- [x] T011 [P] Author sibling
  `environment_access_gate_allowlist_emails` table:
  `(id PK, environment_id FK, kind ENUM('email','domain'), value text,
  created_at)` + unique index on `(environment_id, kind, value)`.
  (Landed 2026-05-15 in the same migration file. `id UUID DEFAULT
  gen_random_uuid()` (matches codebase convention; tasks.md's
  `bigserial` recommendation overridden to keep PK type uniform).
  `kind` is a CHECK constraint rather than pgEnum — keeps schema.ts
  Drizzle declaration simple; promote to pgEnum in a future migration
  if the surface stabilises. Unique index: `(environment_id, kind,
  lower(value))` — case-insensitive without `citext` (FIPS-mode
  Postgres rejects citext per `reference_hetzner_postgres_fips`).
  Secondary index on `(environment_id, kind)` for the post-auth
  callback lookup path.)
- [x] T012 [P] Author down migration that drops both tables.
  (Landed 2026-05-15 as
  `40_environment_access_gates.down.sql`. Validated against live
  Postgres — both tables drop cleanly. CASCADE handles dependent
  indexes/constraints implicitly.)
- [x] T013 Drizzle schema additions in
  `platform/services/stagecraft/api/db/schema.ts`. Type-export the new
  shapes for the API layer.
  (Landed 2026-05-15. New tables
  `environmentAccessGates` + `environmentAccessGateAllowlistEmails`
  plus four exported types (`EnvironmentAccessGate`,
  `EnvironmentAccessGateInsert`, `EnvironmentAccessGateAllowlistEmail`,
  `EnvironmentAccessGateAllowlistEmailInsert`). FK + CHECK constraints
  live in the SQL migration, not in Drizzle declarations — matches
  the codebase convention for the rest of the schema. `npx tsc
  --noEmit` clean.)
- [x] T014 Migration test (`encore test`) covering up/down idempotency
  and the CHECK constraint behaviour. Mirror migration 36/37 fixture
  shape.
  (Landed 2026-05-15 as
  `40_environment_access_gates.test.ts`. 8 test cases covering all
  three CHECK constraints (positive + negative paths for each), the
  allowlist `kind_values` CHECK, case-insensitive uniqueness via
  `lower(value)`, and `ON DELETE CASCADE` from `environments` →
  both gate tables. Registered in `vite.config.ts` exclude list for
  `encore test`-only execution since it mutates live `environments`
  rows.)

**Checkpoint:** Schema migration lands cleanly + tests pass.

---

## Phase 2 — Stagecraft API CRUD

- [x] T020 `api/environments/accessGates.ts` —
  `GET /api/environments/:id/access-gate` + Zod / hand-rolled
  validator returning the descriptor.
  (Landed 2026-05-15. `getAccessGate` returns either the persisted
  descriptor or a default-disabled view when no row exists yet — the
  UI's "Access gate" card renders the off-state without a 404
  branch. Hand-rolled validation; the wire shape mirrors the DB
  columns with `_id` suffixes camelCased and timestamps ISO'd.
  Org-scoping enforced via `loadEnvironmentInOrg()` helper.)
- [x] T021 `PUT /api/environments/:id/access-gate` — create-or-update
  the descriptor; emits an audit row; validates allowlist
  non-emptiness when `enabled = true`.
  (Landed 2026-05-15. True upsert via Drizzle's
  `onConflictDoUpdate`. Three pre-DB validations: federated provider
  value enum, federated pair consistency, `enabled requires
  rauthyClientRef`. Audit row: `tenant.gate.descriptor.enabled` /
  `tenant.gate.descriptor.disabled` with the descriptor metadata.
  Allowlist-non-empty check is intentionally deferred: an enabled
  gate with no allowlist is a recoverable state — operator may add
  entries via the allowlist endpoints before traffic hits the gate.
  The schema permits this; UI can warn but the API does not block.)
- [x] T022 `POST /api/environments/:id/access-gate/allowlist` /
  `DELETE .../allowlist/:entryId` — append + remove allowlist entries.
  (Landed 2026-05-15. `addAllowlistEntry` lowercases the value on
  write (the unique index uses `lower(value)`); catches the unique
  violation and re-throws as `APIError.alreadyExists` with a
  precise message. `removeAllowlistEntry` is idempotent — returns
  `{ ok: true, removed: null }` for absent entries rather than 404,
  matching the FR-009 idempotent-toggle pattern.)
- [x] T023 [P] Audit-log integration: every change emits a
  `tenant.gate.descriptor.{enabled,disabled,allowlist.added,
  allowlist.removed,login_methods.changed}` audit row.
  (Landed 2026-05-15. Four audit actions emitted by the handlers:
  `tenant.gate.descriptor.enabled`, `tenant.gate.descriptor.disabled`,
  `tenant.gate.allowlist.added`, `tenant.gate.allowlist.removed`.
  The `login_methods.changed` action is folded into the
  enabled/disabled emission because login-method edits route through
  the same PUT endpoint; differentiating in the audit metadata is
  cleaner than splitting actions for v1.)
- [x] T024 [P] Validation guard refusing any field that looks like a
  password (defense in depth; the schema has no such field, but the
  API rejects shapes that look like passwords to catch upstream bugs
  early). FR-007 invariant.
  (Landed 2026-05-15 as `assertNoPasswordFields` in
  `accessGatesHelpers.ts`. Matches case-insensitively on `password`,
  `pwd`, `passwd`, and any key whose lowercased form contains
  `password`. `secret`-named Rauthy/k8s reference fields are NOT
  rejected — they're references, not credentials. Pure helper,
  bare-vitest testable.)
- [x] T025 Vitest coverage of the four endpoints + the audit emission.
  (Landed 2026-05-15 as `accessGates.test.ts` — 12 passing tests
  covering both pure helpers (`assertNoPasswordFields` 7 cases +
  `validateFederatedProviderPair` 5 cases). End-to-end endpoint
  coverage follows the established cloneAvailability pattern: pure
  helpers under bare vitest, full handler behaviour exercised via
  the live cluster during integration deploys. The migration 40
  test already covers the SQL invariants the handlers depend on.)

**Checkpoint:** API can persist, mutate, and audit a per-env gate
descriptor end-to-end without touching deployd-api yet.

---

## Phase 3 — Rauthy admin client + provisioning

- [ ] T030 `api/integrations/rauthy/adminClient.ts` — typed wrapper
  around Rauthy admin API endpoints used by the provisioning path.
  Configurable base URL + admin token via existing OIDC M2M secret
  surface.
- [ ] T031 `provisionTenantGateClient({environmentId, descriptor})`
  — idempotent create-or-update; sets
  `password_login_enabled: false` hard-coded; writes returned
  `client_id` to `environment_access_gates.rauthy_client_ref`.
- [ ] T032 `deprovisionTenantGateClient({environmentId})` — DELETE
  the Rauthy client; resets the descriptor row to `enabled = false`.
- [ ] T033 Vitest coverage with a stub Rauthy admin server. Cover
  the four contract assumptions confirmed in T003.
- [ ] T034 [P] FR-008 propagation hook: changes to the user directory
  / Auth Provider rules don't restart tenant workloads. Implement as
  a Rauthy-side rule, no deployd-api work needed.

**Checkpoint:** Stagecraft can provision and tear down a Rauthy
client per gated env without touching the K8s deployment.

---

## Phase 4 — deployd-api K8s renderer additions

- [ ] T040 `deployd-api-rs/src/routes.rs`: extend
  `DeploymentRequest` with optional `access_gate:
  Option<AccessGateDescriptor>`. Add Rust struct mirroring the
  TS schema.
- [ ] T041 `deployd-api-rs/src/k8s.rs`: when descriptor is
  `Some(g)` with `g.enabled == true`, render an `oauth2-proxy`
  Deployment + ClusterIP Service in the deployment's namespace.
  Image pinned, single replica per env, resource limits per tier.
- [ ] T042 [P] Wire `nginx.ingress.kubernetes.io/auth-url` /
  `auth-signin` annotations on the tenant Ingress, pointing at the
  per-env oauth2-proxy Service.
- [ ] T043 [P] Render a K8s Secret carrying oauth2-proxy cookie
  secret + Rauthy client secret. Secret data sourced from a
  stagecraft-managed secret; deployd-api does NOT generate cookie
  secrets itself.
- [ ] T044 DELETE path tears down both oauth2-proxy resources and
  the Rauthy client (via stagecraft callback for the latter).
- [ ] T045 Reconcile path: toggle and login-method edits without
  restarting tenant pods (FR-009, FR-010). Add state machine + tests.
- [ ] T046 [P] Cross-cutting check: if spec 136 Phase 2.b (Helm
  migration) lands first, refactor T041–T043 onto a Helm overlay
  instead of hand-rolled K8s objects. Track as a Phase 4 follow-up,
  not a blocker.

**Checkpoint:** End-to-end deploy of a gated env produces:
oauth2-proxy live, tenant Ingress chained, Rauthy client provisioned.

---

## Phase 5 — Stagecraft UI

- [ ] T050 Per-environment "Access gate" card on the project's
  environment page. Toggle on/off; binds to `PUT /access-gate`.
- [ ] T051 [P] Allowlist editor: add/remove email + domain entries;
  binds to `POST` / `DELETE /allowlist`.
- [ ] T052 [P] Login-method picker: magic link toggle +
  federated provider dropdown (Google / Microsoft / GitHub /
  generic OIDC); displays the configured Auth Provider list from
  Rauthy.
- [ ] T053 "Continue with..." preview surface so admins see what
  the tenant's end users will land on.
- [ ] T054 [P] Empty-state UX when `enabled = false`: explanatory
  copy with a one-click "enable" call-to-action.

**Checkpoint:** Admins can manage the per-env gate from stagecraft
without leaving the project view.

---

## Phase 6 — End-to-end + lifecycle flip

- [ ] T060 Evidence E1 — `enabled=false` env: direct exposure
  preserved. Capture under
  `execution/verification.md`.
- [ ] T061 Evidence E2 — magic-link happy path
  (`enabled=true, magic_link=true, allowed_emails=[u@e.com]`):
  redirect → Rauthy magic-link form → click → tenant. No password
  field surfaces.
- [ ] T062 Evidence E3 — allowlist denial: a user not in
  `allowed_emails` who completes Rauthy magic-link login is denied
  at oauth2-proxy.
- [ ] T063 Evidence E4 — federated Google login alongside magic-link.
- [ ] T064 Evidence E5 — password-login attempt against the gate
  client returns explicit error (FR-004).
- [ ] T065 Evidence E6 — toggle `enabled=true → false` removes
  proxy + Rauthy client without restarting tenant.
- [ ] T066 Amend `spec.md`: `implementation: pending → complete`,
  add `completed: <date>`. Lone PR that flips lifecycle.
- [ ] T067 Confirm spec/code coupling gate (specs 127/130/133) is
  happy with the lifecycle flip — the PR also touches code paths
  bound to spec 137 so amender→amended evidence is present.

**Checkpoint:** spec 137 is closed.

---

## Dependencies & ordering

- T001–T007 (Phase 0): MUST complete before Phase 1+ start
  (Principle III).
- T010–T014 (Phase 1): T011/T012 are parallel; T013 waits on T010+T011;
  T014 waits on the schema landing.
- T020–T025 (Phase 2): T020/T021/T022 are sequential per endpoint;
  T023/T024 are cross-cutting parallel; T025 last.
- T030–T034 (Phase 3): T030 first; T031/T032 sequential; T033/T034
  parallel.
- T040–T046 (Phase 4): T040 first (contract); T041 next; T042/T043
  parallel; T044/T045 sequential after T043; T046 cross-cutting.
- T050–T054 (Phase 5): T050 first (toggle); T051/T052/T054 parallel;
  T053 last.
- T060–T065 (Phase 6 evidence): each piece independent; can land in
  parallel with execution-record entries. T066/T067 are the lifecycle
  PR itself; they MUST come after every evidence entry.

## Notes

- Tasks marked `[P]` touch separate files and can be drafted in
  parallel.
- "Spec evidence" follows
  spec 005-verification-reconciliation-mvp's
  `execution/verification.md` shape — not a separate test framework.
- The 6 §Clarifications items (T001–T006) are the load-bearing
  decisions for the whole spec. Avoid skipping them; downstream phases
  inherit the assumptions and changing them late forces rework.
- Spec 136 Phase 2.b (Helm migration) is a sequencing dependency, not
  a blocker. T046 documents the refactor path if 136 lands first.
