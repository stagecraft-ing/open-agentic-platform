# Stagecraft Web Frontend

React Router v7 frontend for the Stagecraft control plane, served by the Encore.ts backend.

## Stack

- React Router v7 (SSR + HMR)
- TailwindCSS
- TypeScript

## Routes

| Path | Access | Purpose |
|------|--------|---------|
| `/` | Public | Landing page |
| `/signin`, `/signup` | Public | Authentication |
| `/app` | Authenticated | User dashboard |
| `/app/settings` | Authenticated | User settings |
| `/admin` | Admin only | Admin panel |
| `/admin/users` | Admin only | User management |
| `/admin/audit` | Admin only | Audit log viewer |
| `/admin/signin` | Public | Admin sign-in |

## Development

### Full-stack local (no cluster)

The frontend is bundled into the Encore.ts backend and served from a prebuilt `web/build/`. Run from the stagecraft root:

```bash
npm run start
# → http://localhost:4000
```

`npm run start` rebuilds the frontend before starting Encore — web/** edits require a restart to take effect.

### Frontend HMR against the Hetzner cluster

For fast iteration on web/**, run the React Router dev server alongside a mirrord'd Encore backend. Two terminals, both from `platform/`:

```bash
# Terminal 1 — Encore backend on :4000, traffic stolen from the cluster pod
make dev-stagecraft-hetzner

# Terminal 2 — Vite dev server on :3000 with HMR, proxying Encore paths to :4000
make dev-stagecraft-web-hetzner
```

Open http://localhost:3000. Frontend edits hot-reload; SSR loaders reach the mirrord'd Encore via `ENCORE_API_BASE_URL=http://localhost:4000`; browser-initiated hits to Encore-owned paths (`/api/*`, `/auth/oidc*`, `/auth/github*`, `/site`, `/v1/*`, ...) are proxied by vite.

**Auth flow limitation.** Rauthy's OIDC redirect URIs are registered against the cluster domain (e.g. `https://${DOMAIN}/auth/oidc/callback`), not `localhost:3000`. Clicking sign-in leaves the dev origin: the browser lands on the cluster domain after the callback, and the session cookie is set there. For auth-requiring iteration, sign in on the cluster domain (which is still mirrord'd to your local :4000), or register `http://localhost:3000/auth/oidc/callback` as an additional redirect URI in Rauthy.

## Build

```bash
npm run build:frontend
```

The built assets are served by the Encore.ts backend.
