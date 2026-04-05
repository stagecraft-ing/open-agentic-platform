# Detail Page Pattern

## Convention
Detail pages show a single record with metadata, status badge, and tabbed sections. They load the record by ID from route params and may show related data in tabs.

## Template
```vue
<script setup lang="ts">
import { onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { use{Resource}Store } from '../stores/{resource}.store'

const route = useRoute()
const router = useRouter()
const store = use{Resource}Store()

const id = computed(() => route.params.id as string)

onMounted(() => {
  store.fetch{Entity}(id.value)
})

const item = computed(() => store.current{Entity})
const statusBadge = computed(() => {
  const map: Record<string, string> = {
    draft: 'information', submitted: 'important',
    approved: 'success', denied: 'emergency',
  }
  return map[item.value?.status ?? ''] ?? 'information'
})
</script>

<template>
  <goa-container type="non-interactive" :padding="6">
    <goa-button type="tertiary" @click="router.back()">Back</goa-button>

    <div v-if="store.loading" class="loading">Loading...</div>

    <goa-callout v-else-if="store.error" type="emergency" heading="Error">
      {{ store.error }}
    </goa-callout>

    <template v-else-if="item">
      <div class="detail-header">
        <h1>{{ item.{titleField} }}</h1>
        <goa-badge :type="statusBadge" :content="item.status" />
      </div>

      <div class="detail-meta">
        <dl>
          <dt>{Label}</dt><dd>{{ item.{field} }}</dd>
        </dl>
      </div>

      <!-- Tabs for related data -->
      <goa-tabs>
        <goa-tab heading="Overview"><!-- overview content --></goa-tab>
        <goa-tab heading="{Related}"><!-- related data --></goa-tab>
      </goa-tabs>

      <!-- Action buttons (conditional on status/role) -->
      <div class="detail-actions">
        <goa-button v-if="item.status === 'draft'" type="primary"
          @click="store.transition(id, 'submitted')">
          Submit
        </goa-button>
      </div>
    </template>
  </goa-container>
</template>
```

## Rules
1. Load record by `route.params.id` in `onMounted`
2. Show back button at top
3. Display status badge next to title
4. Use tabs for related data sections
5. Conditional action buttons based on status and user role
6. Handle loading, error, and not-found states
