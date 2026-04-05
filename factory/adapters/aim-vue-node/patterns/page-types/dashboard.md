# Dashboard Page Pattern

## Convention
Dashboard pages show an overview with metric cards, summary tables, and quick-action links. They load multiple data sources on mount and display aggregated information.

## Template
```vue
<script setup lang="ts">
import { onMounted, computed } from 'vue'
import { use{Resource}Store } from '../stores/{resource}.store'
import { useAuthStore } from '../stores/auth.store'

const store = use{Resource}Store()
const auth = useAuthStore()

onMounted(() => {
  store.fetchSummary()
})

const metrics = computed(() => [
  { label: '{Metric 1}', value: store.summary?.{field1} ?? 0 },
  { label: '{Metric 2}', value: store.summary?.{field2} ?? 0 },
])
</script>

<template>
  <goa-container type="non-interactive" :padding="6">
    <h1>{Dashboard Title}</h1>

    <goa-callout v-if="store.error" type="emergency" heading="Error">
      {{ store.error }}
    </goa-callout>

    <!-- Metric Cards -->
    <div class="dashboard-cards">
      <div v-for="m in metrics" :key="m.label" class="metric-card">
        <span class="metric-value">{{ m.value }}</span>
        <span class="metric-label">{{ m.label }}</span>
      </div>
    </div>

    <!-- Quick Actions -->
    <div class="quick-actions">
      <goa-button type="primary" @click="$router.push('/{resource}/new')">
        New {Entity}
      </goa-button>
    </div>

    <!-- Summary Table -->
    <goa-table v-if="!store.loading && store.items.length > 0">
      <!-- columns -->
    </goa-table>
  </goa-container>
</template>
```

## Rules
1. Load all data sources in `onMounted`
2. Show metric cards at top, quick actions below, then summary table
3. Use `goa-container` as root wrapper
4. Handle loading, error, and empty states
5. Metric values should be computed from store data
