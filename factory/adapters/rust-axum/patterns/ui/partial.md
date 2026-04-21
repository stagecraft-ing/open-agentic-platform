# Partial Template Pattern

Partials are reusable HTML fragments rendered both inline (via `{% include %}`)
and as HTMX response targets. They enable dynamic updates without full page reloads.

## Convention

- File: `templates/partials/{name}.html`
- No `{% extends %}` — partials are fragments, not full pages
- Variables come from the including template's scope
- Used for: table rows, cards, form fields, toast messages

## Template

```html
<!-- templates/partials/{resource}_row.html -->
<div id="{resource}-{{ item.id }}" class="flex items-center justify-between p-4 border rounded-lg mb-2">
  <div>
    <p class="font-medium text-gray-900">{{ item.{field} }}</p>
    <p class="text-sm text-gray-500">{{ item.created_at }}</p>
  </div>
  <div class="flex gap-2">
    <a href="/{resource}/{{ item.id }}"
      class="text-indigo-600 hover:text-indigo-800 text-sm">View</a>
    <button
      hx-delete="/{resource}/{{ item.id }}"
      hx-target="#{resource}-{{ item.id }}"
      hx-swap="outerHTML"
      hx-confirm="Are you sure?"
      class="text-red-600 hover:text-red-800 text-sm"
    >Delete</button>
  </div>
</div>
```

### Toast partial

```html
<!-- templates/partials/toast.html -->
<div id="toast" class="fixed top-4 right-4 bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded"
     role="alert"
     hx-swap-oob="true">
  <span>{{ message }}</span>
</div>
```

## Example

From `templates/partials/site_row.html`:

```html
<div id="site-{{ site.id }}" class="flex items-center justify-between p-4 border rounded mb-2">
  <span class="text-gray-900">{{ site.url }}</span>
  <button hx-delete="/sites/{{ site.id }}" hx-target="#site-{{ site.id }}" hx-swap="outerHTML"
    class="text-red-600 hover:text-red-800 text-sm">Delete</button>
</div>
```

## Rules

1. Partials never use `{% extends %}` — they are included fragments.
2. Give the root element a unique `id` for HTMX targeting.
3. Use `hx-swap="outerHTML"` on the element to replace itself after mutation.
4. Use `hx-swap-oob="true"` for out-of-band updates (toasts, counters).
5. Keep partials small — one component per file.
