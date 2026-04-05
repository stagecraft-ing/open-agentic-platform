# Profile Page Pattern

## Convention
Profile page displays the authenticated user's information from the auth store. Read-only display of user identity, roles, and organization (if applicable). Requires authentication.

## Template
```vue
<script setup lang="ts">
import { computed } from 'vue'
import { useAuthStore } from '../stores/auth.store'

const auth = useAuthStore()
const user = computed(() => auth.user)
</script>

<template>
  <goa-container type="non-interactive" :padding="6">
    <h1>My Profile</h1>

    <div v-if="user" class="profile-card">
      <dl>
        <dt>Name</dt>
        <dd>{{ user.displayName }}</dd>

        <dt>Email</dt>
        <dd>{{ user.email }}</dd>

        <dt>Roles</dt>
        <dd>{{ user.roles?.join(', ') || 'None' }}</dd>
      </dl>
    </div>

    <div class="profile-actions">
      <goa-button type="tertiary" @click="auth.logout()">Sign Out</goa-button>
    </div>
  </goa-container>
</template>
```

## Rules
1. Route meta: `requiresAuth: true`
2. Data comes from auth store — no separate API call needed
3. Read-only display (user edits happen at the identity provider)
4. Include a sign-out button
