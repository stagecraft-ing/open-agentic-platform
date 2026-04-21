# State (Pinia Store)

## Convention
Stores use Pinia setup syntax (composable-style `defineStore`). State is `ref()`, actions are async functions with loading/try/catch/finally, API calls go through a composable.

## Template
```typescript
import { ref, computed } from 'vue';
import { defineStore } from 'pinia';
import { useInternalApi } from '@/composables/useInternalApi';

export interface {Entity} {
  {entityId}: string;
  // ... fields matching the shared type
}

export const use{Resource}Store = defineStore('{resource}', () => {
  const api = useInternalApi();

  const items = ref<{Entity}[]>([]);
  const currentItem = ref<{Entity} | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const pagination = ref({ page: 1, limit: 20, total: 0 });

  const isEmpty = computed(() => items.value.length === 0);

  async function fetchItems(filters?: Record<string, unknown>) {
    loading.value = true;
    error.value = null;
    try {
      const res = await api.get('/{resources}', { params: filters });
      items.value = res.data.data;
      pagination.value = res.data.pagination;
    } catch (err: unknown) {
      error.value = err instanceof Error ? err.message : 'Failed to load {resources}';
    } finally {
      loading.value = false;
    }
  }

  async function fetchById(id: string) {
    loading.value = true;
    error.value = null;
    try {
      const res = await api.get(`/{resources}/${id}`);
      currentItem.value = res.data;
    } catch (err: unknown) {
      error.value = err instanceof Error ? err.message : 'Failed to load {resource}';
    } finally {
      loading.value = false;
    }
  }

  return { items, currentItem, loading, error, pagination, isEmpty, fetchItems, fetchById };
});
```

## Example
`funding-requests.store.ts` -- public portal store using `useGateway`:
```typescript
import { ref, computed } from 'vue';
import { defineStore } from 'pinia';
import { useGateway } from '@/composables/useGateway';

export interface FundingRequest {
  requestId: string; organizationId: string; status: string;
  requestedFundingAmount: number; proposedProgramName: string; createdAt: string;
}

export const useFundingRequestStore = defineStore('fundingRequests', () => {
  const gateway = useGateway();
  const requests = ref<FundingRequest[]>([]);
  const currentRequest = ref<FundingRequest | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const pagination = ref({ page: 1, limit: 20, total: 0 });

  const draftRequests = computed(() => requests.value.filter((r) => r.status === 'draft'));

  // fetchRequests follows same pattern as template fetchItems -- omitted for brevity

  async function submitRequest(id: string) {
    loading.value = true;
    error.value = null;
    try {
      await gateway.post(`/funding-requests/${id}/transition`, { action: 'submit' });
      const res = await gateway.get(`/funding-requests/${id}`);
      currentRequest.value = res.data;
    } catch (err: unknown) {
      error.value = err instanceof Error ? err.message : 'Failed to submit request';
    } finally {
      loading.value = false;
    }
  }

  return {
    requests, currentRequest, loading, error, pagination,
    draftRequests, submitRequest,
  };
});
```

## Naming
- File: `apps/{stack}/src/stores/{resource}.store.ts` (kebab-case, `.store.ts` suffix)
- Store ID: camelCase string (e.g. `'fundingRequests'`)
- Export: `use{Resource}Store` (PascalCase with `use` prefix and `Store` suffix)

## Rules
1. Always setup syntax (`defineStore('id', () => { ... })`) -- never options syntax
2. Every action: `loading=true`, `error=null`, `try/catch/finally`, `loading=false` in finally
3. API via composable (`useInternalApi` or `useGateway`) -- never raw `axios` in stores
4. Type all state with interfaces defined at the top of the file
5. Expose only what views need -- keep helpers private (not in return object)
6. Use `computed()` for derived state -- never duplicate data across refs
7. Reset `error` to `null` before every action -- stale errors cause misleading UI
