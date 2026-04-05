# Login Page Pattern

Sign-in form that creates a session. Form POSTs credentials, handler
verifies with Argon2, creates session, and redirects.

## Template

```html
{% extends "base.html" %}
{% block title %}Sign In{% endblock %}
{% block content %}
<div class="min-h-[80vh] flex items-center justify-center">
  <div class="w-full max-w-sm space-y-6">
    <h1 class="text-2xl font-bold text-center">Sign In</h1>

    {% if let Some(err) = error %}
      <p class="text-sm text-red-600 text-center">{{ err }}</p>
    {% endif %}

    <form method="post" action="/auth/signin" class="space-y-4">
      <input type="hidden" name="csrf_token" value="{{ csrf_token }}" />
      <div>
        <label for="email" class="block text-sm font-medium mb-1">Email</label>
        <input type="email" id="email" name="email" required
          class="w-full rounded-md border px-3 py-2" />
      </div>
      <div>
        <label for="password" class="block text-sm font-medium mb-1">Password</label>
        <input type="password" id="password" name="password" required
          class="w-full rounded-md border px-3 py-2" />
      </div>
      <button type="submit"
        class="w-full rounded-md bg-indigo-600 py-2 text-white hover:bg-indigo-700">
        Sign In
      </button>
    </form>
  </div>
</div>
{% endblock %}
```

### Handler

```rust
pub async fn signin_post(
    State(state): State<AppState>,
    mut session: Session,
    Form(input): Form<SignInInput>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", input.email)
        .fetch_optional(&state.db)
        .await?;

    let user = match user {
        Some(u) if verify_password(&input.password, &u.password_hash)? => u,
        _ => {
            return Ok(SignInTemplate { error: Some("Invalid credentials".into()), csrf_token: "".into() }
                .into_response());
        }
    };

    session.insert("user_id", user.id).await?;
    Ok(Redirect::to("/app").into_response())
}
```

## Rules

1. Use standard HTML `<form method="post">` — no HTMX for auth forms.
2. Include CSRF token as hidden field.
3. Verify password with Argon2 — never compare plaintext.
4. On failure, re-render form with error message.
5. On success, insert user ID into session and redirect.
