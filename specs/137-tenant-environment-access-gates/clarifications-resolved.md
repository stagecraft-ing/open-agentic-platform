# Phase 0 — Clarifications resolved

> Companion to [`spec.md`](./spec.md) and [`plan.md`](./plan.md). Locks
> the 6 §Clarifications open decisions into concrete commitments that
> downstream phases inherit. Per `plan.md` §"Constitution check"
> Principle III, no code under FR-001..FR-010 lands before this
> document closes.

**Status:** Partial. Decisions 1, 2, 4, 5, 6 are locked from spec
analysis and engineering judgement and are immediately load-bearing
for Phases 1–6. Decision 3 (Rauthy admin API contract) requires
empirical smoke against the running Rauthy instance and remains open
under T003.

---

## Decision 1 — oauth2-proxy topology: **per-environment Deployment**

**Locked.** One `oauth2-proxy` Deployment per gated environment,
isolated config, single replica.

**Rationale.**

- Cookie-domain + redirect-URI multiplexing in a shared proxy is a
  real complexity surface; per-env keeps the proxy's mental model
  one-tenant-one-config.
- Blast radius: a misconfigured allowlist on one tenant cannot bleed
  to another.
- Pod count scales linearly with gated-env count, not tenant count.
  At an estimated 10 gated environments in the first year, that is
  ~10 proxy pods at 50–100Mi each — well under the cluster's per-node
  allocatable on any of the supported targets.

**Revisit criterion.** When gated-env count crosses 50 (≈5GB
aggregate proxy footprint at 100Mi/pod ceiling), file a follow-up
spec to evaluate a multi-tenant proxy with per-host config
multiplexing. Until then, per-env stands.

