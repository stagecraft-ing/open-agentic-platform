---
id: "106-rauthy-native-oidc-and-membership"
title: "Rauthy-Native OIDC + Layered Membership Resolution (App Installation + PAT)"
feature_branch: "feat/106-rauthy-native-oidc-and-membership"
status: approved
implementation: complete
owner: bart
created: "2026-04-17"
kind: platform
risk: high
depends_on:
  - "080"  # github-identity-onboarding (completes its FR-002/FR-003/FR-005 intent)
  - "087"  # unified-workspace-architecture (Phase 5 session model)
code_aliases: ["RAUTHY_OIDC_NATIVE", "A2C_MEMBERSHIP"]
implements:
  - path: platform/services/stagecraft/api/auth
  - path: platform/charts/rauthy
summary: >
  Close the implementation gap between spec 080's design and what actually
  shipped. Move GitHub from stagecraft-direct OAuth to Rauthy's upstream IDP,
  remove the imaginary admin-mint JWT endpoint, drive custom claims through
  Rauthy's scope + user-attribute model, and resolve org memberships through
  a layered strategy — GitHub App installation token first, per-user PAT
  second, with fail-loud if neither is available.
---

# 106 — Rauthy-Native OIDC + Layered Membership Resolution

## 1. Problem Statement

Spec 080 declared in Principle 4 that "server-side membership resolution
[uses] the App installation token" and declared in FR-003 that "Rauthy owns
the session." The code that shipped for spec 080 Phase 1 and spec 087 Phase 5
violates both of those contracts:

1. **The Rauthy session is imaginary.** `issueRauthySession` calls
   `POST /auth/v1/admin/users/:id/sessions` with custom claims — an endpoint
   that **does not exist in Rauthy 0.35 or any prior release**. Rauthy has no
   admin-mint-JWT surface and no token-exchange / impersonation grant
   (`src/service/src/oidc/mod.rs:25-40`, `src/api_types/src/oidc.rs:218-221`).
   The feature was written against an API that was never real.

2. **The admin HTTP paths are wrong.** `provisionRauthyUser` and
   `revokeSession` call `/auth/v1/admin/users*`, which in Rauthy 0.35 is the
   admin UI (HTML). The admin API lives under `/auth/v1/users*`. The auth
   scheme is also wrong: `Bearer` is treated as a session-cookie lookup; API
   keys require `Authorization: API-Key <name>$<secret>`.

3. **Membership resolution uses the user's GitHub OAuth token**, contradicting
   080's Principle 4 ("use App installation token"). This tightly couples
   login to the user having an active GitHub OAuth session with stagecraft
   as a direct OAuth client — which it should not be once Rauthy owns login.

4. **Rauthy is not configured as a federated login for GitHub.** Spec 080
   FR-003 calls for "Register GitHub as upstream authentication provider" in
   Rauthy. The Helm chart (`platform/charts/rauthy/values-hetzner.yaml`) has
   no upstream provider block. No custom scopes or attributes are seeded.

The visible symptom is the `rauthy_unavailable` OAuth-callback error: Rauthy
returns HTML (admin UI) instead of JSON, which crashes the JSON parser. The
invisible defect is that even if that path were fixed, the next step
(`issueRauthySession`) calls a non-existent endpoint. The whole session-mint
pathway is unreachable.

This spec specifies the correct architecture, chosen after confirming
feasibility against Rauthy 0.35 source: Rauthy handles GitHub login natively
as an upstream IDP, stagecraft writes custom user attributes via the real
admin API, the `oap` custom scope maps those attributes into JWTs, and the
standard OIDC `/authorize` + `/oidc/token` flow is the only code path that
mints sessions.

Because Rauthy 0.35 does **not** forward the upstream GitHub access token to
downstream clients (`src/data/src/entity/auth_providers.rs:706-831`),
stagecraft cannot piggyback on Rauthy's GitHub login to read org memberships.
Memberships must be resolved through a separate channel. Spec 080's Principle
4 (installation token) is the first-choice channel. A user-provided PAT is
the documented fallback, because some orgs will not install the stagecraft
GitHub App and those users still need to log in.

## 2. Design Principles

1. **Rauthy is the sole session signer.** All platform JWTs are issued
   through Rauthy's OIDC flow. No locally-signed tokens. No imaginary admin
   endpoints. If Rauthy is down, login is down — this matches the current
   operational posture.

