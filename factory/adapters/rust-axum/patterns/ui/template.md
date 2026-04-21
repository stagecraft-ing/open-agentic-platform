# Askama Template Pattern

Askama templates are HTML files with Rust expressions compiled at build time.
HTMX attributes provide dynamic interactivity without client-side JavaScript.

## Convention

- File: `templates/{resource}/{action}.html`
- Templates extend `base.html` via `{% extends "base.html" %}`
- Dynamic content via `{{ variable }}` and `{% for %}` / `{% if %}`
- HTMX attributes (`hx-get`, `hx-post`, `hx-swap`) for interactivity
- Template structs in handler files with `#[derive(Template)]`

## Template

```html
{% extends "base.html" %}

{% block title %}{Title}{% endblock %}

{% block content %}
<div class="container mx-auto px-4 py-8">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-2xl font-bold text-gray-900">{Title}</h1>
    <button
      hx-get="/{resource}/new"
      hx-target="#modal"
      hx-swap="innerHTML"
      class="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700"
    >
      Add {Entity}
    </button>
  </div>

  <div id="{resource}-list">
    {% if items.is_empty() %}
      <p class="text-gray-500">No {items} yet.</p>
    {% else %}
      {% for item in items %}
        {% include "partials/{resource}_row.html" %}
      {% endfor %}
    {% endif %}
  </div>

  <div id="modal"></div>
</div>
{% endblock %}
```

### Template struct (in handler)

```rust
#[derive(askama::Template)]
#[template(path = "{resource}/list.html")]
pub struct {Entity}ListTemplate {
    pub items: Vec<{Entity}>,
    pub user: SessionUser,
}
```

## Example

From `templates/sites/list.html`:

```html
{% extends "base.html" %}
{% block title %}Sites{% endblock %}
{% block content %}
<div class="container mx-auto px-4 py-8">
  <h1 class="text-2xl font-bold mb-6">Monitored Sites</h1>
  <form hx-post="/sites" hx-target="#site-list" hx-swap="beforeend" class="flex gap-4 mb-6">
    <input type="url" name="url" placeholder="https://example.com"
      class="flex-1 rounded-md border px-3 py-2" required />
    <button type="submit" class="rounded-md bg-indigo-600 px-4 py-2 text-white">Add</button>
  </form>
  <div id="site-list">
    {% for site in sites %}
    <div class="flex items-center justify-between p-4 border rounded mb-2">
      <span>{{ site.url }}</span>
      <button hx-delete="/sites/{{ site.id }}" hx-target="closest div" hx-swap="outerHTML"
        class="text-red-600 hover:text-red-800">Delete</button>
    </div>
    {% endfor %}
  </div>
</div>
{% endblock %}
```

## Rules

1. Always extend `base.html` — never write standalone `<html>` pages.
2. Use HTMX for all dynamic interactions — no `<script>` tags.
3. `hx-target` + `hx-swap` control where and how responses are inserted.
4. Return HTML fragments (partials) for HTMX requests, full pages for direct navigation.
5. Tailwind CSS classes for all styling.
6. Use `{% include %}` for reusable row/card partials.
7. Escape user content by default — Askama auto-escapes `{{ }}` expressions.
