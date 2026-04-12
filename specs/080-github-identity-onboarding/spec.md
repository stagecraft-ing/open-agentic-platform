---
id: "080-github-identity-onboarding"
title: "GitHub Identity and Org Onboarding — App Installation, OAuth Login, Rauthy Sessions"
feature_branch: "feat/080-github-identity-onboarding"
status: draft
kind: platform
created: "2026-04-06"
authors: ["open-agentic-platform"]
language: en
summary: >
  Replaces local email/password auth with GitHub-centered identity. GitHub App
  installation provides org trust anchor, GitHub OAuth provides user identity,
  Rauthy issues platform sessions and claims. Enables self-service project
  creation for authenticated org members.
code_aliases: ["GITHUB_IDENTITY", "ORG_ONBOARDING"]
owner: bart
risk: medium
---

# Feature Specification: GitHub Identity and Org Onboarding

## Phases

| Phase | Title | Status |
|-------|-------|--------|
| phase-1 | GitHub App + OAuth Login + Rauthy Sessions | draft |
| phase-2 | Self-Service Project Creation | draft |
| phase-3 | Team Role Mapping + Sync | draft |
| phase-4 | Enterprise OIDC Federation | draft |

## Purpose

Stagecraft currently uses local email/password auth with separate user/admin cookie tracks, a hardcoded default org, and no real org membership model. This blocks meaningful multi-user collaboration and governed project creation.

This spec replaces that model with GitHub-centered identity where:

- **GitHub App installation** proves org connection and grants integration authority
- **GitHub OAuth login** proves individual human identity
- **Rauthy** issues the actual platform session, tokens, and claims
- **OAP membership** derives from GitHub org membership against installed apps

This turns org membership into a first-class primitive and makes login the entry point into governed self-service.

## Design Principles

1. **GitHub proves identity and membership; Rauthy owns the session.** GitHub is not the IdP. It is the upstream trust source. Rauthy is the platform OIDC authority.

2. **App installation does not equal user authorization.** Installation proves org connection. Platform roles are still controlled by OAP policy.

3. **Two distinct GitHub integrations.** App installation (org-level, server-to-server) and OAuth login (user-level, browser flow) are related but separate primitives.

4. **Server-side membership resolution.** Never rely on client-side org lists. Resolve membership server-side after GitHub login using the App installation token.

5. **Platform claims are OAP-owned.** GitHub provides the membership basis; OAP governance assigns roles, permissions, and approval authority.

## Current State

| Component | Today | Target |
|-----------|-------|--------|
| Auth | Local email/password, Argon2id | GitHub OAuth + Rauthy OIDC sessions |
| Sessions | Two cookies (`__session`, `__admin_session`) | Single Rauthy-issued JWT with org context |
| Org model | Hardcoded `DEFAULT_ORG_ID` | GitHub App installation creates real org |
| Membership | None (users exist in isolation) | GitHub org membership mapped to OAP org |
| Roles | `user` / `admin` system roles | Platform roles (org-level + project-level) |
| User surface | Uptime monitoring only | Full project lifecycle self-service |
| GitHub App | Webhook + token brokering only | Trust anchor for org + identity + integration |
| Rauthy | Deployed (Helm chart) but not wired | Platform OIDC provider, session authority |

## Architecture

### Identity Stack

```
GitHub OAuth ──→ Rauthy (upstream provider) ──→ Platform Session + Claims
                        │
GitHub App ────→ Org Trust Anchor ──→ Membership Verification
```

### Integration Topology

