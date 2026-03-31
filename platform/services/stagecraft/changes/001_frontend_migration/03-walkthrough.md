# Walkthrough: Frontend Migration (Next.js → React Router)

## Overview
Replaced the Next.js frontend with React Router 7 + Vite, ported the Uptime Monitoring UI, and aligned with the Encore monorepo structure. The frontend now uses a single root package.json, React Router's route-based architecture, and the same Encore raw API pattern (`nextjs` at `/!rest`).

## Changes Made

### 1. Package Consolidation (root package.json)
- **Added**: @react-router/dev, @react-router/node, @react-router/serve, react-router, isbot, @tailwindcss/vite, tailwindcss, vite, vite-tsconfig-paths
- **Removed**: next, @headlessui/react, @heroicons/react
- **Updated**: react, react-dom to ^19.2.4; @types/react, @types/react-dom to ^19.x
- **Added script**: `build:frontend`: `cd frontend && react-router build`

### 2. Folder Replacement
- **Deleted**: `frontend/` (Next.js app, layout, page, providers, next.config, etc.)
- **Renamed**: `frontend_new/` → `frontend/`
- **Preserved**: `frontend/app/lib/client.ts` (Encore-generated API client)

### 3. Encore Integration (frontend/frontend.ts)
- **Changed**: Export name from `web` to `nextjs` for generated client compatibility
- **Unchanged**: Path `/!rest`, method `*`, `createRequestListener` from `@react-router/node`
- **Build path**: `./build/server/index.js` (React Router build output)

### 4. Vite Config (frontend/vite.config.ts)
- **Updated**: `~encore` alias from `./encore.gen` to `../encore.gen` (project root)

### 5. Root Layout (frontend/app/root.tsx)
- **Added**: QueryClientProvider with 60s staleTime
- **Wrapped**: `<Outlet />` in QueryClientProvider

### 6. Uptime UI (frontend/app/routes/home.tsx)
- **Replaced**: Welcome component with full Uptime Monitoring UI
- **Components**: SiteList, AddSiteForm, StatusBadge, Badge, TimeDelta
- **Imports**: `~/lib/client` instead of `@/app/lib/client`
- **Features**: Site list with status polling (1s), sites list (10s), add site, delete site, URL validation
- **Fixed**: `client.site.del(site.id)` (removed erroneous second arg from original)
- **Added**: Dark mode Tailwind classes where applicable

### 7. Removed Files
- `frontend/app/welcome/welcome.tsx`
- `frontend/app/welcome/logo-light.svg`
- `frontend/app/welcome/logo-dark.svg`

### 8. Documentation & Config
- **README.md**: Added `npm run build:frontend` before `encore run`
- **.gitignore**: Added `frontend/build`, `frontend/.react-router`

## Benefits

✅ **Monorepo alignment**: Single root package.json per AGENTS.md
✅ **Client compatibility**: `nextjs` endpoint name preserved for generated client
✅ **Modern stack**: React Router 7, Vite 7, Tailwind v4
✅ **Feature parity**: Uptime Monitoring UI fully ported
✅ **Build clarity**: Explicit build step before Encore run

## Verification

- ✅ `npm run build:frontend` succeeds
- ✅ `encore run` serves frontend (HTTP 200 on /)
- ✅ No linter errors in home.tsx, root.tsx

## Impact

- **Breaking**: Must run `npm run build:frontend` before `encore run` (Next.js had no pre-build)
- **Dependencies**: React 18 → 19; Next.js removed
- **Structure**: frontend/ now contains React Router app; no package.json in frontend/

## Next Steps

- Run full E2E verification of Uptime UI (add site, delete, status updates)
- Consider adding `encore run` wrapper that auto-builds frontend if needed
