# Login Page Pattern

## Convention
Login page shows available auth drivers and initiates the auth flow. Guest-only (redirects authenticated users). Uses the auth store to trigger login.

## Template
```vue
<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth.store'

const auth = useAuthStore()
const router = useRouter()

// Redirect if already authenticated
if (auth.isAuthenticated) {
  router.replace('/dashboard')
}

function handleLogin(driver?: string) {
  auth.login(driver)
}
</script>

<template>
  <goa-container type="non-interactive" :padding="6">
    <div class="login-container">
      <h1>Sign In</h1>
      <p>Sign in to access the portal.</p>

      <div class="login-actions">
        <goa-button type="primary" @click="handleLogin()">
          Sign in with {Provider}
        </goa-button>
      </div>

      <p class="login-help">
        Need help? <router-link to="/about">Learn more about this service</router-link>.
      </p>
    </div>
  </goa-container>
</template>
```

## Rules
1. Route meta: `guestOnly: true` — redirect authenticated users away
2. Use auth store's login() method — never construct auth URLs directly
3. Keep the page simple — just a sign-in button and help link
4. Center the login container for visual focus
