# Test (Pinia Store)

## Convention
Store tests use Vitest with a fresh `createPinia()` per test. API composables are mocked. Each `describe` block covers one action. Tests verify initial state, success, and error.

## Template
```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { use{Resource}Store } from '@/stores/{resource}.store';

vi.mock('@/composables/useInternalApi', () => ({
  useInternalApi: () => ({
    get: vi.fn(), post: vi.fn(), patch: vi.fn(), delete: vi.fn(),
  }),
}));
import { useInternalApi } from '@/composables/useInternalApi';

describe('{resource} store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  describe('initial state', () => {
    it('has empty defaults', () => {
      const store = use{Resource}Store();
      expect(store.items).toEqual([]);
      expect(store.currentItem).toBeNull();
      expect(store.loading).toBe(false);
      expect(store.error).toBeNull();
    });
  });

  describe('fetch{Resources}', () => {
    it('sets items on success', async () => {
      const store = use{Resource}Store();
      const api = useInternalApi();
      (api.get as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        data: { data: [{ {entityId}: '1' }], pagination: { page: 1, limit: 20, total: 1 } },
      });
      await store.fetch{Resources}();
      expect(store.items).toHaveLength(1);
      expect(store.loading).toBe(false);
      expect(store.error).toBeNull();
    });

    it('sets error on failure', async () => {
      const store = use{Resource}Store();
      const api = useInternalApi();
      (api.get as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('Network error'));
      await store.fetch{Resources}();
      expect(store.items).toEqual([]);
      expect(store.error).toBe('Network error');
      expect(store.loading).toBe(false);
    });
  });
});
```

## Example
`funding-requests.store.test.ts` -- tests for the public portal store:
```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useFundingRequestStore } from '@/stores/funding-requests.store';

vi.mock('@/composables/useGateway', () => ({
  useGateway: () => ({ get: vi.fn(), post: vi.fn() }),
}));
import { useGateway } from '@/composables/useGateway';

describe('funding-requests store', () => {
  beforeEach(() => { setActivePinia(createPinia()); vi.clearAllMocks(); });

  // initial state tests follow same shape as template -- omitted for brevity

  describe('fetchRequests', () => {
    it('loads requests and pagination', async () => {
      const store = useFundingRequestStore();
      const gw = useGateway();
      (gw.get as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
        data: { data: [{ requestId: 'r-1' }], pagination: { page: 1, limit: 20, total: 1 } },
      });
      await store.fetchRequests({ organizationId: 'org-1' });
      expect(gw.get).toHaveBeenCalledWith('/funding-requests', { params: { organizationId: 'org-1' } });
      expect(store.requests).toHaveLength(1);
    });
    it('sets error on failure', async () => {
      const store = useFundingRequestStore();
      (useGateway().get as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('Unavailable'));
      await store.fetchRequests();
      expect(store.error).toBe('Unavailable');
      expect(store.loading).toBe(false);
    });
  });

  describe('submitRequest', () => {
    it('posts transition and refreshes current request', async () => {
      const store = useFundingRequestStore(); const gw = useGateway();
      (gw.post as ReturnType<typeof vi.fn>).mockResolvedValueOnce({});
      (gw.get as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ data: { requestId: 'r-1', status: 'submitted' } });
      await store.submitRequest('r-1');
      expect(gw.post).toHaveBeenCalledWith('/funding-requests/r-1/transition', { action: 'submit' });
    });
  });
});
```

## Naming
- File: `apps/{stack}/src/stores/{resource}.store.test.ts` (colocated with store)
- Describe block: kebab-case store name; nested describes: one per action

## Rules
1. `setActivePinia(createPinia())` in every `beforeEach` -- never reuse pinia across tests
2. `vi.clearAllMocks()` in `beforeEach` -- prevent mock leakage between tests
3. Mock the API composable (`useGateway` or `useInternalApi`) -- never hit real endpoints
4. Test at least two paths per action: success (assert state) and failure (assert error)
5. Assert `loading` is `false` after action completes -- catches missing `finally`
6. Assert API called with correct URL/params; never test Pinia internals (`$patch`, `$state`)
