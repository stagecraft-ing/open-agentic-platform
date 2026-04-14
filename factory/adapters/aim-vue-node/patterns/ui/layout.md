# Layout Pattern

Vue layout components provide the outer shell for pages — design system header, navigation,
footer, and content area. Layouts wrap `<router-view>` or use slots.

## Convention

- Layout component: `apps/{stack}/src/components/layout/AppLayout.vue`
- Header component: `apps/{stack}/src/components/layout/AppHeader.vue`
- Footer component: `apps/{stack}/src/components/layout/AppFooter.vue`

## Template — AppLayout.vue

```vue
<script setup lang="ts">
import AppHeader from './AppHeader.vue';
import AppFooter from './AppFooter.vue';
</script>

<template>
  <a href="#main-content" class="skip-link">Skip to main content</a>
  <goa-microsite-header type="alpha" />
  <AppHeader />
  <main id="main-content">
    <div class="page-content">
      <slot />
    </div>
  </main>
  <AppFooter />
</template>
```

## Template — AppHeader.vue

```vue
<script setup lang="ts">
import { useAuthStore } from '@/stores/auth.store';
import { useNavigation } from '@/composables/useNavigation';

const auth = useAuthStore();
const { items } = useNavigation();
</script>

<template>
  <goa-app-header :url="'/'" :heading="serviceName">
    <template v-for="item in items" :key="item.path">
      <a :href="item.path">{{ item.label }}</a>
    </template>
    <goa-app-header-menu v-if="auth.isAuthenticated" :heading="auth.displayName">
      <a href="/profile">Profile</a>
      <a href="#" @click.prevent="auth.logout()">Sign out</a>
    </goa-app-header-menu>
    <a v-else href="/login">Sign in</a>
  </goa-app-header>
</template>
```

## Rules

1. Use design system shell components (`goa-microsite-header`, `goa-app-header`, `goa-app-footer`).
2. Include skip-link for accessibility (`<a href="#main-content">`).
3. `goa-microsite-header` type is `alpha` for dev, `beta` for staging, `live` for production.
4. Navigation items registered via `useNavigation()` composable, not hardcoded.
5. Auth state from Pinia `auth.store` — show user menu when authenticated, sign-in link when not.
6. Layout wraps content via `<slot />` — used in `App.vue` as the root component.
7. For dual-stack: public and internal layouts share the same structure but may differ in service name and nav items.
