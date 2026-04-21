# Admin Role Management Page Pattern

## Convention

A **paginated** role list with user counts per role and click-to-open role
detail. Includes role creation, editing, and deletion with safety guards, with
the **complete permission catalogue** rendered on create/edit (every system
permission listed, grouped by domain, with plain-English labels, individually
togglable). Visible only to users with the `admin` role. Located under the
Administration nav section.

### Paginated list

The roles list MUST be paginated even when small — consistency with the rest
of the admin surface matters more than the row count in a particular project.
List state carries the standard pagination quintuple: `page`, `pageSize`
(default 10), `total`, `totalPages`, with column sort on `roleName` and
`userCount`. The API service returns `{ data, total }`.

The `userCount` column is computed server-side (join on `user_role`) — the
client MUST NOT receive raw user lists just to count them. A role with many
users must still render quickly.

Clicking a role row opens its detail view (permissions + assigned users). The
permission toggle modal shown in the template below is the detail surface for
small catalogues; a dedicated detail page is acceptable as the catalogue grows.

### Complete permission catalogue on create/edit

When creating or editing a role, the permission panel MUST render **every
permission in the system**, not just permissions the acting admin currently
holds and not a "recommended" subset. Permissions are:

- **Grouped by domain** (e.g., "Funding Requests", "Users", "Reports") —
  domains come from the `permission.domain` column, sorted alphabetically.
- **Labelled in plain English** — `permission.display_name`, not the
  technical `permission.permission_key`.
- **Individually togglable** — each permission has its own checkbox; no
  bundled "grants" or pre-packaged sets.

This is the operational surface that lets business admins tune access without
code changes. Hiding permissions here means they can only be changed by a
developer with DB access — which defeats the point of runtime role
configuration.

## Template

```vue
<template>
  <section>
    <h2>Maintain Roles</h2>

    <goa-button type="primary" @_click="showCreateModal = true">
      Create Role
    </goa-button>

    <!-- Loading -->
    <goa-skeleton v-if="store.loading" type="text" :count="3" />

    <!-- Error -->
    <goa-callout v-else-if="store.error" type="emergency" heading="Error">
      {{ store.error }}
    </goa-callout>

    <!-- Role List -->
    <goa-table v-else width="100%">
      <thead><tr>
        <th>Role</th>
        <th>Description</th>
        <th>Users</th>
        <th>Actions</th>
      </tr></thead>
      <tbody>
        <tr v-for="role in store.items" :key="role.roleId">
          <td>
            {{ role.roleName }}
            <goa-badge v-if="role.isProtected" type="information" content="Protected" />
          </td>
          <td>{{ role.description }}</td>
          <td>{{ role.userCount }}</td>
          <td>
            <goa-button type="tertiary" size="compact"
              @_click="openPermissions(role)">Permissions</goa-button>
            <goa-button type="tertiary" size="compact"
              @_click="editRole(role)">Edit</goa-button>
            <goa-button type="tertiary" size="compact"
              :disabled="role.isProtected"
              @_click="confirmDelete(role)">Delete</goa-button>
          </td>
        </tr>
      </tbody>
    </goa-table>

    <!-- Permission Toggle Panel -->
    <goa-modal v-if="selectedRole" :open="!!selectedRole"
      :heading="`Permissions: ${selectedRole.roleName}`" @_close="selectedRole = null">
      <div v-for="domain in permissionDomains" :key="domain" class="permission-domain">
        <h4>{{ domain }}</h4>
        <goa-checkbox v-for="perm in permissionsByDomain(domain)" :key="perm.permissionId"
          :name="perm.permissionKey"
          :checked="selectedPermissions.has(perm.permissionId)"
          :text="perm.displayName"
          @_change="(e: Event) => togglePermission(perm.permissionId, (e as CustomEvent).detail.checked)" />
      </div>
      <div slot="actions">
        <goa-button type="primary" @_click="savePermissions">Save</goa-button>
        <goa-button type="tertiary" @_click="selectedRole = null">Cancel</goa-button>
      </div>
    </goa-modal>

    <!-- Create/Edit Role Modal -->
    <goa-modal :open="showCreateModal || !!editingRole"
      :heading="editingRole ? 'Edit Role' : 'Create Role'"
      @_close="showCreateModal = false; editingRole = null">
      <goa-form-item label="Role Name" :error="nameError">
        <goa-input name="roleName" :value="roleForm.roleName"
          @_change="(e: Event) => roleForm.roleName = (e as CustomEvent).detail.value" />
      </goa-form-item>
      <goa-form-item label="Description">
        <goa-textarea name="description" :value="roleForm.description"
          @_change="(e: Event) => roleForm.description = (e as CustomEvent).detail.value" />
      </goa-form-item>
      <div slot="actions">
        <goa-button type="primary" @_click="saveRole">Save</goa-button>
        <goa-button type="tertiary"
          @_click="showCreateModal = false; editingRole = null">Cancel</goa-button>
      </div>
    </goa-modal>

    <!-- Delete Confirmation -->
    <goa-modal :open="!!deletingRole" heading="Confirm Deletion" @_close="deletingRole = null">
      <goa-callout v-if="deletingRole?.userCount > 0" type="important"
        heading="Role has active users">
        {{ deletingRole.userCount }} user(s) currently hold this role.
        They will lose all permissions granted by this role.
      </goa-callout>
      <p>Are you sure you want to delete the role <strong>{{ deletingRole?.roleName }}</strong>?</p>
      <div slot="actions">
        <goa-button type="primary" variant="destructive" @_click="doDelete">Delete</goa-button>
        <goa-button type="tertiary" @_click="deletingRole = null">Cancel</goa-button>
      </div>
    </goa-modal>
  </section>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoleStore } from '@/stores/role.store'
import { usePermissionStore } from '@/stores/permission.store'

const store = useRoleStore()
const permissionStore = usePermissionStore()

const selectedRole = ref<any>(null)
const selectedPermissions = ref(new Set<string>())
const showCreateModal = ref(false)
const editingRole = ref<any>(null)
const deletingRole = ref<any>(null)
const roleForm = ref({ roleName: '', description: '' })
const nameError = ref('')

const permissionDomains = computed(() =>
  [...new Set(permissionStore.items.map(p => p.domain))].sort()
)
function permissionsByDomain(domain: string) {
  return permissionStore.items.filter(p => p.domain === domain)
}

function openPermissions(role: any) {
  selectedRole.value = role
  selectedPermissions.value = new Set(role.permissionIds)
}

function togglePermission(permId: string, checked: boolean) {
  if (checked) selectedPermissions.value.add(permId)
  else selectedPermissions.value.delete(permId)
}

async function savePermissions() {
  await store.updateRolePermissions(selectedRole.value.roleId, [...selectedPermissions.value])
  selectedRole.value = null
  store.fetchAll()
}

function editRole(role: any) {
  editingRole.value = role
  roleForm.value = { roleName: role.roleName, description: role.description }
}

async function saveRole() {
  if (!roleForm.value.roleName.trim()) { nameError.value = 'Role name is required'; return }
  nameError.value = ''
  if (editingRole.value) {
    await store.updateRole(editingRole.value.roleId, roleForm.value)
    editingRole.value = null
  } else {
    await store.createRole(roleForm.value)
    showCreateModal.value = false
  }
  roleForm.value = { roleName: '', description: '' }
  store.fetchAll()
}

function confirmDelete(role: any) { deletingRole.value = role }

async function doDelete() {
  await store.deleteRole(deletingRole.value.roleId)
  deletingRole.value = null
  store.fetchAll()
}

onMounted(() => {
  store.fetchAll()
  permissionStore.fetchAll()
})
</script>
```

