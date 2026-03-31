# Walkthrough: SaaS Template Migration

## Overview

Extended the uptime monitoring project into a SaaS-complete template with authentication, RBAC, admin panel, and audit logging. Two-cookie session model (user + admin), Drizzle for auth schema, and protected routes via loader-based enforcement.

## Changes Made

### 1. Backend - Auth and Admin Services

**api/db/** (new service)
- `schema.ts` - Drizzle schema: users, sessions, audit_log (role enum, session_kind enum)
- `drizzle.ts` - SQLDatabase("auth") + drizzle-orm/node-postgres
- `migrations/1_create_auth_tables.up.sql` - users, sessions, audit_log tables
- `encore.service.ts` - db service
- `index.ts` - re-exports for service loading

**api/auth/** (new service)
- `passwords.ts` - argon2id hash/verify
- `sessions.ts` - newToken(), hashToken() (crypto)
- `auth.ts` - signup, signin, signout, adminSignin, adminSignout, session, adminSession
- BOOTSTRAP_ADMIN_EMAIL env var support in signup
- Cookie helpers: USER_COOKIE (__session), ADMIN_COOKIE (__admin_session), 14-day TTL

**api/admin/** (new service)
- `admin.ts` - listUsers, setRole (with audit log), listAudit
- `encore.service.ts` - admin service

### 2. Frontend - Routes and Layouts

**Public routes**
- `_index.tsx` - Landing with Get started, Sign in, Pricing links
- `pricing.tsx` - Placeholder pricing page
- `signin.tsx` - User sign-in form
- `signup.tsx` - User sign-up form
- `admin.signin.tsx` - Admin sign-in form

**User app** (protected by requireUser)
- `app.tsx` - Layout with nav (Dashboard, Settings), loader enforces session
- `app._index.tsx` - Uptime monitoring dashboard (moved from home.tsx)
- `app.settings.tsx` - Sign out form

**Admin app** (protected by requireAdmin)
- `admin.tsx` - Layout with nav (Admin Home, Users, Audit)
- `admin._index.tsx` - Admin dashboard placeholder
- `admin.users.tsx` - User list with role toggle, setRole action
- `admin.audit.tsx` - Audit log list

### 3. Auth Integration

**web/app/lib/auth.server.ts**
- requireUser(request) - parses __session, calls auth/session, redirects to /signin if invalid
- requireAdmin(request) - parses __admin_session, calls admin/auth/session, checks role
- getCookieToken(request, kind)
- parseCookie(header)

**web/app/lib/auth-api.server.ts**
- Fetch-based helpers for auth endpoints (signin, signup, adminSignin, session, adminSession, signout)
- Used because generated Encore client auth methods return void; we need setCookie from response

**web/app/lib/encore.server.ts**
- createEncoreClient(request) - Client with request origin as base URL

### 4. Route Config (web/app/routes.ts)

- index("routes/_index.tsx") - landing at /
- route("pricing", ...), route("signin", ...), route("signup", ...)
- route("admin/signin", "routes/admin.signin.tsx")
- route("app", "routes/app.tsx", [index, route("settings", ...)])
- route("admin", "routes/admin.tsx", [index, route("users", ...), route("audit", ...)])

### 5. Removed

- `web/app/routes/home.tsx` - content moved to app._index.tsx

## Benefits

- Single command dev: `encore run` (after `npm run build:frontend`)
- Two-cookie model: user and admin sessions isolated
- RBAC: user | admin roles
- Audit log for admin actions
- Protected /app and /admin routes
- Bootstrap admin via BOOTSTRAP_ADMIN_EMAIL env var

## Verification

- `npm run build:frontend` succeeds
- `encore run` starts with auth, admin, db services
- Landing at /, signup/signin at /signup, /signin
- /app requires auth; unauthenticated redirects to /signin
- /admin requires admin session; redirects to /admin/signin
- Uptime monitoring UI under /app

## Impact

- **Breaking**: / now shows landing; uptime UI moved to /app (requires sign-in)
- **New deps**: argon2, drizzle-orm already in package.json
- **New env**: BOOTSTRAP_ADMIN_EMAIL (optional) for first admin on signup
