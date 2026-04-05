# Form Page Pattern

## Convention

A data-entry form using GoA Design System components with reactive
validation. Based on the NewApplicationView pattern from cfs portal.

## Template
```vue
<template>
  <section>
    <h2>{Form Title}</h2>
    <goa-callout v-if="apiError" type="emergency" heading="Submission failed">
      {{ apiError }}
    </goa-callout>
    <form @submit.prevent="handleSubmit">
      <!-- Text Input -->
      <goa-form-item :error="errors.{field}" label="{Label}" requirement="required">
        <goa-input name="{field}" :value="form.{field}"
          @_change="(e: Event) => form.{field} = (e as CustomEvent).detail.value" />
      </goa-form-item>
      <!-- Textarea -->
      <goa-form-item :error="errors.description" label="Description">
        <goa-textarea name="description" :value="form.description"
          @_change="(e: Event) => form.description = (e as CustomEvent).detail.value" rows="4" />
      </goa-form-item>
      <!-- Dropdown -->
      <goa-form-item :error="errors.{enumField}" label="{Enum Label}" requirement="required">
        <goa-dropdown name="{enumField}" :value="form.{enumField}"
          @_change="(e: Event) => form.{enumField} = (e as CustomEvent).detail.value">
          <goa-dropdown-item v-for="opt in {options}" :key="opt.value"
            :value="opt.value" :label="opt.label" />
        </goa-dropdown>
      </goa-form-item>
      <!-- Conditional Section -->
      <template v-if="form.{triggerField} === '{triggerValue}'">
        <goa-form-item :error="errors.{conditionalField}" label="{Conditional Label}" requirement="required">
          <goa-input name="{conditionalField}" :value="form.{conditionalField}"
            @_change="(e: Event) => form.{conditionalField} = (e as CustomEvent).detail.value" />
        </goa-form-item>
      </template>
      <!-- Actions -->
      <div class="form-actions">
        <goa-button type="primary" @_click="handleSubmit" :disabled="submitting">Submit</goa-button>
        <goa-button type="secondary" @_click="router.back()">Cancel</goa-button>
      </div>
    </form>
  </section>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue';
import { useRouter } from 'vue-router';
import { use{Entity}Store } from '@/stores/{resource}.store';
import { {Entity}CreateSchema } from '@shared/schemas/{resource}.schema';

const router = useRouter();
const store = use{Entity}Store();
const form = reactive({ {field}: '', description: '', {enumField}: '', {conditionalField}: '' });
const errors = reactive<Record<string, string>>({});
const apiError = ref('');
const submitting = ref(false);

function validate(): boolean {
  Object.keys(errors).forEach(k => delete errors[k]);
  const result = {Entity}CreateSchema.safeParse(form);
  if (!result.success) {
    for (const [field, msgs] of Object.entries(result.error.flatten().fieldErrors)) {
      errors[field] = msgs?.[0] ?? 'Invalid';
    }
    return false;
  }
  return true;
}

async function handleSubmit() {
  apiError.value = '';
  if (!validate()) return;
  submitting.value = true;
  try {
    const created = await store.create(form);
    router.push(`/{resource}/${created.id}`);
  } catch (err: unknown) {
    apiError.value = err instanceof Error ? err.message : 'An error occurred';
  } finally { submitting.value = false; }
}
</script>
```

## Rules
1. Use `goa-form-item` with `label` and `requirement` props for every field.
2. Field-level errors display via `:error="errors.fieldName"` on `goa-form-item`.
3. API errors display as `goa-callout type="emergency"` above the form.
4. Validate with the shared Zod schema before submission -- never skip.
5. Use `reactive()` for form state, `ref()` for scalar flags.
6. Conditional sections use `v-if` on a trigger field value.
7. Submit button is disabled while `submitting` is true.
8. On success, navigate to the detail or list page -- never stay on the form.
9. Cancel button calls `router.back()` -- never resets the form silently.
10. GoA events use `@_click` / `@_change` (underscore prefix convention).
