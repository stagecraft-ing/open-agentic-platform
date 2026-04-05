# Template Test Pattern

Template tests verify that Askama templates render correctly with given data.
They compile the template struct and check the HTML output.

## Convention

- Test file: `tests/{resource}_ui_test.rs`
- Instantiate template struct with test data
- Call `.render()` and check the HTML output contains expected content
- Test both populated and empty states

## Template

```rust
use askama::Template;
use {crate_name}::handlers::{resource}::{Entity}ListTemplate;
use {crate_name}::models::{Entity};

#[test]
fn test_{resource}_list_renders_items() {
    let template = {Entity}ListTemplate {
        items: vec![
            {Entity} {
                id: uuid::Uuid::new_v4(),
                {field}: "Test Item".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        ],
        user: test_user(),
    };

    let html = template.render().unwrap();
    assert!(html.contains("Test Item"));
    assert!(html.contains("hx-delete"));
}

#[test]
fn test_{resource}_list_renders_empty_state() {
    let template = {Entity}ListTemplate {
        items: vec![],
        user: test_user(),
    };

    let html = template.render().unwrap();
    assert!(html.contains("No {items}"));
}

fn test_user() -> SessionUser {
    SessionUser {
        id: uuid::Uuid::new_v4(),
        email: "test@example.com".to_string(),
    }
}
```

## Example

```rust
use askama::Template;
use site_monitor::handlers::sites::SiteListTemplate;
use site_monitor::models::Site;

#[test]
fn test_site_list_renders() {
    let template = SiteListTemplate {
        sites: vec![Site { id: 1, url: "https://example.com".to_string() }],
    };
    let html = template.render().unwrap();
    assert!(html.contains("https://example.com"));
}

#[test]
fn test_site_list_empty() {
    let template = SiteListTemplate { sites: vec![] };
    let html = template.render().unwrap();
    assert!(html.contains("No sites"));
}
```

## Rules

1. Templates compile at build time — a render test confirms data binding works.
2. Use `.render().unwrap()` in tests — Askama errors are compile-time.
3. Check for key text content, HTMX attributes, and CSS classes.
4. Test empty states to confirm the `{% if items.is_empty() %}` branch.
5. Use a `test_user()` helper for session context.
