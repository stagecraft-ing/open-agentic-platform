# Landing Page Pattern

## Convention
Landing pages introduce the service to unauthenticated users. Hero section with description, conditional content based on auth state, process overview steps, and call-to-action.

## Template
```vue
<script setup lang="ts">
import { computed } from 'vue'
import { useAuthStore } from '../stores/auth.store'

const auth = useAuthStore()
const isAuthenticated = computed(() => auth.isAuthenticated)
</script>

<template>
  <goa-container type="non-interactive" :padding="6">
    <!-- Hero -->
    <div class="hero-section">
      <h1>{Service Name}</h1>
      <p>{Service description paragraph}</p>
    </div>

    <!-- Authenticated: Quick Actions -->
    <div v-if="isAuthenticated" class="quick-start">
      <h2>Get Started</h2>
      <div class="action-cards">
        <div class="action-card" @click="$router.push('/dashboard')">
          <h3>View Dashboard</h3>
          <p>See your current {items}.</p>
        </div>
        <div class="action-card" @click="$router.push('/{resource}/new')">
          <h3>New {Entity}</h3>
          <p>Start a new {entity}.</p>
        </div>
      </div>
    </div>

    <!-- Unauthenticated: Sign-in prompt -->
    <div v-else class="sign-in-prompt">
      <goa-callout type="information" heading="Sign in to continue">
        You need to sign in to access the portal.
      </goa-callout>
      <goa-button type="primary" @click="$router.push('/login')">Sign In</goa-button>
    </div>

    <!-- Process Overview -->
    <div class="process-steps">
      <h2>How It Works</h2>
      <ol>
        <li><strong>Step 1:</strong> {description}</li>
        <li><strong>Step 2:</strong> {description}</li>
        <li><strong>Step 3:</strong> {description}</li>
      </ol>
    </div>
  </goa-container>
</template>
```

## Rules
1. No auth required — page is publicly accessible
2. Show different content based on auth state
3. Use goa-container as root wrapper
4. Hero section with service name and description
5. Process overview should explain the workflow in 3-5 steps
