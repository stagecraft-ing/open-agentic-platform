# Component Pattern

Reusable Vue SFC components for UI building blocks. Components receive data via
props and emit events — they never import stores directly.

## Convention

- Component file: `apps/{stack}/src/components/{Name}.vue`
- GoA wrapper: `apps/{stack}/src/components/goa/{GoabName}.vue`

## Template — Data Display Component

```vue
<script setup lang="ts">
import type { {Entity} } from '@template/shared';

defineProps<{
  item: {Entity};
}>();

defineEmits<{
  select: [id: string];
  delete: [id: string];
}>();
</script>

<template>
  <goa-container>
    <h3>{{ item.displayName }}</h3>
    <p>{{ item.description }}</p>
    <goa-button-group>
      <goa-button type="secondary" @_click="$emit('select', item.id)">
        View
      </goa-button>
      <goa-button type="tertiary" @_click="$emit('delete', item.id)">
        Delete
      </goa-button>
    </goa-button-group>
  </goa-container>
</template>
```

## Template — GoA Wrapper Component

```vue
<script setup lang="ts">
const props = defineProps<{
  value: string;
  label: string;
  error?: string;
  required?: boolean;
}>();

const emit = defineEmits<{
  'update:value': [value: string];
}>();
</script>

<template>
  <goa-input
    :name="label"
    :value="value"
    :error="error"
    :required="required"
    @_change="emit('update:value', ($event as CustomEvent).detail.value)"
  />
</template>
```

## Rules

1. Use `<script setup lang="ts">` — no Options API.
2. Props via `defineProps<{}>()` with TypeScript generics.
3. Events via `defineEmits<{}>()` with typed payloads.
4. Components receive data via props — never import Pinia stores directly.
5. GoA web components use `@_change` / `@_click` events (underscore prefix).
6. GoA wrapper components bridge `v-model` to custom element events.
7. Import shared types from `@template/shared`.
8. No business logic in components — delegate to parent views or composables.
