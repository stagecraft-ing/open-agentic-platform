# Drizzle Schema Pattern

## Convention
Define all tables in `api/db/schema.ts` using Drizzle ORM's `pgTable()`. Each table maps to a Build Spec entity. Use Drizzle's type-safe column builders.

## Template
```typescript
import { pgTable, uuid, text, timestamp, boolean, pgEnum } from "drizzle-orm/pg-core";

// Enums
export const {entityEnum} = pgEnum("{entity_snake}_enum", ["{value1}", "{value2}"]);

// Table
export const {entities} = pgTable("{entity_snake}", {
  id: uuid("id").primaryKey().defaultRandom(),
  {field}: text("{field_snake}").notNull(),
  {refField}: uuid("{ref_snake}").references(() => {refTable}.id),
  {enumField}: {entityEnum}("{field_snake}").default("{default}"),
  {boolField}: boolean("{field_snake}").default(false),
  createdAt: timestamp("created_at").defaultNow(),
  updatedAt: timestamp("updated_at").defaultNow(),
});
```

## Type Mapping

| Build Spec | Drizzle Column | TypeScript |
|-----------|---------------|-----------|
| uuid | uuid().defaultRandom() | string |
| string | text() or varchar() | string |
| text | text() | string |
| integer | integer() | number |
| decimal | numeric({precision, scale}) | string |
| boolean | boolean() | boolean |
| date | date() | string |
| timestamp | timestamp() | Date |
| enum | pgEnum() | union type |
| reference | uuid().references() | string |

## Rules
1. All tables defined in a single `api/db/schema.ts` file
2. Use `uuid().primaryKey().defaultRandom()` for primary keys
3. Use `pgEnum()` for enum fields — define the enum before the table
4. Every FK uses `.references(() => table.column)`
5. Include `createdAt` and `updatedAt` on every table
