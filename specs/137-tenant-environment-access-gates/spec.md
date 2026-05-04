---
id: "137-tenant-environment-access-gates"
title: "Tenant environment access gates — passwordless OIDC via Rauthy"
status: draft
implementation: pending
owner: bart
created: "2026-05-04"
kind: platform
risk: medium
depends_on:
  - "136"  # tenant-hello as reference; gates are added per-environment
  - "087"  # unified-workspace-architecture (environments are stagecraft entities)
implements: []
summary: >
  Per-environment access gating for projects deployed via deployd-api,
  applied above the tenant app so tenant codebases carry no auth logic.
  Passwordless OIDC via our existing Rauthy instance is the only mode —
  users authenticate by magic link or federated upstream IdP (Google,
  Microsoft Entra, GitHub, generic OIDC). Stagecraft and Rauthy clients
  for tenant gates are configured to refuse password login outright;
  the platform never sees, stores, or relays a password.
---

# Feature Specification: Tenant environment access gates

**Feature Branch**: `137-tenant-environment-access-gates`
**Created**: 2026-05-04
**Status**: Draft
**Input**: Tenant projects deployed through deployd-api need optional access
gating that lives in the platform layer (ingress + auth proxy), not in the
tenant codebase. Passwordless only, OIDC via Rauthy only — magic link
and/or federated upstream IdP. No basic auth, no shared passwords, no
password-handling code anywhere in the platform.

## Purpose and charter

Tenant deployments today are exposed as-is on the cluster ingress. There is
no platform-level gate, so every tenant either ships its own auth
implementation (which doubles work for low-stakes pre-launch environments)
or runs unprotected (which leaks pre-launch URLs to anyone with the
hostname). This spec adds an **opt-in, passwordless OIDC gate** the
platform owns:

- `oauth2-proxy` (or equivalent auth-request handler) sits in front of
  each gated tenant Ingress.
- The proxy authenticates against our existing **Rauthy** instance
  (`platform/charts/rauthy/`) using a per-environment OIDC client.
- The Rauthy client has `password_login_enabled: false`. Users reach the
  tenant by completing one of:
  - **Magic link** — Rauthy's built-in passwordless email login. The
    user enters their email, Rauthy mails a one-time link, the click
    completes the OIDC flow.
  - **Federated upstream IdP** — Rauthy's "Auth Providers" feature
    delegates to an upstream OIDC: Google, Microsoft Entra, GitHub, or
    any generic OIDC provider. Rauthy mints its own tokens after
    upstream validation; oauth2-proxy sees only Rauthy.
- Allowlist enforcement is two-layered: Rauthy completes login only for
  users in its directory (or whose upstream-IdP identity matches the
  configured Auth Provider rules), and oauth2-proxy validates the
  returned email against `allowed_emails` / `allowed_domains` on the
  post-auth callback.

Stagecraft and the tenant app **never** see passwords or upstream IdP
tokens. The only identity material that crosses into stagecraft's data
plane is the post-authentication subject (email + sub), and only if the
tenant app explicitly reads it from forwarded headers.

**Explicitly in scope:**

- A schema field on `environments` describing the gate (on/off + Rauthy
  client reference + allowlist).
- A contract addition to `POST /v1/deployments` carrying that descriptor
  through to deployd-api.
