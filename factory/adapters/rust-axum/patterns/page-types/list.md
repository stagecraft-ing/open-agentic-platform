# List Page Pattern

Data table with HTMX-powered pagination, inline delete, and add form.
Server renders the full table; HTMX replaces the table body on pagination/filter.

## Template

```html
{% extends "base.html" %}
{% block title %}{Entities}{% endblock %}
{% block content %}
<div class="container mx-auto px-4 py-8">
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-2xl font-bold">{Entities}</h1>
    <button hx-get="/{resource}/new" hx-target="#modal" hx-swap="innerHTML"
      class="rounded-md bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-700">
      Add {Entity}
    </button>
  </div>

  <table class="min-w-full divide-y divide-gray-200">
    <thead class="bg-gray-50">
      <tr>
        <th class="px-4 py-3 text-left text-sm font-semibold">{Field}</th>
        <th class="px-4 py-3 text-left text-sm font-semibold">Created</th>
        <th class="px-4 py-3"><span class="sr-only">Actions</span></th>
      </tr>
    </thead>
    <tbody id="{resource}-body" class="divide-y bg-white">
      {% if items.is_empty() %}
        <tr><td colspan="3" class="px-4 py-8 text-center text-gray-500">No {entities} yet.</td></tr>
      {% else %}
        {% for item in items %}
        <tr id="{resource}-{{ item.id }}">
          <td class="px-4 py-3 text-sm">{{ item.{field} }}</td>
          <td class="px-4 py-3 text-sm text-gray-500">{{ item.created_at }}</td>
          <td class="px-4 py-3 text-right">
            <a href="/{resource}/{{ item.id }}" class="text-indigo-600 text-sm mr-3">View</a>
            <button hx-delete="/{resource}/{{ item.id }}" hx-target="#{resource}-{{ item.id }}" hx-swap="outerHTML"
              hx-confirm="Delete this {entity}?"
              class="text-red-600 text-sm">Delete</button>
          </td>
        </tr>
        {% endfor %}
      {% endif %}
    </tbody>
  </table>

  {% if total_pages > 1 %}
  <div class="flex gap-2 mt-4">
    {% for p in 1..=total_pages %}
    <a href="/{resource}?page={{ p }}"
      class="px-3 py-1 rounded {% if p == page %}bg-indigo-600 text-white{% else %}border{% endif %}"
      hx-get="/{resource}?page={{ p }}" hx-target="#{resource}-body" hx-swap="innerHTML"
      hx-push-url="true">{{ p }}</a>
    {% endfor %}
  </div>
  {% endif %}

  <div id="modal"></div>
</div>
{% endblock %}
```

## Rules

1. Table body has an `id` for HTMX targeting.
2. Pagination links use both `href` (fallback) and `hx-get` (HTMX swap).
3. Delete buttons target the individual row for removal.
4. `hx-confirm` for destructive actions.
5. Handle empty table state with a full-width message row.
6. Use `hx-push-url="true"` to update browser URL on pagination.
