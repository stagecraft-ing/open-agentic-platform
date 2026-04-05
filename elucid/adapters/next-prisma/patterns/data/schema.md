# Prisma Schema Pattern

All models are defined in a single `prisma/schema.prisma` file. Each model
maps to a Build Spec entity with type-safe field definitions and relations.

## Convention

- Single file: `prisma/schema.prisma`
- Models use PascalCase; fields use camelCase
- Database columns mapped to snake_case via `@@map` and `@map`
- Enums defined above the models that reference them
- Relations use `@relation` with explicit foreign key fields

## Template

```prisma
generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

// --- Enums ---

enum {Entity}Status {
  {VALUE1}
  {VALUE2}
  {VALUE3}

  @@map("{entity_snake}_status")
}

// --- Models ---

model {Entity} {
  id        String   @id @default(uuid())
  {field}   String   @map("{field_snake}")
  {refField} String  @map("{ref_field_snake}")
  status    {Entity}Status @default({DEFAULT})
  createdAt DateTime @default(now()) @map("created_at")
  updatedAt DateTime @updatedAt @map("updated_at")

  // Relations
  {refEntity} {RefEntity} @relation(fields: [{refField}], references: [id], onDelete: Cascade)
  {children}  {Child}[]

  @@map("{entity_snake}")
  @@index([{refField}])
}
```

## Type Mapping

| Build Spec | Prisma | PostgreSQL |
|-----------|--------|-----------|
| uuid | String @id @default(uuid()) | UUID |
| string | String | TEXT |
| text | String | TEXT |
| integer | Int | INTEGER |
| decimal | Decimal | NUMERIC |
| boolean | Boolean | BOOLEAN |
| date | DateTime @db.Date | DATE |
| timestamp | DateTime | TIMESTAMPTZ |
| enum | enum | TEXT + CHECK (via Prisma enum) |
| reference | String + @relation | UUID REFERENCES |

## Example

```prisma
enum CheckStatus {
  UP
  DOWN

  @@map("check_status")
}

model Site {
  id     Int     @id @default(autoincrement())
  url    String  @unique @db.VarChar(2048)
  checks Check[]

  @@map("site")
}

model Check {
  id        Int      @id @default(autoincrement())
  siteId    Int      @map("site_id")
  up        Boolean
  checkedAt DateTime @default(now()) @map("checked_at")

  site Site @relation(fields: [siteId], references: [id], onDelete: Cascade)

  @@index([siteId])
  @@map("check")
}
```

## Rules

1. All models in a single `prisma/schema.prisma` file.
2. Use `@default(uuid())` for UUID primary keys, `@default(autoincrement())` for integer PKs.
3. Map camelCase fields to snake_case columns with `@map`.
4. Map PascalCase models to snake_case tables with `@@map`.
5. Every relation uses `@relation(fields: [...], references: [...])` explicitly.
6. Include `createdAt` and `updatedAt` on every model.
7. Create `@@index` on all foreign key fields.