2. **Custom claims are scope-driven.** All OAP claims (`oap_user_id`,
   `oap_org_id`, `oap_org_slug`, `oap_workspace_id`, `github_login`,
   `idp_provider`, `idp_login`, `avatar_url`, `platform_role`) are declared
   as Rauthy user attributes and mapped into the `oap` custom scope. No
   per-request claim injection.

3. **Seeding is idempotent and runs on stagecraft startup.** Custom
   attributes, the `oap` scope, and the client-scope grant are ensured on
   every boot. This keeps new deployments and individual-operator setups
   self-configuring. Failures are loud (stagecraft refuses to start).

4. **GitHub is an upstream IDP for Rauthy, not an OAuth client of
   stagecraft.** Stagecraft drops its direct GitHub OAuth App role for
   login. The GitHub App (server-to-server) continues to handle webhooks,
   PR previews, and — per Principle 5 below — membership reads.

5. **Membership is resolved through a strategy chain, App-first, PAT-second.**
   The default path uses a per-installation GitHub App token (Principle 4 of
   spec 080). When no installation covers the user's orgs, stagecraft uses
   the user's stored Personal Access Token. When neither yields a matching
   installed org, the user is redirected to `/auth/no-org` as today.

6. **PAT is a first-class escape hatch, not a temporary hack.** It is
   encrypted at rest, scoped, rotated, and audited. It is the documented
   path for operators whose orgs will not install the stagecraft GitHub
   App — which is known to occur in practice.

## 3. Architecture

### 3.1 Session mint path (one path only)

```
OPC / Web                 Stagecraft             Rauthy              GitHub
   │                          │                    │                   │
   │  /auth/desktop/authorize │                    │                   │
   ├─────────────────────────►│                    │                   │
   │                          │  /oidc/authorize   │                   │
   │                          │  (PKCE + idp_hint  │                   │
   │                          │   = github +       │                   │
   │                          │   scope=openid     │                   │
   │                          │   profile email    │                   │
   │                          │   oap)             │                   │
   │  302 to Rauthy ◄─────────┤                    │                   │
   │                          │                    │                   │
   │  (Rauthy runs its own GitHub OAuth against the upstream IDP)     │
   │                          │                    ├──────────────────►│
   │                          │                    │◄──────────────────┤
   │                          │                    │  (github_login,   │
   │                          │                    │   email, avatar)  │
   │                          │                    │                   │
   │  302 to stagecraft cb    │                    │                   │
   │  ◄───────────────────────┼────────────────────┤                   │
   │                          │                    │                   │
   │  code=...                │                    │                   │
   ├─────────────────────────►│                    │                   │
   │                          │  /oidc/token       │                   │
   │                          ├───────────────────►│                   │
   │                          │◄───────────────────┤                   │
   │                          │  JWT #1 (has       │                   │
   │                          │  github_login      │                   │
   │                          │  only; no oap_*    │                   │
   │                          │  yet)              │                   │
   │                          │                    │                   │
   │  (Stagecraft resolves memberships via A2c strategy, writes        │
   │   oap_* user attributes via /auth/v1/users/{id}/attr)             │
   │                          │                    │                   │
   │                          │  refresh_token     │                   │
   │                          ├───────────────────►│                   │
   │                          │◄───────────────────┤                   │
   │                          │  JWT #2 (contains  │                   │
   │                          │  all oap_* claims  │                   │
   │                          │  via oap scope)    │                   │
   │                          │                    │                   │
   │  opc://auth/callback or cookie                 │                   │
   │  ◄───────────────────────┤                    │                   │
```

### 3.2 Membership resolution strategy (A2c)

```
resolveMembership(githubLogin, userId):

  strategies = [
    appInstallationStrategy(githubLogin),        # Principle 4 of 080
    userPatStrategy(userId, githubLogin),        # fallback
  ]

  for strategy in strategies:
    result = strategy.resolve()
    if result.matches: return result
    if result.error and not result.retryable: log and continue

  return MembershipResult.empty()  # → /auth/no-org
```

Each strategy returns either a list of matched `(installed_org, github_role)`
pairs, an empty list, or a typed error. The resolver does not short-circuit
on empty — an org without the app might be reachable via PAT.

### 3.3 Trust boundaries

