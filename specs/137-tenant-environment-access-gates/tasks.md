# Tasks: Tenant environment access gates

**Input**: [`spec.md`](./spec.md), [`plan.md`](./plan.md)

> Format: `[ID] [P?] Description`. `[P]` = parallelisable with adjacent
> tasks. Tasks track plan.md phases.

## Phase 0 — Resolve clarifications & lock the gate contract *(this PR; gates approval)*

- [ ] T001 Decide §Clarification 1 (oauth2-proxy topology). Recommend
  per-env Deployment with explicit "revisit when pod count exceeds N"
  exit criterion. Document in spec.md or
  `clarifications-resolved.md` companion.
- [ ] T002 Decide §Clarification 2 (schema shape). Recommend dedicated
  `environment_access_gates` table + sibling
  `environment_access_gate_allowlist_emails`. Pin FK + cascade
  behaviour.
- [ ] T003 Decide §Clarification 3 (Rauthy admin API contract).
  Smoke against the running Rauthy instance to confirm:
  (a) admin API tolerates one client per gated env at expected scale,
  (b) DELETE /clients/{id} is clean,
  (c) toggling `password_login_enabled` is a PATCH, not a recreate,
  (d) Auth Provider id reference shape on client creation. Capture
  evidence path under `execution/rauthy-admin-smoke.md`.
- [ ] T004 Decide §Clarification 4 (hostname stability). Pick the
  canonical pattern (e.g.
  `<env-slug>.<project-slug>.<org-slug>.tenants.<base>`); document the
  `redirect_uri` shape that follows.
- [ ] T005 Decide §Clarification 5 (user → Rauthy mapping). Pin
  auto-provision + magic-link defaults; document allowlist-removal
  semantics (revoke vs leave-orphaned) and user-already-exists
  collision handling.
- [ ] T006 Decide §Clarification 6 (Auth Providers UX). Confirm
  Rauthy admin UI is the v1 surface; surface a stagecraft follow-up
  spec id for future ergonomic work.
- [ ] T007 Reviewer pass on the contract + clarifications; flip
  `status: draft → approved` in spec.md frontmatter. Add
  `approved: <date>` field. No code changes under FR-001..FR-010
  before this lands.

**Checkpoint:** Phase 0 closes when T007 ships. Phases 1+ are blocked
behind this checkpoint per Principle III.

---

## Phase 1 — Schema migration

- [ ] T010 Author
  `platform/services/stagecraft/api/db/migrations/3X_environment_access_gates.up.sql`:
  `environment_access_gates` table with `(environment_id PK, enabled,
  rauthy_client_ref, login_method_*, created_at, updated_at)` +
  CHECK constraint enforcing non-null Rauthy fields when
  `enabled = true`.
- [ ] T011 [P] Author sibling
  `environment_access_gate_allowlist_emails` table:
  `(id PK, environment_id FK, kind ENUM('email','domain'), value text,
  created_at)` + unique index on `(environment_id, kind, value)`.
- [ ] T012 [P] Author down migration that drops both tables.
- [ ] T013 Drizzle schema additions in
  `platform/services/stagecraft/api/db/schema.ts`. Type-export the new
  shapes for the API layer.
- [ ] T014 Migration test (`encore test`) covering up/down idempotency
  and the CHECK constraint behaviour. Mirror migration 36/37 fixture
  shape.

**Checkpoint:** Schema migration lands cleanly + tests pass.

---

## Phase 2 — Stagecraft API CRUD

- [ ] T020 `api/environments/accessGates.ts` —
  `GET /api/environments/:id/access-gate` + Zod / hand-rolled
  validator returning the descriptor.
- [ ] T021 `PUT /api/environments/:id/access-gate` — create-or-update
  the descriptor; emits an audit row; validates allowlist
  non-emptiness when `enabled = true`.
- [ ] T022 `POST /api/environments/:id/access-gate/allowlist` /
  `DELETE .../allowlist/:entryId` — append + remove allowlist entries.
- [ ] T023 [P] Audit-log integration: every change emits a
  `tenant.gate.descriptor.{enabled,disabled,allowlist.added,
  allowlist.removed,login_methods.changed}` audit row.
- [ ] T024 [P] Validation guard refusing any field that looks like a
  password (defense in depth; the schema has no such field, but the
  API rejects shapes that look like passwords to catch upstream bugs
  early). FR-007 invariant.
- [ ] T025 Vitest coverage of the four endpoints + the audit emission.

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
