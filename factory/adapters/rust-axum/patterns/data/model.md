# Model Pattern

Rust model structs represent database rows. They derive `sqlx::FromRow` for
automatic deserialization from query results and `serde::Serialize` for JSON responses.

## Convention

- File: `src/models/{entity}.rs`
- One struct per database table
- Derives: `sqlx::FromRow`, `serde::Serialize`, `serde::Deserialize`, `Debug`, `Clone`
- Field names match database column names via `#[sqlx(rename_all)]`
- Register in `src/models/mod.rs`

## Template

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct {Entity} {
    pub id: Uuid,
    pub {field}: String,
    #[sqlx(rename = "{ref_snake}")]
    pub {ref_field}: Uuid,
    pub status: String,
    #[sqlx(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
}
```

### Module registration (`src/models/mod.rs`)

```rust
pub mod {entity};

pub use {entity}::{Entity};
```

## Type Mapping

| PostgreSQL | Rust | Crate |
|-----------|------|-------|
| UUID | Uuid | uuid |
| TEXT / VARCHAR | String | std |
| INTEGER | i32 | std |
| BIGINT | i64 | std |
| NUMERIC | Decimal | rust_decimal |
| BOOLEAN | bool | std |
| DATE | NaiveDate | chrono |
| TIMESTAMPTZ | DateTime<Utc> | chrono |
| SERIAL | i32 | std |
| ENUM (as TEXT) | String | std |

## Example

From `src/models/site.rs`:

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Site {
    pub id: i32,
    pub url: String,
}
```

From `src/models/check.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Check {
    pub id: i32,
    #[sqlx(rename = "site_id")]
    pub site_id: i32,
    pub up: bool,
    #[sqlx(rename = "checked_at")]
    pub checked_at: DateTime<Utc>,
}
```

## Rules

1. Every model derives `FromRow`, `Serialize`, `Deserialize`, `Debug`, `Clone`.
2. Use `#[sqlx(rename = "...")]` when Rust field names differ from column names.
3. Use `Uuid` from the `uuid` crate for UUID columns.
4. Use `DateTime<Utc>` from `chrono` for timestamp columns.
5. Register every model in `src/models/mod.rs` with `pub mod` and `pub use`.
6. Keep models as plain data structs — no business logic methods.