**Maps to:** `plan.md` Phase 4 (T041 — render oauth2-proxy
Deployment + ClusterIP Service in the deployment's namespace).

---

## Decision 2 — Schema shape: **dedicated sibling tables**

**Locked.** Two new tables, not a JSONB column on `environments`:

1. `environment_access_gates`
   `(environment_id PK FK→environments.id ON DELETE CASCADE,
   enabled bool NOT NULL DEFAULT false,
   rauthy_client_ref text NULL,
   login_method_magic_link bool NOT NULL DEFAULT true,
   login_method_federated_provider text NULL,
   login_method_federated_provider_client_ref text NULL,
   created_at timestamptz NOT NULL DEFAULT now(),
   updated_at timestamptz NOT NULL DEFAULT now())`
   with `CHECK (enabled = false OR rauthy_client_ref IS NOT NULL)`
   enforcing the non-null Rauthy fields when enabled.

2. `environment_access_gate_allowlist_emails`
   `(id bigserial PK,
   environment_id FK→environments.id ON DELETE CASCADE,
   kind text NOT NULL CHECK (kind IN ('email','domain')),
   value text NOT NULL,
   created_at timestamptz NOT NULL DEFAULT now())`
   with `UNIQUE (environment_id, kind, lower(value))` enforcing
   case-insensitive uniqueness without `citext` (the platform is
   FIPS-mode Postgres per `reference_hetzner_postgres_fips`).

**Rationale.**

- Allowlist rows as rows (not JSON array elements) gives row-level
  FKs, individual audit-log rows per allowlist add/remove, and
  efficient indexing for the post-auth callback check.
- JSONB on `environments` would force `jsonb_array_elements` at
  every allowlist check and make audit-log shape per-entry awkward.
- `ON DELETE CASCADE` keeps gate state consistent with environment
  lifecycle without application-layer fan-out.

**Migration target.** Next free numeric prefix in
`platform/services/stagecraft/api/db/migrations/` at PR time.

**Maps to:** `plan.md` Phase 1 (T010–T014).

---

## Decision 3 — Rauthy admin API contract: **OPEN; requires empirical smoke**

**Not yet locked.** The contract assumptions land in T003 against
the running Rauthy instance:

| Assumption | Confirm by |
|---|---|
| (a) Admin API tolerates ≥10 clients per namespace | `for i in 1..10: POST /auth/v1/clients` cycle; observe stable response times |
| (b) `DELETE /auth/v1/clients/{id}` returns 204 and the client is gone | Round-trip create→delete→get-404 |
| (c) `password_login_enabled` is a PATCHable field (not recreate-only) | `PATCH /auth/v1/clients/{id}` with `{password_login_enabled: false}` then `GET` |
| (d) `auth_provider_id` reference on client creation is supported and references shared Auth Providers cleanly | Create two clients pointing at the same Auth Provider; both list it in their config |

**Evidence path.**
`execution/rauthy-admin-smoke.md` (to be authored in T003). The
smoke runs against `rauthy.platform.svc.cluster.local` (or the
external hostname depending on where the smoke is executed) and
captures request/response pairs verbatim.

**Cluster access required.** This decision blocks Phase 3 (Rauthy
admin client + provisioning). It does NOT block Phase 1 (schema)
or Phase 2 (API CRUD), since those layers store the descriptor
without yet touching Rauthy. Phase 1 + 2 may proceed against the
default assumptions; Phase 3 work is gated on the smoke
confirming (a)–(d) or surfacing a counter-finding that reshapes
the contract.

**Maps to:** `plan.md` Phase 3 (T030–T034).

---

## Decision 4 — Hostname stability: **`<env-slug>.<project-slug>.<org-slug>.tenants.<base>`**

**Locked.** Tenant environment hostnames follow the four-label
pattern:

```
<env-slug>.<project-slug>.<org-slug>.tenants.<base-domain>
```

Example: `staging.checkout.acme.tenants.platform.example.com`.

**Rationale.**

- Stable input to `redirect_uri` on Rauthy clients — toggling the
  gate on/off does not change the hostname, so the Rauthy client
  does not need re-registration on toggle (FR-009).
- Four-label structure unambiguously identifies (org, project, env)
  triple from the hostname alone — useful for ingress-nginx host
  routing, oauth2-proxy cookie-domain scoping (`.<base-domain>`
  scope works for any tenant), and audit-log enrichment.
- `tenants.<base>` segregates tenant hostnames from platform
  hostnames (`stagecraft.<base>`, `rauthy.<base>`, etc.), giving a
  cookie-domain boundary and a clear DNS / wildcard-cert zone for
  tenants.

**Wildcard cert implication.** A single `*.tenants.<base>` cert
covers every tenant env — but the four-label depth requires a
two-level wildcard (`*.*.*.tenants.<base>`), which is NOT supported
by standard X.509 wildcards. Two viable paths:

- **Option A — per-org wildcard cert** (`*.<org-slug>.tenants.<base>`,
  one cert per org). Practical: ~1 cert per onboarded org, signed
  via Let's Encrypt or platform CA. Manageable scale.
- **Option B — per-tenant cert** (`<env-slug>.<project-slug>.<org-slug>.tenants.<base>`,
  one cert per env). cert-manager + Let's Encrypt handles this
  fine at small N but stresses LE rate limits past ~50 envs.

**Selected.** Option A (per-org wildcard cert). cert-manager
provisions `*.<org-slug>.tenants.<base>` on org creation in
stagecraft; deployd-api consumes the cert by reference, no
per-deployment cert work. Falls back to Option B if a tenant
requests a custom hostname outside the `tenants.<base>` zone (out
of scope for this spec).

**Maps to:** `plan.md` Phase 4 (T041, T042 — ingress annotation
shape uses the four-label hostname for cookie-domain + redirect-URI
derivation).

---

## Decision 5 — User → Rauthy user mapping: **auto-provision on allowlist add**

**Locked.** When an admin adds an email to
`environment_access_gate_allowlist_emails`:

1. **If `login_methods.magic_link == true`:** stagecraft calls the
   Rauthy admin API to upsert a Rauthy user with
   `password_login_enabled: false`, `magic_link_enabled: true`,
   no password hash, scope = `openid email profile`. The user
   record exists in Rauthy whether or not they have logged in
   yet; the first magic-link request mails the token.
2. **If only `login_methods.federated` is configured** (no magic
   link): stagecraft does NOT auto-provision a Rauthy user. The
   user record materialises on first federated login via Rauthy's
   silent-link behaviour (Rauthy creates the user account binding
   the upstream identity at OIDC callback time). The allowlist
   entry is purely an oauth2-proxy post-auth gate.

**Edge cases:**

- **Allowlist removal:** the Rauthy user record is NOT deleted on
  allowlist row removal. Rationale: the user may be on multiple
  tenant allowlists; deleting the Rauthy user would break other
  gates. The deny happens at oauth2-proxy. Stagecraft surfaces a
  "purge this user from Rauthy" admin action separately
  (not in scope for this spec — file as follow-up spec).
- **User-already-exists collision:** Rauthy upsert is idempotent
  on `(email, password_login_enabled: false)`. If the user already
  exists in Rauthy with `password_login_enabled: true`, stagecraft
  flips the field via PATCH and emits an audit row
  `rauthy.user.password_login_disabled` recording the
  flip. (Confirmed in Decision 3 (c) once T003 lands.)
- **Domain allowlist entries:** do NOT auto-provision Rauthy users
  on domain entry creation — the universe is unbounded. Magic-link
  login from a domain-allowlisted email triggers Rauthy's
  on-the-fly user creation at email-entry time.

**Maps to:** `plan.md` Phase 2 (T021 PUT handler) and Phase 3
(T031 `provisionTenantGateClient`).

---

## Decision 6 — Auth Providers configuration UX: **Rauthy admin UI for v1**

**Locked.** Configuring upstream OIDC clients (Google, Microsoft
Entra, GitHub, generic OIDC) in Rauthy happens through Rauthy's
own admin UI, not through stagecraft.

**Rationale.**

- Auth Providers are per-Rauthy-deployment, not per-tenant. There
  is typically one Google client, one Microsoft client, etc., across
  the whole platform — same primitives consumed by many tenant
  gate clients.
- Rauthy's admin UI already supports Auth Provider configuration
  natively; reimplementing it in stagecraft is duplicate UX work
  for a low-frequency admin task.
- Stagecraft's role is to surface the configured Auth Providers in
  the per-environment login-method picker (T052) — read-only, by
  ID. Stagecraft calls Rauthy admin API
  `GET /auth/v1/auth_providers` and lists them in the dropdown.

**Follow-up.** A future spec MAY surface Auth Provider
configuration in the stagecraft admin panel for operators who
want a single-pane UX. That spec inherits this one's contracts
(read-only via T052 stays correct; the new spec adds write paths
via the stagecraft surface). Not scoped here.

**Maps to:** `plan.md` Phase 5 (T052 — federated provider
dropdown shows the Rauthy-configured list).

---

## Status field updates *(deferred until T003 lands)*

Per `plan.md` Phase 0 §"Phase 0 deliverables": `status: draft →
approved` flips when Phase 0 closes. This document represents 5/6
clarifications locked; flipping the status before T003 lands would
ship a half-locked contract under approved status. The status flip
is held until T003 surfaces the Rauthy admin smoke evidence (or
amends Decisions 5's collision-handling clause if (c) returns a
counter-finding).

**Trigger for status flip:** T003 evidence lands under
`execution/rauthy-admin-smoke.md`, T007 confirms or amends the
above five decisions in light of T003 findings, then status:
draft → approved in a dedicated commit.

## Cross-spec coherence

This document does not modify any spec outside 137. Cross-spec
references (specs 087, 119, 136) cited above are read-only — no
amend relationships are created by Phase 0. Future phases may
introduce amends if downstream specs need narrative updates
(e.g. stagecraft schema spec gains a §"access-gate sibling tables"
entry); those are filed at the time of the amending PR, not now.

## Diff plan from this document to spec.md

`spec.md` §Clarifications currently lists the 6 items with
"Recommend …" language. After T003 lands and Decisions are
re-confirmed, the spec.md §Clarifications section is amended to
replace "Recommend" prose with "Resolved" cross-references
pointing back to this document (no content duplication — single
source of truth lives here, spec.md gains pointers). The amend
ships in the same PR as the status flip.
