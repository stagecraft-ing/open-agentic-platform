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
- App names: `vue-node-enterprise-template` → `{project.name}`
- Description → `{project.description}`

### 2. Configure Environment Files

Create `.env` from `.env.example` with:
- `APP_NAME={project.display_name}`
- `AUTH_DRIVER=mock` (dev default)
- `SESSION_STORE=memory` (dev default)
- `CORS_ORIGIN=http://localhost:5173`

Create `.env.external.example` (public stack) with auth provider placeholders.
Create `.env.internal.example` (internal stack) with auth provider placeholders.

### 2b. Configure Docker Compose Networking

In `docker-compose.yml`, each **web** service needs **two** API URL environment variables:

| Variable | Purpose | Value |
|---|---|---|
| `VITE_API_URL` | Client-side (browser on host machine) | `http://localhost:{port_api}/api/v1` |
| `API_URL` | Vite dev-server proxy (container-to-container) | `http://{api_service_name}:{internal_port}` |

Each **API** service needs a `WEB_URL` environment variable for auth callback redirects:

| Service | Variable | Value |
|---|---|---|
| api-public | `WEB_URL` | `http://localhost:{port_web_public}` |
| api-internal | `WEB_URL` | `http://localhost:{port_web_internal}` |

The auth controller uses `process.env.WEB_URL` to redirect after login/callback. Without it, the fallback hardcodes port 5173, which breaks the internal portal on a different port. `WEB_URL` must use the **host-accessible** URL (not the Docker service name) because the browser follows the redirect.

For dual stack with a public BFF that proxies to internal, add to the api-public service:

| Service | Variable | Value |
|---|---|---|
| api-public | `INTERNAL_API_URL` | `http://api-internal:{internal_port}` |

The Vite config reads `API_URL` to set the proxy target. Inside Docker, `localhost` resolves to the web container itself — **not** the API container. `API_URL` must use the Docker Compose service name (e.g. `api-public`, `api-internal`) which Docker DNS resolves to the correct container on the shared network.

For dual stack, using `adapter.dual_stack.stacks`:
- web-public: `API_URL: http://api-public:3000`
- web-internal: `API_URL: http://api-internal:3000`

Note: the internal API container listens on port 3000 internally even though docker-compose maps it to host port 3001.

**Port mapping rule:** Each web service's `vite.config.ts` sets `server.port` to its stack's `port_web` value from the manifest. The docker-compose port mapping must match: `{port_web}:{port_web}` (e.g. `5173:5173` for public, `5174:5174` for internal). Do NOT assume all Vite containers listen on 5173 — each uses its own `port_web`.

### 3. Configure Auth Drivers

Based on `build_spec.auth.audiences`:
- Map each audience's method to the adapter's supported_auth driver
- Ensure the correct modules are installed per variant

### 4. Configure Mock Users

Replace generic mock users with business-specific roles from the Build Spec:
- Map each `role_code` to a mock user
- Set display names and permissions matching the Build Spec roles
- **Use exact role strings (case-sensitive)** — the same strings used in `requireRole()` guards and `hasRole()` UI checks. `'citizen'` and `'CITIZEN'` are different strings; a mismatch means every authorization check fails silently (403 on API, hidden UI elements).

**After updating mock users — update the mock driver test (mandatory, do not skip).**

`packages/auth/src/drivers/mock.driver.test.ts` contains assertions against the default three users (`mock-user-1/2/3` with `developer/admin/user` roles). If mock users are changed but this test is not updated, `npm test` fails. Update every assertion that references mock user data:

1. `toHaveLength(N)` — update the count to match the new number of users
2. User index references (`mockUserId=mock-user-*`) — update to an actual new user ID
3. User ID lookups (`user.id toBe('...')`) — update to new user IDs
4. User name assertions — update to the new display names
5. Role assertions (`users[N].roles toContain('...')`) — update to the new role strings at each index

Run `npm test -w packages/auth` immediately after updating — do not proceed to the next step until all mock driver tests pass.

### 5. Layout Selection (Dual Stack)

- **Public apps**: keep the microsite header layout (`goa-microsite-header` + public header nav + `goa-app-footer`).
- **Internal apps**: use the staff sidebar layout — `goa-work-side-menu` IS the chrome. There is NO `goa-app-header` and NO `goa-app-footer` on authenticated internal pages.

**Internal layout structure** (`apps/web-internal/src/components/layout/AppLayout.vue`):
- Flat flex row: `goa-work-side-menu` and `.card-container` as direct children of `.app-layout`
- No `.app-body` wrapper
- `.app-layout` CSS: `height: 100vh; display: flex; overflow: hidden`
- `goa-work-side-menu` props: `heading` (service name), `user-name`, `user-secondary-text`
- Navigation items occupy `slot="primary"`, `slot="secondary"`, `slot="account"` on `goa-work-side-menu-item` elements
- Individual views render inside `<main id="main-content">` within `.desktop-card-container` — views do NOT include `goa-work-side-menu` themselves

**For dual variant**: the template starter shell in `apps/web-internal/` already ships with `goa-work-side-menu`. Do NOT replace it wholesale. Instead:
1. Validate: confirm no `goa-app-header` present, no `.app-body` wrapper, CSS uses `height: 100vh`
2. Customize: update the service name/heading; configure nav items for the project's pages
3. Verify expansion: confirm CSS does not clip the sidebar (`overflow`/`height` correct)

**For single-internal variant**: the starter ships with the public layout and must be swapped. Replace `AppLayout.vue` with the no-header internal layout described above.

Navigation items are registered via `registerNavItem()` in `modules.ts` and consumed by the `useNavigation` composable. `App.vue` passes `primary-items`, `secondary-items`, `account-items` to `AppLayout`.

This step must run during configure — not deferred to feature scaffolding. If the layout shell is wrong, every view the scaffolder generates will fight the wrong container.

## Rules

1. Never hardcode secrets — use `{{PLACEHOLDER}}` pattern for sensitive values
2. Mock auth must be blocked in production (already enforced by template)
3. Session secret minimum 32 characters
4. CORS origin must be explicit (no wildcards)
5. Mock user role strings must be exact case matches for `requireRole()` and `hasRole()`. After Step 4, verify every role string used in auth guards has a corresponding mock user with that exact string (case-sensitive). Template-default roles (`developer`, `admin`, `user`) must be replaced with the project's business roles before feature development begins.
