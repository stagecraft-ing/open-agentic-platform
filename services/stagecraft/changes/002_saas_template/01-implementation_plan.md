# Plan: SaaS Template Migration

## Objective

Extend the uptime monitoring project into a SaaS-complete template with authentication, RBAC, admin panel, and audit logging. Single command dev (`encore run`), single deployable, two-cookie session model.

## Problem Statement

- No authentication; all routes and APIs are public
- No admin capabilities or audit trail
- Uptime monitoring UI is the only frontend; no landing, pricing, or protected areas

## Solution: Add Auth, Admin, and Route Structure

### Phase 1: Backend - Auth and Admin Services

- **api/db/**: Drizzle schema (users, sessions, audit_log), drizzle.ts setup
- **api/auth/**: passwords.ts (argon2id), sessions.ts (token helpers), auth.ts (signup, signin, signout, adminSignin, adminSignout, session, adminSession)
- **api/admin/**: admin.ts (listUsers, setRole, listAudit)
- **Dependencies**: argon2, drizzle-orm, drizzle-kit

### Phase 2: Web Routes

- **Public**: _index.tsx (landing), pricing.tsx, signin.tsx, signup.tsx, admin.signin.tsx
- **User app**: app.tsx (layout + requireUser loader), app._index.tsx (dashboard), app.settings.tsx (signout)
- **Admin app**: admin.tsx (layout + requireAdmin loader), admin._index.tsx, admin.users.tsx, admin.audit.tsx

### Phase 3: Auth Integration

- **web/app/lib/auth.server.ts**: requireUser, requireAdmin, getCookieToken, parseCookie
- Move current home.tsx content to app._index.tsx (uptime monitoring becomes user dashboard)
- Update web/app/routes.ts with new route tree

### Phase 4: Cookie Handling and Client

- Auth endpoints return `setCookie` string; RR actions forward via `redirect(..., { headers: { "Set-Cookie": res.setCookie } })`
- Regenerate Encore client after adding auth/admin endpoints: `encore gen client --output=./web/app/lib/client.ts --env=local`

### Bootstrap Admin

- Add BOOTSTRAP_ADMIN_EMAIL check in signup: if signup email matches, set role to admin

## Verification

- `encore run` serves app
- Signup/signin flow sets __session, redirects to /app
- /app requires auth; unauthenticated redirects to /signin
- Admin signin sets __admin_session; /admin requires admin role
- Admin users and audit pages load
- Uptime monitoring UI works under /app

## Context

Encore.ts + React Router v7 framework mode. Existing site, monitor, slack services remain; auth and admin are new services. Drizzle used for auth schema; site/monitor keep Knex and raw SQL.
