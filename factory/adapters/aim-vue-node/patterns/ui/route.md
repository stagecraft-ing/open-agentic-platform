# Route (Vue Router)

## Convention
Routes live in a single `router/index.ts`. Views are lazy-loaded. Route meta controls auth and document titles. A `beforeEach` guard handles auth redirects.

## Template
```typescript
import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router';
import { useAuthStore } from '@/stores/auth.store';

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'home',
    component: () => import('../views/HomeView.vue'),
    meta: { title: '{App Name}', requiresAuth: false, guestOnly: false },
  },
  {
    path: '/login',
    name: 'login',
    component: () => import('../views/LoginView.vue'),
    meta: { title: 'Sign In', requiresAuth: false, guestOnly: true },
  },
  // ... additional routes
  {
    path: '/:pathMatch(.*)*',
    name: 'not-found',
    component: () => import('../views/NotFoundView.vue'),
    meta: { title: 'Page Not Found', requiresAuth: false, guestOnly: false },
  },
];

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
});

router.beforeEach((to, _from, next) => {
  const auth = useAuthStore();
  if (to.meta.requiresAuth && !auth.isAuthenticated) {
    return next({ name: 'login', query: { redirect: to.fullPath } });
  }
  if (to.meta.guestOnly && auth.isAuthenticated) {
    return next({ name: 'home' });
  }
  next();
});

router.afterEach((to) => {
  const base = '{App Name}';
  document.title = to.meta.title ? `${to.meta.title} - ${base}` : base;
});

export default router;
```

## Example
Concrete routes from the CFS Women's Shelter public portal:
```typescript
const routes: RouteRecordRaw[] = [
  // Public (no auth)
  { path: '/', name: 'home',
    component: () => import('../views/HomeView.vue'),
    meta: { title: 'Home', requiresAuth: false, guestOnly: false } },
  // Guest-only (redirects authenticated users)
  { path: '/login', name: 'login',
    component: () => import('../views/LoginView.vue'),
    meta: { title: 'Sign In', requiresAuth: false, guestOnly: true } },
  // Authenticated
  { path: '/dashboard', name: 'dashboard',
    component: () => import('../views/DashboardView.vue'),
    meta: { title: 'My Applications', requiresAuth: true, guestOnly: false } },
  // Dynamic param
  { path: '/applications/:id', name: 'application-detail',
    component: () => import('../views/ApplicationDetailView.vue'),
    meta: { title: 'Application', requiresAuth: true, guestOnly: false } },
  // 404 catch-all
  { path: '/:pathMatch(.*)*', name: 'not-found',
    component: () => import('../views/NotFoundView.vue'),
    meta: { title: 'Page Not Found', requiresAuth: false, guestOnly: false } },
];
// Guards identical to template above. Base title: "Women's Shelter Funding Portal"
```

## Naming
- File: `apps/{stack}/src/router/index.ts` (one file per web app)
- Route names: kebab-case matching the page id (e.g. `'application-detail'`)
- View imports: always dynamic `() => import('../views/{Name}View.vue')`

## Rules
1. Every view must be lazy-loaded -- never static `import XxxView from '...'`
2. Every route must have `meta: { title, requiresAuth, guestOnly }`
3. `beforeEach` checks `auth.isAuthenticated` -- redirects to `/login` with `redirect` query param
4. `guestOnly: true` routes redirect authenticated users away
5. `afterEach` sets `document.title` from `meta.title`
6. Include a catch-all `/:pathMatch(.*)*` route for 404 handling
7. Route paths use kebab-case (e.g. `/applications/:id`) -- never camelCase in URLs
8. Dual-stack apps: each web app has its own `router/index.ts`
