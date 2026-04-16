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

The frontend is served by Encore.ts in development mode. Run from the stagecraft root:

```bash
npm run start
# → http://localhost:4000
```

Hot module replacement is active during development. No separate frontend dev server is needed.

## Build

```bash
npm run build:frontend
```

The built assets are served by the Encore.ts backend.
