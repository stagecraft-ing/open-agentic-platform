# Walkthrough: Encore + React Router Integration Fixes

## Overview

Aligned auth/admin APIs with the Encore client for typed responses, and fixed request forwarding issues when Encore proxies requests to the React Router frontend. Resolved Content-Type parsing, CSRF validation, and Host/Origin header normalization.

## Changes Made

### 1. Encore Client Alignment

**api/auth/auth.ts**
- Added explicit return types: AuthSigninResponse, AuthSignoutResponse, SessionResponse, SessionClaims
- All 7 handlers now return typed responses for client generation

**api/admin/admin.ts**
- Added explicit return types: UserRow, ListUsersResponse, SetRoleResponse, AuditRow, ListAuditResponse
- All 3 handlers now return typed responses

**web/app/lib/auth-api.server.ts**
- Refactored to use Encore client instead of raw fetch
- Use createEncoreClient(request) for request-scoped base URL
- All helpers (authSignin, authSignup, authAdminSignin, authSession, authAdminSession, authSignout) call client.auth.* methods

### 2. Form Data Parsing

**web/app/lib/form-data.server.ts**
- Check Content-Type before calling request.formData()
- If application/json: use request.json()
- If application/x-www-form-urlencoded or multipart/form-data: use request.formData()
- Fallback: parse body as URLSearchParams when Content-Type is missing or unexpected

### 3. Encore Request Forwarding

**web/app/lib/encore.server.ts**
- When hostname is "origin", fall back to localhost:4000 or ENCORE_API_BASE_URL
- Handles Encore's internal host that doesn't resolve

**web/api.gateway.ts**
- fixRequestHeaders: normalize Origin and Host before passing to React Router
- Build clean headers array instead of mutating rawHeaders in place
- Filter out malformed headers (e.g. URLs used as header names)
- Normalize localhost/127.0.0.1 without port to include port 4000
- Return 404 early for /.well-known/* (Chrome DevTools)

### 4. React Router CSRF

**web/react-router.config.ts**
- Update allowedActionOrigins to host format (localhost:4000) instead of full URLs

## Environment Variables

| Variable | Purpose |
|----------|---------|
| ENCORE_API_BASE_URL | Override API base for Encore client (e.g. production) |
| ENCORE_PUBLIC_HOST | Override public host for request header normalization (default: localhost:4000) |

## Verification

- Signup, signin, signout flows work
- Admin signin and admin panel work
- No "Content-Type was not one of..." errors
- No "host headers are not provided" errors
- No ECONNREFUSED on port 80
- No "invalid header name" errors
