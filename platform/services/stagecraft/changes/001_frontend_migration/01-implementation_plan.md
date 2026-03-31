# Plan: Frontend Migration (Next.js → React Router)

## Objective
Replace the Next.js frontend with React Router 7 + Vite, port the Uptime Monitoring UI, and align with the existing Encore monorepo structure.

## Problem Statement
- Next.js frontend in `frontend/` with separate patterns from `frontend_new/`
- `frontend_new` had its own package.json (violating monorepo approach)
- Duplicate frontend implementations; need single source of truth
- Different endpoint naming (`web` vs `nextjs`) and build approaches

## Solution: Migrate to React Router with Structure Alignment

### 1. Merge Dependencies (Monorepo)
- Move React Router, Vite, Tailwind v4 deps from `frontend_new/package.json` into root `package.json`
- Remove `frontend_new/package.json`
- Remove Next.js and unused deps (@headlessui, @heroicons)

### 2. Replace Frontend Folder
- Delete old `frontend/` (Next.js)
- Rename `frontend_new/` to `frontend/`

### 3. Encore Integration Alignment
- Rename raw API export from `web` to `nextjs` (client compatibility)
- Update `~encore` alias in vite.config to `../encore.gen`
- Keep `encore.service.ts` and `frontend.ts` in `frontend/`

### 4. Port Uptime Monitoring UI
- Move UI from `frontend/app/page.tsx` to `frontend/app/routes/home.tsx`
- Use `~/lib/client` path alias instead of `@/app/lib/client`
- Add QueryClientProvider in `root.tsx`
- Remove Welcome placeholder component

### 5. Build Orchestration
- Add `build:frontend` script: `cd frontend && react-router build`
- Update README with build step before `encore run`
- Add `frontend/build` and `frontend/.react-router` to `.gitignore`

## Verification
- `npm run build:frontend` succeeds
- `encore run` serves frontend at /
- Uptime Monitoring UI loads and functions (sites list, add, delete, status polling)

## Context
Migration from Next.js 14 (App Router) to React Router 7 + Vite 7. Encore raw API serves React Router build via `createRequestListener` from `@react-router/node`.