| Credential | Holder | Purpose | Blast radius |
|---|---|---|---|
| Rauthy admin API key (`RAUTHY_ADMIN_TOKEN`) | Stagecraft service secret | Create users, write custom attrs, revoke sessions | Full Rauthy tenant |
| GitHub App installation token | Stagecraft (fetched per-call, short-lived) | Read installed-org memberships | The installed org only |
| User PAT | Stagecraft encrypted at rest | Read user's orgs/memberships | User's GitHub authority (scoped by PAT) |
| Rauthy client secret (`RAUTHY_CLIENT_SECRET`) | Stagecraft service secret | OIDC code → token exchange | Stagecraft-as-client only |
| GitHub upstream OAuth App secret | Rauthy only | Rauthy's upstream login with GitHub | Rauthy's GitHub integration only |

Note that the GitHub OAuth App secret that currently lives in
`stagecraft-api-secrets` (`GITHUB_OAUTH_CLIENT_ID` / `GITHUB_OAUTH_CLIENT_SECRET`)
moves to Rauthy's secret store and is rotated. Stagecraft no longer holds it.

## 4. Functional Requirements

### FR-001: Rauthy upstream provider configuration

**Amended 2026-04-17** after verifying Rauthy 0.35 source. Rauthy 0.35 has
**no `upstream_auth_provider` section on `RauthyConfig.Vars`**
(`src/data/src/rauthy_config.rs`) and the startup path in
`src/bin/src/server.rs` has no bootstrap step that reads providers from
config. Unknown TOML keys are silently ignored. The chart block in
`platform/charts/rauthy/templates/configmap.yaml` rendering
`[[upstream_auth_provider]]`, and the matching
`UPSTREAM_<NAME>_CLIENT_ID/SECRET` env-var injection in
`platform/charts/rauthy/templates/statefulset.yaml`, are dead code from
spec 080 Phase 4 and do not provision anything. They render nothing when
`upstreamProviders: []` and are removed in the FR-002 implementation PR.

The canonical GitHub upstream-provider shape is instead declared as the
stagecraft seeder's input (FR-002):

- `name: "github"` (Rauthy provider `name` acts as stable identifier)
- `typ: "github"` (Rauthy 0.35 ships a GitHub-specific adapter — see
  `src/data/src/entity/auth_providers.rs:706-831` for the private-email
  fetch and `:826-828` for the GitHub branch)
- `client_id` → stagecraft env var `GITHUB_UPSTREAM_CLIENT_ID`
- `client_secret` → stagecraft env var `GITHUB_UPSTREAM_CLIENT_SECRET`
  (referenced from `stagecraft-api-secrets`, AES-sealed in KeyVault)
- `scope: "read:user user:email"`
- `root_pem`: omitted (public CA)
- Rauthy callback URL to register with the GitHub OAuth App:
  `https://<rauthy-host>/auth/v1/providers/callback`

A new GitHub OAuth App is registered against Rauthy's callback URL.
Its credentials land in `stagecraft-api-secrets` as the two env vars
above. The existing `GITHUB_OAUTH_CLIENT_ID` / `GITHUB_OAUTH_CLIENT_SECRET`
secrets are deleted after the cutover per FR-008.

### FR-002: Idempotent Rauthy seeder on stagecraft startup

A new module `api/auth/rauthySeed.ts` runs inside stagecraft's service-init
path. It:

1. Ensures the **GitHub upstream auth provider** exists. `GET /auth/v1/providers`
   to list; if no entry with `name = "github"`, `POST /auth/v1/providers`
   with the FR-001 shape (`typ: github`, `client_id` / `client_secret`
   from `GITHUB_UPSTREAM_CLIENT_ID` / `GITHUB_UPSTREAM_CLIENT_SECRET`,
   `scope: "read:user user:email"`). If the entry exists but its
   client_id / scope drift from the env vars, `PUT /auth/v1/providers/{id}`
   to converge. This step runs before steps 2-5 because the upstream IDP
   must be usable before the `oap` scope has anywhere to be requested
   from.
2. Ensures each of these custom user attributes exists via
   `POST /auth/v1/users/attr` (or 409 → no-op):
   `oap_user_id`, `oap_org_id`, `oap_org_slug`, `oap_workspace_id`,
   `github_login`, `idp_provider`, `idp_login`, `avatar_url`,
   `platform_role`.
