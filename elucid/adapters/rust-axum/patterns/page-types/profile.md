# Profile Page Pattern

User profile/settings page. Fetches current user from session, renders
edit form for profile fields.

## Template

```html
{% extends "base.html" %}
{% block title %}Profile{% endblock %}
{% block content %}
<div class="container mx-auto px-4 py-8 max-w-2xl">
  <h1 class="text-2xl font-bold mb-6">Profile Settings</h1>

  <div class="border rounded-lg p-6 mb-6">
    <h2 class="text-lg font-semibold mb-4">Account Information</h2>
    <form hx-put="/profile" hx-target="body" class="space-y-4">
      <input type="hidden" name="csrf_token" value="{{ csrf_token }}" />
      <div>
        <label class="block text-sm font-medium mb-1">Email</label>
        <input type="email" name="email" value="{{ user.email }}"
          class="w-full rounded-md border px-3 py-2" />
      </div>
      <div>
        <label class="block text-sm font-medium mb-1">Name</label>
        <input type="text" name="name" value="{{ user.name }}"
          class="w-full rounded-md border px-3 py-2" />
      </div>
      <button type="submit" class="rounded-md bg-indigo-600 px-4 py-2 text-white">
        Update
      </button>
    </form>
  </div>

  <div class="border rounded-lg p-6">
    <h2 class="text-lg font-semibold mb-2">Session</h2>
    <p class="text-sm text-gray-500">Signed in as {{ user.email }}</p>
    <a href="/auth/signout" class="text-sm text-red-600 hover:text-red-800 mt-2 inline-block">Sign Out</a>
  </div>
</div>
{% endblock %}
```

## Rules

1. Fetch user from session — no user ID in the URL.
2. Use HTMX PUT for profile updates.
3. Include CSRF token in the form.
4. Display session info and sign-out link.
