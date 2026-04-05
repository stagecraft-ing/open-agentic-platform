# Router Pattern

Axum routers map HTTP methods and paths to handler functions. Routes are
composed per resource and merged into the main application router.

## Convention

- One file per resource: `src/routes/{resource}.rs`
- Each file exports a `pub fn router() -> Router<AppState>` function
- Routes are composed in `src/routes/mod.rs`
- Use `.route()` for path + method mapping
- Nest resource routers under a path prefix

## Template

```rust
use axum::{
    routing::{get, post, put, delete},
    Router,
};

use crate::handlers::{resource};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{resource}", get({resource}::list).post({resource}::create))
        .route("/{resource}/{:id}", get({resource}::get_by_id)
            .put({resource}::update)
            .delete({resource}::delete))
}
```

### Main router composition (`src/routes/mod.rs`)

```rust
use axum::Router;
use crate::AppState;

mod {resource};
mod auth;

pub fn app_router() -> Router<AppState> {
    Router::new()
        .merge({resource}::router())
        .merge(auth::router())
}
```

## Example

From `src/routes/sites.rs`:

```rust
use axum::{routing::{get, post, delete as del}, Router};
use crate::{handlers::sites, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sites", get(sites::list).post(sites::create))
        .route("/sites/:id", del(sites::delete))
}
```

## Rules

1. Each resource exports a `pub fn router() -> Router<AppState>`.
2. Use `get()`, `post()`, `put()`, `delete()` functions from `axum::routing`.
3. Chain methods on the same path: `.route("/path", get(list).post(create))`.
4. Path parameters use `:param` syntax — Axum extracts them via `Path<T>`.
5. Compose routers in `mod.rs` using `.merge()` — keep the top-level router clean.
6. Auth middleware is applied per-router or globally via `.layer()`.