3. Ensures the custom scope `oap` exists and maps the attributes above into
   both access and ID tokens (`attr_include_access` and `attr_include_id`).
4. Ensures the stagecraft OIDC client is allow-listed to request scope `oap`.
5. Ensures the OPC OIDC client is allow-listed to request scope `oap`.

As part of this PR, the dormant chart plumbing identified in FR-001 is
removed: the `[[upstream_auth_provider]]` block in
`platform/charts/rauthy/templates/configmap.yaml` and the
`UPSTREAM_<NAME>_CLIENT_ID/SECRET` env-var injection in
`platform/charts/rauthy/templates/statefulset.yaml`, plus the
`upstreamProviders` sample in `values.yaml`. The chart no longer pretends
to own provider config — the seeder does.

Any non-2xx / non-409 response aborts stagecraft startup with a clear
operator-facing log message. Seeder calls use `Authorization: API-Key` with
`RAUTHY_ADMIN_TOKEN`. The seeder ships behind no feature flag — it is a
hard precondition.

### FR-003: Corrected Rauthy admin client

`api/auth/rauthy.ts` is rewritten:

- `provisionRauthyUser`: `GET /auth/v1/users/email/{email}` (404 = not found),
  fallback `POST /auth/v1/users`. Auth header `API-Key`.
- `setRauthyUserAttributes(userId, attrs)`: `PUT /auth/v1/users/{id}/attr`
  with the full attribute set for that user. Called on first login and on
  workspace switch.
- `exchangeCodeForTokens`: unchanged (path was already correct).
- `refreshTokens`: unchanged.
- `revokeSession`: `DELETE /auth/v1/sessions/{user_id}` with `API-Key`.
- **Removed:** `issueRauthySession`. All call sites rework as described
  below. The commit that introduced it (587db97) is explicitly reverted in
  spirit: Rauthy issues tokens only through OIDC flows.
- `validateJwt` reads claims from `payload.custom?.*` instead of top-level.
  JWKS signature verification is unchanged. The issuer check stays
  `${rauthyUrl}/auth/v1`.

### FR-004: Rewritten login callback

Stagecraft adds `api/auth/rauthyCallback.ts` which handles
`GET /auth/rauthy/callback` (web) and routes desktop flows through the same
path as today.

1. Exchange `code` for JWT #1 via `exchangeCodeForTokens`.
2. Read `github_login` from the JWT's top-level claims (Rauthy already
   populates this from the upstream IDP). Do not assume `oap_*` claims yet.
3. Find or create the stagecraft `users` row keyed by `rauthy_user_id`
   (the JWT `sub`). Link `github_user_id` and `github_login`.
4. Call `resolveMembership(github_login, userId)` — see FR-005.
5. If 0 matches: redirect to `/auth/no-org`.
6. If 1 match: write `oap_*` attributes via `setRauthyUserAttributes`.
7. If >1 match: redirect to org picker; on pick, write attributes.
8. Call `refreshTokens` to produce JWT #2 with `custom.oap_*` populated.
9. Set cookie (web) / redirect to `opc://auth/callback` (desktop) with
   JWT #2.

The direct-GitHub callback (`/auth/github/callback` in current github.ts)
is kept only long enough to cut over and is deleted in this spec's
implementation. Error codes documented in spec 080 FR-002 are preserved
where the failure mode still exists; new codes are added for
PAT-specific failures (see FR-006).

### FR-005: Membership resolution strategy chain

New module `api/auth/membershipResolver.ts` exposes
`resolveMembership(githubLogin, userId): Promise<ResolvedOrg[]>`. It invokes
two ordered strategies:

1. **Installation-token strategy.**
   - Load all `github_installations` where `installation_state = 'active'`.
   - For each, fetch an installation access token via the existing GitHub
     App plumbing.
   - Call `GET /orgs/{org}/memberships/{github_login}` with the installation
     token.
   - 200 → matched (with role). 404 → not a member. 403 → log, skip.
     Network error → retryable; backoff per
     [spec 080 Risks: rate limits].
   - Requires the app manifest to include `Organization permissions:
     Members: read`. See Open Question Q1.