## Route

```ts
{
  path: '/admin/roles',
  component: () => import('@/views/AdminRolesView.vue'),
  meta: { requiresAuth: true, requiredRole: 'admin', navSection: 'admin', navOrder: 2, title: 'Maintain Roles' }
}
```

## Rules

1. **Admin-only.** Route requires `admin` role. Hidden from navigation for non-admin users.
2. **Full CRUD.** Roles can be created, viewed, edited, and deleted. Read-only views are not acceptable.
3. **Protected roles cannot be deleted.** The `admin` role has `is_protected = true` from seed data. The delete button is disabled for protected roles.
4. **Role deletion requires confirmation.** If the role has active users, a warning callout shows the user count and explains the impact before the user can confirm.
5. **Permission panel groups by domain.** Permissions are displayed with plain-English labels, organized by domain area (e.g., "Funding Requests", "Users"). Toggle on/off per role.
6. **Permission panel only visible when `RBAC_GRANULAR_PERMISSIONS` flag is enabled.** When disabled, the "Permissions" button is hidden and roles grant a fixed default set.
7. **Destructive actions require confirmation.** Delete operations show a confirmation modal.
8. **Immediate feedback.** Show success/error notification after every action.
9. **Lookup table admin pattern.** This same CRUD pattern applies to all lookup table admin pages — list with search, create form, inline/modal editing, delete with confirmation.
10. **Paginated list.** The roles list uses the standard pagination quintuple (`page`, `pageSize`, `total`, `totalPages`, sort) and the API returns `{ data, total }`. `userCount` is computed server-side — never send raw user lists to the client.
11. **Complete permission catalogue on create/edit.** The permission panel shows every permission in the system, grouped by domain, with plain-English labels, individually togglable — no "recommended" subsets, no role-gated permission visibility.
