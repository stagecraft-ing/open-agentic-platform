-- Spec 080 Phase 1: GitHub Identity and Org Onboarding
-- Adds GitHub App installation tracking, user identity linkage,
-- org membership model, and extends users/organizations with GitHub fields.

-- ---------------------------------------------------------------------------
-- Enums
-- ---------------------------------------------------------------------------

CREATE TYPE installation_state AS ENUM ('active', 'suspended', 'deleted');
CREATE TYPE membership_source AS ENUM ('github', 'manual', 'rauthy');
CREATE TYPE org_membership_status AS ENUM ('active', 'suspended', 'removed');
CREATE TYPE platform_role AS ENUM ('owner', 'admin', 'member');

-- ---------------------------------------------------------------------------
-- GitHub App Installations (org trust anchor)
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS github_installations (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  github_org_id     BIGINT NOT NULL UNIQUE,
  github_org_login  TEXT NOT NULL,
  installation_id   BIGINT NOT NULL UNIQUE,
  installation_state installation_state NOT NULL DEFAULT 'active',
  allowed_repos     TEXT,           -- 'all' or comma-separated repo list
  org_id            UUID REFERENCES organizations(id),
  installed_by      TEXT,           -- GitHub login of installer
  created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- User Identity Linkage (GitHub OAuth)
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS user_identities (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id           UUID NOT NULL REFERENCES users(id),
  provider          TEXT NOT NULL DEFAULT 'github',
  provider_user_id  TEXT NOT NULL,        -- GitHub user ID (numeric string)
  provider_login    TEXT NOT NULL,         -- GitHub login handle
  provider_email    TEXT,                  -- primary email if available
  avatar_url        TEXT,
  access_token_enc  TEXT,                  -- encrypted OAuth access token
  refresh_token_enc TEXT,                  -- encrypted OAuth refresh token
  token_expires_at  TIMESTAMPTZ,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(provider, provider_user_id)
);

-- ---------------------------------------------------------------------------
-- Org Membership Linkage
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS org_memberships (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id         UUID NOT NULL REFERENCES users(id),
  org_id          UUID NOT NULL REFERENCES organizations(id),
  source          membership_source NOT NULL DEFAULT 'github',
  github_role     TEXT,              -- admin | member (from GitHub org API)
  platform_role   platform_role NOT NULL DEFAULT 'member',
  status          org_membership_status NOT NULL DEFAULT 'active',
  synced_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
  created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(user_id, org_id)
);

-- ---------------------------------------------------------------------------
-- Extend users table with GitHub identity fields
-- ---------------------------------------------------------------------------

ALTER TABLE users
  ADD COLUMN IF NOT EXISTS github_user_id BIGINT UNIQUE,
  ADD COLUMN IF NOT EXISTS github_login TEXT,
  ADD COLUMN IF NOT EXISTS avatar_url TEXT,
  ADD COLUMN IF NOT EXISTS rauthy_user_id TEXT UNIQUE,
  ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

-- Make password_hash nullable (OAuth users have no password)
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- ---------------------------------------------------------------------------
-- Extend organizations table with GitHub linkage
-- ---------------------------------------------------------------------------

ALTER TABLE organizations
  ADD COLUMN IF NOT EXISTS github_org_id BIGINT UNIQUE,
  ADD COLUMN IF NOT EXISTS github_org_login TEXT,
  ADD COLUMN IF NOT EXISTS github_installation_id BIGINT;

-- Make created_by nullable (orgs auto-created from GitHub App install have no user actor)
ALTER TABLE organizations ALTER COLUMN created_by DROP NOT NULL;