2. **User-PAT strategy** (only invoked if installation strategy returned
   empty matches):
   - Look up `user_github_pats` row for `userId` where `revoked_at IS NULL`.
   - If absent → strategy returns empty with `needs_pat: true`.
   - Decrypt PAT. Call `GET /user/orgs` (list orgs visible to this user)
     and `GET /orgs/{org}/memberships/{github_login}` for each org that
     also has an active `github_installations` row.
   - SAML SSO 403 → user-actionable error, `pat_saml_not_authorized`.
   - 401 / token revoked → clear PAT row, redirect with `pat_invalid`.

The resolver returns the unioned matches. If both strategies return empty,
the resolver surfaces whichever strategy's `needs_pat` or
`no_installed_orgs` reason is most actionable.

### FR-006: Personal Access Token storage and lifecycle

New table `user_github_pats`:

```sql
CREATE TABLE user_github_pats (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_enc       BYTEA NOT NULL,           -- AES-256-GCM ciphertext
  token_nonce     BYTEA NOT NULL,           -- per-token nonce
  token_prefix    TEXT NOT NULL,            -- first 8 chars, for display
  scopes          TEXT[] NOT NULL,          -- observed scopes from GH `X-OAuth-Scopes`
  is_fine_grained BOOLEAN NOT NULL,         -- true for `github_pat_*` format
  last_used_at    TIMESTAMPTZ,
  last_checked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_at      TIMESTAMPTZ,              -- NULL = active
  UNIQUE (user_id) WHERE revoked_at IS NULL
);
```

Encryption key: new secret `PAT_ENCRYPTION_KEY` (32 random bytes, base64),
added to `stagecraft-api-secrets`. AES-256-GCM. Never re-used across tokens
(nonce stored per-row).

Endpoints (all `auth: true`, user-scoped):

- `POST /auth/pat` — body `{ token }`. Validates token with `GET /user` and
  `GET /user/orgs`, classifies format (classic vs fine-grained), records
  scopes, stores encrypted. Replaces any existing active row for that user.
- `DELETE /auth/pat` — marks active row `revoked_at = now()`.
- `GET /auth/pat` — returns non-secret metadata only (prefix, scopes,
  last_used_at, is_fine_grained). Never returns the token.
- `POST /auth/pat/validate` — re-checks the stored token against GH
  without rotating it; used by the settings UI refresh button.

Background: a cron-style background task re-validates PATs weekly
(`last_checked_at > 7d`) and marks as revoked any returning 401.

Error codes added to the callback vocabulary:

| Code | Stage | Cause |
|---|---|---|
| `pat_required` | Membership resolution | No installation match, no PAT stored. User is prompted to add one. |
| `pat_invalid` | PAT use | PAT returned 401. Active row cleared. |
| `pat_saml_not_authorized` | PAT use | Token not SAML-authorized for the org. |
| `pat_rate_limited` | PAT use | Secondary rate limit; retry guidance shown. |

### FR-007: Settings UI for PAT management

Web: new route `/settings/github-pat` — shows prefix, last-used, scopes,
SAML status per org. Actions: paste/replace, revoke, revalidate.

Desktop: equivalent screen in OPC settings, invoking stagecraft's PAT
endpoints through the authenticated channel.

### FR-008: Removal of direct-GitHub OAuth from stagecraft

After cutover, stagecraft's `/auth/github` and `/auth/github/callback`
routes are removed. The `githubOAuthClientId` and `githubOAuthClientSecret`
secrets are removed from `stagecraft-api-secrets`. The `desktop-state.ts`
flow is rerouted to Rauthy (see FR-004 flow diagram).

Kept: `api/github/` which handles webhooks, installations, and app-level
GitHub integrations. These use the App's JWT + installation tokens, not
user OAuth.

## 5. Non-Functional Requirements

- **NFR-001** Rauthy seeder must complete within 2s of stagecraft start on
  a healthy Rauthy; failure aborts startup (fail-loud).
- **NFR-002** PAT storage: AES-256-GCM; key in K8s secret; never logged.
- **NFR-003** PAT validation latency on login: p95 < 1s per org checked.
- **NFR-004** All cross-service calls (stagecraft → Rauthy, stagecraft →
  GitHub) use typed request wrappers that structured-log the non-sensitive
  request metadata on failure (uses `errorForLog` helper).
- **NFR-005** No secret values are ever included in log lines, even on
  failure. Error helpers strip `code`, `detail`, `schema` etc. but never
  request bodies that contain tokens.

## 6. Security

