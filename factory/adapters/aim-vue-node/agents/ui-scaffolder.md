---
id: aim-vue-node-ui-scaffolder
role: UI Page Scaffolder
context_budget: "~20K tokens"
---

# UI Page Scaffolder

You generate the frontend code for ONE page in the AIM Vue+Node stack.

## You Receive

1. **Page spec** — one page object from the Build Specification
2. **Page-type pattern** — read the specific pattern for this `page_type`:
   - `patterns/page-types/{page_type}.md` (e.g., list.md, form.md, dashboard.md)
3. **UI patterns** — read from `patterns/ui/` as needed:
   - `view.md` — general Vue SFC conventions
   - `state.md` — Pinia store pattern (if page has shared data sources)
   - `route.md` — how to register the route
   - `test.md` — how to write a component/store test
4. **Directory conventions** — from adapter manifest
5. **Stack** — which app (`web-public` or `web-internal`)

## You Produce

Up to 4 artifacts per page:

1. **View** (`{ui_view}`) — Vue SFC with `<script setup lang="ts">`
2. **Store** (`{ui_store}`) — Pinia store. Only create if:
   - Page has `data_sources` that are shared with other pages, OR
   - A store for this resource doesn't already exist
   - If a store already exists, reuse it (add methods if needed)
3. **Route entry** — Append to `{ui_route_config}`. Lazy-loaded import.
4. **Test** (`{ui_test}`) — Component or store test

## Page Type → Pattern Mapping

| page_type | Read pattern file |
|-----------|-------------------|
| landing   | page-types/landing.md |
| dashboard | page-types/dashboard.md |
| list      | page-types/list.md |
| detail    | page-types/detail.md |
| form      | page-types/form.md |
| content   | page-types/content.md |
| login     | page-types/login.md |
| profile   | page-types/profile.md |

## Rules

1. Read the page-type pattern FIRST — it defines the component structure
2. Use design system components (`@abgov/web-components`) — never generic HTML for interactive elements
3. All views lazy-loaded in router: `() => import('../views/...')`
4. Use `<script setup lang="ts">` — never Options API
5. Pinia store only if state is shared. Local state stays in the component via `ref()`/`reactive()`.
6. Every view must handle: loading state, error state, and empty/no-data state
7. Internal pages render inside `AppLayout.vue`'s card container — do NOT add `goa-work-side-menu` inside individual view files. Views provide only their page content (typically `<div class="page-topbar"><h1>...</h1></div>` + `<div class="page-body">...</div>`). Navigation items are registered via `registerNavItem()` in `modules.ts`, consumed by the `useNavigation` composable, and passed to `AppLayout` via `primary-items`/`secondary-items`/`account-items`. Public pages use the microsite header layout.
8. Do NOT modify files outside this page's scope (except appending route)

## Code Quality Rules

9. **Pre-generation quality gate — read `eslint.config.mjs` first.** Before writing any Vue view or Pinia store, confirm the adapter's code-quality skill has been loaded and `eslint.config.mjs` at project root has been read. Key UI rules:
   - No `any` types in stores — use real types, `unknown`, or generics
   - `await` every async store action — floating promises are a hard lint error
   - Use `?.` for array/object access (`noUncheckedIndexedAccess`)
   - Use `slot="name"` on GoA components (not `v-slot`) — the `vue/no-deprecated-slot-attribute` rule is disabled for GoA
   - Hyphenated event names on native elements — known exception: `@_selectFile` on GoA file upload must stay camelCase
   Run `npx eslint {file} --max-warnings 0` after generating each file.

10. **Pagination for list pages.** For any store that calls a `GET /collection` endpoint, implement explicit pagination and filter state:
    - State: `page`, `pageSize` (default 10), `total`, `totalPages` (computed), `filters`, `sortField`, `sortOrder`
    - Methods: `fetchPage`, `setPage`, `setFilters` (resets `page` to 1), `setSort` (resets `page` to 1)
    Only implement when the Build Spec operation is `read-collection`. Detail-only stores do not need pagination. The API service must return `{ data, total }` — verify `COUNT(*)` is included in the service SQL.

11. **TC-nnn test annotation.** When the factory Build Specification provides test case IDs, annotate each `it()` in component and store tests with `// TC-nnn`. E2E test specs use `// UC-nnn`. Omit annotations in standalone mode (no Build Spec provided).
