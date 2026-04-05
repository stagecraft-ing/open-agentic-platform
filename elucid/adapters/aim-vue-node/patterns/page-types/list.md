# List Page Pattern

## Convention

A filterable, paginated table view using GoA Design System components.
Based on the staff request queue (DashboardView) pattern from cfs portal.

## Template

```vue
<template>
  <section>
    <h2>{Page Title}</h2>

    <!-- Filter Bar -->
    <div class="filter-bar">
      <goa-form-item label="{Filter Label}">
        <goa-dropdown name="{filter}" :value="filters.{filter}"
          @_change="(e: Event) => filters.{filter} = (e as CustomEvent).detail.value">
          <goa-dropdown-item value="" label="All" />
          <goa-dropdown-item v-for="opt in {options}" :key="opt"
            :value="opt" :label="opt" />
        </goa-dropdown>
      </goa-form-item>
      <goa-button type="secondary" @_click="applyFilters">Apply</goa-button>
      <goa-button type="tertiary" @_click="clearFilters">Clear</goa-button>
    </div>

    <!-- Loading -->
    <goa-skeleton v-if="store.loading" type="text" :count="5" />

    <!-- Error -->
    <goa-callout v-else-if="store.error" type="emergency" heading="Error">
      {{ store.error }}
    </goa-callout>

    <!-- Empty -->
    <goa-callout v-else-if="store.items.length === 0" type="information"
      heading="No results">
      {Empty state guidance message.}
    </goa-callout>

    <!-- Table -->
    <goa-table v-else width="100%">
      <thead><tr>
        <th>{Column Headers}</th>
      </tr></thead>
      <tbody>
        <tr v-for="item in store.items" :key="item.id">
          <td>{{ item.{field} }}</td>
          <td>{{ formatDate(item.createdAt) }}</td>
          <td>{{ formatCurrency(item.amount) }}</td>
          <td><goa-badge :type="statusBadge(item.status)" :content="item.status" /></td>
          <td><goa-button type="tertiary" size="compact"
            @_click="router.push(`/{resource}/${item.id}`)">View</goa-button></td>
        </tr>
      </tbody>
    </goa-table>

    <!-- Pagination -->
    <div v-if="store.total > 0" class="pagination">
      <goa-button type="tertiary" :disabled="store.page <= 1"
        @_click="changePage(store.page - 1)">Previous</goa-button>
      <span>Page {{ store.page }} of {{ totalPages }}</span>
      <goa-button type="tertiary" :disabled="store.page >= totalPages"
        @_click="changePage(store.page + 1)">Next</goa-button>
    </div>
  </section>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { use{Entity}Store } from '@/stores/{resource}.store';

const router = useRouter();
const store = use{Entity}Store();
const filters = ref({ {filter}: '' });

const totalPages = computed(() => Math.ceil(store.total / store.pageSize));

function applyFilters() { store.fetchList({ ...filters.value, page: 1 }); }
function clearFilters() {
  filters.value = { {filter}: '' };
  store.fetchList({ page: 1 });
}
function changePage(p: number) { store.fetchList({ ...filters.value, page: p }); }

function formatCurrency(v: number) {
  return new Intl.NumberFormat('en-CA', { style: 'currency', currency: 'CAD' }).format(v);
}
function formatDate(d: string) {
  return new Date(d).toLocaleDateString('en-CA');
}
function statusBadge(s: string) {
  const map: Record<string, string> = {
    draft: 'information', submitted: 'midtone',
    approved: 'success', denied: 'emergency',
  };
  return map[s] ?? 'information';
}

onMounted(() => store.fetchList({ page: 1 }));
</script>
```

## Rules

1. Always show loading, error, and empty states -- never a blank page.
2. Use `goa-table` for tabular data, not custom HTML tables.
3. Filter bar uses `goa-dropdown` and `goa-form-item`, not raw selects.
4. Pagination is Previous/Next buttons with page X of Y display.
5. Format helpers for currency (`Intl.NumberFormat`), dates, and status badges.
6. GoA events use `@_click` / `@_change` (underscore prefix).
7. Table actions use `type="tertiary" size="compact"` buttons.
8. Fetch initial data in `onMounted`, not in the store constructor.
