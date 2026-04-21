# Content Page Pattern

Static content page (help, about, terms). Template with no data fetching.

## Template

```html
{% extends "base.html" %}
{% block title %}{Title}{% endblock %}
{% block content %}
<article class="prose prose-gray max-w-3xl mx-auto px-4 py-8">
  <h1>{Title}</h1>
  <p>{IntroText}</p>

  <h2>Section One</h2>
  <p>{SectionContent}</p>

  <h2>Section Two</h2>
  <ul>
    <li>{Item1}</li>
    <li>{Item2}</li>
  </ul>
</article>
{% endblock %}
```

### Handler

```rust
#[derive(askama::Template)]
#[template(path = "content/{page}.html")]
pub struct ContentTemplate {
    pub user: Option<SessionUser>,
}

pub async fn content_page(session: Session) -> impl IntoResponse {
    ContentTemplate { user: session.get::<SessionUser>("user").ok().flatten() }
}
```

## Rules

1. Template struct has no data fields beyond auth context.
2. Use Tailwind `prose` class for typographic styling.
3. Can optionally require auth if content is private.
