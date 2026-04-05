# Content Page Pattern

## Convention
Static content pages (About, Help, FAQ) display informational text with no data fetching. Minimal logic, mostly template markup with GoA typography and layout components.

## Template
```vue
<script setup lang="ts">
// No data fetching — static content only
</script>

<template>
  <goa-container type="non-interactive" :padding="6">
    <h1>{Page Title}</h1>

    <section>
      <h2>{Section Heading}</h2>
      <p>{Content paragraph}</p>
    </section>

    <section>
      <h2>{Another Section}</h2>
      <p>{More content}</p>
      <ul>
        <li>{List item}</li>
      </ul>
    </section>

    <!-- Optional: External links or contact info -->
    <section>
      <h2>Need Help?</h2>
      <p>Contact us at <a href="mailto:{email}">{email}</a></p>
    </section>
  </goa-container>
</template>
```

## Rules
1. No API calls — pure static content
2. Use semantic HTML (h1, h2, p, ul, ol)
3. Wrap in goa-container
4. Keep script setup minimal (empty or just imports for shared layout)
