# View (Vue SFC)

## Convention
Every page is a Vue 3 SFC using `<script setup lang="ts">`. Views import design system components, use Pinia stores for shared state, and handle loading/error/empty states explicitly.

## Template
```vue
<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { GoabButton, GoabContainer, GoabCallout } from '@abgov/vue-components';
import { use{Resource}Store } from '@/stores/{resource}.store';

const store = use{Resource}Store();

onMounted(async () => {
  await store.fetch{Resources}();
});

const {derivedValue} = computed(() => store.{items}.filter(/* ... */));
</script>

<template>
  <goa-container accent="thin" heading="{Page Title}">
    <div v-if="store.loading" class="loading-indicator">Loading...</div>
    <GoabCallout v-else-if="store.error" type="emergency" heading="Error">
      {{ store.error }}
    </GoabCallout>
    <GoabCallout v-else-if="store.{items}.length === 0" type="information"
      heading="No results">No {resources} found.</GoabCallout>
    <div v-else>
      <!-- Page-specific content here -->
    </div>
  </goa-container>
</template>
```

## Example
```vue
<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { GoabButton, GoabCallout, GoabTable, GoabBadge } from '@abgov/vue-components';
import { useFundingRequestStore } from '@/stores/funding-requests.store';
import { useAuthStore } from '@/stores/auth.store';
import { useRouter } from 'vue-router';

const store = useFundingRequestStore();
const auth = useAuthStore();
const router = useRouter();

onMounted(() => store.fetchRequests({ organizationId: auth.user?.organizationId }));

const pendingCount = computed(() =>
  store.requests.filter((r) => r.status === 'draft').length,
);
</script>

<template>
  <goa-container accent="thin" heading="My Applications">
    <div v-if="store.loading">Loading...</div>
    <GoabCallout v-else-if="store.error" type="emergency" heading="Error">
      {{ store.error }}
    </GoabCallout>
    <GoabCallout v-else-if="store.requests.length === 0" type="information"
      heading="No applications">No funding applications yet.</GoabCallout>
    <template v-else>
      <p>{{ pendingCount }} draft application(s).</p>
      <GoabTable width="100%">
        <thead>
          <tr><th>Program</th><th>Status</th><th>Amount</th><th></th></tr>
        </thead>
        <tbody>
          <tr v-for="req in store.requests" :key="req.requestId">
            <td>{{ req.proposedProgramName }}</td>
            <td><GoabBadge :type="req.status" :content="req.status" /></td>
            <td>${{ req.requestedFundingAmount.toLocaleString() }}</td>
            <td>
              <GoabButton type="tertiary" size="compact"
                @click="router.push({ name: 'application-detail', params: { id: req.requestId } })">
                View
              </GoabButton>
            </td>
          </tr>
        </tbody>
      </GoabTable>
    </template>
  </goa-container>
</template>
```

## Naming
- File: `apps/{stack}/src/views/{PageName}View.vue` (PascalCase + `View` suffix)
- Examples: `DashboardView.vue`, `ApplicationDetailView.vue`

## Rules
1. Always `<script setup lang="ts">` -- never Options API, never `defineComponent()`
2. Import design system components from `@abgov/vue-components` -- never raw HTML for buttons, tables, callouts
3. Handle all three states in template: loading, error, content (with empty sub-state)
4. Use `v-if` / `v-else-if` / `v-else` chain -- never nested `v-if`
5. Keep business logic in stores -- views call store actions and read store state
6. Use `computed()` for derived display values -- never compute in template expressions
7. Route params via `useRoute().params` -- never parse `window.location`
8. Use `goa-container` as the outermost content wrapper for every page
