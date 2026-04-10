# Authorization Pattern

## Architecture Principle

**The identity provider (IdP) establishes identity only — it does not determine
application roles or permissions.** Application roles are stored in and resolved
from the application database. After successful IdP authentication, the
application performs a database lookup to resolve the user's role assignments.
The session is populated with database-resolved permissions.

IdP group memberships and role claims in the token are ignored for authorization
decisions. This ensures that role changes take effect immediately (within one
request) without requiring token re-issuance.

## Role & Permission Model

```
User ──< UserRole >── Role ──< RolePermission >── Permission
```

- **Users** are authenticated via the IdP. Their identity (email, display name)
  comes from the IdP token. Their *roles* come from the database.
- **Roles** are application-defined (e.g., `admin`, `case-worker`, `applicant`).
  Roles can be created, edited, and deleted at runtime through the admin UI.
- **Permissions** are code-defined (e.g., `funding-requests:create`,
  `users:manage`). The permission catalogue is fixed at build time — adding a
  new permission requires a code change and deployment.
- **Role-to-Permission mapping** is configurable at runtime by admin users
  through the admin UI — no code changes, no redeployment.

### Default Behavior

All permissions default to denied. A user has a permission only if explicitly
granted via a role they hold. Users may hold multiple roles simultaneously;
their effective permissions are the union of all role permissions.

## Session & Role Invalidation

When a user's role assignment changes, access revocation must take effect within
one request — not at session expiry.

### Implementation

```ts
// At login: store role snapshot and version in session
req.session.user = {
  userId: dbUser.user_id,
  email: idpClaims.email,
  roles: dbUser.roles,             // resolved from DB, not from token
  permissions: dbUser.permissions,  // flattened from role-permission join
  roleVersion: dbUser.role_version, // timestamp from users table
}
```

```ts
// middleware/require-auth.middleware.ts
export function requireAuth(req: Request, res: Response, next: NextFunction) {
  if (!req.session?.user) return res.status(401).json(buildErrorResponse(req, { code: 'UNAUTHENTICATED' }))

  // Check if roles have been updated since session was created
  const currentVersion = await getUserRoleVersion(req.session.user.userId)
  if (currentVersion > req.session.user.roleVersion) {
    req.session.destroy(() => {
      res.status(401).json(buildErrorResponse(req, { code: 'SESSION_INVALIDATED', message: 'Your permissions have changed. Please sign in again.' }))
    })
    return
  }
  next()
}
```

The `role_version` column on the users table is updated via a trigger or
application code whenever a user's role assignments change. This ensures
revoked access takes effect within one request.

## Two-Tier Authorization Middleware

### Route-Level: `requireRole(roleName)`

Applied at route definition. Blocks requests from users who do not hold the
specified role.

```ts
// routes/admin.routes.ts
router.get('/api/v1/admin/users', requireAuth, requireRole('admin'), adminController.listUsers)
```

### Action-Level: `requirePermission(permissionName)`

Checked within handlers or as additional route middleware. Enforces granular
permission checks beyond role membership.

```ts
// routes/funding-request.routes.ts
router.post('/api/v1/funding-requests',
  requireAuth,
  requireRole('case-worker'),
  requirePermission('funding-requests:create'),
  fundingRequestController.create
)
```

### Middleware Implementations

```ts
// middleware/require-role.middleware.ts
export function requireRole(...roles: string[]): RequestHandler {
  return (req, res, next) => {
    const userRoles = req.session?.user?.roles ?? []
    if (!roles.some(r => userRoles.includes(r))) {
      return res.status(403).json(buildErrorResponse(req, {
        code: 'FORBIDDEN', message: 'Insufficient role'
      }))
    }
    next()
  }
}

// middleware/require-permission.middleware.ts
export function requirePermission(permission: string): RequestHandler {
  return (req, res, next) => {
    const perms = req.session?.user?.permissions ?? []
    if (!perms.includes(permission)) {
      return res.status(403).json(buildErrorResponse(req, {
        code: 'FORBIDDEN', message: 'Insufficient permission'
      }))
    }
    next()
  }
}
```

## Permission Management API

When the `RBAC_GRANULAR_PERMISSIONS` feature flag is enabled:

```
GET  /api/v1/admin/permissions            → list all permissions grouped by domain
PUT  /api/v1/admin/roles/:id/permissions  → update role-to-permission mappings
```

Permissions are displayed with plain-English labels, grouped by domain area
(e.g., "Funding Requests", "Users", "Reports"). Business staff can toggle
individual permissions on/off per role without code changes.

## Safety Guards

1. **Last-admin protection.** The system blocks deletion of the last user with
   the `admin` role. The admin role itself is non-deletable (seed data flag).
2. **Role deletion guards.** Deleting a role that has active user assignments
   requires explicit confirmation. The system warns which users will be affected.
3. **No direct resource access.** Users interacting through the admin UI have
   no access to underlying databases, internal APIs, or infrastructure — all
   operations are mediated by the application's API.
4. **Audit trail.** All role assignment changes, permission changes, and role
   creation/deletion events are written to the audit log with: timestamp, acting
   admin identity, affected entity, old state, new state.

## DDL Requirements

```sql
-- Roles table
CREATE TABLE role (
  role_id       UUID PRIMARY KEY,
  role_name     VARCHAR(100) UNIQUE NOT NULL,
  description   TEXT,
  is_protected  BOOLEAN DEFAULT FALSE,  -- true for seed roles like 'admin'
  created_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  updated_at    TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Permissions table (code-defined catalogue)
CREATE TABLE permission (
  permission_id   UUID PRIMARY KEY,
  permission_key  VARCHAR(200) UNIQUE NOT NULL,  -- e.g., 'funding-requests:create'
  display_name    VARCHAR(200) NOT NULL,
  domain          VARCHAR(100) NOT NULL,          -- grouping label
  created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- User-Role junction
CREATE TABLE user_role (
  user_id    UUID NOT NULL REFERENCES app_user(user_id),
  role_id    UUID NOT NULL REFERENCES role(role_id),
  granted_by UUID REFERENCES app_user(user_id),
  granted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (user_id, role_id)
);

-- Role-Permission junction
CREATE TABLE role_permission (
  role_id        UUID NOT NULL REFERENCES role(role_id) ON DELETE CASCADE,
  permission_id  UUID NOT NULL REFERENCES permission(permission_id) ON DELETE CASCADE,
  PRIMARY KEY (role_id, permission_id)
);

-- Role version for session invalidation
ALTER TABLE app_user ADD COLUMN role_version TIMESTAMP DEFAULT CURRENT_TIMESTAMP;
```

## Rules

1. **IdP = identity only.** Never read roles or permissions from IdP tokens/claims.
2. **DB = authorization.** All role/permission resolution from the application database.
3. **Immediate revocation.** `role_version` ensures revoked access takes effect within one request.
4. **Two-tier middleware.** `requireRole` at route level, `requirePermission` at action level.
5. **Permissions are code-defined.** Adding a permission requires deployment. Mapping permissions to roles does not.
6. **Default deny.** Users have no permissions unless explicitly granted via role membership.
7. **Admin role is protected.** Cannot be deleted. Last admin assignment cannot be removed.
8. **Audit everything.** Role assignments, permission changes, role lifecycle events.
9. **Hide unauthorized UI.** Navigation items and controls for functionality a user lacks permission for must be hidden — only admin sees all functionality.
