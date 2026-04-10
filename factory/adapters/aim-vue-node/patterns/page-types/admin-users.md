# Admin User Management Page Pattern

## Convention

A paginated user list with role badges and an inline role-assignment panel.
Visible only to users with the `admin` role. Located under the Administration
nav section.

## Template

```vue
<template>
  <section>
    <h2>Maintain Users</h2>

    <!-- Search -->
    <goa-form-item label="Search users">
      <goa-input name="search" :value="search"
        @_change="(e: Event) => search = (e as CustomEvent).detail.value"
        leadingicon="search" placeholder="Name or email" />
    </goa-form-item>

    <!-- Loading -->
    <goa-skeleton v-if="store.loading" type="text" :count="5" />

    <!-- Error -->
    <goa-callout v-else-if="store.error" type="emergency" heading="Error">
      {{ store.error }}
    </goa-callout>

    <!-- User Table -->
    <goa-table v-else width="100%">
      <thead><tr>
        <th>Name</th>
        <th>Email</th>
        <th>Roles</th>
        <th>Actions</th>
      </tr></thead>
      <tbody>
        <tr v-for="user in store.items" :key="user.userId">
          <td>{{ user.displayName }}</td>
          <td>{{ user.email }}</td>
          <td>
            <goa-badge v-for="role in user.roles" :key="role"
              type="midtone" :content="role" />
          </td>
          <td>
            <goa-button type="tertiary" size="compact"
              @_click="openRolePanel(user)">Manage Roles</goa-button>
          </td>
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

    <!-- Role Assignment Panel -->
    <goa-modal v-if="selectedUser" :open="!!selectedUser"
      heading="Manage Roles" @_close="selectedUser = null">
      <p>Assigning roles for <strong>{{ selectedUser.displayName }}</strong></p>
      <div class="role-checkboxes">
        <goa-checkbox v-for="role in allRoles" :key="role.roleId"
          :name="role.roleName"
          :checked="selectedUser.roles.includes(role.roleName)"
          :text="role.roleName"
          @_change="(e: Event) => toggleRole(role.roleId, (e as CustomEvent).detail.checked)" />
      </div>
      <div slot="actions">
        <goa-button type="primary" @_click="saveRoles">Save</goa-button>
        <goa-button type="tertiary" @_click="selectedUser = null">Cancel</goa-button>
      </div>
    </goa-modal>
  </section>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useAdminUserStore } from '@/stores/admin-user.store'
import { useRoleStore } from '@/stores/role.store'

const store = useAdminUserStore()
const roleStore = useRoleStore()
const search = ref('')
const selectedUser = ref<any>(null)
const pendingRoleIds = ref<string[]>([])

const totalPages = computed(() => Math.ceil(store.total / store.pageSize))
const allRoles = computed(() => roleStore.items)

function openRolePanel(user: any) {
  selectedUser.value = user
  pendingRoleIds.value = [...user.roleIds]
}

function toggleRole(roleId: string, checked: boolean) {
  if (checked) pendingRoleIds.value.push(roleId)
  else pendingRoleIds.value = pendingRoleIds.value.filter(id => id !== roleId)
}

async function saveRoles() {
  await store.updateUserRoles(selectedUser.value.userId, pendingRoleIds.value)
  selectedUser.value = null
  store.fetchList({ search: search.value, page: store.page })
}

function changePage(p: number) { store.fetchList({ search: search.value, page: p }) }

onMounted(() => {
  store.fetchList({ page: 1 })
  roleStore.fetchAll()
})
</script>
```

## Route

```ts
{
  path: '/admin/users',
  component: () => import('@/views/AdminUsersView.vue'),
  meta: { requiresAuth: true, requiredRole: 'admin', navSection: 'admin', navOrder: 1, title: 'Maintain Users' }
}
```

## Rules

1. **Admin-only.** Route requires `admin` role. Hidden from navigation for non-admin users.
2. **Role assignment uses checkboxes.** Multi-select with plain-English labels — not a dropdown.
3. **Immediate feedback.** Show success/error notification after saving roles.
4. **Search by name or email.** Server-side filtering with debounced input.
5. **Role badges in table.** Each user row shows their current roles as `goa-badge` components.
6. **Admin nav section.** All admin pages group under an "Administration" navigation section, visible only to admin role.