```
┌──────────────────────────────────────────────────────────────┐
│  GitHub                                                       │
│                                                               │
│  ┌─────────────┐              ┌──────────────┐               │
│  │ GitHub App  │              │ GitHub OAuth  │               │
│  │ Installation│              │ App           │               │
│  └──────┬──────┘              └───────┬──────┘               │
│         │ webhooks, API               │ auth code flow       │
└─────────┼─────────────────────────────┼──────────────────────┘
          │                             │
          ▼                             ▼
┌──────────────────────────────────────────────────────────────┐
│  Stagecraft                                                   │
│                                                               │
│  ┌─────────────────────────┐  ┌────────────────────────────┐ │
│  │ GitHub Service          │  │ Auth Service               │ │
│  │ • webhook handler       │  │ • OAuth callback           │ │
│  │ • installation registry │  │ • membership resolution    │ │
│  │ • token brokering       │  │ • Rauthy user provisioning │ │
│  └─────────────────────────┘  └──────────────┬─────────────┘ │
│                                              │               │
│                                              ▼               │
│                               ┌────────────────────────────┐ │
│                               │ Rauthy                     │ │
│                               │ • OIDC/OAuth2 provider     │ │
│                               │ • Session issuance         │ │
│                               │ • Custom claims (org, role)│ │
│                               │ • Token refresh            │ │
│                               └────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

---

## Phase 1: GitHub App + OAuth Login + Rauthy Sessions

### Scope

- GitHub App installation webhook creates OAP org record
- GitHub OAuth login flow in OPC and Stagecraft web
- Server-side org membership resolution
- Rauthy-backed session issuance with org context
- Basic membership sync on login
- Deprecate local email/password auth

### Data Model

#### New Tables

```sql
-- GitHub App installations (org trust anchor)
CREATE TABLE github_installations (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  github_org_id   BIGINT NOT NULL UNIQUE,
  github_org_login TEXT NOT NULL,
  installation_id  BIGINT NOT NULL UNIQUE,
  installation_state TEXT NOT NULL DEFAULT 'active',  -- active | suspended | deleted
  allowed_repos   TEXT,       -- 'all' or comma-separated repo list
  org_id          UUID REFERENCES organizations(id),  -- linked OAP org
  installed_by    TEXT,       -- GitHub login of installer
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- User identity linkage
CREATE TABLE user_identities (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id         UUID NOT NULL REFERENCES users(id),
  provider        TEXT NOT NULL DEFAULT 'github',
  provider_user_id TEXT NOT NULL,       -- GitHub user ID (numeric string)
  provider_login  TEXT NOT NULL,         -- GitHub login handle
  provider_email  TEXT,                  -- primary email if available
  avatar_url      TEXT,
  access_token_enc TEXT,                 -- encrypted OAuth access token (for API calls)
  refresh_token_enc TEXT,                -- encrypted OAuth refresh token
  token_expires_at TIMESTAMPTZ,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(provider, provider_user_id)
);

-- Org membership linkage
CREATE TABLE org_memberships (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id         UUID NOT NULL REFERENCES users(id),
  org_id          UUID NOT NULL REFERENCES organizations(id),
  source          TEXT NOT NULL DEFAULT 'github',  -- github | manual | rauthy
  github_role     TEXT,           -- admin | member (from GitHub org API)
  platform_role   TEXT NOT NULL DEFAULT 'member',  -- owner | admin | member
  status          TEXT NOT NULL DEFAULT 'active',  -- active | suspended | removed
  synced_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(user_id, org_id)
);
```

#### Modified Tables

```sql
-- users: add GitHub identity fields, make password optional
ALTER TABLE users
  ADD COLUMN github_user_id BIGINT UNIQUE,
  ADD COLUMN github_login TEXT,
  ADD COLUMN avatar_url TEXT,
  ADD COLUMN rauthy_user_id TEXT UNIQUE,
  ALTER COLUMN password_hash DROP NOT NULL;  -- OAuth users have no password

-- organizations: add GitHub linkage
ALTER TABLE organizations
  ADD COLUMN github_org_id BIGINT UNIQUE,
  ADD COLUMN github_org_login TEXT,
  ADD COLUMN github_installation_id BIGINT;
```

### FR-001: GitHub App Installation Webhook

When the Stagecraft GitHub App is installed into a GitHub organization:

1. `installation.created` webhook fires (already handled in `api/github/webhook.ts`)
2. Handler creates or updates `github_installations` row with:
   - `github_org_id` from `payload.installation.account.id`
   - `github_org_login` from `payload.installation.account.login`
   - `installation_id` from `payload.installation.id`
   - `installation_state = 'active'`
   - `allowed_repos` from `payload.repositories` or `'all'`
3. Handler creates or links an `organizations` row:
   - If org with matching `github_org_id` exists: link it
   - Otherwise: create new org with `slug = github_org_login`
4. Emit audit log entry: `org.github_app_installed`

On `installation.deleted`:
- Set `github_installations.installation_state = 'deleted'`
- Do NOT delete the org or any data — soft transition only
- Emit audit log entry: `org.github_app_uninstalled`

On `installation.suspend` / `installation.unsuspend`:
- Update `installation_state` accordingly

### FR-002: GitHub OAuth Login Flow

**Prerequisites:**
- Register a GitHub OAuth App (separate from the GitHub App)
- Configure Rauthy with GitHub as an upstream authentication provider

**Flow:**

```
Browser                  Stagecraft              GitHub              Rauthy
  │                         │                      │                   │
  ├─ GET /auth/github ──────►                      │                   │
  │                         ├─ redirect ───────────►                   │
  │  ◄──────────────────────┤  (OAuth authorize)   │                   │
  │                         │                      │                   │
  │  (user authorizes)      │                      │                   │
  │                         │                      │                   │
  ├─ GET /auth/github/cb ──►│                      │                   │
  │  (with ?code=xxx)       ├─ POST token ─────────►                   │
  │                         │  ◄───────────────────┤ access_token      │
  │                         │                      │                   │
  │                         ├─ GET /user ──────────►                   │
  │                         │  ◄───────────────────┤ github identity   │
  │                         │                      │                   │
  │                         ├─ resolve org membership (server-side)    │
  │                         │                      │                   │
  │                         ├─ provision/link Rauthy user ────────────►│
  │                         │  ◄──────────────────────────────────────┤│
  │                         │                          session tokens  │
  │                         │                                          │
  │  ◄──────────────────────┤  Set-Cookie (Rauthy session)            │
  │                         │                                          │
```

**Server-side steps after receiving GitHub access token:**

1. Call `GET https://api.github.com/user` to get GitHub identity
2. Call `GET https://api.github.com/user/orgs` to get org memberships
3. Match orgs against `github_installations` where `installation_state = 'active'`
4. Find or create `users` row (match by `github_user_id` or email)
5. Upsert `user_identities` row with provider tokens
6. Upsert `org_memberships` for each matching installed org
7. Provision or link user in Rauthy (via Rauthy admin API)
8. Request Rauthy to issue session with custom claims:
   - `sub`: Rauthy user ID
   - `oap_user_id`: internal user ID
   - `oap_org_id`: selected org ID
   - `oap_org_slug`: org slug
   - `github_login`: GitHub handle
   - `platform_role`: org-level role
9. If exactly one org match: auto-select
10. If multiple matches: redirect to org picker (`/auth/org-select`)
11. If zero matches: show "no connected org" error page (`/auth/no-org`)

**Error handling:**

Each stage of the callback produces a specific error code on failure, redirecting to `/signin?error=<code>`. This replaces a single catch-all so users see actionable messages:

| Error code | Stage | User-facing message | Cause |
|------------|-------|---------------------|-------|
| `github_denied` | OAuth authorize | "GitHub login was denied." | User cancelled or GitHub returned `?error` |
| `no_email` | GitHub identity | "Could not retrieve your email." | No verified email on GitHub account |
| `token_failed` | Token exchange | "GitHub authentication failed." | Code expired, secret misconfigured |
| `github_api_failed` | GitHub API | "Could not reach GitHub." | GitHub API unreachable or returned error |
| `account_error` | User upsert | "Failed to create or link your account." | Database error during user/identity creation |
| `membership_failed` | Org resolution | "Could not resolve your organization memberships." | DB or GitHub API error during membership resolution |
| `rauthy_unavailable` | Rauthy provision | "Identity service is temporarily unavailable." | Rauthy unreachable or admin API error |
| `session_expired` | Org select | "Session expired." | `__pending_org` cookie missing or invalid |
| `oauth_failed` | Session creation | "GitHub login failed." | Final catch-all for session/cookie failures |

**Frontend auth routes:**

The OAuth callback redirects to these frontend pages (React Router, registered in `web/app/routes.ts`):

| Path | Component | Purpose |
|------|-----------|---------|
| `/signin` | `routes/signin.tsx` | Login page; displays `?error=<code>` messages |
| `/auth/no-org` | `routes/auth.no-org.tsx` | Zero org matches — prompts user to request app installation |
| `/auth/org-select` | `routes/auth.org-select.tsx` | Multi-org picker; reads `__pending_org` cookie |

### FR-003: Rauthy Integration

Rauthy is already deployed (`platform/charts/rauthy/`) but not wired to Stagecraft. This phase wires it:

**Rauthy Configuration:**
- Register GitHub as upstream authentication provider
- Configure custom scopes/claims for OAP context (`oap_org_id`, `platform_role`)
- Register Stagecraft as an OIDC client (for web app)
- Register OPC as an OIDC client (for desktop app — PKCE flow)

**Session Model:**
- Replace dual-cookie model with single Rauthy-issued session
- Access token (short-lived) + refresh token (long-lived) via httpOnly cookies
- All API endpoints validate the Rauthy JWT via Encore auth handler
- Claims carry org context so every request is org-scoped

**Encore Auth Handler:**
```typescript
// Wire Encore's auth handler to validate Rauthy JWTs
import { authHandler } from "encore.dev/auth";

interface AuthData {
  userId: string;
  orgId: string;
  orgSlug: string;
  githubLogin: string;
  platformRole: "owner" | "admin" | "member";
}

export const auth = authHandler(async (params): Promise<AuthData> => {
  // Validate Rauthy JWT from Authorization header or cookie
  // Extract and return OAP claims
});
```

### FR-004: OPC Login

OPC (Tauri desktop app) uses PKCE OAuth flow:

1. OPC opens system browser to `https://rauthy.{domain}/authorize` with PKCE challenge
2. Rauthy shows GitHub login (upstream provider)
3. User authenticates with GitHub
4. Rauthy redirects to OPC custom scheme (`opc://auth/callback`)
5. OPC exchanges code for tokens via Rauthy token endpoint
6. OPC stores tokens securely (Tauri secure storage)
7. All OPC API calls to Stagecraft include Rauthy access token
8. OPC refreshes tokens automatically using refresh token

### FR-005: Org Membership Resolution

Membership is resolved server-side on every login:

```typescript
async function resolveOrgMemberships(githubAccessToken: string, userId: string) {
  // 1. Get user's GitHub org memberships
  const orgs = await githubApi.getUserOrgs(githubAccessToken);

  // 2. Match against installed GitHub Apps
  const installations = await db.select().from(githubInstallations)
    .where(eq(githubInstallations.installationState, 'active'));

  const matchedOrgs = installations.filter(inst =>
    orgs.some(org => org.id === inst.githubOrgId)
  );

  // 3. Upsert org_memberships for matches
  for (const match of matchedOrgs) {
    const githubOrg = orgs.find(o => o.id === match.githubOrgId);
    await db.insert(orgMemberships).values({
      userId,
      orgId: match.orgId,
      source: 'github',
      githubRole: githubOrg.role,  // 'admin' or 'member'
      platformRole: 'member',      // default; elevated by OAP policy
      status: 'active',
      syncedAt: new Date(),
    }).onConflictDoUpdate({
      target: [orgMemberships.userId, orgMemberships.orgId],
      set: { githubRole: githubOrg.role, syncedAt: new Date(), status: 'active' },
    });
  }

  // 4. Mark stale memberships (user left org)
  // ...

  return matchedOrgs;
}
```

### Non-Functional Requirements

- **NFR-001:** OAuth tokens stored encrypted at rest (user_identities.access_token_enc)
- **NFR-002:** Rauthy access tokens have max 15-minute TTL; refresh tokens 14 days
- **NFR-003:** Login-time membership resolution must complete in < 3 seconds
- **NFR-004:** Local email/password auth preserved as fallback during migration but hidden from UI

### Migration Strategy

1. Deploy new schema (additive — no breaking changes)
2. Wire Rauthy as OIDC provider
3. Add GitHub OAuth endpoints alongside existing auth
4. Switch UI to GitHub login (hide email/password forms)
5. Migrate existing admin users by linking GitHub identities
6. Remove old auth endpoints after transition period

---

## Phase 2: Self-Service Project Creation

### Scope

Authenticated org members can create projects through Stagecraft, including repo creation, GitHub Actions wiring, and deployment preparation.

### FR-006: Project Creation Flow

```
Member signs in ──→ Org context established ──→ Create Project
                                                     │
                                   ┌─────────────────┼──────────────────┐
                                   │                 │                  │
                                   ▼                 ▼                  ▼
                            Create/link repo   Wire GH Actions   Provision deploy
                                   │                 │                  │
                                   └─────────────────┼──────────────────┘
                                                     ▼
                                              Project ready for
                                              governed pipeline
```

**Steps:**

1. User navigates to project creation in Stagecraft
2. Stagecraft checks `org_memberships` and org-level permission `project:create`
3. User provides: project name, description, adapter selection
4. Stagecraft uses GitHub App installation token to:
   - Create repository in the org (or connect existing)
   - Seed repo with adapter template (from Factory adapters)
   - Configure branch protection rules
   - Create GitHub Actions workflow files
   - Set required secrets/variables
5. Stagecraft creates:
   - `projects` row (linked to org)
   - `project_repos` row (linked to GitHub repo + installation)
   - `environments` rows (default: development, staging, production)
   - `project_members` row (creator as project admin)
6. Emit audit log entry: `project.created`
7. Return project dashboard URL

### FR-007: Org-Level Permissions

New permission model at org level:

| Permission | Default for `member` | Default for `admin` | Default for `owner` |
|------------|---------------------|---------------------|---------------------|
| `project:create` | yes | yes | yes |
| `project:delete` | no | yes | yes |
| `org:manage_members` | no | yes | yes |
| `org:manage_policies` | no | no | yes |
| `org:manage_billing` | no | no | yes |
| `factory:init` | yes | yes | yes |
| `factory:confirm` | no | yes | yes |
| `deploy:production` | no | no | yes |

These are stored as policy in the org context and evaluated at request time. The default can be overridden by OAP policy configuration.

### FR-008: GitHub Repo Initialization

When creating a new repo through Stagecraft:

```typescript
async function createProjectRepo(orgInstallation: GitHubInstallation, params: {
  repoName: string;
  adapter: string;
  isPrivate: boolean;
}) {
  const installToken = await brokerInstallationToken(orgInstallation.installationId, {
    contents: 'write',
    administration: 'write',
    actions: 'write',
  });

  // 1. Create the repo
  const repo = await githubApi.createRepo(installToken, {
    org: orgInstallation.githubOrgLogin,
    name: params.repoName,
    private: params.isPrivate,
    auto_init: true,
  });

  // 2. Push adapter template contents
  await seedRepoFromAdapter(installToken, repo, params.adapter);

  // 3. Configure branch protection
  await githubApi.updateBranchProtection(installToken, repo, 'main', {
    required_status_checks: { strict: true, contexts: ['oap/verify'] },
    required_pull_request_reviews: { required_approving_review_count: 1 },
  });

  // 4. Create GitHub Actions workflow
  await createOapWorkflow(installToken, repo);

  return repo;
}
```

---

## Phase 3: Team Role Mapping + Sync

### Scope

- Map GitHub teams to OAP project/workspace roles
- Background membership sync job
- Revocation and session invalidation on org removal

### FR-009: GitHub Team to OAP Role Mapping

```sql
CREATE TABLE github_team_role_mappings (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id          UUID NOT NULL REFERENCES organizations(id),
  github_team_slug TEXT NOT NULL,
  github_team_id  BIGINT NOT NULL,
  target_scope    TEXT NOT NULL,    -- 'org' | 'project'
  target_id       UUID,            -- NULL for org-level, project_id for project-level
  role            TEXT NOT NULL,    -- platform_role or project_member_role
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(org_id, github_team_slug, target_scope, target_id)
);
```

Admin configures mappings:
- GitHub team `engineering` -> org role `member` (default)
- GitHub team `platform-leads` -> org role `admin`
- GitHub team `project-x-devs` -> project `X` role `developer`
- GitHub team `deploy-approvers` -> project `X` role `deployer`

### FR-010: Membership Sync Job

- Runs on configurable schedule (default: every 6 hours)
- For each active installation:
  - Fetch org members via GitHub App API
  - Compare against `org_memberships`
  - Add new members, mark removed members as `status = 'removed'`
  - Fetch team memberships for mapped teams
  - Update project-level roles accordingly
- Emit audit events for all changes

### FR-011: Revocation

When a user is removed from a GitHub org (detected via sync or webhook):
- Set `org_memberships.status = 'removed'`
- Revoke active Rauthy sessions for that user+org combination
- Emit audit log entry: `membership.revoked`
- Do NOT delete user data or project contributions

### FR-012: Org Picker (Multi-Org Users)

When a user belongs to multiple installed orgs:
- After GitHub login, redirect to `/auth/org-select`
- Display list of available orgs with roles
- User selects org -> Rauthy session claims updated with selected org context
- Org selection persisted in session; switchable from app header

---

## Phase 4: Enterprise OIDC Federation

### Scope

- Support additional upstream identity providers via Rauthy (Azure AD, Okta, Google Workspace)
- SAML-to-OIDC bridge for enterprise SSO
- JIT provisioning from enterprise IdPs
- Custom claims mapping per enterprise tenant

This phase is intentionally light on detail. It depends on Rauthy's upstream provider capabilities and enterprise customer requirements.

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| GitHub org membership visibility requires specific OAuth scopes | Login fails silently | Request `read:org` scope; fail explicitly with actionable error |
| App installation does not auto-authorize all members | Over-permissioning | Default to `member` role; sensitive roles require explicit OAP assignment |
| Rauthy upstream provider config complexity | Deployment friction | Provide tested Helm values and init scripts |
| Sync lag between GitHub org changes and OAP state | Stale access | Login-time re-sync + background job + webhook for immediate revocation |
| GitHub rate limits on membership API calls | Sync failures for large orgs | Cache membership, use conditional requests (ETags), exponential backoff |
| OAuth token storage security | Token theft | Encrypt at rest, short-lived access tokens, audit token usage |

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| Rauthy deployment | Deployed (Helm chart in `platform/charts/rauthy/`) | Needs upstream provider config |
| GitHub App | Configured (webhook + token brokering working) | Needs OAuth App registration |
| Encore auth handler | Stub exists (`AuthData = null`) | Must be wired to Rauthy JWT validation |
| Drizzle migrations | Infrastructure exists | New migration for identity tables |

## Test Plan

### Phase 1

- [ ] GitHub App installation creates `github_installations` + `organizations` row
- [ ] GitHub App uninstall sets `installation_state = 'deleted'`, preserves data
- [ ] GitHub OAuth login creates user, resolves org membership, issues Rauthy session
- [ ] Login with zero matching orgs redirects to `/auth/no-org` with login handle
- [ ] Login with one matching org auto-selects and redirects to `/app`
- [ ] Login with multiple matching orgs redirects to `/auth/org-select`
- [ ] Token exchange failure shows "GitHub authentication failed" (`token_failed`)
- [ ] GitHub API failure shows "Could not reach GitHub" (`github_api_failed`)
- [ ] Account DB error shows "Failed to create or link your account" (`account_error`)
- [ ] Rauthy unavailable shows "Identity service is temporarily unavailable" (`rauthy_unavailable`)
- [ ] Membership resolution failure shows actionable error (`membership_failed`)
- [ ] Rauthy JWT carries correct OAP claims (`oap_org_id`, `platform_role`)
- [ ] Encore auth handler rejects expired/invalid JWTs
- [ ] All existing API endpoints enforce auth via Encore auth handler
- [ ] OPC PKCE flow issues valid Rauthy tokens
- [ ] Token refresh works without re-login

### Phase 2

- [ ] Member with `project:create` can create project
- [ ] Member without `project:create` gets 403
- [ ] Repo creation in GitHub org succeeds with correct template
- [ ] Branch protection configured on new repo
- [ ] GitHub Actions workflow created
- [ ] Project + repo + env + member records created atomically
- [ ] Audit log captures project creation with actor identity

### Phase 3

- [ ] Team-to-role mapping correctly assigns project roles
- [ ] Sync job detects and processes membership changes
- [ ] Org removal revokes sessions and marks membership removed
- [ ] Org picker works for multi-org users
- [ ] Org switch updates session claims
