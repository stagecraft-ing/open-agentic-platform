# Test Pattern

Integration tests spin up the Axum app with a test database and make real
HTTP requests. Each test file covers one resource's endpoints.

## Convention

- Test file: `tests/{resource}_test.rs`
- Use `axum::test` helpers or `reqwest` against a spawned server
- Each test gets a clean database state via transaction rollback
- Test both success and error paths

## Template

```rust
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use {crate_name}::app;

async fn setup() -> axum::Router {
    dotenvy::from_filename(".env.test").ok();
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    app(pool)
}

#[tokio::test]
async fn test_list_{resource}_returns_200() {
    let app = setup().await;

    let response = app
        .oneshot(Request::get("/{resource}").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_{resource}_returns_201() {
    let app = setup().await;

    let body = json!({ "{field}": "test value" });
    let response = app
        .oneshot(
            Request::post("/{resource}")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let item: Value = serde_json::from_slice(&body).unwrap();
    assert!(item.get("id").is_some());
}

#[tokio::test]
async fn test_delete_{resource}_returns_204() {
    let app = setup().await;

    let response = app
        .oneshot(
            Request::delete("/{resource}/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
```

## Example

From `tests/sites_test.rs`:

```rust
use axum::{body::Body, http::{Request, StatusCode}};
use serde_json::json;
use tower::ServiceExt;
use site_monitor::app;

#[tokio::test]
async fn test_list_sites() {
    let app = setup().await;
    let resp = app
        .oneshot(Request::get("/sites").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_site() {
    let app = setup().await;
    let body = json!({ "url": "https://example.com" });
    let resp = app
        .oneshot(
            Request::post("/sites")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}
```

## Rules

1. Use `tower::ServiceExt::oneshot()` — send one request without spawning a server.
2. Each test constructs a fresh `Router` via `setup()` — tests are isolated.
3. Run migrations in `setup()` to ensure schema exists.
4. Use `.env.test` for test database URL — never test against production.
5. Test response status codes and body shapes.
6. Test validation errors (missing fields, invalid types) return 400/422.
