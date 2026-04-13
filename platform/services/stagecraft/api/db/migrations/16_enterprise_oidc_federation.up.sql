-- Enterprise OIDC Federation (spec 080 Phase 4)
-- Adds support for Azure AD, Okta, Google Workspace as upstream identity providers.

-- 1. Add 'oidc' to membership_source enum
ALTER TYPE membership_source ADD VALUE IF NOT EXISTS 'oidc';

-- 2. OIDC Provider registration (per-org upstream IdP configuration)
CREATE TABLE oidc_providers (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id          UUID NOT NULL REFERENCES organizations(id),
  name            TEXT NOT NULL,                            -- display name (e.g. "Contoso Azure AD")
  provider_type   TEXT NOT NULL DEFAULT 'oidc',             -- oidc | azure-ad | okta | google-workspace | saml-bridge
  issuer          TEXT NOT NULL,                            -- OIDC issuer URL (e.g. https://login.microsoftonline.com/{tenant}/v2.0)
  client_id       TEXT NOT NULL,
  client_secret_enc TEXT NOT NULL,                          -- encrypted client secret
  scopes          TEXT NOT NULL DEFAULT 'openid profile email', -- space-separated OIDC scopes
  claims_mapping  JSONB NOT NULL DEFAULT '{}',              -- map IdP claim names to OAP fields
  email_domain    TEXT,                                     -- e.g. "contoso.com" for domain-based IdP routing
  auto_provision  BOOLEAN NOT NULL DEFAULT true,            -- JIT user provisioning on first login
  status          TEXT NOT NULL DEFAULT 'active',           -- active | disabled | pending
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(org_id, issuer)
);

-- 3. OIDC group-to-role mappings (analogous to github_team_role_mappings)
CREATE TABLE oidc_group_role_mappings (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id          UUID NOT NULL REFERENCES organizations(id),
  provider_id     UUID NOT NULL REFERENCES oidc_providers(id) ON DELETE CASCADE,
  idp_group_id    TEXT NOT NULL,        -- group ID from the IdP (e.g. Azure AD group object ID)
  idp_group_name  TEXT,                 -- display name for readability
  target_scope    target_scope NOT NULL, -- 'org' | 'project' (reuse existing enum)
  target_id       UUID,                 -- NULL for org-level, project_id for project-level
  role            TEXT NOT NULL,         -- platform_role or project_member_role
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(org_id, provider_id, idp_group_id, target_scope, target_id)
);

-- 4. Make desktop_refresh_tokens.github_login nullable + add provider-agnostic columns
ALTER TABLE desktop_refresh_tokens ALTER COLUMN github_login DROP NOT NULL;
ALTER TABLE desktop_refresh_tokens ALTER COLUMN github_login SET DEFAULT '';
ALTER TABLE desktop_refresh_tokens ADD COLUMN IF NOT EXISTS idp_provider TEXT NOT NULL DEFAULT '';
ALTER TABLE desktop_refresh_tokens ADD COLUMN IF NOT EXISTS idp_login TEXT NOT NULL DEFAULT '';

-- 5. Add provider-agnostic identity fields to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS idp_provider TEXT;    -- 'github' | 'azure-ad' | 'okta' | etc.
ALTER TABLE users ADD COLUMN IF NOT EXISTS idp_subject TEXT;     -- provider-specific user ID

-- 6. Index for email-domain routing during OIDC login
CREATE INDEX IF NOT EXISTS idx_oidc_providers_email_domain ON oidc_providers(email_domain) WHERE email_domain IS NOT NULL AND status = 'active';
