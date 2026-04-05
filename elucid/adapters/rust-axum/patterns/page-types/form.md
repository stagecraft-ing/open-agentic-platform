# Form Page Pattern

Create/edit form submitted via HTMX POST. On success, redirect to the
detail or list page. On validation error, re-render the form with errors.

## Template

```html
{% extends "base.html" %}
{% block title %}Create {Entity}{% endblock %}
{% block content %}
<div class="container mx-auto px-4 py-8 max-w-2xl">
  <h1 class="text-2xl font-bold mb-6">Create {Entity}</h1>

  <form hx-post="/{resource}" hx-target="body" hx-push-url="true" class="space-y-4">
    <input type="hidden" name="csrf_token" value="{{ csrf_token }}" />

    <div>
      <label for="{field}" class="block text-sm font-medium mb-1">{Field}</label>
      <input type="text" id="{field}" name="{field}"
        value="{% if form.is_some() %}{{ form.as_ref().unwrap().{field} }}{% endif %}"
        class="w-full rounded-md border px-3 py-2{% if errors.contains_key(&quot;{field}&quot;) %} border-red-500{% endif %}"
        required />
      {% if let Some(err) = errors.get("{field}") %}
        <p class="text-sm text-red-600 mt-1">{{ err }}</p>
      {% endif %}
    </div>

    <button type="submit"
      class="rounded-md bg-indigo-600 px-4 py-2 text-white hover:bg-indigo-700">
      Create
    </button>
  </form>
</div>
{% endblock %}
```

### Handler (POST)

```rust
pub async fn create(
    State(state): State<AppState>,
    session: Session,
    Form(input): Form<Create{Entity}Input>,
) -> Result<impl IntoResponse, AppError> {
    let user = require_auth(&session)?;

    // Validate
    if let Err(errors) = validate(&input) {
        return Ok({Entity}FormTemplate {
            form: Some(input),
            errors,
            user,
        }.into_response());
    }

    {resource}_service::create(&state.db, input).await?;
    Ok(Redirect::to("/{resource}").into_response())
}
```

## Rules

1. Use `Form<T>` extractor for form data — not `Json<T>`.
2. Include CSRF token as hidden field.
3. On validation error, re-render the form with errors and previous values.
4. On success, redirect to list or detail page.
5. `hx-target="body"` replaces the full page (for redirects).
6. Preserve form values on re-render via the `form` template variable.
