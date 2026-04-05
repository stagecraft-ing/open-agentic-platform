# Prisma Query Pattern

Prisma Client provides a type-safe query API. All database access goes through
the singleton client in `src/lib/db.ts`.

## Convention

- Import from `@/lib/db` — never create new `PrismaClient` instances
- Use Prisma's fluent API: `findMany`, `findUnique`, `create`, `update`, `delete`
- Include relations with `include` or select specific fields with `select`
- Use `where` for filtering, `orderBy` for sorting, `skip`/`take` for pagination

## Template

```ts
import { prisma } from "@/lib/db";

// List with filtering and pagination
const items = await prisma.{entity}.findMany({
  where: {
    status: "ACTIVE",
    {refField}: refId,
  },
  include: {
    {relation}: true,
  },
  orderBy: { createdAt: "desc" },
  skip: (page - 1) * pageSize,
  take: pageSize,
});

// Count for pagination
const total = await prisma.{entity}.count({
  where: { status: "ACTIVE" },
});

// Single record with relations
const item = await prisma.{entity}.findUnique({
  where: { id },
  include: {
    {relation}: { orderBy: { createdAt: "desc" } },
  },
});

// Create with nested relation
const created = await prisma.{entity}.create({
  data: {
    {field}: value,
    {relation}: {
      create: { {childField}: childValue },
    },
  },
  include: { {relation}: true },
});

// Update
const updated = await prisma.{entity}.update({
  where: { id },
  data: { {field}: newValue },
});

// Transaction (multiple operations atomically)
const [item, auditEntry] = await prisma.$transaction([
  prisma.{entity}.create({ data }),
  prisma.auditEntry.create({ data: auditData }),
]);
```

## Example

```ts
// Paginated list of sites with latest check
const sites = await prisma.site.findMany({
  include: {
    checks: { orderBy: { checkedAt: "desc" }, take: 1 },
  },
  orderBy: { id: "asc" },
  skip: (page - 1) * 20,
  take: 20,
});

// Create site with first check
const site = await prisma.site.create({
  data: {
    url: "https://example.com",
    checks: {
      create: { up: true },
    },
  },
  include: { checks: true },
});
```

## Rules

1. Always use the singleton `prisma` from `@/lib/db`.
2. Use `include` sparingly — only load relations you actually render.
3. Use `$transaction` for operations that must be atomic.
4. Use `skip`/`take` for pagination — never load unbounded result sets.
5. Never use `$queryRaw` with string interpolation — use `Prisma.sql` tagged template.
