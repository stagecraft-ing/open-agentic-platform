# Page Type: Help / FAQ

An authenticated help page with collapsible FAQ sections using GoA accordion
components. Static content or loaded once from a configuration source.

## Convention

- View: `apps/{stack}/src/views/HelpView.vue`
- Route: `{ path: '/help', name: 'help', meta: { requiresAuth: true } }`

## Template

```vue
<script setup lang="ts">
// Static FAQ data — replace with API call if content is dynamic
const sections = [
  {
    heading: 'Getting Started',
    items: [
      { question: 'How do I create a new request?', answer: 'Navigate to...' },
      { question: 'What documents do I need?', answer: 'You will need...' },
    ],
  },
  {
    heading: 'Account & Access',
    items: [
      { question: 'How do I reset my password?', answer: 'Use the...' },
    ],
  },
];
</script>

<template>
  <goa-container>
    <h1>Help & FAQ</h1>
    <goa-callout type="information">
      For urgent issues, contact support at support@example.gov.ab.ca
    </goa-callout>

    <section v-for="section in sections" :key="section.heading">
      <h2>{{ section.heading }}</h2>
      <goa-accordion>
        <goa-accordion-item
          v-for="item in section.items"
          :key="item.question"
          :heading="item.question"
        >
          <p>{{ item.answer }}</p>
        </goa-accordion-item>
      </goa-accordion>
    </section>
  </goa-container>
</template>
```

## Rules

1. Use `goa-accordion` and `goa-accordion-item` for FAQ sections.
2. Group questions under semantic `<h2>` section headings.
3. Include a `goa-callout` with support contact information.
4. Route meta `requiresAuth` depends on the audience — public help pages may be unauthenticated.
5. Content is static unless the Build Spec specifies a dynamic FAQ data source.
6. Wrap in `goa-container` for consistent page width.
