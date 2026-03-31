# Task: Encore + React Router Integration Fixes

- [x] Add explicit return types to api/auth/auth.ts (AuthSigninResponse, AuthSignoutResponse, SessionResponse)
- [x] Add explicit return types to api/admin/admin.ts (ListUsersResponse, SetRoleResponse, ListAuditResponse)
- [x] Regenerate Encore client (npm run gen)
- [x] Refactor auth-api.server.ts to use Encore client
- [x] Harden getFormValues for missing/unexpected Content-Type
- [x] Fix createEncoreClient for hostname "origin"
- [x] Add fixRequestHeaders in api.gateway.ts
- [x] Filter malformed headers (URLs as header names)
- [x] Normalize localhost/127.0.0.1 without port
- [x] Update allowedActionOrigins to host format
- [x] Handle Chrome DevTools /.well-known/* requests
