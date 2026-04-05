# Handler Pattern

Axum handlers are async functions that accept extractors and return `impl IntoResponse`.
Each handler is a standalone function — no controller class, no method routing in the function.

## Convention

- One file per resource: `src/handlers/{resource}.rs`
- Extractors provide typed access to request data: `Json<T>`, `Path<T>`, `Query<T>`, `State<AppState>`
- Return `impl IntoResponse` — Axum converts `Json<T>`, `Html<String>`, `StatusCode`, etc.
- Errors via `AppError` enum that implements `IntoResponse`
- Auth checked via session extractor

## Template

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{Entity};
use crate::services::{resource}_service;
use crate::AppState;

/// List all {entities}
pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<{Entity}>>, AppError> {
    let items = {resource}_service::find_all(&state.db).await?;
    Ok(Json(items))
}

/// Get a single {entity} by ID
pub async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<{Entity}>, AppError> {
    let item = {resource}_service::find_by_id(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(item))
}

/// Create a new {entity}
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<Create{Entity}Input>,
) -> Result<(StatusCode, Json<{Entity}>), AppError> {
    let item = {resource}_service::create(&state.db, input).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

/// Delete a {entity}
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    {resource}_service::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

## Example

From `src/handlers/sites.rs`:

```rust
use axum::{extract::{Path, State}, http::StatusCode, Json};
use crate::{error::AppError, models::Site, services::site_service, AppState};

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Site>>, AppError> {
    let sites = site_service::find_all(&state.db).await?;
    Ok(Json(sites))
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<site_service::CreateSiteInput>,
) -> Result<(StatusCode, Json<Site>), AppError> {
    let site = site_service::create(&state.db, input).await?;
    Ok((StatusCode::CREATED, Json(site)))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    site_service::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

## Rules

1. Every handler is a standalone `pub async fn` — not a method on a struct.
2. Use extractors for all request data — never parse raw bytes manually.
3. Return `Result<T, AppError>` — never `.unwrap()` or `.expect()` in handlers.
4. Handlers delegate to service layer — no direct SQLx calls.
5. Use `StatusCode::CREATED` for POST success, `StatusCode::NO_CONTENT` for DELETE.
6. `State<AppState>` carries the database pool and shared config.
7. Path parameters use `Path<T>` extractor with typed deserialization.