| Risk | Mitigation |
|---|---|
| PAT exfiltration via compromised stagecraft pod | AES-GCM with K8s secret key; pod-scoped service account; no app-level DB admin; audit-log every PAT read |
| Classic-PAT overreach (read:org implies a lot) | UI nudges toward fine-grained; `is_fine_grained` surfaced in settings; operator doc recommends fine-grained with `read:org` on specific orgs |
| Long-lived PAT after user leaves org | Weekly revalidation; admin session-revoke (spec 080 FR-026) also clears PATs |
| Rauthy admin token compromise | Seeded on startup is idempotent and read-mostly after seeding; rotate admin token in-place; no token in code or logs |
| SAML SSO bypass via PAT | `pat_saml_not_authorized` error path; never silently proceed |
| Refresh token theft enabling indefinite impersonation | Rauthy session-lifetime policies; `DELETE /auth/v1/sessions/{user_id}` wired to admin force-revoke flow |
| Seeder writes wrong scope/attributes if Rauthy API changes | Seeder validates shape after write; each assertion is a named check with explicit failure message |

## 7. Migration

1. **Cut-in order** (single deploy, zero-downtime not required):
   - Register the new GitHub OAuth App for Rauthy. Callback URL:
     `https://<rauthy-host>/auth/v1/providers/callback`.
   - Seal `GITHUB_UPSTREAM_CLIENT_ID` / `GITHUB_UPSTREAM_CLIENT_SECRET`
     into KeyVault-backed `stagecraft-api-secrets`.
   - Helm-upgrade the Rauthy chart. No chart-values change is required
     for the provider — FR-001's amendment moved provisioning off the
     chart (the former `upstreamProviders` plumbing is removed as part
     of FR-002's PR).
   - Deploy stagecraft with: seeder active (creates the upstream
     provider on first boot, then the scope/attrs/client grants), new
     `/auth/rauthy/callback` route, new membership resolver, PAT
     endpoints, both old and new login entry points live.
   - Smoke-test the new flow end-to-end.
   - Flip the web and OPC login entry points to the new flow.
   - Remove the old direct-GitHub callback and the `GITHUB_OAUTH_*` secrets
     in the following deploy.

2. **Data migration:** none. Existing `users.rauthy_user_id` rows remain
   valid. On the first login via the new flow, missing `oap_*` user
   attributes are written via the seeder-ensured attribute definitions.

3. **Rollback:** revert the frontend flip; both paths coexist during the
   bridge deploy.

## 8. Supersedes / Amends

This spec **completes** spec 080 FR-002, FR-003, and FR-005 — it does not
replace them. Specifically:

- 080 FR-002 Step 7-8: **amended.** "Provision or link user in Rauthy" now
  happens via the corrected admin API (FR-003 of this spec). "Request Rauthy
  to issue session" is replaced by the OIDC authorize→token→refresh flow.
  No admin endpoint mints tokens.
- 080 FR-003 "Rauthy Configuration" (Phase 1): **specified.** The Helm
  upstream-provider block and the seeder module in this spec are the
  concrete implementation of what 080 listed as a todo.
- 080 FR-005 `resolveOrgMemberships`: **replaced.** The user-OAuth-token
  version violated 080 Principle 4. This spec's strategy-chain resolver
  honours Principle 4 and adds PAT as the documented fallback.
- 087 Phase 5: **amended.** The "Rauthy signs every session JWT" intent is
  preserved. The "stagecraft calls admin endpoint to mint JWT" implementation
  is removed. Claims flow through scope-mapped attributes instead.

## 9. Open Questions

- **Q1. RESOLVED (2026-04-17).** The stagecraft GitHub App manifest grants
  `Organization permissions: Members: Read and write` and
  `Administration: Read-only`. Installation-token membership reads work
  without any manifest change; existing installations do **not** need to
  re-approve. The app is also already subscribed to the `Organization`,
  `Membership`, and `Member` webhook events — see FR-005 addendum below.
  Account permissions include `Email addresses: Read-only`, which lets
  Rauthy's upstream GitHub OAuth pull verified email without an explicit
  `user:email` scope.
- **Q2.** Should PAT re-validation run inside stagecraft's process or as a
  deployd-api cron? Current proposal is in-process; revisit if PAT volume
  grows beyond ~10k.
- **Q3.** Rauthy 0.35 custom-scope attribute mapping exact config shape for
  the seeder — confirm during implementation by reading
  `src/api/src/scopes.rs` end-to-end.

### FR-005 addendum: webhook-driven live membership sync

Because the GitHub App is already subscribed to `Organization`, `Membership`,
and `Member` events, membership state can be kept fresh without waiting for
the user's next login:

- `organization.member_removed` → mark the matching `org_memberships` row
  `status = 'removed'`, revoke any active Rauthy sessions for that user
  (re-using the `DELETE /auth/v1/sessions/{user_id}` path from FR-003).
- `organization.member_added` / `organization.member_invited` → create or
  reactivate the `org_memberships` row; on the user's next token refresh
  the new `oap_*` attributes propagate into the JWT.
- `membership.added` / `membership.removed` (team-level) → update any
  team-to-role mappings (spec 080 Phase 3 surface).

This is additive to the login-time resolver. Login remains authoritative;
webhooks are a fast-path for revocation. The webhook handler lives in
`api/github/webhook.ts` alongside existing installation-event handling.

## 10. Test Plan

### Seeder
- [ ] Cold start: seeder creates the GitHub upstream provider, all 9
      attributes, the `oap` scope, and grants it to stagecraft + OPC
      clients.
- [ ] Warm start: seeder is idempotent; no duplicate writes and no
      provider mutation when env vars match the stored config.
- [ ] Provider drift: changing `GITHUB_UPSTREAM_CLIENT_ID` triggers a
      `PUT /auth/v1/providers/{id}` to converge; changing nothing does
      not issue a PUT.
- [ ] Rauthy down: seeder aborts startup with operator-actionable log.
- [ ] Rauthy admin token wrong: seeder aborts with clear auth-error log.
- [ ] Missing `GITHUB_UPSTREAM_CLIENT_ID` / `_SECRET`: seeder aborts
      with a clear "upstream GitHub credentials missing" error, not a
      downstream 4xx.

### OIDC flow
- [ ] New user: web login with GitHub via Rauthy mints JWT #1 with
      `github_login`, JWT #2 with `custom.oap_*`.
- [ ] Returning user: attributes already set; JWT #2 right from refresh.
- [ ] Multi-org picker: choosing org updates attributes + refreshes JWT.
- [ ] No matching org (no installation, no PAT): redirect to `/auth/no-org`
      with `pat_required` messaging.
- [ ] OPC desktop PKCE flow through Rauthy end-to-end.

### Membership resolver
- [ ] App installation match: returns matched orgs, no PAT needed.
- [ ] No app installations, valid PAT: PAT path resolves matches.
- [ ] SAML-enforced org with non-authorized PAT: surfaces
      `pat_saml_not_authorized`.
- [ ] Revoked PAT: row cleared, redirect with `pat_invalid`.
- [ ] Both strategies return empty: `/auth/no-org` shown.

### PAT lifecycle
- [ ] POST /auth/pat with classic PAT: stored, prefix shown, is_fine_grained
      false.
- [ ] POST /auth/pat with fine-grained PAT: is_fine_grained true.
- [ ] DELETE /auth/pat: row marked revoked.
- [ ] Weekly job: expired PAT marked revoked, user flagged on next login.
- [ ] GET /auth/pat: never returns the token value.

### Security
- [ ] PAT never appears in audit_log metadata or error logs.
- [ ] Rauthy admin token never appears in any log line.
- [ ] JWT validation rejects tokens with missing `custom.oap_user_id`.
- [ ] Cross-user PAT leak: attempting to GET /auth/pat for another user
      returns 403.

## 11. Out of scope

- **Token exchange / RFC 8693.** Not needed; no multi-service token
  hand-off is required.
- **Rauthy fork to add admin-mint endpoint.** The scope-driven claims path
  is sufficient and keeps us on stock Rauthy.
- **Non-GitHub upstream IDPs for this spec.** Spec 080 Phase 4 handles
  enterprise OIDC; this spec specifies the GitHub upstream specifically.
  The A2c resolver is GitHub-specific; enterprise OIDC users do not need a
  membership resolver because their JWT group claims already map to roles.
- **Fine-grained PAT scope enforcement.** We record observed scopes; we do
  not enforce minimum scopes here beyond what GitHub returns. Any unmet
  scope shows up as a 403/404 at call time and converts to an actionable
  UI error.
