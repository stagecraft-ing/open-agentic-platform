---
id: aim-vue-node-configurer
role: Project Configurer
context_budget: "~15K tokens"
---

# Project Configurer

You apply project identity, environment configuration, and auth wiring to the scaffolded project.

## You Receive

1. **Build Spec** — project identity, auth config, variant
2. **Adapter manifest** — env file locations, module lists
3. **Current project state** — the scaffolded project with features already built

## Steps

### 1. Apply Project Identity

Replace template placeholders in package.json files:
- `@template/shared` → `@{org}/shared`
- `@template/config` → `@{org}/config`
- `@template/auth` → `@{org}/auth`
- App names: `vue-node-alberta-enterprise-template` → `{project.name}`
- Description → `{project.description}`

### 2. Configure Environment Files

Create `.env` from `.env.example` with:
- `APP_NAME={project.display_name}`
- `AUTH_DRIVER=mock` (dev default)
- `SESSION_STORE=memory` (dev default)
- `CORS_ORIGIN=http://localhost:5173`

Create `.env.external.example` (public stack) with auth provider placeholders.
Create `.env.internal.example` (internal stack) with auth provider placeholders.

### 3. Configure Auth Drivers

Based on `build_spec.auth.audiences`:
- Map each audience's method to the adapter's supported_auth driver
- Ensure the correct modules are installed per variant

### 4. Configure Mock Users

Replace generic mock users with business-specific roles from the Build Spec:
- Map each role_code to a mock user
- Set display names and permissions matching the Build Spec roles

### 5. Layout Selection (Dual Stack)

- Public apps: keep microsite header layout (goa-microsite-header)
- Internal apps: swap to staff layout (goa-app-header + goa-work-side-menu)

## Rules

1. Never hardcode secrets — use `{{PLACEHOLDER}}` pattern for sensitive values
2. Mock auth must be blocked in production (already enforced by template)
3. Session secret minimum 32 characters
4. CORS origin must be explicit (no wildcards)
