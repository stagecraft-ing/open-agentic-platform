# Base Layout Pattern

The base layout provides the HTML shell, navigation, and shared assets.
All page templates extend this layout via Askama template inheritance.

## Convention

- File: `templates/base.html`
- Defines blocks: `title`, `content`, `scripts`
- Includes HTMX, Tailwind CSS, and CSRF meta tag
- Navigation adapts based on auth state

## Template

```html
<!DOCTYPE html>
<html lang="en" class="h-full">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta name="csrf-token" content="{{ csrf_token }}" />
  <title>{% block title %}{AppName}{% endblock %} - {AppName}</title>
  <link rel="stylesheet" href="/static/styles.css" />
  <script src="https://unpkg.com/htmx.org@2.0.4"></script>
</head>
<body class="h-full bg-gray-50" hx-headers='{"X-CSRF-Token": "{{ csrf_token }}"}'>
  <nav class="bg-white border-b border-gray-200">
    <div class="container mx-auto px-4 flex items-center justify-between h-16">
      <a href="/" class="font-bold text-xl text-gray-900">{AppName}</a>
      <div class="flex items-center gap-4">
        {% if user.is_some() %}
          <span class="text-sm text-gray-600">{{ user.as_ref().unwrap().email }}</span>
          <a href="/auth/signout" class="text-sm text-red-600 hover:text-red-800">Sign Out</a>
        {% else %}
          <a href="/auth/signin" class="text-sm text-indigo-600 hover:text-indigo-800">Sign In</a>
        {% endif %}
      </div>
    </div>
  </nav>

  <main>
    {% block content %}{% endblock %}
  </main>

  <div id="toast-container" class="fixed top-4 right-4 z-50"></div>

  {% block scripts %}{% endblock %}
</body>
</html>
```

### Base template struct

```rust
// Shared fields for all page templates
pub struct BaseContext {
    pub csrf_token: String,
    pub user: Option<SessionUser>,
}
```

## Example

From a minimal base layout:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>{% block title %}App{% endblock %}</title>
  <script src="https://unpkg.com/htmx.org@2.0.4"></script>
  <link rel="stylesheet" href="/static/styles.css" />
</head>
<body hx-headers='{"X-CSRF-Token": "{{ csrf_token }}"}'>
  <nav class="border-b px-6 py-4 flex justify-between">
    <a href="/" class="font-bold">Site Monitor</a>
  </nav>
  <main class="p-6">{% block content %}{% endblock %}</main>
</body>
</html>
```

## Rules

1. Include HTMX via CDN `<script>` in the head — no npm/bundler needed.
2. Set `hx-headers` on `<body>` to include CSRF token in all HTMX requests.
3. Define `{% block content %}` — all pages override this.
4. Tailwind CSS served as a pre-built static file (built by standalone CLI).
5. Never inline JavaScript — all interactivity via HTMX attributes.
