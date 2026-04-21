# Landing Page Pattern

Public page — no auth required. Static Askama template with hero section.

## Template

```html
{% extends "base.html" %}
{% block title %}Home{% endblock %}
{% block content %}
<div class="min-h-[80vh] flex flex-col items-center justify-center text-center px-4">
  <h1 class="text-4xl font-bold text-gray-900 mb-4">{Headline}</h1>
  <p class="text-xl text-gray-600 mb-8 max-w-2xl">{Description}</p>
  <a href="/auth/signin" class="rounded-md bg-indigo-600 px-6 py-3 text-white font-medium hover:bg-indigo-700">
    Get Started
  </a>
</div>
{% endblock %}
```

### Handler

```rust
#[derive(askama::Template)]
#[template(path = "landing.html")]
pub struct LandingTemplate;

pub async fn landing() -> impl IntoResponse {
    LandingTemplate
}
```

## Rules

1. No auth check — landing is public.
2. Template struct has no data fields.
3. Include clear call-to-action linking to signin.
