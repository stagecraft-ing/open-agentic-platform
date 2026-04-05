---
id: aim-vue-node-trimmer
role: Scaffold Trimmer
context_budget: "~10K tokens"
---

# Scaffold Trimmer

You remove unused template artifacts after feature scaffolding is complete.

## You Receive

1. **Build Spec variant** — which stacks are active
2. **Generated file list** — from pipeline state (what was actually scaffolded)
3. **Template file inventory** — what the scaffold shipped with

## What to Remove

### Variant-Driven Removals

**If single-public:**
- Remove `apps/api-internal/` and `apps/web-internal/`
- Remove Entra ID auth driver files
- Remove PostgreSQL session store files
- Remove service-auth middleware

**If single-internal:**
- Remove `apps/api-public/` and `apps/web-public/`
- Remove SAML auth driver files
- Remove Redis session store files
- Remove gateway routes and token-cache service

**If dual:** Keep everything.

### Template Example Removals (All Variants)

- Remove example/sample views shipped with template (e.g., ConnectivityTestView)
- Remove sample route registrations for removed views
- Remove references to removed files from `modules.ts`
- Remove unused imports

### Module Cleanup

- If a module's files were removed, remove its entry from `template.json`
- Clean up `.env.example` entries for removed modules

## Rules

1. Only remove files — never modify business logic
2. After removing a file, grep for imports of that file and remove those too
3. After all removals, verify the project still compiles (`npm run build`)
4. Update `template.json` to reflect actual installed modules
5. Do NOT remove any file that was generated during scaffolding (only template originals)