- deployd-api rendering the K8s objects for an enabled gate
  (oauth2-proxy Deployment + Service, Rauthy client provisioned via
  Rauthy's admin API, Ingress annotated with `auth-url`/`auth-signin`).
- Stagecraft UI for managing the gate per environment: toggle on/off,
  edit allowlist, choose which login methods Rauthy surfaces (magic
  link, federated, or both).
- A Rauthy client provisioning path: stagecraft creates one OIDC client
  per gated environment via Rauthy's admin API.

**Explicitly out of scope:**

- Basic auth, shared passwords, htpasswd Secrets, or any other
  password-bearing mechanism. Removed by directive.
- Tenant app-level auth (the tenant remains free to layer its own).
- Replacing Rauthy with a different IdP — Rauthy is the chosen primitive.
- Single sign-on across stagecraft and gated tenant environments — its
  own design decision and would inherit from this spec, not the other
  way around.
- Email allowlist UX for non-administrators (this spec only covers the
  admin path; self-service invitation flows are separate).
- Passkey / WebAuthn login. Rauthy supports it but this spec scopes to
  the two requested methods (magic link + federated). A follow-up spec
  can enable passkey on existing tenant clients without schema change.

## Current state vs intent

**Current state:**
- `environments` (`platform/services/stagecraft/api/db/schema.ts:219-233`)
  has `projectId, name, kind, k8sNamespace, autoDeployBranch,
  requiresApproval`. No access-gate field.
- deployd-api's `POST /v1/deployments` contract
  (`platform/services/deployd-api-rs/src/routes.rs:19-30`) takes
  `tenant_id, app_id, env_id, release_sha, artifact_ref, lane,
  app_slug?, env_slug?, desired_routes?`. No gate descriptor.
- deployd-api's K8s renderer
  (`platform/services/deployd-api-rs/src/k8s.rs:51-82`) creates raw
  `Deployment + Service + Ingress` with no auth annotations.
- Rauthy is deployed (`platform/charts/rauthy/`) and serves stagecraft's
  own auth, but no tenant Ingress has ever been configured against it,
  and no Auth Providers (Google/Microsoft/GitHub upstreams) are
  configured today.
- ingress-nginx is the cluster ingress controller (per
  `platform/CLAUDE.md`); `auth-url` / `auth-signin` annotations are
  available natively for chaining to oauth2-proxy.

**Intent:**
- Per-environment gate descriptor with `enabled: bool` + Rauthy client
  reference + allowlist + login-method config.
- Secret material (oauth2-proxy cookie secrets, Rauthy client secrets)
  lives in K8s Secrets, never in stagecraft Postgres. No password hashes
  exist anywhere in the platform.
- deployd-api's create-deployment path provisions the gate atomically
  with the tenant Deployment when `enabled: true`; the destroy path
  tears it down.
- Per-environment oauth2-proxy + per-environment Rauthy client gives
  isolation; pod count is bounded by the count of gated environments,
  not tenants.

## Access-gate contract *(normative)*

Stored on `environments` (or in a sibling table — see Clarifications):

```
access_gate:
  enabled: bool

  # Required when enabled == true
  rauthy_client_ref: <client_id allocated in Rauthy>
  allowed_emails: [<email>...]     # explicit allowlist; matched case-insensitively
  allowed_domains: [<domain>...]   # e.g., "example.com"; matched against email suffix
  login_methods:
    magic_link: bool               # default true
    federated:                     # null = federated login disabled
      provider: "google" | "microsoft" | "github" | "generic_oidc"
      provider_client_ref: <Auth Provider id configured in Rauthy>
```

Rauthy clients created per gated environment carry:
- `redirect_uri`: the oauth2-proxy callback for that environment's
  hostname.
- `allowed_origins`: the tenant hostname(s).
- `scopes`: `openid email profile` only — no app-specific claims.
- `enabled_login_flows`: subset of `{magic_link, federated}` matching
  the env's `login_methods` config.
- `password_login_enabled`: **false**, hard-coded across every tenant
  gate client. This is the load-bearing constraint that keeps the
  platform out of password handling.

Rauthy Auth Providers (the upstream IdPs) are configured at the Rauthy
deployment level, not per tenant. A tenant gate references an Auth
Provider by id; multiple tenants can share an Auth Provider (e.g., one
Google client for all gates that want Google login) without leaking
identity across tenants because each tenant has its own Rauthy client
binding the upstream identity to a tenant-scoped Rauthy session.

## Functional Requirements *(MVP)*

- **FR-001** Stagecraft persists a per-environment access-gate
  descriptor with `enabled: bool` + Rauthy client reference + allowlist
  + login-method config.
- **FR-002** When `enabled: false`, deployd-api renders the tenant
  Deployment + Service + Ingress with no auth annotations (existing
  behavior preserved).
- **FR-003** When `enabled: true`, deployd-api provisions:
  (a) a Rauthy OIDC client with `password_login_enabled: false` and the
      configured login methods,
  (b) an `oauth2-proxy` Deployment + Service for that environment,
      configured with the Rauthy client and `allowed_emails` /
      `allowed_domains`,
  (c) Ingress annotations `auth-url` / `auth-signin` chaining the
      tenant Ingress to the proxy.
  All three are created atomically with the tenant Deployment;
  partial-success states roll back.
- **FR-004** Tenant gate Rauthy clients refuse password authentication.
  Magic link and/or federated upstream IdP are the only completion paths.
- **FR-005** Allowlist enforcement is two-layered: Rauthy refuses login
  for users not in its directory or not authorized by the Auth Provider
  rules; oauth2-proxy validates `allowed_emails` / `allowed_domains` on
  the post-auth callback as defense in depth.
- **FR-006** `DELETE /v1/deployments/{id}` cleans up gate resources
  (oauth2-proxy Deployment + Service, Rauthy client) along with the
  tenant workload. Auth Providers configured at the Rauthy level remain
  (they're shared across tenants).
- **FR-007** Stagecraft never stores a user password, hash, or upstream
  IdP token. Rauthy is the only system that touches user identity
  material; stagecraft sees only the post-authentication subject.
- **FR-008** Rotating tenant access updates the Rauthy user directory
  (for magic link) or the Auth Provider's upstream allowlist (for
  federated); changes propagate without tenant workload restart and
  without redeploying the gate.
- **FR-009** Toggling `enabled` true→false or false→true triggers a
  reconcile that adds/removes the gate resources without a tenant
  workload restart.
- **FR-010** Editing `login_methods` (e.g., adding federated Google
  login to an env that had magic-link only) updates the Rauthy client's
  `enabled_login_flows` without recreating the client or the proxy.

## Success Criteria

- A `development` environment created with `access_gate: {enabled: false}`
  is reachable directly (existing behavior preserved).
- Toggling `enabled: true` with `login_methods: {magic_link: true}` and
  `allowed_emails: [user@example.com]` causes the tenant URL to redirect
  to Rauthy, which presents the magic-link form (no password field),
  mails the user a one-time link, and on click drops them onto the
  tenant.
- A user not in `allowed_emails` who completes magic-link login at
  Rauthy is denied at the oauth2-proxy layer and never reaches the
  tenant.
- Adding federated Google login (`login_methods.federated: {provider:
  "google", provider_client_ref: ...}`) gives users a "Continue with
  Google" option at Rauthy in addition to magic link; the email
  allowlist still applies to the Google-issued identity.
- A tenant Rauthy client returns an explicit error if a password login
  is attempted via the API — `password_login_enabled: false` is
  honored end-to-end.
- Toggling `enabled: false` removes the oauth2-proxy and Rauthy client,
  the tenant Ingress reverts to direct exposure, and the tenant
  workload was not restarted.

## Clarifications

### Outstanding decisions

1. **oauth2-proxy topology — per-env vs shared.** Per-env: one
   `oauth2-proxy` Deployment per gated environment, isolated config.
   Shared: a single multi-tenant proxy routing by hostname. Per-env is
   simpler to reason about; shared is leaner on resources but introduces
   cookie-domain and redirect-URI multiplexing concerns. Recommend
   **per-env** for the first cut; revisit if pod count becomes painful.

2. **Schema shape — column on `environments` vs sibling table.**
   `access_gate JSONB` on `environments` is faster to ship; a dedicated
   `environment_access_gates` table is cleaner for audit history and
   cascading row-level FKs (e.g., the allowed-emails list as rows).
   Recommend **sibling table** so allowlist rows link directly without
   JSON wrangling.

3. **Rauthy admin API contract.** Rauthy supports programmatic client
   registration. Stagecraft calls this on gate-config changes. Need to
   confirm: (a) the admin API tolerates volume (one client per gated
   env), (b) revocation is clean, (c) toggling `password_login_enabled`
   on an existing client is supported (vs requiring client recreation),
   (d) Auth Providers can be referenced by id from a client without
   per-tenant duplication.

4. **Hostname stability for OAuth callback URLs.** Rauthy clients have
   fixed `redirect_uri`s. Tenant environments need a stable hostname
   convention (e.g., `<env-slug>.<project-slug>.<org-slug>.tenants.<base>`)
   decided up front so that toggling the gate doesn't require Rauthy
   client edits.

5. **Stagecraft user → Rauthy user mapping.** When an admin types an
   email into the allowlist, does that auto-provision a Rauthy user
   (with magic-link enabled) or does it only allow login attempts?
   Auto-provision is more user-friendly; allow-on-demand is safer
   when only the federated path is configured. Recommend
   **auto-provision a Rauthy user with `password_login_enabled: false`
   and magic-link enabled** when `login_methods.magic_link == true`,
   plus silent linking of the upstream identity on first federated
   login.

6. **Auth Providers configuration UX.** Rauthy Auth Providers
   (Google/Microsoft/GitHub upstream OIDC clients) are configured at
   the Rauthy deployment level. This spec provisions tenant gate
   clients but assumes Auth Providers already exist in Rauthy. The
   admin UX for configuring those upstreams (entering Google client
   ID + secret into Rauthy) is out of scope for stagecraft; admins do
   it in Rauthy directly. A follow-up spec could surface that in the
   stagecraft admin panel if useful.

### What this spec does NOT decide

- The tenant Helm chart shape (covered by spec 136 follow-ups).
- CI / container-build for tenant repos (separate spec).
- Cross-environment SSO (a tenant deployed at multiple environments
  re-authenticating users at each gate is correct default behavior;
  unifying that is its own design problem).
- Audit log shape for gate-protected accesses — request-level audit
  (who viewed what, when) belongs in deployd-api or a sidecar, not
  this spec.
- Passkey / WebAuthn as a third login method. Out of scope by directive;
  enabling it later is an additive change to `login_methods`.
