# SQLx Query Pattern

SQLx provides compile-time checked SQL queries via `query!` and `query_as!`
macros. The compiler verifies SQL syntax, column types, and parameter types
against the actual database schema.

## Convention

- Use `sqlx::query_as!` for SELECT queries — maps rows to structs
- Use `sqlx::query!` for INSERT/UPDATE/DELETE — returns affected rows
- Parameter placeholders: `$1`, `$2`, etc.
- All queries in service layer — never in handlers

## Template

```rust
use sqlx::PgPool;
use crate::models::{Entity};

// Fetch all
let items = sqlx::query_as!(
    {Entity},
    "SELECT id, {field}, created_at, updated_at FROM {entity_snake} ORDER BY created_at DESC"
)
.fetch_all(pool)
.await?;

// Fetch one (optional)
let item = sqlx::query_as!(
    {Entity},
    "SELECT id, {field}, created_at, updated_at FROM {entity_snake} WHERE id = $1",
    id
)
.fetch_optional(pool)
.await?;

// Insert returning created row
let created = sqlx::query_as!(
    {Entity},
    "INSERT INTO {entity_snake} ({field}) VALUES ($1) RETURNING id, {field}, created_at, updated_at",
    input.{field}
)
.fetch_one(pool)
.await?;

// Update
sqlx::query!(
    "UPDATE {entity_snake} SET {field} = $1, updated_at = NOW() WHERE id = $2",
    input.{field},
    id
)
.execute(pool)
.await?;

// Delete
sqlx::query!("DELETE FROM {entity_snake} WHERE id = $1", id)
    .execute(pool)
    .await?;

// Count
let count = sqlx::query_scalar!("SELECT COUNT(*) FROM {entity_snake}")
    .fetch_one(pool)
    .await?;

// Paginated query
let items = sqlx::query_as!(
    {Entity},
    "SELECT * FROM {entity_snake} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    page_size as i64,
    ((page - 1) * page_size) as i64
)
.fetch_all(pool)
.await?;
```

## Example

```rust
let sites = sqlx::query_as!(Site, "SELECT id, url FROM site ORDER BY id")
    .fetch_all(pool)
    .await?;

let site = sqlx::query_as!(
    Site,
    "INSERT INTO site (url) VALUES ($1) RETURNING id, url",
    input.url
)
.fetch_one(pool)
.await?;

sqlx::query!("DELETE FROM site WHERE id = $1", id)
    .execute(pool)
    .await?;
```

## Rules

1. Always use `query_as!` or `query!` macros — compile-time checked.
2. Never format SQL strings with `format!()` or string concatenation.
3. Use `$1`, `$2` parameter placeholders — SQLx binds them safely.
4. Use `.fetch_all()` for lists, `.fetch_one()` for single required, `.fetch_optional()` for nullable.
5. Use `RETURNING *` for INSERT/UPDATE when you need the created/updated row.
6. Set `DATABASE_URL` in `.env` for compile-time query checking during `cargo build`.
