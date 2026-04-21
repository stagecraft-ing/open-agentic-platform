# Service Pattern

Services encapsulate business logic and database access via SQLx. Handlers
delegate to services — they never execute queries directly.

## Convention

- One file per resource: `src/services/{resource}.rs`
- Functions accept `&PgPool` as the first argument
- Use `sqlx::query_as!` for compile-time checked queries
- Audit trail on every mutation
- Input types defined as structs with `serde::Deserialize`

## Template

```rust
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{Entity};

#[derive(serde::Deserialize)]
pub struct Create{Entity}Input {
    pub {field}: String,
    pub {ref_field}: Uuid,
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<{Entity}>, AppError> {
    let items = sqlx::query_as!(
        {Entity},
        "SELECT id, {field}, {ref_field}, created_at, updated_at FROM {entity_snake} ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(items)
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<{Entity}>, AppError> {
    let item = sqlx::query_as!(
        {Entity},
        "SELECT id, {field}, {ref_field}, created_at, updated_at FROM {entity_snake} WHERE id = $1",
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(item)
}

pub async fn create(pool: &PgPool, input: Create{Entity}Input) -> Result<{Entity}, AppError> {
    let item = sqlx::query_as!(
        {Entity},
        "INSERT INTO {entity_snake} ({field}, {ref_field}) VALUES ($1, $2) RETURNING *",
        input.{field},
        input.{ref_field}
    )
    .fetch_one(pool)
    .await?;

    // Audit trail
    sqlx::query!(
        "INSERT INTO audit_entry (user_id, action_code, entity_type, entity_id) VALUES ($1, $2, $3, $4)",
        auth_user.id, // extracted from JWT claims via AuthUser extractor
        "create_{entity_snake}",
        "{Entity}",
        item.id
    )
    .execute(pool)
    .await?;

    Ok(item)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query!("DELETE FROM {entity_snake} WHERE id = $1", id)
        .execute(pool)
        .await?;

    Ok(())
}
```

## Example

From `src/services/site_service.rs`:

```rust
use sqlx::PgPool;
use crate::{error::AppError, models::Site};

#[derive(serde::Deserialize)]
pub struct CreateSiteInput {
    pub url: String,
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<Site>, AppError> {
    let sites = sqlx::query_as!(Site, "SELECT id, url FROM site ORDER BY id")
        .fetch_all(pool)
        .await?;
    Ok(sites)
}

pub async fn create(pool: &PgPool, input: CreateSiteInput) -> Result<Site, AppError> {
    let site = sqlx::query_as!(
        Site,
        "INSERT INTO site (url) VALUES ($1) RETURNING id, url",
        input.url
    )
    .fetch_one(pool)
    .await?;
    Ok(site)
}

pub async fn delete(pool: &PgPool, id: i32) -> Result<(), AppError> {
    sqlx::query!("DELETE FROM site WHERE id = $1", id)
        .execute(pool)
        .await?;
    Ok(())
}
```

## Rules

1. Always accept `&PgPool` — never create new connections in services.
2. Use `sqlx::query_as!` for SELECT queries — compile-time type checking.
3. Use `sqlx::query!` for INSERT/UPDATE/DELETE — compile-time SQL validation.
4. Never format SQL strings — always use `$1`, `$2` parameter placeholders.
5. Every mutation writes an audit entry (if audit is enabled in Build Spec).
6. Return `Result<T, AppError>` — never unwrap.
7. Input structs derive `serde::Deserialize` for JSON extraction in handlers.
