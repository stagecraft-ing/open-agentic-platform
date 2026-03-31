# Plan: Encore + React Router Integration Fixes

## Objective

Align auth/admin APIs with the Encore client for typed responses, and fix request forwarding issues when Encore proxies requests to the React Router frontend service.

## Problem Statement

- Auth/admin endpoints lacked explicit return types; generated client returned `void`
- Encore request forwarding caused: invalid Origin/Host headers, Content-Type parsing errors, ECONNREFUSED on wrong port
- React Router 7.12+ CSRF protection required valid Origin and Host headers

## Solution

### Phase 1: Encore Client Alignment

- Add explicit response types to auth endpoints (AuthSigninResponse, AuthSignoutResponse, SessionResponse)
- Add explicit response types to admin endpoints (ListUsersResponse, SetRoleResponse, ListAuditResponse)
- Regenerate Encore client
- Refactor auth-api.server.ts to use Encore client instead of raw fetch

### Phase 2: Form Data Parsing

- Harden getFormValues to handle missing/unexpected Content-Type
- Add fallback: parse body as URLSearchParams when Content-Type is stripped by proxy

### Phase 3: Encore Request Forwarding

- Fix createEncoreClient when hostname is "origin" (Encore internal host)
- Add fixRequestHeaders in api.gateway.ts to normalize Origin and Host
- Filter malformed headers (URLs as header names)
- Normalize localhost/127.0.0.1 without port to include port 4000

### Phase 4: React Router CSRF

- Update allowedActionOrigins to host format (localhost:4000)
- Handle Chrome DevTools /.well-known/* requests

## Environment Variables

- `ENCORE_API_BASE_URL` - Override API base for Encore client (e.g. production)
- `ENCORE_PUBLIC_HOST` - Override public host for request header normalization (default: localhost:4000)

## Verification

- Signup, signin, signout flows work
- Admin signin and admin panel work
- No "Content-Type was not one of..." errors
- No "host headers are not provided" errors
- No ECONNREFUSED on port 80
