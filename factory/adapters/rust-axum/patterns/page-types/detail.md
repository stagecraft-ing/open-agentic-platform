# Detail Page Pattern

Single record view. Handler fetches by ID from the URL path parameter.

## Template

```html
{% extends "base.html" %}
{% block title %}{{ item.{field} }}{% endblock %}
{% block content %}
<div class="container mx-auto px-4 py-8">
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-2xl font-bold">{{ item.{field} }}</h1>
    <a href="/{resource}/{{ item.id }}/edit"
      class="rounded-md bg-indigo-600 px-4 py-2 text-sm text-white">Edit</a>
  </div>

  <dl class="grid grid-cols-1 sm:grid-cols-2 gap-4">
    <div class="border rounded-lg p-4">
      <dt class="text-sm font-medium text-gray-500">Status</dt>
      <dd class="mt-1">{{ item.status }}</dd>
    </div>
    <div class="border rounded-lg p-4">
      <dt class="text-sm font-medium text-gray-500">Created</dt>
      <dd class="mt-1">{{ item.created_at }}</dd>
    </div>
  </dl>
</div>
{% endblock %}
```

### Handler

```rust
#[derive(askama::Template)]
#[template(path = "{resource}/detail.html")]
pub struct {Entity}DetailTemplate {
    pub item: {Entity},
    pub user: SessionUser,
}

pub async fn detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    session: Session,
) -> Result<impl IntoResponse, AppError> {
    let user = require_auth(&session)?;
    let item = {resource}_service::find_by_id(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok({Entity}DetailTemplate { item, user })
}
```

## Rules

1. Use `Path(id)` extractor to get the URL parameter.
2. Return `AppError::NotFound` if the record doesn't exist.
3. Use `<dl>` for field/value pairs.
4. Link to edit page for mutations.
