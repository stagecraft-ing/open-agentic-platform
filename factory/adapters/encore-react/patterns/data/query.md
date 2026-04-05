# Drizzle Query Pattern

## Convention
Use Drizzle ORM's query builder for all database operations. Import `db` from the service's database declaration and `eq`, `and`, `desc` from `drizzle-orm`.

## Patterns

### Select by ID
```typescript
import { db } from "../db/drizzle";
import { {entities} } from "../db/schema";
import { eq } from "drizzle-orm";

const result = await db.select().from({entities}).where(eq({entities}.id, id));
return result[0] ?? null;
```

### List with Filters
```typescript
import { and, eq, desc } from "drizzle-orm";

const conditions = [];
if (filters.status) conditions.push(eq({entities}.status, filters.status));
if (filters.userId) conditions.push(eq({entities}.userId, filters.userId));

const result = await db
  .select()
  .from({entities})
  .where(conditions.length > 0 ? and(...conditions) : undefined)
  .orderBy(desc({entities}.createdAt))
  .limit(limit)
  .offset(offset);
```

### Insert
```typescript
const result = await db
  .insert({entities})
  .values({
    {field}: input.{field},
    {refField}: input.{refField},
  })
  .returning();
return result[0];
```

### Update
```typescript
const result = await db
  .update({entities})
  .set({
    {field}: input.{field},
    updatedAt: new Date(),
  })
  .where(eq({entities}.id, id))
  .returning();
return result[0] ?? null;
```

### Delete
```typescript
const result = await db
  .delete({entities})
  .where(eq({entities}.id, id))
  .returning();
return result[0] ?? null;
```

### Transaction
```typescript
import { db } from "../db/drizzle";

const result = await db.transaction(async (tx) => {
  const parent = await tx.insert(parents).values({...}).returning();
  await tx.insert(children).values({ parentId: parent[0].id, ... });
  return parent[0];
});
```

## Rules
1. Always use Drizzle query builder — no raw SQL strings
2. Use `.returning()` on insert/update/delete to get the result
3. Use `eq()`, `and()`, `or()` from `drizzle-orm` for conditions
4. Use `.transaction()` for multi-table operations
5. Null check on single-row results: `result[0] ?? null`
