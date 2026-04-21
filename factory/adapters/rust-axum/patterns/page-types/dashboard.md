# Dashboard Page Pattern

Overview page with metrics. Handler queries aggregates and renders the template.
HTMX polling refreshes metrics periodically.

## Template

```html
{% extends "base.html" %}
{% block title %}Dashboard{% endblock %}
{% block content %}
<div class="container mx-auto px-4 py-8">
  <h1 class="text-2xl font-bold mb-6">Dashboard</h1>

  <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8"
       hx-get="/dashboard/metrics" hx-trigger="every 30s" hx-swap="innerHTML">
    <div class="p-6 border rounded-lg bg-white">
      <p class="text-sm text-gray-500">Total {Entities}</p>
      <p class="text-3xl font-bold">{{ entity_count }}</p>
    </div>
    <div class="p-6 border rounded-lg bg-white">
      <p class="text-sm text-gray-500">Active</p>
      <p class="text-3xl font-bold">{{ active_count }}</p>
    </div>
  </div>

  <h2 class="text-lg font-semibold mb-3">Recent Activity</h2>
  {% if recent.is_empty() %}
    <p class="text-gray-500">No activity yet.</p>
  {% else %}
    {% for item in recent %}
      <div class="py-3 border-b">{{ item.{field} }} — {{ item.created_at }}</div>
    {% endfor %}
  {% endif %}
</div>
{% endblock %}
```

### Handler

```rust
#[derive(askama::Template)]
#[template(path = "dashboard/index.html")]
pub struct DashboardTemplate {
    pub entity_count: i64,
    pub active_count: i64,
    pub recent: Vec<{Entity}>,
    pub user: SessionUser,
}

pub async fn dashboard(State(state): State<AppState>, session: Session) -> Result<impl IntoResponse, AppError> {
    let user = require_auth(&session)?;
    let entity_count = sqlx::query_scalar!("SELECT COUNT(*) FROM {entity_snake}")
        .fetch_one(&state.db).await?;
    let recent = sqlx::query_as!({Entity}, "SELECT * FROM {entity_snake} ORDER BY created_at DESC LIMIT 5")
        .fetch_all(&state.db).await?;
    Ok(DashboardTemplate { entity_count: entity_count.unwrap_or(0), active_count: 0, recent, user })
}
```

## Rules

1. Use `hx-trigger="every 30s"` for auto-refreshing metrics.
2. Query aggregates with `COUNT(*)` — don't load all records.
3. Limit recent activity to 5-10 items.
4. Handle empty states in the template.
