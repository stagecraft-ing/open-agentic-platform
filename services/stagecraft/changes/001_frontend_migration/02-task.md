# Task: Frontend Migration

- [x] Merge frontend_new deps into root package.json
- [x] Remove frontend_new/package.json
- [x] Delete old frontend/ (Next.js)
- [x] Rename frontend_new to frontend
- [x] Update frontend.ts: rename web → nextjs
- [x] Update vite.config.ts: ~encore → ../encore.gen
- [x] Copy client.ts to frontend/app/lib/
- [x] Add QueryClientProvider to root.tsx
- [x] Port Uptime UI to app/routes/home.tsx
- [x] Remove Welcome component and assets
- [x] Add build:frontend script to root package.json
- [x] Update README with build step
- [x] Add frontend/build, frontend/.react-router to .gitignore
- [x] Verify build succeeds
- [x] Verify encore run serves frontend
